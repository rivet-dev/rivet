use std::collections::VecDeque;
use std::ops::{Bound, RangeBounds};
use std::sync::Arc;

use bytes::{BufMut, Bytes, BytesMut};
use chrono::{DateTime, Utc};
use flatbuffers::{
    FlatBufferBuilder, ForwardsUOffset, InvalidFlatbuffer, Vector, VerifierOptions, WIPOffset,
};
use log::warn;
use ulid::Ulid;

use crate::bytes_range::BytesRange;
use crate::checkpoint;
use crate::compactor_state::{
    Compaction as CompactorCompaction, CompactionContext as CompactorCompactionContext,
    CompactionSpec as CompactorCompactionSpec, CompactionStatus,
    Compactions as CompactorCompactions, SourceId,
};
use crate::db_state::{self, FilterFormat, SsTableInfo, SsTableInfoCodec, SstType};
use crate::db_state::{SsTableHandle, SsTableView};
use crate::subcompaction::Subcompaction as CompactorSubcompaction;

#[path = "./generated/root_generated.rs"]
#[allow(warnings, clippy::disallowed_macros, clippy::disallowed_types, clippy::disallowed_methods, unreachable_pub)]
#[rustfmt::skip]
mod root_generated;
pub(crate) use root_generated::{
    BlockMeta, BlockMetaArgs, BlockStats as FbBlockStats, BlockStatsArgs as FbBlockStatsArgs,
    FilterFormat as FbFilterFormat, ManifestV1, ManifestV2, ManifestV2Args, SsTableIndex,
    SsTableIndexArgs, SsTableInfo as FbSsTableInfo, SsTableInfoArgs, SstStats as FbSstStats,
    SstStatsArgs as FbSstStatsArgs,
};

use crate::config::CompressionCodec;
use crate::db_state::SsTableId;
use crate::error::SlateDBError;
use crate::flatbuffer_types::root_generated::{
    BoundType, Checkpoint, CheckpointArgs, CheckpointMetadata, CompactedSsTable,
    CompactedSsTableArgs, CompactedSsTableV2, CompactedSsTableV2Args, CompactedSsTableView,
    CompactedSsTableViewArgs, Compaction as FbCompaction, CompactionArgs as FbCompactionArgs,
    CompactionContext as FbCompactionContext, CompactionSpec as FbCompactionSpec,
    CompactionStatus as FbCompactionStatus, CompactionsV1, CompactionsV1Args, CompressionFormat,
    DrainSegmentSpec, DrainSegmentSpecArgs, Segment as FbSegment, SegmentArgs as FbSegmentArgs,
    SortedRunV2, SortedRunV2Args, SstType as FbSstType, Subcompaction as FbSubcompaction,
    SubcompactionArgs as FbSubcompactionArgs, TieredCompactionContext as FbTieredCompactionContext,
    TieredCompactionContextArgs as FbTieredCompactionContextArgs, TieredCompactionSpec,
    TieredCompactionSpecArgs, Ulid as FbUlid, UlidArgs as FbUlidArgs, Uuid, UuidArgs,
};
// V1-only manifest encoder types; only test fixtures build V1 manifests
#[cfg(test)]
use crate::flatbuffer_types::root_generated::{
    ManifestV1Args, SortedRun as FbSortedRunV1, SortedRunArgs as FbSortedRunV1Args,
};
use crate::format::sst::SST_FORMAT_VERSION;
use crate::manifest::{ExternalDb, LsmTreeState, Manifest, ManifestCore, Segment};
use crate::partitioned_keyspace::RangePartitionedKeySpace;
use crate::seq_tracker::SequenceTracker;
use crate::utils::clamp_allocated_size_bytes;
use slatedb_txn_obj::ObjectCodec;

pub(crate) const MANIFEST_FORMAT_VERSION: u16 = 2;
/// v2 (current) adds the `DrainSegmentSpec` union variant per RFC-0024 and
/// persists the tiered destination explicitly so L0-only compactions round-trip
/// across restart. v1 files have no destination field and infer it from SR
/// inputs on decode.
pub(crate) const COMPACTIONS_FORMAT_VERSION: u16 = 2;
pub(crate) const SUPPORTED_COMPACTIONS_FORMAT_VERSIONS: &[u16] = &[1, 2];
pub(crate) const ORIGINAL_SST_FORMAT_VERSION: u16 = SST_FORMAT_VERSION;

/// FlatBuffer verifier options with increased table limit.
/// The default limit is 1M tables, but with compression enabled, SST files
/// can have many more blocks than the uncompressed file size would suggest.
fn verifier_options() -> VerifierOptions {
    VerifierOptions {
        max_tables: 1_000_000_000,
        ..Default::default()
    }
}

/// A wrapper around a `Bytes` buffer containing a FlatBuffer-encoded `SsTableIndex`.
#[derive(PartialEq, Eq, Clone)]
pub(crate) struct SsTableIndexOwned {
    data: Bytes,
}

impl SsTableIndexOwned {
    pub(crate) fn new(data: Bytes) -> Result<Self, InvalidFlatbuffer> {
        flatbuffers::root_with_opts::<SsTableIndex>(&verifier_options(), &data)?;
        Ok(Self { data })
    }

    pub(crate) fn borrow(&self) -> SsTableIndex<'_> {
        let raw = &self.data;
        unsafe { flatbuffers::root_unchecked::<SsTableIndex>(raw) }
    }

    pub(crate) fn data(&self) -> Bytes {
        self.data.clone()
    }

    pub(crate) fn clamp_allocated_size(&self) -> Self {
        Self::new(clamp_allocated_size_bytes(&self.data))
            .expect("clamped buffer could not be decoded to index")
    }

    /// Returns the size of the SSTable index in bytes.
    pub(crate) fn size(&self) -> usize {
        self.data.len()
    }
}

impl RangePartitionedKeySpace for SsTableIndex<'_> {
    fn partitions(&self) -> usize {
        self.block_meta().len()
    }

    fn partition_first_key(&self, partition: usize) -> &[u8] {
        self.block_meta().get(partition).first_key().bytes()
    }
}

#[derive(Clone)]
pub(crate) struct FlatBufferSsTableInfoCodec {}

impl SsTableInfoCodec for FlatBufferSsTableInfoCodec {
    fn encode(&self, info: &SsTableInfo) -> Bytes {
        Self::create_from_sst_info(info)
    }

    fn decode(&self, bytes: &Bytes) -> Result<SsTableInfo, SlateDBError> {
        let info = flatbuffers::root_with_opts::<FbSsTableInfo>(&verifier_options(), bytes)?;
        Ok(Self::sst_info(&info))
    }

    fn clone_box(&self) -> Box<dyn SsTableInfoCodec> {
        Box::new(Self {})
    }
}

impl FlatBufferSsTableInfoCodec {
    pub(crate) fn sst_info(info: &FbSsTableInfo) -> SsTableInfo {
        let first_entry: Option<Bytes> = info
            .first_entry()
            .map(|entry| Bytes::copy_from_slice(entry.bytes()));
        let last_entry: Option<Bytes> = info
            .last_entry()
            .map(|entry| Bytes::copy_from_slice(entry.bytes()));

        SsTableInfo {
            first_entry,
            last_entry,
            index_offset: info.index_offset(),
            index_len: info.index_len(),
            filter_offset: info.filter_offset(),
            filter_len: info.filter_len(),
            compression_codec: info.compression_format().into(),
            sst_type: info.sst_type().into(),
            stats_offset: info.stats_offset(),
            stats_len: info.stats_len(),
            filter_format: info.filter_format().into(),
        }
    }

    pub(crate) fn create_from_sst_info(info: &SsTableInfo) -> Bytes {
        let builder = FlatBufferBuilder::new();
        let mut db_fb_builder = DbFlatBufferBuilder::new(builder);
        db_fb_builder.create_sst_info(info)
    }
}

pub(crate) struct FlatBufferManifestCodec {}

impl ObjectCodec<Manifest> for FlatBufferManifestCodec {
    fn encode(&self, manifest: &Manifest) -> Bytes {
        // RFC-0004 Phase 2 of the manifest V1->V2 rollout: write V2
        // universally, read V1+V2. V1 read support (and the V1 encoder,
        // retained below for tests) stays until no V1 manifests remain
        Self::create_from_manifest(manifest)
    }

    fn decode(&self, bytes: &Bytes) -> Result<Manifest, Box<dyn std::error::Error + Send + Sync>> {
        if bytes.len() < 2 {
            return Err(Box::new(SlateDBError::EmptyManifest));
        }
        let version = u16::from_be_bytes([bytes[0], bytes[1]]);
        let unversioned_bytes = bytes.slice(2..);
        match version {
            1 => {
                let manifest = flatbuffers::root_with_opts::<ManifestV1>(
                    &verifier_options(),
                    unversioned_bytes.as_ref(),
                )?;
                Self::manifest_v1(&manifest)
            }
            2 => {
                let manifest = flatbuffers::root_with_opts::<ManifestV2>(
                    &verifier_options(),
                    unversioned_bytes.as_ref(),
                )?;
                Self::manifest_v2(&manifest)
            }
            _ => Err(Box::new(SlateDBError::InvalidVersion {
                format_name: "manifest",
                supported_versions: vec![1, 2],
                actual_version: version,
            })),
        }
    }
}

impl FlatBufferManifestCodec {
    fn decode_uuid(uuid: Uuid) -> uuid::Uuid {
        uuid::Uuid::from_u64_pair(uuid.high(), uuid.low())
    }

    fn decode_bytes_range(range: root_generated::BytesRange) -> BytesRange {
        let start_key = Self::decode_bytes_bound(range.start_bound());
        let end_key = Self::decode_bytes_bound(range.end_bound());
        BytesRange::new(start_key, end_key)
    }

    fn decode_bytes_bound(bound: root_generated::BytesBound) -> Bound<Bytes> {
        match (bound.bound_type(), bound.key()) {
            (BoundType::Included, Some(key)) => {
                Bound::Included(Bytes::copy_from_slice(key.bytes()))
            }
            (BoundType::Excluded, Some(key)) => {
                Bound::Excluded(Bytes::copy_from_slice(key.bytes()))
            }
            (BoundType::Unbounded, None) => Bound::Unbounded,
            (bound_type, key) => {
                unreachable!("Unsupported bound type: {:?}, key: {:?}", bound_type, key)
            }
        }
    }

