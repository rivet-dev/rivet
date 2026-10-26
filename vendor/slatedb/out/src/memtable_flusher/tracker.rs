//! Flush tracker — the event loop that coordinates the parallel L0 flush pipeline.
//!
//! The tracker owns:
//! - immutable-memtable frontier capture
//! - upload dispatch policy
//! - coordination between uploader and manifest_writer
//!
//! It does not own:
//! - flush request semantics (see [`super::MemtableFlusher`])
//! - SST build/upload execution
//! - manifest mutation
//! - manifest durability sequencing

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
use futures::StreamExt;
use log::debug;
use slatedb_common::metrics::{CounterFn, MetricsRecorderHelper};

use crate::checkpoint::CheckpointCreateResult;
use crate::config::CheckpointOptions;
use crate::db::DbInner;
use crate::dispatcher::MessageHandler;
use crate::error::SlateDBError;
use crate::mem_table::ImmutableMemtable;
use crate::memtable_flusher::manifest_writer::{FlushResult, ManifestWriter};
use crate::memtable_flusher::uploader::{UploadJob, UploadedMemtable, Uploader};
use crate::memtable_flusher::FlushTarget;
use crate::utils::IdGenerator;
use fail_parallel::fail_point;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;
use tokio::sync::oneshot;
use ulid::Ulid;

macro_rules! memtable_flush_stat_name {
    ($suffix:expr) => {
        concat!("slatedb.memtable_flush.", $suffix)
    };
}

pub(crate) const MEMTABLE_FREEZE_COUNT: &str = memtable_flush_stat_name!("memtable_freeze_count");
pub(crate) const CHECKPOINT_REQUEST_COUNT: &str =
    memtable_flush_stat_name!("checkpoint_request_count");
pub(crate) const FLUSH_REQUEST_COUNT: &str = memtable_flush_stat_name!("flush_request_count");
pub(crate) const L0_UPLOAD_COUNT: &str = memtable_flush_stat_name!("l0_upload_count");
pub(crate) const L0_FLUSH_COUNT: &str = memtable_flush_stat_name!("l0_flush_count");
pub(crate) const MANIFEST_REFRESH_COUNT: &str = memtable_flush_stat_name!("manifest_refresh_count");

pub(crate) struct FlushTrackerStats {
    pub(crate) memtable_freeze_count: Arc<dyn CounterFn>,
    pub(crate) checkpoint_request_count: Arc<dyn CounterFn>,
    pub(crate) flush_request_count: Arc<dyn CounterFn>,
    pub(crate) l0_upload_count: Arc<dyn CounterFn>,
    pub(crate) l0_flush_count: Arc<dyn CounterFn>,
    pub(crate) manifest_refresh_count: Arc<dyn CounterFn>,
}

impl FlushTrackerStats {
    pub(crate) fn new(recorder: &MetricsRecorderHelper) -> Self {
        Self {
            memtable_freeze_count: recorder.counter(MEMTABLE_FREEZE_COUNT).register(),
            checkpoint_request_count: recorder.counter(CHECKPOINT_REQUEST_COUNT).register(),
            flush_request_count: recorder.counter(FLUSH_REQUEST_COUNT).register(),
            l0_upload_count: recorder.counter(L0_UPLOAD_COUNT).register(),
            l0_flush_count: recorder.counter(L0_FLUSH_COUNT).register(),
            manifest_refresh_count: recorder.counter(MANIFEST_REFRESH_COUNT).register(),
        }
    }
}

/// Unified message type for the flush tracker's event loop.
pub(crate) enum TrackerMessage {
    /// A memtable may have been frozen. Triggers reconcile and dispatch.
    MemtableFrozen,
    /// Flush request from an external caller waiting for a result.
    FlushRequest {
        target: FlushTarget,
        sender: oneshot::Sender<Result<FlushResult, SlateDBError>>,
    },
    /// Checkpoint creation request from an external caller.
    CheckpointRequest {
        target: FlushTarget,
        options: CheckpointOptions,
        sender: oneshot::Sender<Result<CheckpointCreateResult, SlateDBError>>,
    },
    /// An upload worker completed successfully.
    UploadComplete(UploadedMemtable),
    /// Durable progress advanced through a new contiguous flush frontier (inclusive).
    FlushComplete { through_seq: u64 },
    /// Remote manifest changes were merged into local state.
    ManifestRefreshed,
    /// Poll the remote manifest and signal the caller when complete.
    PollManifest {
        sender: oneshot::Sender<Result<(), SlateDBError>>,
    },
}

impl std::fmt::Debug for TrackerMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MemtableFrozen => write!(f, "MemtableFrozen"),
            Self::FlushRequest { .. } => write!(f, "FlushRequest"),
            Self::CheckpointRequest { .. } => write!(f, "CheckpointRequest"),
            Self::UploadComplete(u) => {
                write!(
                    f,
                    "UploadComplete(first_seq={}, last_seq={})",
                    u.first_seq, u.last_seq
                )
            }
            Self::FlushComplete { through_seq } => {
                write!(f, "FlushComplete(through_seq={through_seq})")
            }
            Self::ManifestRefreshed => write!(f, "ManifestRefreshed"),
            Self::PollManifest { .. } => write!(f, "PollManifest"),
        }
    }
}

pub(super) struct FlushTracker {
    inner: Arc<DbInner>,
    uploader: Uploader,
    manifest_writer: ManifestWriter,
    frontier: TrackedImmFrontier,
    stats: FlushTrackerStats,
}

impl FlushTracker {
    pub(super) fn new(
        inner: Arc<DbInner>,
        uploader: Uploader,
        manifest_writer: ManifestWriter,
    ) -> Self {
        let stats = FlushTrackerStats::new(&inner.recorder);
        Self {
            inner,
            uploader,
            manifest_writer,
            frontier: TrackedImmFrontier::new(),
            stats,
        }
    }
}

#[async_trait]
impl MessageHandler<TrackerMessage> for FlushTracker {
    async fn handle(&mut self, message: TrackerMessage) -> Result<(), SlateDBError> {
        match message {
            TrackerMessage::MemtableFrozen => {
                self.stats.memtable_freeze_count.increment(1);
                self.reconcile_and_dispatch().await
            }
            TrackerMessage::FlushRequest { target, sender } => {
                self.stats.flush_request_count.increment(1);
                self.handle_flush_request(target, sender).await
            }
            TrackerMessage::CheckpointRequest {
                target,
                options,
                sender,
            } => {
                self.stats.checkpoint_request_count.increment(1);
                self.handle_checkpoint_request(target, options, sender)
                    .await
            }
            TrackerMessage::UploadComplete(uploaded) => {
                self.stats.l0_upload_count.increment(1);
                self.handle_uploaded(uploaded).await
            }
            TrackerMessage::FlushComplete { through_seq } => {
                self.stats.l0_flush_count.increment(1);
                self.frontier.retire_through(through_seq);
                self.reconcile_and_dispatch().await
            }
            TrackerMessage::ManifestRefreshed => {
                self.stats.manifest_refresh_count.increment(1);
                self.reconcile_and_dispatch().await
            }
            TrackerMessage::PollManifest { sender } => self.manifest_writer.send_poll(sender),
        }
    }