    // ManifestV1 has no view IDs, so we use SST IDs during migration.
    // This also allows to use l0_last_compacted from V1 which is an SST ID rather than SST view ID.
    #[allow(clippy::disallowed_methods)]
    pub(crate) fn manifest_v1(
        manifest: &ManifestV1,
    ) -> Result<Manifest, Box<dyn std::error::Error + Send + Sync>> {
        let l0_last_compacted = manifest.l0_last_compacted().map(|id| id.ulid());
        let mut l0 = VecDeque::new();

        for man_sst in manifest.l0().iter() {
            let sst_ulid = man_sst.id().ulid();
            let format_version = man_sst
                .format_version()
                .unwrap_or(ORIGINAL_SST_FORMAT_VERSION);
            let sst_info = FlatBufferSsTableInfoCodec::sst_info(&man_sst.info());
            let handle = SsTableHandle::new(SsTableId::from(sst_ulid), format_version, sst_info);
            let l0_sst = SsTableView::new_projected(
                sst_ulid,
                handle,
                man_sst.visible_range().map(Self::decode_bytes_range),
            );
            l0.push_back(l0_sst);
        }
        let mut compacted = Vec::new();
        for manifest_sr in manifest.compacted().iter() {
            let mut ssts = Vec::new();
            for manifest_sst in manifest_sr.ssts().iter() {
                let sst_ulid = manifest_sst.id().ulid();
                let format_version = manifest_sst
                    .format_version()
                    .unwrap_or(ORIGINAL_SST_FORMAT_VERSION);
                let info = FlatBufferSsTableInfoCodec::sst_info(&manifest_sst.info());
                let handle = SsTableHandle::new(SsTableId::from(sst_ulid), format_version, info);
                ssts.push(SsTableView::new_projected(
                    sst_ulid,
                    handle,
                    manifest_sst.visible_range().map(Self::decode_bytes_range),
                ));
            }
            compacted.push(db_state::SortedRun::new(manifest_sr.id(), ssts))
        }
        let checkpoints: Vec<checkpoint::Checkpoint> = manifest
            .checkpoints()
            .iter()
            .map(|cp| checkpoint::Checkpoint {
                id: Self::decode_uuid(cp.id()),
                manifest_id: cp.manifest_id(),
                expire_time: match cp.checkpoint_expire_time_s() {
                    0 => None,
                    _ => Some(
                        DateTime::<Utc>::from_timestamp(cp.checkpoint_expire_time_s() as i64, 0)
                            .expect("invalid timestamp"),
                    ),
                },
                create_time: DateTime::<Utc>::from_timestamp(
                    cp.checkpoint_create_time_s() as i64,
                    0,
                )
                .expect("invalid timestamp"),
                name: cp.name().map(|s| s.to_string()),
            })
            .collect();
        let sequence_tracker = match manifest.sequence_tracker() {
            Some(bytes) => SequenceTracker::from_bytes(bytes.bytes())
                .expect("Invalid encoding of sequence tracker in manifest."),
            None => SequenceTracker::new(),
        };
        let external_dbs = manifest
            .external_dbs()
            .map(|external_dbs| {
                external_dbs
                    .iter()
                    .map(|db| ExternalDb {
                        path: db.path().to_string(),
                        source_checkpoint_id: Self::decode_uuid(db.source_checkpoint_id()),
                        final_checkpoint_id: db
                            .final_checkpoint_id()
                            .map(|id| Self::decode_uuid(id)),
                        sst_ids: db
                            .sst_ids()
                            .iter()
                            .map(|id| SsTableId::from(id.ulid()))
                            .collect(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let core = ManifestCore {
            initialized: manifest.initialized(),
            tree: Arc::new(LsmTreeState {
                // In V1, view IDs are SST IDs, so both fields are the same.
                last_compacted_l0_sst_view_id: l0_last_compacted,
                last_compacted_l0_sst_id: l0_last_compacted,
                l0,
                compacted,
            }),
            // Segmentation is V2-only. V1 manifests are always unsegmented.
            segments: vec![],
            segment_extractor_name: None,
            next_wal_sst_id: manifest.wal_id_last_seen() + 1,
            replay_after_wal_id: manifest.replay_after_wal_id(),
            last_l0_seq: manifest.last_l0_seq(),
            last_l0_clock_tick: manifest.last_l0_clock_tick(),
            checkpoints,
            wal_object_store_uri: manifest.wal_object_store_uri().map(|uri| uri.to_string()),
            recent_snapshot_min_seq: manifest.recent_snapshot_min_seq(),
            sequence_tracker,
        };
        Ok(Manifest {
            external_dbs,
            core,
            writer_epoch: manifest.writer_epoch(),
            compactor_epoch: manifest.compactor_epoch(),
        })
    }

    fn decode_compacted_sst_view(
        view: &CompactedSsTableView,
        sst_lookup: &std::collections::HashMap<Ulid, SsTableHandle>,
    ) -> Result<SsTableView, Box<dyn std::error::Error + Send + Sync>> {
        let id = view.id().ulid();
        let ulid = view.sst_id().ulid();
        let handle = sst_lookup
            .get(&ulid)
            .ok_or_else(|| format!("CompactedSsTableView references unknown SST id: {ulid}"))?
            .clone();
        Ok(SsTableView::new_projected(
            id,
            handle,
            view.visible_range().map(Self::decode_bytes_range),
        ))
    }

    pub(crate) fn manifest_v2(
        manifest: &ManifestV2,
    ) -> Result<Manifest, Box<dyn std::error::Error + Send + Sync>> {
        let sst_lookup: std::collections::HashMap<Ulid, SsTableHandle> = manifest
            .ssts()
            .iter()
            .map(|fb_sst| {
                let ulid = fb_sst.id().ulid();
                let handle = FlatBufferCompactionsCodec::compacted_sst_v2(fb_sst);
                (ulid, handle)
            })
            .collect();
        let tree = Self::decode_lsm_tree_v2(
            manifest.last_compacted_l0_sst_view_id(),
            manifest.l0(),
            manifest.compacted(),
            &sst_lookup,
        )?;
        let mut segments = manifest
            .segments()
            .map(|fb_segments| {
                fb_segments
                    .iter()
                    .map(|fb_segment| Self::decode_segment(&fb_segment, &sst_lookup))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();
        // Uphold the `ManifestCore::segments` invariant (sorted by prefix)
        // even if the on-disk order happens to differ.
        segments.sort_by(|a, b| a.prefix.cmp(&b.prefix));
        let segment_extractor_name = manifest.segment_extractor_name().map(|s| s.to_string());
        let checkpoints: Vec<checkpoint::Checkpoint> = manifest
            .checkpoints()
            .iter()
            .map(|cp| checkpoint::Checkpoint {
                id: Self::decode_uuid(cp.id()),
                manifest_id: cp.manifest_id(),
                expire_time: match cp.checkpoint_expire_time_s() {
                    0 => None,
                    _ => Some(
                        DateTime::<Utc>::from_timestamp(cp.checkpoint_expire_time_s() as i64, 0)
                            .expect("invalid timestamp"),
                    ),
                },
                create_time: DateTime::<Utc>::from_timestamp(
                    cp.checkpoint_create_time_s() as i64,
                    0,
                )
                .expect("invalid timestamp"),
                name: cp.name().map(|s| s.to_string()),
            })
            .collect();
        let sequence_tracker = match manifest.sequence_tracker() {
            Some(bytes) => SequenceTracker::from_bytes(bytes.bytes())
                .expect("Invalid encoding of sequence tracker in manifest."),
            None => SequenceTracker::new(),
        };
        let external_dbs = manifest
            .external_dbs()
            .map(|external_dbs| {
                external_dbs
                    .iter()
                    .map(|db| ExternalDb {
                        path: db.path().to_string(),
                        source_checkpoint_id: Self::decode_uuid(db.source_checkpoint_id()),
                        final_checkpoint_id: db
                            .final_checkpoint_id()
                            .map(|id| Self::decode_uuid(id)),
                        sst_ids: db
                            .sst_ids()
                            .iter()
                            .map(|id| SsTableId::from(id.ulid()))
                            .collect(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let core = ManifestCore {
            initialized: manifest.initialized(),
            tree: Arc::new(tree),
            segments,
            segment_extractor_name,
            next_wal_sst_id: manifest.wal_id_last_seen() + 1,
            replay_after_wal_id: manifest.replay_after_wal_id(),
            last_l0_seq: manifest.last_l0_seq(),
            last_l0_clock_tick: manifest.last_l0_clock_tick(),
            checkpoints,
            wal_object_store_uri: None,
            recent_snapshot_min_seq: manifest.recent_snapshot_min_seq(),
            sequence_tracker,
        };
        Ok(Manifest {
            external_dbs,
            core,
            writer_epoch: manifest.writer_epoch(),
            compactor_epoch: manifest.compactor_epoch(),
        })
    }

    fn decode_sorted_runs_v2(
        runs: Vector<'_, ForwardsUOffset<SortedRunV2<'_>>>,
        sst_lookup: &std::collections::HashMap<Ulid, SsTableHandle>,
    ) -> Result<Vec<db_state::SortedRun>, Box<dyn std::error::Error + Send + Sync>> {
        runs.iter()
            .map(
                |sr| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
                    let ssts = sr
                        .ssts()
                        .iter()
                        .map(|view| Self::decode_compacted_sst_view(&view, sst_lookup))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(db_state::SortedRun::new(sr.id(), ssts))
                },
            )
            .collect()
    }

    fn decode_segment(
        fb_segment: &FbSegment,
        sst_lookup: &std::collections::HashMap<Ulid, SsTableHandle>,
    ) -> Result<Segment, Box<dyn std::error::Error + Send + Sync>> {
        let prefix = Bytes::copy_from_slice(fb_segment.prefix().bytes());
        let tree = Self::decode_lsm_tree_v2(
            fb_segment.last_compacted_l0_sst_view_id(),
            fb_segment.l0(),
            fb_segment.compacted(),
            sst_lookup,
        )?;
        Ok(Segment {
            prefix,
            tree: Arc::new(tree),
        })
    }

    /// Decode the V2 wire shape shared by the unsegmented tree (`ManifestV2`)
    /// and each `Segment`: an optional last-compacted view id, an L0 vector,
    /// and a sorted-runs vector.
    fn decode_lsm_tree_v2(
        last_compacted_l0_sst_view_id: Option<FbUlid>,
        l0: Vector<'_, ForwardsUOffset<CompactedSsTableView<'_>>>,
        compacted: Vector<'_, ForwardsUOffset<SortedRunV2<'_>>>,
        sst_lookup: &std::collections::HashMap<Ulid, SsTableHandle>,
    ) -> Result<LsmTreeState, Box<dyn std::error::Error + Send + Sync>> {
        let last_compacted_l0_sst_view_id = last_compacted_l0_sst_view_id.map(|id| id.ulid());
        let l0: VecDeque<SsTableView> = l0
            .iter()
            .map(|view| Self::decode_compacted_sst_view(&view, sst_lookup))
            .collect::<Result<_, _>>()?;
        let compacted = Self::decode_sorted_runs_v2(compacted, sst_lookup)?;
        Ok(LsmTreeState {
            last_compacted_l0_sst_view_id,
            // `last_compacted_l0_sst_id` is V1-only (see LsmTreeState) and has
            // no V2 wire representation, so it stays `None` after decode.
            last_compacted_l0_sst_id: None,
            l0,
            compacted,
        })
    }

    /// Retained for tests that construct V1 manifest fixtures to verify
    /// decode-side backward compatibility; production code always writes
    /// V2 (see [`FlatBufferManifestCodec::encode`]).
    #[cfg(test)]
    pub(crate) fn create_from_manifest_v1(manifest: &Manifest) -> Bytes {
        let builder = FlatBufferBuilder::new();
        let mut db_fb_builder = DbFlatBufferBuilder::new(builder);
        db_fb_builder.create_manifest_v1(manifest)
    }

    pub(crate) fn create_from_manifest(manifest: &Manifest) -> Bytes {
        let builder = FlatBufferBuilder::new();
        let mut db_fb_builder = DbFlatBufferBuilder::new(builder);
        db_fb_builder.create_manifest(manifest)
    }
}

pub(crate) struct FlatBufferCompactionsCodec {}

impl ObjectCodec<CompactorCompactions> for FlatBufferCompactionsCodec {
    fn encode(&self, compactions: &CompactorCompactions) -> Bytes {
        Self::create_from_compactions(compactions)
    }

    fn decode(
        &self,
        bytes: &Bytes,
    ) -> Result<CompactorCompactions, Box<dyn std::error::Error + Send + Sync>> {
        Self::decode_compactions(bytes).map_err(|e| Box::new(e) as _)
    }
}

impl FlatBufferCompactionsCodec {
    pub(crate) fn decode_compactions(bytes: &Bytes) -> Result<CompactorCompactions, SlateDBError> {
        if bytes.len() < 2 {
            return Err(SlateDBError::InvalidCompaction);
        }
        let version = u16::from_be_bytes([bytes[0], bytes[1]]);
        if !SUPPORTED_COMPACTIONS_FORMAT_VERSIONS.contains(&version) {
            return Err(SlateDBError::InvalidVersion {
                format_name: "compactions",
                supported_versions: SUPPORTED_COMPACTIONS_FORMAT_VERSIONS.to_vec(),
                actual_version: version,
            });
        }
        let unversioned_bytes = bytes.slice(2..);
        let fb = flatbuffers::root_with_opts::<CompactionsV1>(
            &verifier_options(),
            unversioned_bytes.as_ref(),
        )?;
        Self::compactions(&fb, version)
    }

    pub(crate) fn compactions(
        compactions: &CompactionsV1,
        version: u16,
    ) -> Result<CompactorCompactions, SlateDBError> {
        let recent_compactions = compactions
            .recent_compactions()
            .iter()
            .map(|c| Self::compaction(&c, version))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CompactorCompactions::new(compactions.compactor_epoch())
            .with_compactions(recent_compactions))
    }

    fn compaction(
        compaction: &FbCompaction,
        version: u16,
    ) -> Result<CompactorCompaction, SlateDBError> {
        let spec = Self::compaction_spec(compaction, version)?;
        let status = CompactionStatus::from(compaction.status());
        let worker = compaction.worker().and_then(Self::worker_spec);
        let mut output_ssts = compaction
            .output_ssts()
            .map(|ssts| ssts.iter().map(Self::compacted_sst).collect())
            .unwrap_or_default();
        // The `ctx` union is schema-extensible, but the only variant we ever
        // write (and the only one the in-memory model (compactor_state.rs) supports) is
        // `TieredCompactionContext`. Any other variant decodes as `None`.
        // If/when we do support different types of compactions, we should extend the in-memory
        // model to define the context as an enum.
        let ctx = match compaction.ctx_type() {
            FbCompactionContext::NONE => None,
            FbCompactionContext::TieredCompactionContext => compaction
                .ctx_as_tiered_compaction_context()
                .map(Self::compaction_context),
            _ => {
                warn!(
                    "unknown compaction context type: {:?}",
                    compaction.ctx_type()
                );
                return Err(SlateDBError::InvalidCompaction);
            }
        };
        if status == CompactionStatus::Running
            || status == CompactionStatus::Scheduled
            || status == CompactionStatus::Submitted
        {
            // When upgrading to 0.14 its possible RUNNING compactions have output ssts in
            // output_ssts instead of ctx. In this case we just drop them and the compaction
            // restarts. We could migrate them to ctx, but opt not to since we don't know
            // how to set retention_min_seq
            output_ssts = vec![];
        }
        Ok(CompactorCompaction::new(compaction.id().ulid(), spec)
            .with_status(status)
            .with_worker(worker)
            .with_output_ssts(output_ssts)
            .with_ctx(ctx))
    }

    fn compaction_context(ctx: FbTieredCompactionContext) -> CompactorCompactionContext {
        let subcompactions = ctx
            .subcompactions()
            .map(|subs| subs.iter().map(Self::subcompaction).collect())
            .unwrap_or_default();
        CompactorCompactionContext::new(subcompactions, Some(ctx.retention_min_seq()))
    }

    fn subcompaction(subcompaction: FbSubcompaction) -> CompactorSubcompaction {
        let range = FlatBufferManifestCodec::decode_bytes_range(subcompaction.range());
        let output_ssts = subcompaction
            .output_ssts()
            .map(|ssts| ssts.iter().map(Self::compacted_sst).collect())
            .unwrap_or_default();
        CompactorSubcompaction::new(range).with_output_ssts(output_ssts)
    }

    fn worker_spec(
        worker: root_generated::WorkerSpec,
    ) -> Option<crate::compactor_state::WorkerSpec> {
        // worker_id is the discriminator for "claimed" vs "unclaimed"; treat a
        // missing string as unclaimed even if the table itself was emitted.
        worker.worker_id().map(|id| {
            crate::compactor_state::WorkerSpec::new(id.to_string(), worker.last_heartbeat_ms())
        })
    }

    fn compaction_spec(
        compaction: &FbCompaction,
        version: u16,
    ) -> Result<CompactorCompactionSpec, SlateDBError> {
        match compaction.spec_type() {
            FbCompactionSpec::TieredCompactionSpec => {
                let Some(spec) = compaction.spec_as_tiered_compaction_spec() else {
                    return Err(SlateDBError::InvalidCompaction);
                };
                let mut sources = Vec::new();
                if let Some(view_ids) = spec.l0_view_ids() {
                    sources.extend(view_ids.iter().map(|id| SourceId::SstView(id.ulid())));
                } else if let Some(ssts) = spec.ssts() {
                    // Backward compat: old specs stored SST ULIDs, not view IDs.
                    // These won't match any view precisely, but this path only
                    // applies to in-flight compactions from before the upgrade.
                    sources.extend(ssts.iter().map(|s| SourceId::SstView(s.ulid())));
                }
                let sorted_runs: Vec<u32> = spec
                    .sorted_runs()
                    .map(|runs| runs.iter().collect())
                    .unwrap_or_default();
                sources.extend(sorted_runs.iter().copied().map(SourceId::SortedRun));
                let destination = if version >= 2 {
                    spec.destination()
                } else {
                    // Legacy v1 files do not encode destination explicitly.
                    sorted_runs.iter().copied().min().unwrap_or(0)
                };
                // Pre-segment specs serialized before this field existed have
                // no segment; treat as the compatibility-encoded `prefix=""`
                // segment (root tree).
                let segment = spec
                    .segment()
                    .map(|s| Bytes::copy_from_slice(s.bytes()))
                    .unwrap_or_default();
                Ok(CompactorCompactionSpec::for_segment(
                    segment,
                    sources,
                    destination,
                ))
            }
            FbCompactionSpec::DrainSegmentSpec => {
                let Some(spec) = compaction.spec_as_drain_segment_spec() else {
                    return Err(SlateDBError::InvalidCompaction);
                };
                let mut sources = Vec::new();
                if let Some(view_ids) = spec.l0_view_ids() {
                    sources.extend(view_ids.iter().map(|id| SourceId::SstView(id.ulid())));
                }
                if let Some(sorted_runs) = spec.sorted_runs() {
                    sources.extend(sorted_runs.iter().map(SourceId::SortedRun));
                }
                let segment = Bytes::copy_from_slice(spec.segment().bytes());
                Ok(CompactorCompactionSpec::drain_segment(segment, sources))
            }
            _ => Err(SlateDBError::InvalidCompaction),
        }
    }

    fn compacted_sst(compacted_sst: CompactedSsTable) -> SsTableHandle {
        let id = SsTableId::from(compacted_sst.id().ulid());
        let format_version = compacted_sst
            .format_version()
            .unwrap_or(ORIGINAL_SST_FORMAT_VERSION);
        let info = FlatBufferSsTableInfoCodec::sst_info(&compacted_sst.info());
        SsTableHandle::new(id, format_version, info)
    }

    fn compacted_sst_v2(compacted_sst: CompactedSsTableV2) -> SsTableHandle {
        let id = SsTableId::from(compacted_sst.id().ulid());
        let format_version = compacted_sst
            .format_version()
            .unwrap_or(ORIGINAL_SST_FORMAT_VERSION);
        let info = FlatBufferSsTableInfoCodec::sst_info(&compacted_sst.info());
        SsTableHandle::new(id, format_version, info)
    }

    pub(crate) fn create_from_compactions(compactions: &CompactorCompactions) -> Bytes {
        let builder = FlatBufferBuilder::new();
        let mut db_fb_builder = DbFlatBufferBuilder::new(builder);
        db_fb_builder.create_compactions(compactions)
    }
}

impl FbUlid<'_> {
    pub(crate) fn ulid(&self) -> Ulid {
        Ulid::from((self.high(), self.low()))
    }
}

struct DbFlatBufferBuilder<'b> {
    builder: FlatBufferBuilder<'b>,
}

/// Offsets for the V2 wire shape shared by the unsegmented tree (`ManifestV2`)
/// and each `Segment`. Produced by `add_lsm_tree_v2`.
struct LsmTreeV2Offsets<'b> {
    l0: WIPOffset<Vector<'b, ForwardsUOffset<CompactedSsTableView<'b>>>>,
    last_compacted_l0_sst_view_id: Option<WIPOffset<FbUlid<'b>>>,
    compacted: WIPOffset<Vector<'b, ForwardsUOffset<SortedRunV2<'b>>>>,
}

impl<'b> DbFlatBufferBuilder<'b> {
    fn new(builder: FlatBufferBuilder<'b>) -> Self {
        Self { builder }
    }

    fn add_sst_info(&mut self, info: &SsTableInfo) -> WIPOffset<FbSsTableInfo<'b>> {
        let first_entry = match info.first_entry.as_ref() {
            None => None,
            Some(first_entry_vector) => Some(self.builder.create_vector(first_entry_vector)),
        };
        let last_entry = match info.last_entry.as_ref() {
            None => None,
            Some(last_entry_vector) => Some(self.builder.create_vector(last_entry_vector)),
        };
        FbSsTableInfo::create(
            &mut self.builder,
            &SsTableInfoArgs {
                first_entry,
                last_entry,
                index_offset: info.index_offset,
                index_len: info.index_len,
                filter_offset: info.filter_offset,
                filter_len: info.filter_len,
                compression_format: info.compression_codec.into(),
                sst_type: info.sst_type.into(),
                stats_offset: info.stats_offset,
                stats_len: info.stats_len,
                filter_format: info.filter_format.into(),
            },
        )
    }

    fn add_ulid(&mut self, ulid: &Ulid) -> WIPOffset<FbUlid<'b>> {
        let uidu128 = ulid.0;
        let high = (uidu128 >> 64) as u64;
        let low = ((uidu128 << 64) >> 64) as u64;
        FbUlid::create(&mut self.builder, &FbUlidArgs { high, low })
    }

    fn add_compacted_sst_id(&mut self, ulid: &Ulid) -> WIPOffset<FbUlid<'b>> {
        self.add_ulid(ulid)
    }

    fn add_compacted_sst_ids<'a, I>(
        &mut self,
        sst_ids: I,
    ) -> WIPOffset<Vector<'b, ForwardsUOffset<FbUlid<'b>>>>
    where
        I: Iterator<Item = &'a SsTableId>,
    {
        let sst_ids: Vec<WIPOffset<FbUlid>> = sst_ids
            .map(|id| id.value())
            .map(|id| self.add_compacted_sst_id(&id))
            .collect();
        self.builder.create_vector(sst_ids.as_ref())
    }

    fn add_ulids<'a, I>(&mut self, ulids: I) -> WIPOffset<Vector<'b, ForwardsUOffset<FbUlid<'b>>>>
    where
        I: Iterator<Item = &'a Ulid>,
    {
        let ulids: Vec<WIPOffset<FbUlid>> = ulids.map(|ulid| self.add_ulid(ulid)).collect();
        self.builder.create_vector(ulids.as_ref())
    }

    fn add_compacted_sst(&mut self, handle: &SsTableHandle) -> WIPOffset<CompactedSsTable<'b>> {
        let ulid = handle.id.value();
        let compacted_sst_id = self.add_compacted_sst_id(&ulid);
        let compacted_sst_info = self.add_sst_info(&handle.info);
        CompactedSsTable::create(
            &mut self.builder,
            &CompactedSsTableArgs {
                id: Some(compacted_sst_id),
                info: Some(compacted_sst_info),
                visible_range: None,
                format_version: Some(handle.format_version),
            },
        )
    }

    fn add_compacted_ssts<'a, I>(
        &mut self,
        ssts: I,
    ) -> WIPOffset<Vector<'b, ForwardsUOffset<CompactedSsTable<'b>>>>
    where
        I: Iterator<Item = &'a SsTableHandle>,
    {
        let compacted_ssts: Vec<WIPOffset<CompactedSsTable>> =
            ssts.map(|sst| self.add_compacted_sst(sst)).collect();
        self.builder.create_vector(compacted_ssts.as_ref())
    }

    /// V1-only; only test fixtures construct V1 manifests now that
    /// `encode()` writes V2 universally (see [`FlatBufferManifestCodec::encode`]).
    #[cfg(test)]
    fn add_compacted_sst_from_view(
        &mut self,
        view: &SsTableView,
    ) -> WIPOffset<CompactedSsTable<'b>> {
        let ulid = view.sst.id.value();
        let compacted_sst_id = self.add_compacted_sst_id(&ulid);
        let compacted_sst_info = self.add_sst_info(&view.sst.info);
        let visible_range = view.visible_range.as_ref().map(|r| self.add_bytes_range(r));
        CompactedSsTable::create(
            &mut self.builder,
            &CompactedSsTableArgs {
                id: Some(compacted_sst_id),
                info: Some(compacted_sst_info),
                visible_range,
                format_version: Some(view.sst.format_version),
            },
        )
    }