    async fn cleanup(
        &mut self,
        mut messages: BoxStream<'async_trait, TrackerMessage>,
        result: Result<(), SlateDBError>,
    ) -> Result<(), SlateDBError> {
        let err = result.err().unwrap_or(SlateDBError::Closed);
        self.drain_with_error(&mut messages, &err).await;
        // Orphan SSTs from in-flight uploads at shutdown are reaped by the
        // background garbage collector.
        Ok(())
    }
}

impl FlushTracker {
    async fn handle_flush_request(
        &mut self,
        target: FlushTarget,
        sender: oneshot::Sender<Result<FlushResult, SlateDBError>>,
    ) -> Result<(), SlateDBError> {
        fail_point!(
            Arc::clone(&self.inner.fp_registry),
            "flush-memtable-to-l0",
            |_| { Ok(()) }
        );
        if let Err(err) = self.reconcile_and_dispatch().await {
            let _ = sender.send(Err(err.clone()));
            return Err(err);
        }
        let through_seq = self.frontier.resolve_target(target);
        self.manifest_writer.send_flush(through_seq, sender)?;
        Ok(())
    }

    async fn handle_checkpoint_request(
        &mut self,
        target: FlushTarget,
        options: CheckpointOptions,
        sender: oneshot::Sender<Result<CheckpointCreateResult, SlateDBError>>,
    ) -> Result<(), SlateDBError> {
        if let Err(err) = self.reconcile_and_dispatch().await {
            let _ = sender.send(Err(err.clone()));
            return Err(err);
        }
        let through_seq = self.frontier.resolve_target(target);
        self.manifest_writer
            .send_checkpoint(through_seq, options, sender)?;
        self.dispatch_ready_memtables()
    }

    async fn handle_uploaded(&mut self, uploaded: UploadedMemtable) -> Result<(), SlateDBError> {
        debug!(
            "l0 upload completed [first_seq={}, last_seq={}, sst_ids={:?}]",
            uploaded.first_seq,
            uploaded.last_seq,
            uploaded
                .segments
                .iter()
                .map(|s| &s.sst_handle.id)
                .collect::<Vec<_>>()
        );
        self.frontier
            .set_state(uploaded.last_seq, TrackedImmState::WritingManifest);
        self.manifest_writer.notify_uploaded(uploaded).await?;
        Ok(())
    }

    /// Check for newly frozen immutable memtables and dispatch any that are ready.
    /// New IMMs are typically announced through flush commands (e.g. from the write
    /// path), but we re-check on every event to also pick up L0 slots freed by
    /// compaction via manifest refresh.
    async fn reconcile_and_dispatch(&mut self) -> Result<(), SlateDBError> {
        let imm_memtables: Vec<_> = {
            let guard = self.inner.state.read();
            guard.state().imm_memtable.iter().rev().cloned().collect()
        };
        self.frontier.register(imm_memtables.into_iter());
        self.dispatch_ready_memtables()
    }

    /// RFC-0024 §Backpressure: returns `true` iff every segment the
    /// next memtable will touch has room for one more L0 SST under
    /// both `l0_max_ssts` and `l0_max_ssts_per_key`. A memtable that
    /// touches multiple segments waits for all of them — its SSTs
    /// publish atomically and the global `last_l0_seq` invariant
    /// pins commits to seqno order.
    ///
    /// When the imm's touched-segment set is empty (no extractor
    /// configured, or the imm came from a path that bypassed
    /// validation) we fall back to the max-across-trees heuristic.
    fn can_dispatch(&self, imm: &ImmutableMemtable) -> bool {
        let state = self.inner.state.read().state();
        let core = state.core();
        let settings = &self.inner.settings;
        let touched = imm.touched_segments();
        if touched.is_empty() {
            // Fallback path — no precomputed segment set. Use the
            // legacy max-across-trees heuristic with `reserved`
            // counted against every tree.
            let (max_l0_len, max_peak) =
                core.trees().fold((0_usize, 0_usize), |(len, peak), tree| {
                    (
                        len.max(tree.l0.len()),
                        peak.max(crate::db_state::max_l0_overlap(&tree.l0)),
                    )
                });
            let reserved = self.frontier.reserved_l0_slots();
            if max_l0_len + reserved >= settings.l0_max_ssts {
                self.inner.db_stats.l0_stall_count_num_ssts.increment(1);
                return false;
            }
            if max_peak + reserved >= settings.l0_max_ssts_per_key {
                self.inner
                    .db_stats
                    .l0_stall_count_num_ssts_per_key
                    .increment(1);
                return false;
            }
            return true;
        }
        // Per-segment check: every touched segment must have room
        // accounting for in-flight reservations against that segment.
        // `core.segments` is sorted by prefix, so use a binary search
        // rather than a linear scan.
        for prefix in touched.iter() {
            let (tree_l0_len, tree_peak) =
                match core.segments.binary_search_by(|s| s.prefix.cmp(prefix)) {
                    Ok(idx) => {
                        let tree = &core.segments[idx].tree;
                        (tree.l0.len(), crate::db_state::max_l0_overlap(&tree.l0))
                    }
                    Err(_) => (0, 0),
                };
            let reserved = self.frontier.reserved_l0_slots_for(prefix);
            if tree_l0_len + reserved >= settings.l0_max_ssts {
                self.inner.db_stats.l0_stall_count_num_ssts.increment(1);
                return false;
            }
            if tree_peak + reserved >= settings.l0_max_ssts_per_key {
                self.inner
                    .db_stats
                    .l0_stall_count_num_ssts_per_key
                    .increment(1);
                return false;
            }
        }
        true
    }

    fn dispatch_ready_memtables(&mut self) -> Result<(), SlateDBError> {
        loop {
            // Strict seq order: skipping a blocked older imm
            // deadlocks against the manifest writer's FIFO commit
            // (#1687).
            let next_idx = self
                .frontier
                .tracked
                .iter()
                .position(|t| matches!(t.state, TrackedImmState::PendingDispatch));
            let Some(idx) = next_idx else {
                return Ok(());
            };
            if !self.can_dispatch(&self.frontier.tracked[idx].imm_memtable) {
                return Ok(());
            }
            self.frontier.tracked[idx].state = TrackedImmState::Uploading;
            let tracked = &self.frontier.tracked[idx];
            let imm_memtable = Arc::clone(&tracked.imm_memtable);
            let last_seq = tracked.last_seq;
            debug!(
                "dispatching l0 upload [first_seq={}, last_seq={}]",
                tracked.first_seq, last_seq
            );

            // Allocate physical SST ids here, in seqno-ordered dispatch, so
            // their ULID timestamps never fall below an earlier-dispatched
            // (and thus earlier-published) L0's (RFC-0029).
            let segment_sst_ids = allocate_segment_sst_ids(&self.inner, &imm_memtable);
            self.uploader
                .submit(UploadJob::new(imm_memtable, segment_sst_ids))?;
        }
    }

    /// Drain remaining messages during shutdown. Process completions so
    /// durable progress advances as far as possible. Reject new flush and
    /// checkpoint requests with the given error.
    async fn drain_with_error(
        &mut self,
        messages: &mut (impl futures::Stream<Item = TrackerMessage> + Unpin),
        err: &SlateDBError,
    ) {
        while let Some(message) = messages.next().await {
            match message {
                TrackerMessage::FlushRequest { sender, .. } => {
                    let _ = sender.send(Err(err.clone()));
                }
                TrackerMessage::CheckpointRequest { sender, .. } => {
                    let _ = sender.send(Err(err.clone()));
                }
                TrackerMessage::PollManifest { sender } => {
                    let _ = sender.send(Err(err.clone()));
                }
                other => {
                    let _ = self.handle(other).await;
                }
            }
        }
    }
}