    fn add_compacted_sst_v2(
        &mut self,
        handle: &SsTableHandle,
    ) -> WIPOffset<CompactedSsTableV2<'b>> {
        let ulid = handle.id.value();
        let compacted_sst_id = self.add_compacted_sst_id(&ulid);
        let compacted_sst_info = self.add_sst_info(&handle.info);
        CompactedSsTableV2::create(
            &mut self.builder,
            &CompactedSsTableV2Args {
                id: Some(compacted_sst_id),
                info: Some(compacted_sst_info),
                format_version: Some(handle.format_version),
            },
        )
    }

    fn add_compacted_sst_view(
        &mut self,
        view: &SsTableView,
    ) -> WIPOffset<CompactedSsTableView<'b>> {
        let ulid = view.sst.id.value();
        let sst_id = self.add_compacted_sst_id(&ulid);
        let visible_range = view.visible_range.as_ref().map(|r| self.add_bytes_range(r));
        let id = Some(self.add_ulid(&view.id));
        CompactedSsTableView::create(
            &mut self.builder,
            &CompactedSsTableViewArgs {
                sst_id: Some(sst_id),
                visible_range,
                id,
            },
        )
    }

    fn add_compacted_sst_views<'a, I>(
        &mut self,
        ssts: I,
    ) -> WIPOffset<Vector<'b, ForwardsUOffset<CompactedSsTableView<'b>>>>
    where
        I: Iterator<Item = &'a SsTableView>,
    {
        let views: Vec<WIPOffset<CompactedSsTableView>> =
            ssts.map(|sst| self.add_compacted_sst_view(sst)).collect();
        self.builder.create_vector(views.as_ref())
    }

    fn add_sorted_run_v2(
        &mut self,
        sorted_run: &db_state::SortedRun,
    ) -> WIPOffset<SortedRunV2<'b>> {
        let ssts = self.add_compacted_sst_views(sorted_run.sst_views().iter());
        SortedRunV2::create(
            &mut self.builder,
            &SortedRunV2Args {
                id: sorted_run.id,
                ssts: Some(ssts),
            },
        )
    }

    fn add_sorted_runs_v2(
        &mut self,
        sorted_runs: &[db_state::SortedRun],
    ) -> WIPOffset<Vector<'b, ForwardsUOffset<SortedRunV2<'b>>>> {
        let sorted_runs_fbs: Vec<WIPOffset<SortedRunV2>> = sorted_runs
            .iter()
            .map(|sr| self.add_sorted_run_v2(sr))
            .collect();
        self.builder.create_vector(sorted_runs_fbs.as_ref())
    }

    fn add_segment(&mut self, segment: &Segment) -> WIPOffset<FbSegment<'b>> {
        let prefix = self.builder.create_vector(segment.prefix.as_ref());
        let tree = self.add_lsm_tree_v2(&segment.tree);
        FbSegment::create(
            &mut self.builder,
            &FbSegmentArgs {
                prefix: Some(prefix),
                last_compacted_l0_sst_view_id: tree.last_compacted_l0_sst_view_id,
                l0: Some(tree.l0),
                compacted: Some(tree.compacted),
            },
        )
    }

    /// Encode the V2 wire shape shared by the unsegmented tree and each
    /// `Segment`. Returns the three offsets (l0, last-compacted view id,
    /// compacted runs) so the caller can attach them to either a `ManifestV2`
    /// or a `Segment` flatbuffer table.
    fn add_lsm_tree_v2(&mut self, tree: &LsmTreeState) -> LsmTreeV2Offsets<'b> {
        let l0 = self.add_compacted_sst_views(tree.l0.iter());
        let last_compacted_l0_sst_view_id = tree
            .last_compacted_l0_sst_view_id
            .as_ref()
            .map(|ulid| self.add_compacted_sst_id(ulid));
        let compacted = self.add_sorted_runs_v2(&tree.compacted);
        LsmTreeV2Offsets {
            l0,
            last_compacted_l0_sst_view_id,
            compacted,
        }
    }

    fn add_segments(
        &mut self,
        segments: &[Segment],
    ) -> WIPOffset<Vector<'b, ForwardsUOffset<FbSegment<'b>>>> {
        let segment_offsets: Vec<WIPOffset<FbSegment>> =
            segments.iter().map(|s| self.add_segment(s)).collect();
        self.builder.create_vector(segment_offsets.as_ref())
    }

    #[cfg(test)]
    fn add_sorted_run_v1(
        &mut self,
        sorted_run: &db_state::SortedRun,
    ) -> WIPOffset<FbSortedRunV1<'b>> {
        let ssts: Vec<WIPOffset<CompactedSsTable>> = sorted_run
            .sst_views()
            .iter()
            .map(|view| self.add_compacted_sst_from_view(view))
            .collect();
        let ssts = self.builder.create_vector(ssts.as_ref());
        FbSortedRunV1::create(
            &mut self.builder,
            &FbSortedRunV1Args {
                id: sorted_run.id,
                ssts: Some(ssts),
            },
        )
    }

    #[cfg(test)]
    fn add_sorted_runs_v1(
        &mut self,
        sorted_runs: &[db_state::SortedRun],
    ) -> WIPOffset<Vector<'b, ForwardsUOffset<FbSortedRunV1<'b>>>> {
        let sorted_runs_fbs: Vec<WIPOffset<FbSortedRunV1>> = sorted_runs
            .iter()
            .map(|sr| self.add_sorted_run_v1(sr))
            .collect();
        self.builder.create_vector(sorted_runs_fbs.as_ref())
    }

    fn add_compaction_spec(
        &mut self,
        spec: &CompactorCompactionSpec,
    ) -> (FbCompactionSpec, WIPOffset<flatbuffers::UnionWIPOffset>) {
        let mut view_ids = Vec::new();
        let mut sorted_runs = Vec::new();
        for source in spec.sources().iter() {
            match source {
                SourceId::SstView(id) => view_ids.push(*id),
                SourceId::SortedRun(id) => sorted_runs.push(*id),
            }
        }
        let l0_view_ids = (!view_ids.is_empty()).then(|| self.add_ulids(view_ids.iter()));
        let sorted_runs =
            (!sorted_runs.is_empty()).then(|| self.builder.create_vector(sorted_runs.as_slice()));
        let segment = self.builder.create_vector(spec.segment().as_ref());
        match spec {
            CompactorCompactionSpec::Tiered(_) => {
                let tiered_spec = TieredCompactionSpec::create(
                    &mut self.builder,
                    &TieredCompactionSpecArgs {
                        ssts: None,
                        sorted_runs,
                        destination: spec.destination().expect("tiered spec"),
                        l0_view_ids,
                        segment: Some(segment),
                    },
                );
                (
                    FbCompactionSpec::TieredCompactionSpec,
                    tiered_spec.as_union_value(),
                )
            }
            CompactorCompactionSpec::DrainSegment(_) => {
                let drain_spec = DrainSegmentSpec::create(
                    &mut self.builder,
                    &DrainSegmentSpecArgs {
                        segment: Some(segment),
                        l0_view_ids,
                        sorted_runs,
                    },
                );
                (
                    FbCompactionSpec::DrainSegmentSpec,
                    drain_spec.as_union_value(),
                )
            }
        }
    }

    fn add_compaction(&mut self, compaction: &CompactorCompaction) -> WIPOffset<FbCompaction<'b>> {
        let id = self.add_ulid(&compaction.id());
        let (spec_type, spec) = self.add_compaction_spec(compaction.spec());
        let status = FbCompactionStatus::from(compaction.status());
        let worker = compaction.worker().map(|w| self.add_worker_spec(w));
        // The in-memory context always maps onto the `TieredCompactionContext`
        // union variant; the union exists only to keep the schema extensible.
        let (ctx_type, ctx) = match compaction.ctx() {
            Some(ctx) => (
                FbCompactionContext::TieredCompactionContext,
                Some(self.add_tiered_compaction_context(ctx).as_union_value()),
            ),
            None => (FbCompactionContext::NONE, None),
        };
        let output_ssts = self.add_compacted_ssts(compaction.output_ssts().iter());
        FbCompaction::create(
            &mut self.builder,
            &FbCompactionArgs {
                id: Some(id),
                spec_type,
                spec: Some(spec),
                status,
                output_ssts: Some(output_ssts),
                worker,
                ctx_type,
                ctx,
            },
        )
    }

    fn add_tiered_compaction_context(
        &mut self,
        ctx: &CompactorCompactionContext,
    ) -> WIPOffset<FbTieredCompactionContext<'b>> {
        let subcompactions = (!ctx.subcompactions().is_empty())
            .then(|| self.add_subcompactions(ctx.subcompactions().iter()));
        FbTieredCompactionContext::create(
            &mut self.builder,
            &FbTieredCompactionContextArgs {
                subcompactions,
                retention_min_seq: ctx.retention_min_seq().unwrap_or_default(),
            },
        )
    }

    fn add_subcompaction(
        &mut self,
        subcompaction: &CompactorSubcompaction,
    ) -> WIPOffset<FbSubcompaction<'b>> {
        let range = self.add_bytes_range(subcompaction.range());
        let output_ssts = (!subcompaction.output_ssts().is_empty())
            .then(|| self.add_compacted_ssts(subcompaction.output_ssts().iter()));
        FbSubcompaction::create(
            &mut self.builder,
            &FbSubcompactionArgs {
                range: Some(range),
                output_ssts,
            },
        )
    }

    fn add_subcompactions<'a, I>(
        &mut self,
        subcompactions: I,
    ) -> WIPOffset<Vector<'b, ForwardsUOffset<FbSubcompaction<'b>>>>
    where
        I: Iterator<Item = &'a CompactorSubcompaction>,
    {
        let subcompactions: Vec<WIPOffset<FbSubcompaction>> = subcompactions
            .map(|sub| self.add_subcompaction(sub))
            .collect();
        self.builder.create_vector(subcompactions.as_ref())
    }

    fn add_worker_spec(
        &mut self,
        worker: &crate::compactor_state::WorkerSpec,
    ) -> WIPOffset<root_generated::WorkerSpec<'b>> {
        let worker_id = self.builder.create_string(&worker.worker_id);
        root_generated::WorkerSpec::create(
            &mut self.builder,
            &root_generated::WorkerSpecArgs {
                worker_id: Some(worker_id),
                last_heartbeat_ms: worker.last_heartbeat_ms,
            },
        )
    }

    fn add_compactions<'a, I>(
        &mut self,
        compactions: I,
    ) -> WIPOffset<Vector<'b, ForwardsUOffset<FbCompaction<'b>>>>
    where
        I: Iterator<Item = &'a CompactorCompaction>,
    {
        let compactions: Vec<WIPOffset<FbCompaction>> =
            compactions.map(|c| self.add_compaction(c)).collect();
        self.builder.create_vector(compactions.as_ref())
    }

    fn add_uuid(&mut self, uuid: uuid::Uuid) -> WIPOffset<Uuid<'b>> {
        let (high, low) = uuid.as_u64_pair();
        Uuid::create(&mut self.builder, &UuidArgs { high, low })
    }

    fn add_checkpoint(&mut self, checkpoint: &checkpoint::Checkpoint) -> WIPOffset<Checkpoint<'b>> {
        let id = self.add_uuid(checkpoint.id);
        let checkpoint_expire_time_s =
            checkpoint.expire_time.map(|t| t.timestamp()).unwrap_or(0) as u32;
        let checkpoint_create_time_s = checkpoint.create_time.timestamp() as u32;
        let name = checkpoint
            .name
            .as_ref()
            .map(|n| self.builder.create_string(n));
        Checkpoint::create(
            &mut self.builder,
            &CheckpointArgs {
                id: Some(id),
                manifest_id: checkpoint.manifest_id,
                checkpoint_expire_time_s,
                checkpoint_create_time_s,
                metadata: None,
                metadata_type: CheckpointMetadata::NONE,
                name,
            },
        )
    }

    fn add_checkpoints(
        &mut self,
        checkpoints: &[checkpoint::Checkpoint],
    ) -> WIPOffset<Vector<'b, ForwardsUOffset<Checkpoint<'b>>>> {
        let checkpoints_fb_vec: Vec<WIPOffset<Checkpoint>> =
            checkpoints.iter().map(|c| self.add_checkpoint(c)).collect();
        self.builder.create_vector(checkpoints_fb_vec.as_ref())
    }

    fn add_bytes_range(&mut self, range: &BytesRange) -> WIPOffset<root_generated::BytesRange<'b>> {
        let start_bound = self.add_bytes_bound(range.start_bound());
        let end_bound = self.add_bytes_bound(range.end_bound());
        root_generated::BytesRange::create(
            &mut self.builder,
            &root_generated::BytesRangeArgs {
                start_bound: Some(start_bound),
                end_bound: Some(end_bound),
            },
        )
    }

    fn add_bytes_bound(
        &mut self,
        bound: Bound<&Bytes>,
    ) -> WIPOffset<root_generated::BytesBound<'b>> {
        let (bound_type, key) = match bound {
            Bound::Included(key) => (BoundType::Included, Some(self.builder.create_vector(key))),
            Bound::Excluded(key) => (BoundType::Excluded, Some(self.builder.create_vector(key))),
            Bound::Unbounded => (BoundType::Unbounded, None),
        };
        root_generated::BytesBound::create(
            &mut self.builder,
            &root_generated::BytesBoundArgs { key, bound_type },
        )
    }

    fn create_compactions(&mut self, compactions: &CompactorCompactions) -> Bytes {
        let compactions_fb = self.add_compactions(compactions.iter());
        let compactions = CompactionsV1::create(
            &mut self.builder,
            &CompactionsV1Args {
                compactor_epoch: compactions.compactor_epoch,
                recent_compactions: Some(compactions_fb),
            },
        );
        self.builder.finish(compactions, None);
        let mut bytes = BytesMut::new();
        bytes.put_u16(COMPACTIONS_FORMAT_VERSION);
        bytes.put_slice(self.builder.finished_data());
        bytes.into()
    }

    fn create_manifest(&mut self, manifest: &Manifest) -> Bytes {
        let core = &manifest.core;

        // Collect all unique SSTs from l0, compacted runs, and segments.
        let mut unique_ssts: std::collections::BTreeMap<Ulid, &SsTableHandle> =
            std::collections::BTreeMap::new();
        for tree in core.trees() {
            for view in tree.l0.iter() {
                unique_ssts.entry(view.sst.id.value()).or_insert(&view.sst);
            }
            for sr in tree.compacted.iter() {
                for view in sr.sst_views() {
                    unique_ssts.entry(view.sst.id.value()).or_insert(&view.sst);
                }
            }
        }
        let ssts = {
            let sst_offsets: Vec<WIPOffset<CompactedSsTableV2>> = unique_ssts
                .values()
                .copied()
                .map(|handle| self.add_compacted_sst_v2(handle))
                .collect();
            self.builder.create_vector(sst_offsets.as_ref())
        };

        let tree = self.add_lsm_tree_v2(&core.tree);
        let segments = self.add_segments(&core.segments);
        let segment_extractor_name = core
            .segment_extractor_name
            .as_ref()
            .map(|name| self.builder.create_string(name));
        let checkpoints = self.add_checkpoints(&core.checkpoints);
        let external_dbs = if manifest.external_dbs.is_empty() {
            None
        } else {
            let external_dbs: Vec<WIPOffset<root_generated::ExternalDb>> = manifest
                .external_dbs
                .iter()
                .map(|external_db| {
                    let db_external_db_args = root_generated::ExternalDbArgs {
                        path: Some(self.builder.create_string(&external_db.path)),
                        source_checkpoint_id: Some(self.add_uuid(external_db.source_checkpoint_id)),
                        final_checkpoint_id: external_db
                            .final_checkpoint_id
                            .map(|id| self.add_uuid(id)),
                        sst_ids: Some(self.add_compacted_sst_ids(external_db.sst_ids.iter())),
                    };
                    root_generated::ExternalDb::create(&mut self.builder, &db_external_db_args)
                })
                .collect();
            Some(self.builder.create_vector(external_dbs.as_ref()))
        };

        let sequence_tracker_data = core.sequence_tracker.to_bytes();
        let sequence_tracker = self.builder.create_vector(sequence_tracker_data.as_slice());

        let manifest = ManifestV2::create(
            &mut self.builder,
            &ManifestV2Args {
                manifest_id: 0, // todo: get rid of me
                external_dbs,
                initialized: core.initialized,
                writer_epoch: manifest.writer_epoch,
                compactor_epoch: manifest.compactor_epoch,
                replay_after_wal_id: core.replay_after_wal_id,
                wal_id_last_seen: core.next_wal_sst_id - 1,
                last_compacted_l0_sst_view_id: tree.last_compacted_l0_sst_view_id,
                ssts: Some(ssts),
                l0: Some(tree.l0),
                compacted: Some(tree.compacted),
                last_l0_clock_tick: core.last_l0_clock_tick,
                checkpoints: Some(checkpoints),
                last_l0_seq: core.last_l0_seq,
                recent_snapshot_min_seq: core.recent_snapshot_min_seq,
                sequence_tracker: Some(sequence_tracker),
                segments: Some(segments),
                segment_extractor_name,
            },
        );
        self.builder.finish(manifest, None);
        let mut bytes = BytesMut::new();
        bytes.put_u16(MANIFEST_FORMAT_VERSION);
        bytes.put_slice(self.builder.finished_data());
        bytes.into()
    }

    #[cfg(test)]
    fn create_manifest_v1(&mut self, manifest: &Manifest) -> Bytes {
        let core = &manifest.core;

        let l0: Vec<WIPOffset<CompactedSsTable>> = core
            .tree
            .l0
            .iter()
            .map(|view| self.add_compacted_sst_from_view(view))
            .collect();
        let l0 = self.builder.create_vector(l0.as_ref());

        // V1's l0_last_compacted is an SST ID, not a view ID.
        let mut l0_last_compacted = None;
        if let Some(sst_id) = core.tree.last_compacted_l0_sst_id.as_ref() {
            l0_last_compacted = Some(self.add_compacted_sst_id(sst_id))
        }
        let compacted = self.add_sorted_runs_v1(&core.tree.compacted);
        let checkpoints = self.add_checkpoints(&core.checkpoints);
        let external_dbs = if manifest.external_dbs.is_empty() {
            None
        } else {
            let external_dbs: Vec<WIPOffset<root_generated::ExternalDb>> = manifest
                .external_dbs
                .iter()
                .map(|external_db| {
                    let db_external_db_args = root_generated::ExternalDbArgs {
                        path: Some(self.builder.create_string(&external_db.path)),
                        source_checkpoint_id: Some(self.add_uuid(external_db.source_checkpoint_id)),
                        final_checkpoint_id: external_db
                            .final_checkpoint_id
                            .map(|id| self.add_uuid(id)),
                        sst_ids: Some(self.add_compacted_sst_ids(external_db.sst_ids.iter())),
                    };
                    root_generated::ExternalDb::create(&mut self.builder, &db_external_db_args)
                })
                .collect();
            Some(self.builder.create_vector(external_dbs.as_ref()))
        };

        let wal_object_store_uri = core
            .wal_object_store_uri
            .as_ref()
            .map(|uri| self.builder.create_string(uri));
        let sequence_tracker_data = core.sequence_tracker.to_bytes();
        let sequence_tracker = self.builder.create_vector(sequence_tracker_data.as_slice());

        let manifest = ManifestV1::create(
            &mut self.builder,
            &ManifestV1Args {
                manifest_id: 0,
                external_dbs,
                initialized: core.initialized,
                writer_epoch: manifest.writer_epoch,
                compactor_epoch: manifest.compactor_epoch,
                replay_after_wal_id: core.replay_after_wal_id,
                wal_id_last_seen: core.next_wal_sst_id - 1,
                l0_last_compacted,
                l0: Some(l0),
                compacted: Some(compacted),
                last_l0_clock_tick: core.last_l0_clock_tick,
                checkpoints: Some(checkpoints),
                last_l0_seq: core.last_l0_seq,
                wal_object_store_uri,
                recent_snapshot_min_seq: core.recent_snapshot_min_seq,
                sequence_tracker: Some(sequence_tracker),
            },
        );
        self.builder.finish(manifest, None);
        let mut bytes = BytesMut::new();
        bytes.put_u16(1);
        bytes.put_slice(self.builder.finished_data());
        bytes.into()
    }

    fn create_sst_info(&mut self, info: &SsTableInfo) -> Bytes {
        let copy = self.add_sst_info(info);
        self.builder.finish(copy, None);
        Bytes::copy_from_slice(self.builder.finished_data())
    }
}

impl From<Option<CompressionCodec>> for CompressionFormat {
    fn from(value: Option<CompressionCodec>) -> Self {
        match value {
            None => CompressionFormat::None,
            Some(codec) => match codec {
                #[cfg(feature = "snappy")]
                CompressionCodec::Snappy => CompressionFormat::Snappy,
                #[cfg(feature = "lz4")]
                CompressionCodec::Lz4 => CompressionFormat::Lz4,
                #[cfg(feature = "zlib")]
                CompressionCodec::Zlib => CompressionFormat::Zlib,
                #[cfg(feature = "zstd")]
                CompressionCodec::Zstd => CompressionFormat::Zstd,
            },
        }
    }
}

impl From<SstType> for FbSstType {
    fn from(value: SstType) -> Self {
        match value {
            SstType::Compacted => FbSstType::Compacted,
            SstType::Wal => FbSstType::Wal,
        }
    }
}

impl From<FbSstType> for SstType {
    fn from(value: FbSstType) -> Self {
        match value {
            FbSstType::Compacted => SstType::Compacted,
            FbSstType::Wal => SstType::Wal,
            _ => unreachable!("unknown SstType value: {:?}", value),
        }
    }
}

impl From<FilterFormat> for FbFilterFormat {
    fn from(value: FilterFormat) -> Self {
        match value {
            FilterFormat::Legacy => FbFilterFormat::Legacy,
            FilterFormat::Composite => FbFilterFormat::Composite,
        }
    }
}