/// Allocate one physical SST id per segment `imm` will flush to, keyed by
/// segment prefix.
///
/// Without an extractor the sole segment is the compatibility-encoded
/// `prefix=""` segment; with one, the segments are the imm's touched prefixes.
/// A segment that retention later prunes to empty simply leaves its id unused.
fn allocate_segment_sst_ids(inner: &DbInner, imm: &ImmutableMemtable) -> BTreeMap<Bytes, Ulid> {
    let prefixes: BTreeSet<Bytes> = if inner.segment_extractor.is_some() {
        imm.touched_segments()
    } else {
        BTreeSet::from([Bytes::new()])
    };
    let mut rng = inner.rand.rng();
    prefixes
        .into_iter()
        .map(|prefix| (prefix, rng.gen_ulid(inner.system_clock.as_ref())))
        .collect()
}

struct TrackedImm {
    first_seq: u64,
    last_seq: u64,
    imm_memtable: Arc<ImmutableMemtable>,
    state: TrackedImmState,
}

/// Tracks the frontier of immutable memtables being flushed to L0.
///
/// Encapsulates dedup, state transitions, and target resolution.
struct TrackedImmFrontier {
    tracked: VecDeque<TrackedImm>,
}

impl TrackedImmFrontier {
    fn new() -> Self {
        Self {
            tracked: VecDeque::new(),
        }
    }

    /// Register newly frozen immutable memtables, deduplicating by `last_seq`.
    fn register(&mut self, imm_memtables: impl Iterator<Item = Arc<ImmutableMemtable>>) {
        for imm_memtable in imm_memtables {
            let first_seq = imm_memtable
                .table()
                .first_seq()
                .expect("immutable memtable has no entries");
            let last_seq = imm_memtable
                .table()
                .last_seq()
                .expect("immutable memtable has no entries");
            if self.tracked.iter().any(|t| t.last_seq == last_seq) {
                continue;
            }
            self.tracked.push_back(TrackedImm {
                first_seq,
                last_seq,
                imm_memtable,
                state: TrackedImmState::PendingDispatch,
            });
        }
    }

    /// Resolves a flush target to the sequence that must become durable.
    fn resolve_target(&self, target: FlushTarget) -> Option<u64> {
        match target {
            FlushTarget::CurrentDurable => None,
            FlushTarget::All => self.tracked.back().map(|t| t.last_seq),
        }
    }

    /// Number of in-flight slots (uploading or writing manifest).
    /// Used by the legacy fallback path in `can_dispatch` when no
    /// touched-segment set is available.
    fn reserved_l0_slots(&self) -> usize {
        self.tracked
            .iter()
            .filter(|t| {
                matches!(
                    t.state,
                    TrackedImmState::Uploading | TrackedImmState::WritingManifest
                )
            })
            .count()
    }

    /// Number of in-flight slots that target the segment with the
    /// given `prefix`. An in-flight imm with a populated
    /// `touched_segments` is counted iff the set contains `prefix`.
    /// An in-flight imm with an *empty* set is conservatively counted
    /// against every prefix — its targets are unknown so we can't
    /// rule it out. In a fully wired pipeline (writes + replay both
    /// stamp the set) this fallback path doesn't fire.
    fn reserved_l0_slots_for(&self, prefix: &Bytes) -> usize {
        self.tracked
            .iter()
            .filter(|t| {
                matches!(
                    t.state,
                    TrackedImmState::Uploading | TrackedImmState::WritingManifest
                )
            })
            .filter(|t| {
                t.imm_memtable
                    .table()
                    .touched_segments_empty_or_contains(prefix)
            })
            .count()
    }

    /// Transition the next `PendingDispatch` entry to `Uploading` and return it.
    #[cfg(test)]
    fn prepare_next_upload(&mut self) -> Option<&TrackedImm> {
        let index = self
            .tracked
            .iter()
            .position(|t| matches!(t.state, TrackedImmState::PendingDispatch))?;
        self.tracked[index].state = TrackedImmState::Uploading;
        Some(&self.tracked[index])
    }

    /// Transition a tracked entry to the given state.
    fn set_state(&mut self, last_seq: u64, state: TrackedImmState) {
        let tracked = self
            .tracked
            .iter_mut()
            .find(|t| t.last_seq == last_seq)
            .expect("tracked imm not found for last_seq");
        tracked.state = state;
    }

    /// Remove tracked entries through the given sequence (inclusive).
    fn retire_through(&mut self, through_seq: u64) {
        while self
            .tracked
            .front()
            .is_some_and(|t| t.last_seq <= through_seq)
        {
            self.tracked.pop_front().expect("checked above");
        }
    }
}

enum TrackedImmState {
    PendingDispatch,
    Uploading,
    WritingManifest,
}

#[cfg(test)]
mod tests {
    use crate::batch_write::BatchWriterMessage;
    use crate::block_cache_policy::BlockCachePolicy;
    use crate::config::{CheckpointOptions, Settings};
    use crate::db::DbInner;
    use crate::db_state::{
        FilterFormat, SsTableHandle, SsTableId, SsTableInfo, SsTableView, SstType,
    };
    use crate::db_stats::{
        L0_STALL_COUNT, L0_STALL_TYPE_LABEL, L0_STALL_TYPE_NUM_SSTS, L0_STALL_TYPE_NUM_SSTS_PER_KEY,
    };
    use crate::db_status::{ClosedResultWriter, DbStatusManager};
    use crate::error::SlateDBError;
    use crate::format::sst::{SsTableFormat, SST_FORMAT_VERSION_LATEST};
    use crate::manifest::store::{FenceableManifest, ManifestStore, StoredManifest};
    use crate::manifest::ManifestCore;
    use crate::mem_table::{ImmutableMemtable, WritableKVTable};
    use crate::memtable_flusher::uploader::Uploader;
    use crate::memtable_flusher::{FlushTarget, MemtableFlusher};
    use crate::paths::PathResolver;
    use crate::prefix_extractor::PrefixExtractor;
    use crate::tablestore::{TableStore, TableStoreKind};
    use crate::test_utils::FixedThreeBytePrefixExtractor;
    use crate::types::RowEntry;
    use crate::utils::{SafeSender, WatchableOnceCell};

    use crate::wal::test_utils::FakeWalWriter;
    use crate::wal::WalWriter;
    use bytes::Bytes;
    use fail_parallel::FailPointRegistry;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use slatedb_common::clock::{DefaultSystemClock, MockSystemClock, SystemClock};
    use slatedb_common::metrics::{
        lookup_metric_with_labels, DefaultMetricsRecorder, MetricLevel, MetricsRecorder,
        MetricsRecorderHelper,
    };
    use slatedb_common::DbRand;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::runtime::Handle;
    use tokio::time::timeout;

    struct TestHarness {
        inner: Arc<DbInner>,
        manifest: FenceableManifest,
        object_store: Arc<dyn ObjectStore>,
        path: String,
    }

    async fn setup_harness(
        path: &str,
        settings: Settings,
        fp_registry: Arc<FailPointRegistry>,
    ) -> TestHarness {
        setup_harness_with_recorder(
            path,
            settings,
            fp_registry,
            MetricsRecorderHelper::noop(),
            None,
            Arc::new(DefaultSystemClock::new()),
        )
        .await
    }