impl From<FbFilterFormat> for FilterFormat {
    fn from(value: FbFilterFormat) -> Self {
        match value {
            FbFilterFormat::Legacy => FilterFormat::Legacy,
            FbFilterFormat::Composite => FilterFormat::Composite,
            _ => unreachable!("unknown FilterFormat value: {:?}", value),
        }
    }
}

impl From<CompressionFormat> for Option<CompressionCodec> {
    fn from(value: CompressionFormat) -> Self {
        match value {
            #[cfg(feature = "snappy")]
            CompressionFormat::Snappy => Some(CompressionCodec::Snappy),
            #[cfg(feature = "lz4")]
            CompressionFormat::Lz4 => Some(CompressionCodec::Lz4),
            #[cfg(feature = "zlib")]
            CompressionFormat::Zlib => Some(CompressionCodec::Zlib),
            #[cfg(feature = "zstd")]
            CompressionFormat::Zstd => Some(CompressionCodec::Zstd),
            _ => None,
        }
    }
}

impl From<CompactionStatus> for FbCompactionStatus {
    fn from(value: CompactionStatus) -> Self {
        match value {
            CompactionStatus::Submitted => FbCompactionStatus::Submitted,
            CompactionStatus::Scheduled => FbCompactionStatus::Scheduled,
            CompactionStatus::Running => FbCompactionStatus::Running,
            CompactionStatus::Compacted => FbCompactionStatus::Compacted,
            CompactionStatus::Completed => FbCompactionStatus::Completed,
            CompactionStatus::Failed => FbCompactionStatus::Failed,
        }
    }
}

impl From<FbCompactionStatus> for CompactionStatus {
    fn from(value: FbCompactionStatus) -> Self {
        match value {
            FbCompactionStatus::Submitted => CompactionStatus::Submitted,
            FbCompactionStatus::Scheduled => CompactionStatus::Scheduled,
            FbCompactionStatus::Running => CompactionStatus::Running,
            FbCompactionStatus::Compacted => CompactionStatus::Compacted,
            FbCompactionStatus::Completed => CompactionStatus::Completed,
            FbCompactionStatus::Failed => CompactionStatus::Failed,
            _ => CompactionStatus::Submitted,
        }
    }
}

#[cfg(test)]
pub(crate) mod test_utils {
    use crate::flatbuffer_types::SsTableIndexOwned;

    pub(crate) fn assert_index_clamped(index1: &SsTableIndexOwned, index2: &SsTableIndexOwned) {
        assert_eq!(index1.data, index2.data);
        assert_ne!(index1.data.as_ptr(), index2.data.as_ptr());
    }
}

#[cfg(test)]
mod tests {
    use crate::bytes_range::BytesRange;
    use crate::compactor_state::{
        Compaction, CompactionContext, CompactionSpec, CompactionStatus, Compactions, SourceId,
    };
    use crate::db_state::{SortedRun, SsTableHandle, SsTableId, SsTableInfo, SsTableView, SstType};
    use crate::flatbuffer_types::{
        FlatBufferCompactionsCodec, FlatBufferManifestCodec, SsTableIndexOwned,
    };
    use crate::manifest::{ExternalDb, LsmTreeState, Manifest, ManifestCore, Segment};
    use crate::subcompaction::Subcompaction;
    use crate::{checkpoint, error::SlateDBError};
    use rstest::rstest;
    use slatedb_txn_obj::ObjectCodec;
    use std::collections::VecDeque;
    use std::sync::Arc;

    use crate::compactor_state::CompactionStatus as CompactorCompactionStatus;
    use crate::flatbuffer_types::test_utils::assert_index_clamped;
    use crate::format::sst::{SsTableFormat, SST_FORMAT_VERSION_LATEST};
    use crate::test_utils::build_test_sst;
    use bytes::{BufMut, Bytes, BytesMut};
    use chrono::{DateTime, Utc};

    use super::root_generated::CompactionStatus as FbCompactionStatus;
    use super::{root_generated, COMPACTIONS_FORMAT_VERSION, MANIFEST_FORMAT_VERSION};

    #[test]
    fn test_should_encode_decode_manifest_checkpoints() {
        // given:
        let mut core = ManifestCore::new();
        core.checkpoints = vec![
            checkpoint::Checkpoint {
                id: uuid::Uuid::new_v4(),
                manifest_id: 1,
                expire_time: None,
                create_time: DateTime::<Utc>::from_timestamp(100, 0).expect("invalid timestamp"),
                name: None,
            },
            checkpoint::Checkpoint {
                id: uuid::Uuid::new_v4(),
                manifest_id: 2,
                expire_time: Some(
                    DateTime::<Utc>::from_timestamp(1000, 0).expect("invalid timestamp"),
                ),
                create_time: DateTime::<Utc>::from_timestamp(200, 0).expect("invalid timestamp"),
                name: Some("test_checkpoint".to_string()),
            },
        ];
        let manifest = Manifest::initial(core);
        let codec = FlatBufferManifestCodec {};

        // when:
        let bytes = codec.encode(&manifest);
        let decoded = codec.decode(&bytes).expect("failed to decode manifest");

        // then:
        assert_eq!(manifest, decoded);
    }

    #[test]
    fn test_should_encode_decode_external_dbs() {
        // given:
        let mut manifest = Manifest::initial(ManifestCore::new());
        manifest.external_dbs = vec![
            ExternalDb {
                path: "/path/to/external/first".to_string(),
                source_checkpoint_id: uuid::Uuid::new_v4(),
                final_checkpoint_id: Some(uuid::Uuid::new_v4()),
                sst_ids: vec![
                    SsTableId::from(ulid::Ulid::new()),
                    SsTableId::from(ulid::Ulid::new()),
                ],
            },
            ExternalDb {
                path: "/path/to/external/second".to_string(),
                source_checkpoint_id: uuid::Uuid::new_v4(),
                final_checkpoint_id: Some(uuid::Uuid::new_v4()),
                sst_ids: vec![SsTableId::from(ulid::Ulid::new())],
            },
        ];
        let codec = FlatBufferManifestCodec {};

        // when:
        let bytes = codec.encode(&manifest);
        let decoded = codec.decode(&bytes).expect("failed to decode manifest");

        // then:
        assert_eq!(manifest, decoded);
    }

    #[test]
    fn test_should_encode_decode_ssts_with_visible_ranges() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        fn new_sst_handle(first_entry: &[u8], visible_range: Option<BytesRange>) -> SsTableView {
            let ulid = ulid::Ulid::from_parts(COUNTER.fetch_add(1, Ordering::Relaxed), 0);
            SsTableView::new_projected(
                ulid,
                SsTableHandle::new(
                    SsTableId::from(ulid),
                    SST_FORMAT_VERSION_LATEST,
                    SsTableInfo {
                        first_entry: Some(Bytes::copy_from_slice(first_entry)),
                        ..Default::default()
                    },
                ),
                visible_range,
            )
        }

        // given:
        let mut manifest = Manifest::initial(ManifestCore::new());
        Arc::make_mut(&mut manifest.core.tree).l0 = VecDeque::from(vec![
            new_sst_handle(b"a", None),
            new_sst_handle(b"a", Some(BytesRange::from_ref("c"..="d"))),
        ]);
        Arc::make_mut(&mut manifest.core.tree).compacted = vec![
            SortedRun::new(
                0,
                [
                    new_sst_handle(b"a", None),
                    new_sst_handle(b"d", Some(BytesRange::from_ref("e".."f"))),
                ],
            ),
            SortedRun::new(
                0,
                [
                    new_sst_handle(b"a", None),
                    new_sst_handle(b"c", Some(BytesRange::from_ref("c"..))),
                    new_sst_handle(b"d", Some(BytesRange::from_ref("e".."f"))),
                ],
            ),
        ];

        let codec = FlatBufferManifestCodec {};

        // when:
        let bytes = codec.encode(&manifest);
        let decoded = codec.decode(&bytes).expect("failed to decode manifest");