    async fn setup_harness_with_recorder(
        path: &str,
        settings: Settings,
        fp_registry: Arc<FailPointRegistry>,
        db_metrics: MetricsRecorderHelper,
        segment_extractor: Option<Arc<dyn PrefixExtractor>>,
        system_clock: Arc<dyn SystemClock>,
    ) -> TestHarness {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = path.to_string();
        let rand = Arc::new(DbRand::new(42));
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from(path.clone()),
            Arc::clone(&object_store),
        ));
        let stored_manifest = StoredManifest::create_new_db(
            Arc::clone(&manifest_store),
            ManifestCore::new_with_wal_object_store(None),
            Arc::clone(&system_clock),
        )
        .await
        .unwrap();
        let table_store = Arc::new(TableStore::new_with_fp_registry(
            Arc::clone(&object_store),
            SsTableFormat::default(),
            PathResolver::from_root(Path::from(path.clone())),
            Arc::clone(&fp_registry),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let status_manager = DbStatusManager::new(0);
        let (write_tx, _) =
            SafeSender::<BatchWriterMessage>::unbounded_channel(status_manager.result_reader());
        let wal_writer = Box::new(FakeWalWriter::new(0));
        let inner = Arc::new(
            DbInner::new(
                settings,
                Arc::clone(&system_clock),
                Arc::clone(&rand),
                Arc::clone(&table_store),
                stored_manifest.prepare_dirty().unwrap(),
                Arc::new(MemtableFlusher::new(&status_manager)),
                write_tx,
                wal_writer.observer(),
                db_metrics,
                fp_registry,
                None,
                Arc::new(status_manager),
                segment_extractor,
            )
            .await
            .unwrap(),
        );
        let manifest =
            FenceableManifest::init_writer(stored_manifest, Duration::from_secs(300), system_clock)
                .await
                .unwrap();
        TestHarness {
            inner,
            manifest,
            object_store,
            path,
        }
    }

    fn freeze_value_imm(
        inner: &Arc<DbInner>,
        key: &[u8],
        value: &[u8],
        recent_flushed_wal_id: u64,
    ) {
        let seq = inner.oracle.next_seq();
        let mut guard = inner.state.write();
        guard.memtable().put(RowEntry::new_value(key, value, seq));
        let last_seq = guard.memtable().table().last_seq().unwrap();
        guard.freeze_memtable(recent_flushed_wal_id);
        // Advance the durable seq to simulate a wal flush
        inner.oracle.advance_durable_seq(last_seq);
    }

    fn freeze_merge_imm(
        inner: &Arc<DbInner>,
        key: &[u8],
        value: &[u8],
        recent_flushed_wal_id: u64,
    ) {
        let seq = inner.oracle.next_seq();
        let mut guard = inner.state.write();
        guard.memtable().put(RowEntry::new_merge(key, value, seq));
        let last_seq = guard.memtable().table().last_seq().unwrap();
        guard.freeze_memtable(recent_flushed_wal_id);
        // Advance the durable seq to simulate a wal flush
        inner.oracle.advance_durable_seq(last_seq);
    }

    async fn latest_manifest_checkpoint_count(
        path: &str,
        object_store: Arc<dyn ObjectStore>,
    ) -> usize {
        let manifest_store = ManifestStore::new(&Path::from(path), object_store);
        let manifest = manifest_store.read_latest_manifest().await.unwrap();
        manifest.manifest.core.checkpoints.len()
    }

    fn seeded_l0_handle(seed: u128, first_key: &[u8]) -> SsTableHandle {
        seeded_l0_handle_with_bounds(seed, first_key, None)
    }

    fn seeded_l0_handle_with_bounds(
        seed: u128,
        first_key: &[u8],
        last_key: Option<&[u8]>,
    ) -> SsTableHandle {
        // Seed test L0s with an old timestamp so live uploads are always newer.
        SsTableHandle::new(
            SsTableId::from(ulid::Ulid::from_parts(0, seed)),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo {
                first_entry: Some(Bytes::copy_from_slice(first_key)),
                last_entry: last_key.map(Bytes::copy_from_slice),
                index_offset: 0,
                index_len: 0,
                filter_offset: 0,
                filter_len: 0,
                compression_codec: None,
                sst_type: SstType::Compacted,
                stats_offset: 0,
                stats_len: 0,
                filter_format: FilterFormat::default(),
            },
        )
    }

    async fn set_remote_l0_disjoint(
        path: &str,
        object_store: Arc<dyn ObjectStore>,
        ranges: &[(&[u8], &[u8])],
    ) {
        let manifest_store = Arc::new(ManifestStore::new(&Path::from(path), object_store));
        let mut stored_manifest =
            StoredManifest::load(manifest_store, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let mut dirty = stored_manifest.prepare_dirty().unwrap();
        Arc::make_mut(&mut dirty.value.core.tree).l0.clear();
        for (seed, (first, last)) in ranges.iter().enumerate() {
            Arc::make_mut(&mut dirty.value.core.tree)
                .l0
                .push_back(SsTableView::new(
                    ulid::Ulid::new(),
                    seeded_l0_handle_with_bounds(seed as u128, first, Some(last)),
                ));
        }
        stored_manifest.update(dirty).await.unwrap();
    }

    fn set_local_l0_disjoint(harness: &TestHarness, ranges: &[(&[u8], &[u8])]) {
        let mut guard = harness.inner.state.write();
        guard.modify(|modifier| {
            Arc::make_mut(&mut modifier.state.manifest.value.core.tree)
                .l0
                .clear();
            for (seed, (first, last)) in ranges.iter().enumerate() {
                Arc::make_mut(&mut modifier.state.manifest.value.core.tree)
                    .l0
                    .push_back(SsTableView::new(
                        ulid::Ulid::new(),
                        seeded_l0_handle_with_bounds(seed as u128, first, Some(last)),
                    ));
            }
        });
    }

    async fn set_remote_l0_len(path: &str, object_store: Arc<dyn ObjectStore>, l0_len: usize) {
        let manifest_store = Arc::new(ManifestStore::new(&Path::from(path), object_store));
        let mut stored_manifest =
            StoredManifest::load(manifest_store, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let mut dirty = stored_manifest.prepare_dirty().unwrap();
        Arc::make_mut(&mut dirty.value.core.tree).l0.clear();
        for idx in 0..l0_len {
            Arc::make_mut(&mut dirty.value.core.tree)
                .l0
                .push_back(SsTableView::new(
                    ulid::Ulid::new(),
                    seeded_l0_handle(idx as u128, format!("seed-{idx}").as_bytes()),
                ));
        }
        stored_manifest.update(dirty).await.unwrap();
    }

    fn set_local_segment_l0_len(harness: &TestHarness, prefix: &[u8], l0_len: usize) {
        use crate::manifest::{LsmTreeState, Segment};
        let mut guard = harness.inner.state.write();
        guard.modify(|modifier| {
            let mut l0 = std::collections::VecDeque::new();
            for idx in 0..l0_len {
                l0.push_back(SsTableView::new(
                    ulid::Ulid::new(),
                    seeded_l0_handle(idx as u128, format!("local-segment-seed-{idx}").as_bytes()),
                ));
            }
            modifier.state.manifest.value.core.segments = vec![Segment {
                prefix: Bytes::copy_from_slice(prefix),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0,
                    compacted: vec![],
                }),
            }];
        });
    }

    fn set_local_l0_len(harness: &TestHarness, l0_len: usize) {
        let mut guard = harness.inner.state.write();
        guard.modify(|modifier| {
            Arc::make_mut(&mut modifier.state.manifest.value.core.tree)
                .l0
                .clear();
            for idx in 0..l0_len {
                Arc::make_mut(&mut modifier.state.manifest.value.core.tree)
                    .l0
                    .push_back(SsTableView::new(
                        ulid::Ulid::new(),
                        seeded_l0_handle(idx as u128, format!("local-seed-{idx}").as_bytes()),
                    ));
            }
        });
    }

    struct StartedFlusher {
        inner: Arc<DbInner>,
        flusher: MemtableFlusher,
        executor: crate::dispatcher::MessageHandlerExecutor,
        closed_result: WatchableOnceCell<Result<(), SlateDBError>>,
    }

    impl StartedFlusher {
        async fn shutdown(&self) {
            MemtableFlusher::shutdown(&self.executor).await;
        }

        async fn shutdown_uploader_with_error(&self, err: SlateDBError) {
            self.closed_result.write_result(Err(err));
            Uploader::shutdown(&self.executor).await;
        }
    }

    impl std::ops::Deref for StartedFlusher {
        type Target = MemtableFlusher;
        fn deref(&self) -> &Self::Target {
            &self.flusher
        }
    }

    fn start_flusher(harness: TestHarness) -> StartedFlusher {
        let inner = Arc::clone(&harness.inner);
        let closed_result = WatchableOnceCell::new();
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());
        let executor = crate::dispatcher::MessageHandlerExecutor::new(
            Arc::new(closed_result.clone()) as Arc<dyn ClosedResultWriter>,
            system_clock,
        );
        let flusher = MemtableFlusher::new(&closed_result);
        flusher
            .start(
                Arc::clone(&harness.inner),
                harness.manifest,
                &Handle::current(),
                &executor,
                &closed_result,
            )
            .unwrap();
        executor.monitor_on(&Handle::current()).unwrap();
        StartedFlusher {
            inner,
            flusher,
            executor,
            closed_result,
        }
    }

    #[tokio::test]
    async fn flush_all_waits_for_durable_upload() {
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_flush_all",
            Settings::default(),
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"k1", b"v1", 11);

        let result = timeout(Duration::from_secs(5), flusher.flush(FlushTarget::All))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(result.durable_seq, 1);

        flusher.shutdown().await;
    }

    #[tokio::test]
    async fn flush_all_waits_for_multiple_imms() {
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_flush_multiple",
            Settings::default(),
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"k1", b"v1", 0);
        freeze_value_imm(&flusher.inner, b"k2", b"v2", 0);

        let result = timeout(Duration::from_secs(5), flusher.flush(FlushTarget::All))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(result.durable_seq, 2);

        flusher.shutdown().await;
    }

    #[tokio::test]
    async fn should_resolve_multiple_flush_waiters_on_one_durable_advance() {
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_multiple_waiters",
            Settings::default(),
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"k1", b"v1", 15);

        let (first, second) = tokio::join!(
            timeout(Duration::from_secs(5), flusher.flush(FlushTarget::All)),
            timeout(Duration::from_secs(5), flusher.flush(FlushTarget::All))
        );

        let first = first.unwrap().unwrap();
        let second = second.unwrap().unwrap();
        assert_eq!(first.durable_seq, 1);
        assert_eq!(second, first);

        flusher.shutdown().await;
    }

    #[tokio::test]
    async fn checkpoint_waits_for_flush_barrier() {
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_checkpoint",
            Settings::default(),
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        let before =
            latest_manifest_checkpoint_count(&harness.path, Arc::clone(&harness.object_store))
                .await;
        let path = harness.path.clone();
        let object_store = Arc::clone(&harness.object_store);
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"k1", b"v1", 21);

        let checkpoint = timeout(
            Duration::from_secs(5),
            flusher.create_checkpoint(FlushTarget::All, CheckpointOptions::default()),
        )
        .await
        .unwrap()
        .unwrap();

        let after = latest_manifest_checkpoint_count(&path, object_store).await;
        assert!(checkpoint.manifest_id > 0);
        assert_eq!(after, before + 1);

        flusher.shutdown().await;
    }

    #[tokio::test]
    async fn checkpoint_current_durable_succeeds_when_l0_is_full() {
        let settings = Settings {
            l0_max_ssts: 1,
            manifest_poll_interval: Duration::from_millis(10),
            ..Settings::default()
        };
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_checkpoint_l0_backpressure",
            settings,
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        set_remote_l0_len(&harness.path, Arc::clone(&harness.object_store), 1).await;
        let before =
            latest_manifest_checkpoint_count(&harness.path, Arc::clone(&harness.object_store))
                .await;
        let path = harness.path.clone();
        let object_store = Arc::clone(&harness.object_store);
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"k1", b"v1", 61);

        // A CurrentDurable checkpoint should complete promptly even when L0 is
        // full — it captures whatever is already durable without waiting for
        // the flush pipeline to drain.
        let checkpoint = timeout(
            Duration::from_secs(5),
            flusher.create_checkpoint(FlushTarget::CurrentDurable, CheckpointOptions::default()),
        )
        .await
        .unwrap()
        .unwrap();
        let after = latest_manifest_checkpoint_count(&path, object_store).await;
        assert!(checkpoint.manifest_id > 0);
        assert_eq!(after, before + 1);

        flusher.shutdown().await;
    }

    #[tokio::test]
    async fn fatal_upload_failure_propagates_to_flush_waiter() {
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_build_failure",
            Settings::default(),
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        // freeze a merge entry with no merge operator → fatal build error
        let flusher = start_flusher(harness);
        freeze_merge_imm(&flusher.inner, b"k1", b"merge", 31);

        // The flush fails because the upload error propagates through
        // the executor's closed_result to the flush waiter.
        let flush_result = timeout(Duration::from_secs(5), flusher.flush(FlushTarget::All))
            .await
            .unwrap();
        assert!(
            flush_result.is_err(),
            "expected error, got {:?}",
            flush_result
        );
        assert!(
            !matches!(flush_result, Err(SlateDBError::Closed)),
            "expected specific error, got Closed"
        );

        flusher.shutdown().await;
    }

    #[tokio::test]
    async fn manifest_writer_fencing_propagates_to_flush_waiter() {
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_manifest_fenced",
            Settings::default(),
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        // Fence the manifest before starting the flusher's manifest writer.
        let _fence = {
            let manifest_store = Arc::new(ManifestStore::new(
                &Path::from(harness.path.clone()),
                Arc::clone(&harness.object_store),
            ));
            let stored_manifest =
                StoredManifest::load(manifest_store, Arc::new(DefaultSystemClock::new()))
                    .await
                    .unwrap();
            FenceableManifest::init_writer(
                stored_manifest,
                Duration::from_secs(300),
                Arc::new(DefaultSystemClock::new()),
            )
            .await
            .unwrap()
        };

        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"k1", b"v1", 11);

        // The flush should fail with a fencing error.
        let flush_result = timeout(Duration::from_secs(5), flusher.flush(FlushTarget::All))
            .await
            .unwrap();
        assert!(
            matches!(flush_result, Err(SlateDBError::Fenced)),
            "expected Fenced, got {:?}",
            flush_result
        );

        flusher.shutdown().await;
    }

    #[tokio::test]
    async fn flush_waiter_receives_error_when_reconcile_fails_before_manifest_writer() {
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_reconcile_flush_error",
            Settings::default(),
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"k1", b"v1", 11);
        flusher
            .shutdown_uploader_with_error(SlateDBError::Fenced)
            .await;

        let flush_result = timeout(Duration::from_secs(5), flusher.flush(FlushTarget::All))
            .await
            .unwrap();
        assert!(
            matches!(flush_result, Err(SlateDBError::Fenced)),
            "expected Fenced, got {:?}",
            flush_result
        );

        flusher.shutdown().await;
    }

    #[tokio::test]
    async fn checkpoint_waiter_receives_error_when_reconcile_fails_before_manifest_writer() {
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_reconcile_checkpoint_error",
            Settings::default(),
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"k1", b"v1", 11);
        flusher
            .shutdown_uploader_with_error(SlateDBError::Fenced)
            .await;

        let checkpoint_result = timeout(
            Duration::from_secs(5),
            flusher.create_checkpoint(FlushTarget::All, CheckpointOptions::default()),
        )
        .await
        .unwrap();
        assert!(
            matches!(checkpoint_result, Err(SlateDBError::Fenced)),
            "expected Fenced, got {:?}",
            checkpoint_result
        );

        flusher.shutdown().await;
    }

    #[tokio::test]
    async fn should_wait_for_manifest_refresh_before_dispatching_when_l0_is_full() {
        let settings = Settings {
            l0_max_ssts: 1,
            manifest_poll_interval: Duration::from_millis(10),
            ..Settings::default()
        };
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_l0_backpressure",
            settings,
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        set_local_l0_len(&harness, 1);
        set_remote_l0_len(&harness.path, Arc::clone(&harness.object_store), 1).await;
        let path = harness.path.clone();
        let object_store = Arc::clone(&harness.object_store);
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"k1", b"v1", 41);

        {
            let flush = flusher.flush(FlushTarget::All);
            tokio::pin!(flush);
            assert!(timeout(Duration::from_millis(100), &mut flush)
                .await
                .is_err());

            // Clear both local and remote L0 so the flusher can make progress.
            {
                let mut guard = flusher.inner.state.write();
                guard.modify(|modifier| {
                    Arc::make_mut(&mut modifier.state.manifest.value.core.tree)
                        .l0
                        .clear()
                });
            }
            set_remote_l0_len(&path, object_store, 0).await;

            let result = timeout(Duration::from_secs(5), &mut flush)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(result.durable_seq, 1);
        }

        flusher.shutdown().await;
    }

    #[tokio::test]
    async fn l0_stall_count_increments_when_l0_is_full() {
        let settings = Settings {
            l0_max_ssts: 1,
            manifest_poll_interval: Duration::from_millis(10),
            ..Settings::default()
        };
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(
            metrics_recorder.clone() as Arc<dyn MetricsRecorder>,
            MetricLevel::default(),
        );
        let harness = setup_harness_with_recorder(
            "/tmp/test_parallel_l0_flush_flusher_l0_stall_count",
            settings,
            Arc::new(FailPointRegistry::new()),
            helper,
            None,
            Arc::new(DefaultSystemClock::new()),
        )
        .await;
        set_local_l0_len(&harness, 1);
        set_remote_l0_len(&harness.path, Arc::clone(&harness.object_store), 1).await;
        let path = harness.path.clone();
        let object_store = Arc::clone(&harness.object_store);
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"k1", b"v1", 41);

        let flush = flusher.flush(FlushTarget::All);
        tokio::pin!(flush);
        assert!(timeout(Duration::from_millis(100), &mut flush)
            .await
            .is_err());
        assert!(
            lookup_metric_with_labels(
                &metrics_recorder,
                L0_STALL_COUNT,
                &[(L0_STALL_TYPE_LABEL, L0_STALL_TYPE_NUM_SSTS)],
            )
            .unwrap_or(0)
                > 0
        );
        assert_eq!(
            lookup_metric_with_labels(
                &metrics_recorder,
                L0_STALL_COUNT,
                &[(L0_STALL_TYPE_LABEL, L0_STALL_TYPE_NUM_SSTS_PER_KEY)],
            ),
            Some(0)
        );

        {
            let mut guard = flusher.inner.state.write();
            guard.modify(|modifier| {
                Arc::make_mut(&mut modifier.state.manifest.value.core.tree)
                    .l0
                    .clear()
            });
        }
        set_remote_l0_len(&path, object_store, 0).await;

        let result = timeout(Duration::from_secs(5), &mut flush)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.durable_seq, 1);

        flusher.shutdown().await;
    }

    #[tokio::test]
    async fn flush_proceeds_when_l0_total_high_but_disjoint_ranges() {
        // Mirrors a post-rescaling (union) manifest: L0 total exceeds the
        // single-source `l0_max_ssts`, but each L0 covers a disjoint key
        // range so no point is covered by more than one L0. The per-key cap
        // should allow flushes to proceed.
        let settings = Settings {
            l0_max_ssts: 100,
            l0_max_ssts_per_key: 2,
            manifest_poll_interval: Duration::from_millis(10),
            ..Settings::default()
        };
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_per_key_disjoint",
            settings,
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        let ranges: &[(&[u8], &[u8])] = &[
            (b"a0", b"a9"),
            (b"b0", b"b9"),
            (b"c0", b"c9"),
            (b"d0", b"d9"),
            (b"e0", b"e9"),
        ];
        set_local_l0_disjoint(&harness, ranges);
        set_remote_l0_disjoint(&harness.path, Arc::clone(&harness.object_store), ranges).await;
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"k1", b"v1", 41);

        let result = timeout(Duration::from_secs(5), flusher.flush(FlushTarget::All))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.durable_seq, 1);

        flusher.shutdown().await;
    }

    /// RFC-0024 §Backpressure: l0_max_ssts is per-tree, not global.
    /// A full segment must stall flushes even when the unsegmented
    /// tree is empty. Mirrors `should_wait_for_manifest_refresh_before_dispatching_when_l0_is_full`
    /// but pre-populates a *segment's* L0 instead of the unsegmented one.
    #[tokio::test]
    async fn flush_blocked_when_segment_l0_is_full() {
        let settings = Settings {
            l0_max_ssts: 1,
            manifest_poll_interval: Duration::from_millis(10),
            ..Settings::default()
        };
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_segment_backpressure",
            settings,
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        // Unsegmented tree stays empty; segment "aaa" is at the limit.
        set_local_segment_l0_len(&harness, b"aaa", 1);
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"aaa-1", b"v1", 41);

        let flush = flusher.flush(FlushTarget::All);
        tokio::pin!(flush);
        // Blocked — segment "aaa" is at l0_max_ssts.
        assert!(timeout(Duration::from_millis(100), &mut flush)
            .await
            .is_err());

        // Drain the segment locally; flush should now progress.
        {
            let mut guard = flusher.inner.state.write();
            guard.modify(|modifier| {
                modifier.state.manifest.value.core.segments.clear();
            });
        }

        let result = timeout(Duration::from_secs(5), &mut flush)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.durable_seq, 1);

        flusher.shutdown().await;
    }

    /// RFC-0024 §Backpressure: per-segment isolation. When the
    /// memtable's `touched_segments` is precomputed, a saturated
    /// segment that the memtable does *not* touch must not block
    /// dispatch. Pre-populates segment "aaa" past the limit, freezes
    /// a memtable whose touched-segment set is `{"bbb"}` only, and
    /// expects the flush to dispatch normally.
    #[tokio::test]
    async fn flush_proceeds_when_saturated_segment_is_untouched() {
        let settings = Settings {
            l0_max_ssts: 1,
            manifest_poll_interval: Duration::from_millis(10),
            ..Settings::default()
        };
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_per_segment_isolation",
            settings,
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        // Saturate segment "aaa" — but the memtable below only
        // claims to touch "bbb", so dispatch should not block.
        set_local_segment_l0_len(&harness, b"aaa", 1);
        let flusher = start_flusher(harness);
        // Stamp `{"bbb"}` onto the active memtable before freezing
        // so the imm carries that touched-segment set forward.
        {
            let guard = flusher.inner.state.write();
            let mut bbb_only = std::collections::BTreeSet::new();
            bbb_only.insert(Bytes::from_static(b"bbb"));
            guard.memtable().record_touched_segments(bbb_only);
        }
        freeze_value_imm(&flusher.inner, b"bbb-1", b"v1", 41);

        let result = timeout(Duration::from_secs(5), flusher.flush(FlushTarget::All))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.durable_seq, 1);

        flusher.shutdown().await;
    }

    /// Cold/empty segments must not stall flushes when nothing else is
    /// over the limit. With `l0_max_ssts = 4`, three empty segments
    /// alongside an unsegmented tree of length 0 should still let a
    /// fresh flush dispatch immediately.
    #[tokio::test]
    async fn flush_proceeds_when_segments_are_below_limit() {
        use crate::manifest::{LsmTreeState, Segment};
        let settings = Settings {
            l0_max_ssts: 4,
            manifest_poll_interval: Duration::from_millis(10),
            ..Settings::default()
        };
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_cold_segments",
            settings,
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        // Three sibling segments, each well below the limit.
        {
            let mut guard = harness.inner.state.write();
            guard.modify(|modifier| {
                modifier.state.manifest.value.core.segments = vec![
                    Segment {
                        prefix: Bytes::from_static(b"aaa"),
                        tree: Arc::new(LsmTreeState::default()),
                    },
                    Segment {
                        prefix: Bytes::from_static(b"bbb"),
                        tree: Arc::new(LsmTreeState::default()),
                    },
                    Segment {
                        prefix: Bytes::from_static(b"ccc"),
                        tree: Arc::new(LsmTreeState::default()),
                    },
                ];
            });
        }
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"aaa-1", b"v1", 41);

        let result = timeout(Duration::from_secs(5), flusher.flush(FlushTarget::All))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.durable_seq, 1);

        flusher.shutdown().await;
    }

    #[tokio::test]
    async fn flush_blocked_by_per_key_cap_when_ranges_overlap() {
        // A single wide-range L0 covers the whole key space. With per-key
        // cap of 1, the peak overlap is already 1 so flushes must block
        // until L0 drains.
        let settings = Settings {
            l0_max_ssts: 100,
            l0_max_ssts_per_key: 1,
            manifest_poll_interval: Duration::from_millis(10),
            ..Settings::default()
        };
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_per_key_blocks",
            settings,
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        let ranges: &[(&[u8], &[u8])] = &[(b"aaa", b"zzz")];
        set_local_l0_disjoint(&harness, ranges);
        set_remote_l0_disjoint(&harness.path, Arc::clone(&harness.object_store), ranges).await;
        let path = harness.path.clone();
        let object_store = Arc::clone(&harness.object_store);
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"k1", b"v1", 42);

        let flush = flusher.flush(FlushTarget::All);
        tokio::pin!(flush);
        // Blocked — per-key cap reached.
        assert!(timeout(Duration::from_millis(100), &mut flush)
            .await
            .is_err());

        // Drain L0 locally and remotely; flush should now progress.
        {
            let mut guard = flusher.inner.state.write();
            guard.modify(|modifier| {
                Arc::make_mut(&mut modifier.state.manifest.value.core.tree)
                    .l0
                    .clear()
            });
        }
        set_remote_l0_disjoint(&path, object_store, &[]).await;

        let result = timeout(Duration::from_secs(5), &mut flush)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.durable_seq, 1);

        flusher.shutdown().await;
    }

    #[tokio::test]
    async fn l0_stall_count_increments_for_per_key_cap() {
        let settings = Settings {
            l0_max_ssts: 100,
            l0_max_ssts_per_key: 1,
            manifest_poll_interval: Duration::from_millis(10),
            ..Settings::default()
        };
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(
            metrics_recorder.clone() as Arc<dyn MetricsRecorder>,
            MetricLevel::default(),
        );
        let harness = setup_harness_with_recorder(
            "/tmp/test_parallel_l0_flush_flusher_l0_stall_count_per_key",
            settings,
            Arc::new(FailPointRegistry::new()),
            helper,
            None,
            Arc::new(DefaultSystemClock::new()),
        )
        .await;
        let ranges: &[(&[u8], &[u8])] = &[(b"aaa", b"zzz")];
        set_local_l0_disjoint(&harness, ranges);
        set_remote_l0_disjoint(&harness.path, Arc::clone(&harness.object_store), ranges).await;
        let path = harness.path.clone();
        let object_store = Arc::clone(&harness.object_store);
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"k1", b"v1", 42);

        let flush = flusher.flush(FlushTarget::All);
        tokio::pin!(flush);
        assert!(timeout(Duration::from_millis(100), &mut flush)
            .await
            .is_err());
        assert_eq!(
            lookup_metric_with_labels(
                &metrics_recorder,
                L0_STALL_COUNT,
                &[(L0_STALL_TYPE_LABEL, L0_STALL_TYPE_NUM_SSTS)],
            ),
            Some(0)
        );
        assert!(
            lookup_metric_with_labels(
                &metrics_recorder,
                L0_STALL_COUNT,
                &[(L0_STALL_TYPE_LABEL, L0_STALL_TYPE_NUM_SSTS_PER_KEY)],
            )
            .unwrap_or(0)
                > 0
        );

        {
            let mut guard = flusher.inner.state.write();
            guard.modify(|modifier| {
                Arc::make_mut(&mut modifier.state.manifest.value.core.tree)
                    .l0
                    .clear()
            });
        }
        set_remote_l0_disjoint(&path, object_store, &[]).await;

        let result = timeout(Duration::from_secs(5), &mut flush)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.durable_seq, 1);

        flusher.shutdown().await;
    }

    #[tokio::test]
    async fn inflight_flush_waiter_resolves_on_shutdown() {
        let harness = setup_harness(
            "/tmp/test_parallel_l0_flush_flusher_clean_shutdown",
            Settings::default(),
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        let flusher = start_flusher(harness);
        freeze_value_imm(&flusher.inner, b"k1", b"v1", 11);
        freeze_value_imm(&flusher.inner, b"k2", b"v2", 12);

        // Start a flush that requires both epochs to become durable.
        let flush = flusher.flush(FlushTarget::All);
        tokio::pin!(flush);

        // Give the pipeline a moment to start processing.
        tokio::task::yield_now().await;

        // Shut down. The uploader and manifest writer drain first, so
        // in-flight uploads complete. The tracker drains last and
        // processes the completion messages.
        flusher.shutdown().await;

        // The flush should either succeed (uploads completed during drain)
        // or fail with Closed (uploads didn't finish in time). Either is
        // acceptable — the key invariant is that the waiter always receives
        // a response rather than a dropped channel.
        let result = timeout(Duration::from_secs(5), flush)
            .await
            .expect("timed out waiting for flush result");
        assert!(result.is_ok() || result.is_err());
    }

    fn imm_with_touched(entries: &[(&[u8], &[u8], u64)], touched: &[&[u8]]) -> ImmutableMemtable {
        let table = WritableKVTable::new();
        for (key, value, seq) in entries {
            table.put(RowEntry::new_value(key, value, *seq));
        }
        if !touched.is_empty() {
            table.record_touched_segments(
                touched.iter().map(|p| Bytes::copy_from_slice(p)).collect(),
            );
        }
        ImmutableMemtable::new(table, 0)
    }

    #[tokio::test]
    async fn allocate_segment_sst_ids_without_extractor_uses_empty_prefix() {
        let harness = setup_harness(
            "/tmp/test_allocate_segment_sst_ids_empty_prefix",
            Settings::default(),
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        let imm = imm_with_touched(&[(b"k1", b"v1", 1)], &[]);

        let ids = super::allocate_segment_sst_ids(&harness.inner, &imm);

        assert_eq!(ids.len(), 1);
        assert!(ids.contains_key(&Bytes::new()));
    }

    #[tokio::test]
    async fn allocate_segment_sst_ids_with_extractor_covers_touched_segments() {
        let harness = setup_harness_with_recorder(
            "/tmp/test_allocate_segment_sst_ids_segments",
            Settings::default(),
            Arc::new(FailPointRegistry::new()),
            MetricsRecorderHelper::noop(),
            Some(Arc::new(FixedThreeBytePrefixExtractor)),
            Arc::new(DefaultSystemClock::new()),
        )
        .await;
        let imm = imm_with_touched(
            &[(b"aaa-1", b"v1", 1), (b"bbb-1", b"v2", 2)],
            &[b"aaa", b"bbb"],
        );

        let ids = super::allocate_segment_sst_ids(&harness.inner, &imm);

        let prefixes: Vec<&[u8]> = ids.keys().map(|k| k.as_ref()).collect();
        assert_eq!(prefixes, vec![&b"aaa"[..], &b"bbb"[..]]);
    }

    /// RFC-0029 ordering guarantee at the allocation level: ids minted for a
    /// later-dispatched memtable never carry an earlier ULID timestamp than an
    /// earlier-dispatched one, so `newest_l0` cannot advance past a pending L0.
    #[tokio::test]
    async fn allocate_segment_sst_ids_do_not_regress_across_dispatch() {
        let clock = Arc::new(MockSystemClock::new());
        let harness = setup_harness_with_recorder(
            "/tmp/test_allocate_segment_sst_ids_ordering",
            Settings::default(),
            Arc::new(FailPointRegistry::new()),
            MetricsRecorderHelper::noop(),
            None,
            clock.clone(),
        )
        .await;

        let imm1 = imm_with_touched(&[(b"k1", b"v1", 1)], &[]);
        let first =
            super::allocate_segment_sst_ids(&harness.inner, &imm1)[&Bytes::new()].timestamp_ms();

        clock.advance(Duration::from_millis(10)).await;

        let imm2 = imm_with_touched(&[(b"k2", b"v2", 2)], &[]);
        let second =
            super::allocate_segment_sst_ids(&harness.inner, &imm2)[&Bytes::new()].timestamp_ms();

        assert!(
            second > first,
            "later dispatch must not regress: first={first}, second={second}"
        );
    }

    mod frontier_tests {
        use crate::mem_table::{ImmutableMemtable, WritableKVTable};
        use crate::memtable_flusher::tracker::{TrackedImmFrontier, TrackedImmState};
        use crate::memtable_flusher::FlushTarget;
        use crate::types::RowEntry;
        use std::sync::Arc;

        fn make_imm(seq: u64) -> Arc<ImmutableMemtable> {
            let table = WritableKVTable::new();
            table.put(RowEntry::new_value(
                &format!("k{seq}").into_bytes(),
                b"v",
                seq,
            ));
            Arc::new(ImmutableMemtable::new(table, 0))
        }

        #[test]
        fn register_deduplicates_by_last_seq() {
            let mut frontier = TrackedImmFrontier::new();
            let imm = make_imm(1);
            frontier.register(std::iter::once(Arc::clone(&imm)));
            frontier.register(std::iter::once(imm));
            assert_eq!(frontier.tracked.len(), 1);
        }

        #[test]
        fn register_assigns_sequential_sequences() {
            let mut frontier = TrackedImmFrontier::new();
            frontier.register([make_imm(1), make_imm(2)].into_iter());
            assert_eq!(frontier.tracked[0].first_seq, 1);
            assert_eq!(frontier.tracked[0].last_seq, 1);
            assert_eq!(frontier.tracked[1].first_seq, 2);
            assert_eq!(frontier.tracked[1].last_seq, 2);
        }

        #[test]
        fn resolve_target_all_returns_last_seq() {
            let mut frontier = TrackedImmFrontier::new();
            assert_eq!(frontier.resolve_target(FlushTarget::All), None);
            frontier.register([make_imm(1), make_imm(2)].into_iter());
            assert_eq!(frontier.resolve_target(FlushTarget::All), Some(2));
        }

        #[test]
        fn resolve_target_current_durable_returns_none() {
            let mut frontier = TrackedImmFrontier::new();
            frontier.register(std::iter::once(make_imm(1)));
            assert_eq!(frontier.resolve_target(FlushTarget::CurrentDurable), None);
        }

        #[test]
        fn prepare_next_upload_transitions_to_uploading() {
            let mut frontier = TrackedImmFrontier::new();
            frontier.register(std::iter::once(make_imm(1)));
            let tracked = frontier.prepare_next_upload().unwrap();
            assert_eq!(tracked.last_seq, 1);
            assert!(matches!(tracked.state, TrackedImmState::Uploading));
            // No more pending
            assert!(frontier.prepare_next_upload().is_none());
        }

        #[test]
        fn set_state_updates_tracked_entry() {
            let mut frontier = TrackedImmFrontier::new();
            frontier.register(std::iter::once(make_imm(1)));
            frontier.set_state(1, TrackedImmState::WritingManifest);
            assert!(matches!(
                frontier.tracked[0].state,
                TrackedImmState::WritingManifest
            ));
        }

        #[test]
        #[should_panic(expected = "tracked imm not found for last_seq")]
        fn set_state_panics_for_unknown_seq() {
            let mut frontier = TrackedImmFrontier::new();
            frontier.set_state(99, TrackedImmState::Uploading);
        }

        #[test]
        fn retire_through_removes_entries() {
            let mut frontier = TrackedImmFrontier::new();
            frontier.register([make_imm(1), make_imm(2), make_imm(3)].into_iter());
            frontier.retire_through(2);
            assert_eq!(frontier.tracked.len(), 1);
            assert_eq!(frontier.tracked[0].last_seq, 3);
        }

        #[test]
        fn reserved_l0_slots_counts_in_flight() {
            let mut frontier = TrackedImmFrontier::new();
            frontier.register([make_imm(1), make_imm(2), make_imm(3)].into_iter());
            assert_eq!(frontier.reserved_l0_slots(), 0);
            frontier.prepare_next_upload(); // seq 1 -> Uploading
            assert_eq!(frontier.reserved_l0_slots(), 1);
            frontier.set_state(1, TrackedImmState::WritingManifest);
            assert_eq!(frontier.reserved_l0_slots(), 1);
            frontier.retire_through(1);
            assert_eq!(frontier.reserved_l0_slots(), 0);
        }
    }
}