        // then:
        assert_eq!(manifest, decoded);
    }

    #[test]
    fn test_should_encode_manifest_v2_deterministically() {
        fn new_sst_view(index: u64) -> SsTableView {
            let ulid = ulid::Ulid::from_parts(index, u128::from(index * 17));
            SsTableView::identity(SsTableHandle::new(
                SsTableId::from(ulid),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo {
                    first_entry: Some(Bytes::from(format!("first-{index:03}"))),
                    last_entry: Some(Bytes::from(format!("last-{index:03}"))),
                    index_offset: index * 100,
                    index_len: 17 + index,
                    filter_offset: 2000 + index,
                    filter_len: 11 + index,
                    ..Default::default()
                },
            ))
        }

        let mut manifest = Manifest::initial(ManifestCore::new());
        let root = Arc::make_mut(&mut manifest.core.tree);
        root.l0 = (0..16).map(new_sst_view).collect();
        root.compacted = (0..4)
            .map(|run| {
                SortedRun::new(
                    run as u32,
                    (0..8).map(|offset| new_sst_view(100 + run * 8 + offset)),
                )
            })
            .collect();
        manifest.core.segment_extractor_name = Some("test".to_string());
        manifest.core.segments = (0..4)
            .map(|segment| {
                let tree = LsmTreeState {
                    l0: (0..4)
                        .map(|offset| new_sst_view(1_000 + segment * 16 + offset))
                        .collect(),
                    compacted: vec![SortedRun::new(
                        100 + segment as u32,
                        (0..4).map(|offset| new_sst_view(2_000 + segment * 16 + offset)),
                    )],
                    ..Default::default()
                };
                Segment {
                    prefix: Bytes::from(format!("segment-{segment}")),
                    tree: Arc::new(tree),
                }
            })
            .collect();

        let expected = FlatBufferManifestCodec::create_from_manifest(&manifest);
        for _ in 0..64 {
            assert_eq!(
                expected,
                FlatBufferManifestCodec::create_from_manifest(&manifest)
            );
        }
    }

    #[tokio::test]
    async fn test_should_set_compacted_sst_type_for_regular_builder() {
        // given/when:
        let format = SsTableFormat::default();
        let sst = build_test_sst(&format, 1).await;

        // then:
        assert_eq!(sst.info.sst_type, SstType::Compacted);
    }

    #[tokio::test]
    async fn test_should_clamp_index_alloc() {
        let format = SsTableFormat::default();
        let sst = build_test_sst(&format, 3).await;
        let data = sst.remaining_as_bytes();
        let start_off = sst.info.index_offset as usize;
        let end_off = sst.info.index_offset as usize + sst.info.index_len as usize;
        let index_bytes = data.slice(start_off..end_off);
        let index = SsTableIndexOwned::new(index_bytes).unwrap();

        let clamped = index.clamp_allocated_size();

        assert_index_clamped(&clamped, &index);
    }

    #[test]
    fn test_should_validate_manifest_version() {
        let codec = FlatBufferManifestCodec {};

        // Create a valid V1 manifest
        let mut fbb = flatbuffers::FlatBufferBuilder::new();
        let l0 = fbb.create_vector::<flatbuffers::WIPOffset<_>>(&[]);
        let compacted = fbb.create_vector::<flatbuffers::WIPOffset<_>>(&[]);
        let checkpoints = fbb.create_vector::<flatbuffers::WIPOffset<_>>(&[]);

        let manifest = root_generated::ManifestV1::create(
            &mut fbb,
            &root_generated::ManifestV1Args {
                manifest_id: 0,
                external_dbs: None,
                initialized: false,
                writer_epoch: 0,
                compactor_epoch: 0,
                replay_after_wal_id: 0,
                wal_id_last_seen: 0,
                l0_last_compacted: None,
                l0: Some(l0),
                compacted: Some(compacted),
                checkpoints: Some(checkpoints),
                last_l0_clock_tick: 0,
                last_l0_seq: 0,
                wal_object_store_uri: None,
                recent_snapshot_min_seq: 0,
                sequence_tracker: None,
            },
        );
        fbb.finish(manifest, None);
        let fb_data = fbb.finished_data();

        // Test V1 decodes successfully with version byte 1
        let mut bytes = BytesMut::with_capacity(2 + fb_data.len());
        bytes.put_u16(1);
        bytes.put_slice(fb_data);
        let v1_bytes = bytes.freeze();
        codec.decode(&v1_bytes).expect("Should decode V1 manifest");

        // Test encode/decode round-trip (writes V2 universally, per RFC-0004
        // Phase 2 of the manifest V1->V2 rollout)
        let manifest = Manifest::initial(ManifestCore::new());
        let encoded = codec.encode(&manifest);
        assert_eq!(
            u16::from_be_bytes([encoded[0], encoded[1]]),
            MANIFEST_FORMAT_VERSION
        );
        codec
            .decode(&encoded)
            .expect("Should decode manifest round-trip");

        // Test unsupported version
        let mut bytes = BytesMut::with_capacity(2 + fb_data.len());
        bytes.put_u16(MANIFEST_FORMAT_VERSION + 1);
        bytes.put_slice(fb_data);
        let invalid_bytes = bytes.freeze();

        match codec.decode(&invalid_bytes) {
            Err(err) => {
                let Some(SlateDBError::InvalidVersion {
                    format_name,
                    supported_versions,
                    actual_version,
                }) = err.downcast_ref()
                else {
                    panic!("Expected SlateDBError::InvalidVersion but got {:?}", err);
                };
                assert_eq!(*format_name, "manifest");
                assert_eq!(*supported_versions, vec![1u16, 2]);
                assert_eq!(*actual_version, MANIFEST_FORMAT_VERSION + 1);
            }
            _ => panic!("Should fail with version mismatch"),
        }
    }

    #[test]
    fn test_should_encode_decode_wal_object_store_uri_in_v1() {
        let mut manifest = Manifest::initial(ManifestCore::new());
        manifest.core.wal_object_store_uri = Some("s3://bucket/path".to_string());

        let codec = FlatBufferManifestCodec {};
        let bytes = FlatBufferManifestCodec::create_from_manifest_v1(&manifest);
        let decoded = codec.decode(&bytes).unwrap();

        assert_eq!(manifest, decoded);
    }

    #[test]
    fn test_should_not_encode_wal_object_store_uri_in_v2() {
        let mut manifest = Manifest::initial(ManifestCore::new());
        manifest.core.wal_object_store_uri = Some("s3://bucket/path".to_string());

        let codec = FlatBufferManifestCodec {};
        let bytes = FlatBufferManifestCodec::create_from_manifest(&manifest);
        let decoded = codec.decode(&bytes).unwrap();

        assert_eq!(
            u16::from_be_bytes([bytes[0], bytes[1]]),
            MANIFEST_FORMAT_VERSION
        );

        let mut expected = manifest.clone();
        expected.core.wal_object_store_uri = None;
        assert_eq!(expected, decoded);
    }

    #[test]
    fn test_should_round_trip_segments_in_v2() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        fn new_sst_view() -> SsTableView {
            let ulid = ulid::Ulid::from_parts(COUNTER.fetch_add(1, Ordering::Relaxed), 0);
            SsTableView::new_projected(
                ulid,
                SsTableHandle::new(
                    SsTableId::from(ulid),
                    SST_FORMAT_VERSION_LATEST,
                    SsTableInfo {
                        first_entry: Some(Bytes::from_static(b"a")),
                        ..Default::default()
                    },
                ),
                None,
            )
        }

        // given: a manifest with two named segments, each carrying its own L0
        // and sorted run state.
        let mut manifest = Manifest::initial(ManifestCore::new());
        manifest.core.segment_extractor_name = Some("hour-bucket".to_string());
        // Constructed in sorted-by-prefix order to match the
        // `ManifestCore::segments` invariant.
        manifest.core.segments = vec![
            Segment {
                prefix: Bytes::from_static(b"hour=11/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::new(),
                    compacted: vec![SortedRun::new(0, [new_sst_view(), new_sst_view()])],
                }),
            },
            Segment {
                prefix: Bytes::from_static(b"hour=12/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::from(vec![new_sst_view(), new_sst_view()]),
                    compacted: vec![SortedRun::new(1, [new_sst_view()])],
                }),
            },
        ];

        let codec = FlatBufferManifestCodec {};

        // when: encoded via the V2 encoder (the V1 encoder does not carry
        // segment state).
        let bytes = FlatBufferManifestCodec::create_from_manifest(&manifest);
        let decoded = codec.decode(&bytes).expect("failed to decode manifest");

        // then: the manifest round-trips on the V2 wire.
        assert_eq!(
            u16::from_be_bytes([bytes[0], bytes[1]]),
            MANIFEST_FORMAT_VERSION
        );
        assert_eq!(manifest, decoded);
    }

    #[test]
    fn test_should_round_trip_empty_segments_via_v2() {
        // V2 with no segmented state still round-trips: segments empty,
        // extractor name absent.
        let manifest = Manifest::initial(ManifestCore::new());
        let codec = FlatBufferManifestCodec {};

        let bytes = FlatBufferManifestCodec::create_from_manifest(&manifest);
        let decoded = codec.decode(&bytes).expect("failed to decode manifest");

        assert!(decoded.core.segments.is_empty());
        assert!(decoded.core.segment_extractor_name.is_none());
    }

    #[test]
    fn test_should_encode_decode_retention_min_seq() {
        let mut manifest = Manifest::initial(ManifestCore::new());
        manifest.core.last_l0_seq = 11111;
        manifest.core.recent_snapshot_min_seq = 12345;

        let codec = FlatBufferManifestCodec {};
        let bytes = codec.encode(&manifest);
        let decoded = codec.decode(&bytes).unwrap();

        assert_eq!(decoded.core.recent_snapshot_min_seq, 12345);

        // Test None case
        let mut manifest_none = Manifest::initial(ManifestCore::new());
        manifest_none.core.recent_snapshot_min_seq = 0;

        let bytes_none = codec.encode(&manifest_none);
        let decoded_none = codec.decode(&bytes_none).unwrap();

        assert_eq!(decoded_none.core.recent_snapshot_min_seq, 0);
    }

    #[test]
    fn test_should_encode_decode_compactions() {
        fn new_output_sst(first_key: &[u8]) -> SsTableHandle {
            SsTableHandle::new(
                SsTableId::from(ulid::Ulid::new()),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo {
                    first_entry: Some(Bytes::copy_from_slice(first_key)),
                    ..Default::default()
                },
            )
        }

        let output_ssts = vec![new_output_sst(b"a"), new_output_sst(b"m")];
        let compaction_l0 = Compaction::new(
            ulid::Ulid::new(),
            CompactionSpec::new(vec![SourceId::SstView(ulid::Ulid::new())], 11),
        )
        .with_status(CompactionStatus::Running)
        .with_ctx(Some(CompactionContext::new(
            vec![Subcompaction::new(BytesRange::unbounded()).with_output_ssts(output_ssts)],
            Some(0),
        )));
        let compaction_sr = Compaction::new(
            ulid::Ulid::new(),
            CompactionSpec::new(vec![SourceId::SortedRun(5), SourceId::SortedRun(3)], 3),
        )
        .with_status(CompactionStatus::Failed);
        // Targets a named segment to exercise the non-empty `segment` path
        // through encode/decode.
        let segment_prefix = Bytes::from_static(b"hour=12/");
        let compaction_segment = Compaction::new(
            ulid::Ulid::new(),
            CompactionSpec::for_segment(
                segment_prefix.clone(),
                vec![SourceId::SortedRun(8), SourceId::SortedRun(6)],
                6,
            ),
        )
        .with_status(CompactionStatus::Running);
        // Drain spec exercises the new `DrainSegmentSpec` union variant
        // (RFC-0024). The codec must round-trip both spec kinds.
        let drain_prefix = Bytes::from_static(b"hour=10/");
        let compaction_drain = Compaction::new(
            ulid::Ulid::new(),
            CompactionSpec::drain_segment(
                drain_prefix.clone(),
                vec![SourceId::SstView(ulid::Ulid::new()), SourceId::SortedRun(4)],
            ),
        )
        .with_status(CompactionStatus::Submitted);
        let mut compactions = Compactions::new(9);
        compactions.insert(compaction_l0.clone());
        compactions.insert(compaction_sr.clone());
        compactions.insert(compaction_segment.clone());
        compactions.insert(compaction_drain.clone());

        let codec = FlatBufferCompactionsCodec {};
        let bytes = codec.encode(&compactions);
        let decoded = codec.decode(&bytes).expect("failed to decode compactions");

        assert_eq!(decoded.compactor_epoch, compactions.compactor_epoch);
        assert_eq!(
            decoded
                .get(&compaction_l0.id())
                .expect("missing l0 compaction"),
            &compaction_l0
        );
        assert_eq!(
            decoded
                .get(&compaction_sr.id())
                .expect("missing sr compaction"),
            &compaction_sr
        );
        let decoded_segment = decoded
            .get(&compaction_segment.id())
            .expect("missing segment compaction");
        assert_eq!(decoded_segment, &compaction_segment);
        assert_eq!(decoded_segment.spec().segment(), &segment_prefix);
        let decoded_drain = decoded
            .get(&compaction_drain.id())
            .expect("missing drain compaction");
        assert_eq!(decoded_drain, &compaction_drain);
        assert!(decoded_drain.spec().is_drain());
        assert_eq!(decoded_drain.spec().segment(), &drain_prefix);
        assert_eq!(decoded_drain.spec().destination(), None);
    }

    #[test]
    fn test_should_encode_decode_compaction_with_subcompactions() {
        fn new_output_sst(first_key: &[u8]) -> SsTableHandle {
            SsTableHandle::new(
                SsTableId::from(ulid::Ulid::new()),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo {
                    first_entry: Some(Bytes::copy_from_slice(first_key)),
                    ..Default::default()
                },
            )
        }

        // given: a compaction exercising all subcompaction range shapes
        // (unbounded edges and a bounded middle range), including ranges with
        // and without output SSTs
        let subcompactions = vec![
            Subcompaction::new(BytesRange::new(
                std::ops::Bound::Unbounded,
                std::ops::Bound::Excluded(Bytes::from_static(b"g")),
            ))
            .with_output_ssts(vec![new_output_sst(b"a"), new_output_sst(b"d")]),
            Subcompaction::new(BytesRange::new(
                std::ops::Bound::Included(Bytes::from_static(b"g")),
                std::ops::Bound::Excluded(Bytes::from_static(b"q")),
            ))
            .with_output_ssts(vec![new_output_sst(b"g")]),
            Subcompaction::new(BytesRange::new(
                std::ops::Bound::Included(Bytes::from_static(b"q")),
                std::ops::Bound::Unbounded,
            )),
        ];
        let compaction = Compaction::new(
            ulid::Ulid::new(),
            CompactionSpec::new(vec![SourceId::SortedRun(5), SourceId::SortedRun(3)], 3),
        )
        .with_status(CompactionStatus::Running)
        .with_ctx(Some(CompactionContext::new(subcompactions, Some(0))));
        let mut compactions = Compactions::new(1);
        compactions.insert(compaction.clone());

        // when: the compactions are encoded and decoded
        let codec = FlatBufferCompactionsCodec {};
        let bytes = codec.encode(&compactions);
        let decoded = codec.decode(&bytes).expect("failed to decode compactions");

        // then: the decoded compaction matches the original
        let decoded_compaction = decoded.get(&compaction.id()).expect("missing compaction");
        assert_eq!(decoded_compaction, &compaction);
    }

    #[test]
    fn test_should_roundtrip_v2_l0_only_compaction_destination() {
        // Regression guard for issue #1652: an L0-only tiered compaction has
        // no SR inputs, so the pre-fix decoder inferred destination=0 and
        // dropped any real destination N>0 across restart. With the explicit
        // `destination` field on the v2 wire format, the destination must
        // round-trip through encode/decode.
        let compaction = Compaction::new(
            ulid::Ulid::new(),
            CompactionSpec::new(vec![SourceId::SstView(ulid::Ulid::new())], 7),
        );
        let mut compactions = Compactions::new(1);
        compactions.insert(compaction.clone());

        let codec = FlatBufferCompactionsCodec {};
        let bytes = codec.encode(&compactions);
        let decoded = codec.decode(&bytes).expect("failed to decode compactions");

        let decoded_compaction = decoded.get(&compaction.id()).expect("missing compaction");
        assert_eq!(decoded_compaction.spec().destination(), Some(7));
    }

    #[test]
    fn test_should_round_trip_compaction_statuses() {
        let statuses = [
            CompactionStatus::Submitted,
            CompactionStatus::Running,
            CompactionStatus::Compacted,
            CompactionStatus::Completed,
            CompactionStatus::Failed,
        ];

        for status in statuses {
            let fb_status = FbCompactionStatus::from(status);
            let round_trip = CompactionStatus::from(fb_status);
            assert_eq!(round_trip, status);
        }
    }

    #[test]
    fn test_should_validate_compactions_version() {
        let codec = FlatBufferCompactionsCodec {};
        let compactions = Compactions::new(1);
        let bytes = codec.encode(&compactions);

        let invalid_version = COMPACTIONS_FORMAT_VERSION + 1;
        let mut invalid_bytes = bytes.to_vec();
        invalid_bytes[0] = (invalid_version >> 8) as u8;
        invalid_bytes[1] = invalid_version as u8;

        let result = codec.decode(&Bytes::from(invalid_bytes));

        match result {
            Err(err) => {
                let Some(SlateDBError::InvalidVersion {
                    format_name,
                    supported_versions,
                    actual_version,
                }) = err.downcast_ref()
                else {
                    panic!("Expected SlateDBError::InvalidVersion but got {:?}", err);
                };
                assert_eq!(*format_name, "compactions");
                assert_eq!(
                    *supported_versions,
                    super::SUPPORTED_COMPACTIONS_FORMAT_VERSIONS.to_vec()
                );
                assert_eq!(*actual_version, invalid_version);
            }
            _ => panic!("Should fail with version mismatch"),
        }
    }

    #[test]
    fn test_should_round_trip_sst_type_wal() {
        // given:
        use crate::db_state::SsTableInfoCodec;
        let codec = super::FlatBufferSsTableInfoCodec {};
        let info = SsTableInfo {
            first_entry: Some(Bytes::from_static(b"key1")),
            sst_type: SstType::Wal,
            ..Default::default()
        };

        // when:
        let bytes = codec.encode(&info);
        let decoded = codec.decode(&bytes).unwrap();

        // then:
        assert_eq!(decoded.sst_type, SstType::Wal);
        assert_eq!(decoded, info);
    }

    #[test]
    fn test_should_round_trip_sst_type_compacted() {
        // given:
        use crate::db_state::SsTableInfoCodec;
        let codec = super::FlatBufferSsTableInfoCodec {};
        let info = SsTableInfo {
            first_entry: Some(Bytes::from_static(b"key1")),
            sst_type: SstType::Compacted,
            ..Default::default()
        };

        // when:
        let bytes = codec.encode(&info);
        let decoded = codec.decode(&bytes).unwrap();

        // then:
        assert_eq!(decoded.sst_type, SstType::Compacted);
        assert_eq!(decoded, info);
    }

    #[test]
    fn test_should_default_sst_type_to_compacted_for_missing_field() {
        // given: manually build an SsTableInfo FlatBuffer without calling add_sst_type,
        // simulating a legacy SST that was written before the sst_type field existed
        use super::root_generated::SsTableInfoBuilder;
        use crate::db_state::SsTableInfoCodec;

        let mut fbb = flatbuffers::FlatBufferBuilder::new();
        let first_entry = fbb.create_vector(b"key1");
        let mut builder = SsTableInfoBuilder::new(&mut fbb);
        builder.add_first_entry(first_entry);
        // deliberately not calling builder.add_sst_type(...)
        let info = builder.finish();
        fbb.finish(info, None);
        let bytes = Bytes::copy_from_slice(fbb.finished_data());

        // when:
        let codec = super::FlatBufferSsTableInfoCodec {};
        let decoded = codec.decode(&bytes).unwrap();

        // then:
        assert_eq!(decoded.sst_type, SstType::Compacted);
    }

    #[test]
    fn test_should_round_trip_filter_format_composite() {
        use crate::db_state::{FilterFormat, SsTableInfoCodec};
        let codec = super::FlatBufferSsTableInfoCodec {};
        let info = SsTableInfo {
            first_entry: Some(Bytes::from_static(b"key1")),
            filter_format: FilterFormat::Composite,
            ..Default::default()
        };

        let bytes = codec.encode(&info);
        let decoded = codec.decode(&bytes).unwrap();

        assert_eq!(decoded.filter_format, FilterFormat::Composite);
        assert_eq!(decoded, info);
    }

    #[test]
    fn test_should_default_filter_format_to_legacy_for_missing_field() {
        use super::root_generated::SsTableInfoBuilder;
        use crate::db_state::{FilterFormat, SsTableInfoCodec};

        let mut fbb = flatbuffers::FlatBufferBuilder::new();
        let first_entry = fbb.create_vector(b"key1");
        let mut builder = SsTableInfoBuilder::new(&mut fbb);
        builder.add_first_entry(first_entry);
        // deliberately not calling builder.add_filter_format(...)
        let info = builder.finish();
        fbb.finish(info, None);
        let bytes = Bytes::copy_from_slice(fbb.finished_data());

        let codec = super::FlatBufferSsTableInfoCodec {};
        let decoded = codec.decode(&bytes).unwrap();

        assert_eq!(decoded.filter_format, FilterFormat::Legacy);
    }

    #[test]
    fn test_should_encode_decode_manifest_sst_with_version_set() {
        // given: a manifest with one L0 SST and one sorted run SST
        let mut manifest = Manifest::initial(ManifestCore::new());
        Arc::make_mut(&mut manifest.core.tree).l0 =
            VecDeque::from(vec![SsTableView::identity(SsTableHandle::new(
                SsTableId::from(ulid::Ulid::new()),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo {
                    first_entry: Some(Bytes::from_static(b"l0key")),
                    ..Default::default()
                },
            ))]);
        Arc::make_mut(&mut manifest.core.tree).compacted = vec![SortedRun::new(
            1,
            [SsTableView::identity(SsTableHandle::new(
                SsTableId::from(ulid::Ulid::new()),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo {
                    first_entry: Some(Bytes::from_static(b"srkey")),
                    ..Default::default()
                },
            ))],
        )];
        let codec = FlatBufferManifestCodec {};

        // when:
        let bytes = codec.encode(&manifest);
        let decoded = codec.decode(&bytes).expect("failed to decode manifest");

        // then:
        assert_eq!(
            decoded.core.tree.l0[0].sst.format_version,
            SST_FORMAT_VERSION_LATEST
        );
        assert_eq!(
            decoded.core.tree.compacted[0].sst_views()[0]
                .sst
                .format_version,
            SST_FORMAT_VERSION_LATEST
        );
        assert_eq!(manifest, decoded);
    }

    #[test]
    fn test_should_decode_sst_with_no_version_set_using_original_version() {
        // given: manually build a manifest flatbuffer without setting format_version
        // on CompactedSsTable entries, simulating a legacy manifest
        use super::root_generated::{
            CompactedSsTable, CompactedSsTableArgs, ManifestV1, ManifestV1Args,
            SortedRun as FbSortedRun, SortedRunArgs, SsTableInfo as FbSsTableInfo, SsTableInfoArgs,
        };
        let mut fbb = flatbuffers::FlatBufferBuilder::new();
        // Build an L0 SST without format_version
        let first_entry = fbb.create_vector(b"l0key");
        let l0_info = FbSsTableInfo::create(
            &mut fbb,
            &SsTableInfoArgs {
                first_entry: Some(first_entry),
                ..Default::default()
            },
        );
        let l0_ulid = ulid::Ulid::new();
        let l0_id = root_generated::Ulid::create(
            &mut fbb,
            &root_generated::UlidArgs {
                high: (l0_ulid.0 >> 64) as u64,
                low: ((l0_ulid.0 << 64) >> 64) as u64,
            },
        );
        let l0_sst = CompactedSsTable::create(
            &mut fbb,
            &CompactedSsTableArgs {
                id: Some(l0_id),
                info: Some(l0_info),
                visible_range: None,
                format_version: None, // deliberately not set
            },
        );
        let l0_vec = fbb.create_vector(&[l0_sst]);
        // Build a sorted run SST without format_version
        let sr_first_entry = fbb.create_vector(b"srkey");
        let sr_info = FbSsTableInfo::create(
            &mut fbb,
            &SsTableInfoArgs {
                first_entry: Some(sr_first_entry),
                ..Default::default()
            },
        );
        let sr_ulid = ulid::Ulid::new();
        let sr_id = root_generated::Ulid::create(
            &mut fbb,
            &root_generated::UlidArgs {
                high: (sr_ulid.0 >> 64) as u64,
                low: ((sr_ulid.0 << 64) >> 64) as u64,
            },
        );
        let sr_sst = CompactedSsTable::create(
            &mut fbb,
            &CompactedSsTableArgs {
                id: Some(sr_id),
                info: Some(sr_info),
                visible_range: None,
                format_version: None, // deliberately not set
            },
        );
        let sr_ssts_vec = fbb.create_vector(&[sr_sst]);
        let sorted_run = FbSortedRun::create(
            &mut fbb,
            &SortedRunArgs {
                id: 1,
                ssts: Some(sr_ssts_vec),
            },
        );
        let compacted_vec = fbb.create_vector(&[sorted_run]);
        let checkpoints_vec =
            fbb.create_vector::<flatbuffers::ForwardsUOffset<root_generated::Checkpoint>>(&[]);
        let manifest = ManifestV1::create(
            &mut fbb,
            &ManifestV1Args {
                l0: Some(l0_vec),
                compacted: Some(compacted_vec),
                checkpoints: Some(checkpoints_vec),
                ..Default::default()
            },
        );
        fbb.finish(manifest, None);
        let mut bytes = BytesMut::new();
        bytes.put_u16(1); // V1 format
        bytes.put_slice(fbb.finished_data());
        let bytes = bytes.freeze();

        // when:
        let codec = FlatBufferManifestCodec {};
        let decoded = codec.decode(&bytes).expect("failed to decode manifest");

        // then: format_version should default to ORIGINAL_SST_FORMAT_VERSION
        assert_eq!(
            decoded.core.tree.l0[0].sst.format_version,
            super::ORIGINAL_SST_FORMAT_VERSION
        );
        assert_eq!(
            decoded.core.tree.compacted[0].sst_views()[0]
                .sst
                .format_version,
            super::ORIGINAL_SST_FORMAT_VERSION
        );
    }

    #[test]
    fn test_should_encode_decode_compaction_output_sst_with_version_set() {
        // given: a compaction with one output SST
        let output_sst = SsTableHandle::new(
            SsTableId::from(ulid::Ulid::new()),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo {
                first_entry: Some(Bytes::from_static(b"key1")),
                ..Default::default()
            },
        );
        let compaction = Compaction::new(
            ulid::Ulid::new(),
            CompactionSpec::new(vec![SourceId::SstView(ulid::Ulid::new())], 0),
        )
        .with_status(CompactionStatus::Running)
        .with_ctx(Some(CompactionContext::new(
            vec![Subcompaction::new(BytesRange::unbounded()).with_output_ssts(vec![output_sst])],
            Some(0),
        )));
        let mut compactions = Compactions::new(1);
        compactions.insert(compaction.clone());
        let codec = FlatBufferCompactionsCodec {};

        // when:
        let bytes = codec.encode(&compactions);
        let decoded = codec.decode(&bytes).expect("failed to decode compactions");

        // then:
        let decoded_compaction = decoded.get(&compaction.id()).expect("missing compaction");
        assert_eq!(
            decoded_compaction.subcompactions()[0].output_ssts()[0].format_version,
            SST_FORMAT_VERSION_LATEST
        );
        assert_eq!(decoded_compaction, &compaction);
    }

    #[rstest]
    #[case(FbCompactionStatus::Submitted, CompactorCompactionStatus::Submitted)]
    #[case(FbCompactionStatus::Scheduled, CompactorCompactionStatus::Scheduled)]
    #[case(FbCompactionStatus::Running, CompactorCompactionStatus::Running)]
    fn test_should_clear_legacy_top_level_output_ssts_on_pre_compacted_states(
        #[case] fb_status: FbCompactionStatus,
        #[case] status: CompactorCompactionStatus,
    ) {
        // given: manually build a compactions flatbuffer whose intermediate output is recorded
        // only in the top-level `output_ssts` field, with no `ctx` (a file written by a
        // version predating RFC-0028 per-range output)
        use super::root_generated::{
            CompactedSsTable, CompactedSsTableArgs, Compaction as FbCompaction,
            CompactionArgs as FbCompactionArgs, CompactionSpec as FbCompactionSpec, CompactionsV1,
            CompactionsV1Args, SsTableInfo as FbSsTableInfo, SsTableInfoArgs, TieredCompactionSpec,
            TieredCompactionSpecArgs,
        };
        let mut fbb = flatbuffers::FlatBufferBuilder::new();
        let first_entry = fbb.create_vector(b"key1");
        let sst_info = FbSsTableInfo::create(
            &mut fbb,
            &SsTableInfoArgs {
                first_entry: Some(first_entry),
                ..Default::default()
            },
        );
        let sst_ulid = ulid::Ulid::new();
        let sst_id = root_generated::Ulid::create(
            &mut fbb,
            &root_generated::UlidArgs {
                high: (sst_ulid.0 >> 64) as u64,
                low: ((sst_ulid.0 << 64) >> 64) as u64,
            },
        );
        let output_sst = CompactedSsTable::create(
            &mut fbb,
            &CompactedSsTableArgs {
                id: Some(sst_id),
                info: Some(sst_info),
                visible_range: None,
                format_version: None, // deliberately not set
            },
        );
        let output_ssts_vec = fbb.create_vector(&[output_sst]);
        // Build a compaction with a tiered spec
        let source_ulid = ulid::Ulid::new();
        let source_id = root_generated::Ulid::create(
            &mut fbb,
            &root_generated::UlidArgs {
                high: (source_ulid.0 >> 64) as u64,
                low: ((source_ulid.0 << 64) >> 64) as u64,
            },
        );
        let source_ssts = fbb.create_vector(&[source_id]);
        let segment_bytes = fbb.create_vector::<u8>(&[]);
        let spec = TieredCompactionSpec::create(
            &mut fbb,
            &TieredCompactionSpecArgs {
                ssts: Some(source_ssts),
                sorted_runs: None,
                destination: 0,
                l0_view_ids: None,
                segment: Some(segment_bytes),
            },
        );
        let compaction_ulid = ulid::Ulid::new();
        let compaction_id = root_generated::Ulid::create(
            &mut fbb,
            &root_generated::UlidArgs {
                high: (compaction_ulid.0 >> 64) as u64,
                low: ((compaction_ulid.0 << 64) >> 64) as u64,
            },
        );
        let compaction = FbCompaction::create(
            &mut fbb,
            &FbCompactionArgs {
                id: Some(compaction_id),
                spec_type: FbCompactionSpec::TieredCompactionSpec,
                spec: Some(spec.as_union_value()),
                status: fb_status,
                output_ssts: Some(output_ssts_vec),
                worker: None,
                ctx_type: root_generated::CompactionContext::NONE,
                ctx: None,
            },
        );
        let compactions_vec = fbb.create_vector(&[compaction]);
        let compactions = CompactionsV1::create(
            &mut fbb,
            &CompactionsV1Args {
                compactor_epoch: 1,
                recent_compactions: Some(compactions_vec),
            },
        );
        fbb.finish(compactions, None);
        let mut bytes = BytesMut::new();
        bytes.put_u16(COMPACTIONS_FORMAT_VERSION);
        bytes.put_slice(fbb.finished_data());
        let bytes = bytes.freeze();

        // when:
        let codec = FlatBufferCompactionsCodec {};
        let decoded = codec.decode(&bytes).expect("failed to decode compactions");

        // then: legacy top-level output is omitted
        let decoded_compaction = decoded.get(&compaction_ulid).expect("missing compaction");
        assert_eq!(decoded_compaction.status(), status);
        assert!(decoded_compaction.ctx().is_none());
        assert!(decoded_compaction.output_ssts().is_empty());
    }

    #[test]
    fn test_should_decode_tiered_compaction_with_missing_segment_as_root_tree() {
        // given: a compactions flatbuffer encoding a TieredCompactionSpec with
        // no segment field set, simulating a spec serialized before the
        // segment field was added to the schema.
        use super::root_generated::{
            Compaction as FbCompaction, CompactionArgs as FbCompactionArgs,
            CompactionSpec as FbCompactionSpec, CompactionStatus as FbCompactionStatus,
            CompactionsV1, CompactionsV1Args, TieredCompactionSpec, TieredCompactionSpecArgs,
        };
        let mut fbb = flatbuffers::FlatBufferBuilder::new();
        let source_ulid = ulid::Ulid::new();
        let source_id = root_generated::Ulid::create(
            &mut fbb,
            &root_generated::UlidArgs {
                high: (source_ulid.0 >> 64) as u64,
                low: ((source_ulid.0 << 64) >> 64) as u64,
            },
        );
        let source_ssts = fbb.create_vector(&[source_id]);
        let spec = TieredCompactionSpec::create(
            &mut fbb,
            &TieredCompactionSpecArgs {
                ssts: Some(source_ssts),
                sorted_runs: None,
                destination: 0,
                l0_view_ids: None,
                segment: None,
            },
        );
        let compaction_ulid = ulid::Ulid::new();
        let compaction_id = root_generated::Ulid::create(
            &mut fbb,
            &root_generated::UlidArgs {
                high: (compaction_ulid.0 >> 64) as u64,
                low: ((compaction_ulid.0 << 64) >> 64) as u64,
            },
        );
        let compaction = FbCompaction::create(
            &mut fbb,
            &FbCompactionArgs {
                id: Some(compaction_id),
                spec_type: FbCompactionSpec::TieredCompactionSpec,
                spec: Some(spec.as_union_value()),
                status: FbCompactionStatus::Running,
                output_ssts: None,
                worker: None,
                ctx_type: root_generated::CompactionContext::NONE,
                ctx: None,
            },
        );
        let compactions_vec = fbb.create_vector(&[compaction]);
        let compactions = CompactionsV1::create(
            &mut fbb,
            &CompactionsV1Args {
                compactor_epoch: 1,
                recent_compactions: Some(compactions_vec),
            },
        );
        fbb.finish(compactions, None);
        let mut bytes = BytesMut::new();
        bytes.put_u16(COMPACTIONS_FORMAT_VERSION);
        bytes.put_slice(fbb.finished_data());
        let bytes = bytes.freeze();

        // when:
        let codec = FlatBufferCompactionsCodec {};
        let decoded = codec.decode(&bytes).expect("failed to decode compactions");

        // then: missing segment decodes as the compatibility-encoded `prefix=""`
        // segment (root tree).
        let decoded_compaction = decoded.get(&compaction_ulid).expect("missing compaction");
        assert_eq!(decoded_compaction.spec().segment(), &Bytes::new());
    }

    #[test]
    fn test_should_decode_v1_tiered_compaction_with_legacy_inferred_destination() {
        // given: a v1 compactions flatbuffer encoding a tiered compaction with
        // SR inputs but no explicit destination field. v1 readers must infer
        // the destination as `min(sorted_runs)` for backward compatibility.
        use super::root_generated::{
            Compaction as FbCompaction, CompactionArgs as FbCompactionArgs,
            CompactionSpec as FbCompactionSpec, CompactionStatus as FbCompactionStatus,
            CompactionsV1, CompactionsV1Args, TieredCompactionSpec, TieredCompactionSpecArgs,
        };
        let mut fbb = flatbuffers::FlatBufferBuilder::new();
        let sorted_runs = fbb.create_vector(&[5u32, 3u32]);
        let segment_bytes = fbb.create_vector::<u8>(&[]);
        let spec = TieredCompactionSpec::create(
            &mut fbb,
            &TieredCompactionSpecArgs {
                ssts: None,
                sorted_runs: Some(sorted_runs),
                destination: 0, // absent on v1 wire; default 0 is skipped
                l0_view_ids: None,
                segment: Some(segment_bytes),
            },
        );
        let compaction_ulid = ulid::Ulid::new();
        let compaction_id = root_generated::Ulid::create(
            &mut fbb,
            &root_generated::UlidArgs {
                high: (compaction_ulid.0 >> 64) as u64,
                low: ((compaction_ulid.0 << 64) >> 64) as u64,
            },
        );
        let compaction = FbCompaction::create(
            &mut fbb,
            &FbCompactionArgs {
                id: Some(compaction_id),
                spec_type: FbCompactionSpec::TieredCompactionSpec,
                spec: Some(spec.as_union_value()),
                status: FbCompactionStatus::Running,
                output_ssts: None,
                worker: None,
                ctx_type: root_generated::CompactionContext::NONE,
                ctx: None,
            },
        );
        let compactions_vec = fbb.create_vector(&[compaction]);
        let compactions = CompactionsV1::create(
            &mut fbb,
            &CompactionsV1Args {
                compactor_epoch: 1,
                recent_compactions: Some(compactions_vec),
            },
        );
        fbb.finish(compactions, None);
        let mut bytes = BytesMut::new();
        bytes.put_u16(1);
        bytes.put_slice(fbb.finished_data());
        let bytes = bytes.freeze();

        // when:
        let codec = FlatBufferCompactionsCodec {};
        let decoded = codec.decode(&bytes).expect("failed to decode compactions");

        // then: destination is inferred as min(sorted_runs) = 3.
        let decoded_compaction = decoded.get(&compaction_ulid).expect("missing compaction");
        assert_eq!(decoded_compaction.spec().destination(), Some(3));
    }

    #[test]
    fn test_should_roundtrip_v1_write_with_visible_ranges() {
        // Build SsTableViews using SST ULIDs as view IDs (matching V1 decode behavior).
        fn new_view(first_entry: &[u8], visible_range: Option<BytesRange>) -> SsTableView {
            let ulid = ulid::Ulid::new();
            SsTableView::new_projected(
                ulid,
                SsTableHandle::new(
                    SsTableId::from(ulid),
                    SST_FORMAT_VERSION_LATEST,
                    SsTableInfo {
                        first_entry: Some(Bytes::copy_from_slice(first_entry)),
                        ..Default::default()
                    },
                ),
                visible_range,
            )
        }

        let mut manifest = Manifest::initial(ManifestCore::new());
        Arc::make_mut(&mut manifest.core.tree).l0 = VecDeque::from(vec![
            new_view(b"a", None),
            new_view(b"b", Some(BytesRange::from_ref("c"..="d"))),
        ]);
        Arc::make_mut(&mut manifest.core.tree).compacted = vec![
            SortedRun::new(
                1,
                [
                    new_view(b"e", None),
                    new_view(b"f", Some(BytesRange::from_ref("g".."h"))),
                ],
            ),
            SortedRun::new(
                2,
                [
                    new_view(b"i", None),
                    new_view(b"j", Some(BytesRange::from_ref("k"..))),
                ],
            ),
        ];
        Arc::make_mut(&mut manifest.core.tree).last_compacted_l0_sst_view_id =
            Some(manifest.core.tree.l0[0].id);
        Arc::make_mut(&mut manifest.core.tree).last_compacted_l0_sst_id =
            Some(manifest.core.tree.l0[0].sst.id.value());
        manifest.writer_epoch = 42;
        manifest.compactor_epoch = 7;
        manifest.core.next_wal_sst_id = 10;
        manifest.core.replay_after_wal_id = 5;
        manifest.core.last_l0_seq = 100;
        manifest.core.last_l0_clock_tick = 999;

        let codec = FlatBufferManifestCodec {};

        // Encode as V1 and decode back.
        let v1_bytes = FlatBufferManifestCodec::create_from_manifest_v1(&manifest);
        assert_eq!(u16::from_be_bytes([v1_bytes[0], v1_bytes[1]]), 1);
        let decoded = codec
            .decode(&v1_bytes)
            .expect("failed to decode V1 manifest");

        assert_eq!(manifest, decoded);
    }

    #[test]
    fn test_should_roundtrip_v1_write_with_external_dbs() {
        let mut manifest = Manifest::initial(ManifestCore::new());
        manifest.external_dbs = vec![ExternalDb {
            path: "/path/to/external".to_string(),
            source_checkpoint_id: uuid::Uuid::new_v4(),
            final_checkpoint_id: Some(uuid::Uuid::new_v4()),
            sst_ids: vec![
                SsTableId::from(ulid::Ulid::new()),
                SsTableId::from(ulid::Ulid::new()),
            ],
        }];

        let codec = FlatBufferManifestCodec {};
        let v1_bytes = FlatBufferManifestCodec::create_from_manifest_v1(&manifest);
        let decoded = codec
            .decode(&v1_bytes)
            .expect("failed to decode V1 manifest");

        assert_eq!(manifest, decoded);
    }

    #[test]
    fn test_v1_and_v2_roundtrip_produce_same_manifest() {
        // When view IDs equal SST IDs (identity views), V1 and V2 should decode
        // to the same in-memory Manifest.
        let mut manifest = Manifest::initial(ManifestCore::new());
        Arc::make_mut(&mut manifest.core.tree).l0 =
            VecDeque::from(vec![SsTableView::identity(SsTableHandle::new(
                SsTableId::from(ulid::Ulid::new()),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo {
                    first_entry: Some(Bytes::from_static(b"l0key")),
                    ..Default::default()
                },
            ))]);
        Arc::make_mut(&mut manifest.core.tree).compacted = vec![SortedRun::new(
            1,
            [SsTableView::identity(SsTableHandle::new(
                SsTableId::from(ulid::Ulid::new()),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo {
                    first_entry: Some(Bytes::from_static(b"srkey")),
                    ..Default::default()
                },
            ))],
        )];
        manifest.writer_epoch = 5;
        manifest.compactor_epoch = 3;

        let codec = FlatBufferManifestCodec {};
        let v1_bytes = FlatBufferManifestCodec::create_from_manifest_v1(&manifest);
        let v2_bytes = FlatBufferManifestCodec::create_from_manifest(&manifest);

        let decoded_v1 = codec.decode(&v1_bytes).expect("failed to decode V1");
        let decoded_v2 = codec.decode(&v2_bytes).expect("failed to decode V2");

        assert_eq!(decoded_v1, decoded_v2);
    }
}
