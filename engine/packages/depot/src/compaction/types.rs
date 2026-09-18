pub(crate) use std::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
};

pub(crate) use anyhow::{Context, Result, bail, ensure};
pub(crate) use futures_util::{FutureExt, TryStreamExt};
pub(crate) use gas::prelude::*;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use sha2::{Digest, Sha256};
pub(crate) use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::{
		IsolationLevel::{Serializable, Snapshot},
		end_of_key_range,
	},
};

pub(crate) use crate::{
	CMP_BULK_ACTIVITY_EARLY_TIMEOUT, CMP_FDB_BATCH_MAX_KEYS, CMP_FDB_BATCH_MAX_VALUE_BYTES,
	CMP_FDB_OVERSIZED_TXID_MAX_VALUE_BYTES, CMP_INSTALL_MAX_WRITE_BYTES,
	CMP_MAX_PENDING_CLEANUP_JOB_IDS, CMP_STAGE_CLEANUP_REF_PAGE_KEYS, CMP_STAGE_MAX_WRITE_BYTES,
	CMP_STAGE_ORPHAN_SCAN_MAX_JOBS, COMMIT_STAGE_ORPHAN_SCAN_MAX_TXIDS, HOT_DRAIN_HEAD_GRAIN_TXIDS,
	MANAGER_RECLAIM_REJECTION_BACKOFF_BASE_MS, MANAGER_RECLAIM_REJECTION_BACKOFF_MAX_MS,
	MAX_BUCKET_DEPTH,
	conveyer::{
		delta_blob, history_pin, keys,
		ltx::{DecodedLtx, LtxHeader, decode_ltx_v3, encode_ltx_v3},
		quota, shard_blob,
		types::{
			BranchState, BucketCatalogDbFact, BucketForkFact, BucketId, CommitRow, CompactionRoot,
			DBHead, DatabaseBranchId, DatabaseBranchRecord, DbHistoryPin, DirtyPage,
			FoldIndexEntry, PitrIntervalCoverage, PitrPolicy, RetiredColdObject, ShardCachePolicy,
			SqliteCmpDirty, StagedHotShardRef, decode_bucket_catalog_db_fact,
			decode_bucket_fork_fact, decode_commit_row, decode_compaction_root,
			decode_database_branch_owner, decode_database_branch_record, decode_db_head,
			decode_fold_index_entry, decode_pitr_interval_coverage, decode_pitr_policy,
			decode_shard_cache_policy, decode_sqlite_cmp_dirty, decode_staged_hot_shard_ref,
			encode_compaction_root, encode_fold_index_entry, encode_pitr_interval_coverage,
			encode_staged_hot_shard_ref,
		},
		udb,
	},
	metrics,
};

pub const DATABASE_BRANCH_ID_TAG: &str = "database_branch_id";

pub type CompactionInputFingerprint = [u8; 32];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbManagerInput {
	pub database_branch_id: DatabaseBranchId,
	#[serde(default)]
	pub actor_id: Option<String>,
	#[cfg(feature = "test-faults")]
	#[serde(default)]
	pub disable_planning_timers: bool,
}

impl DbManagerInput {
	pub fn new(database_branch_id: DatabaseBranchId, actor_id: Option<String>) -> Self {
		DbManagerInput {
			database_branch_id,
			actor_id,
			#[cfg(feature = "test-faults")]
			disable_planning_timers: false,
		}
	}

	#[cfg(feature = "test-faults")]
	pub fn with_planning_timers_disabled(
		database_branch_id: DatabaseBranchId,
		actor_id: Option<String>,
	) -> Self {
		DbManagerInput {
			database_branch_id,
			actor_id,
			disable_planning_timers: true,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbHotCompactorInput {
	pub database_branch_id: DatabaseBranchId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbColdCompactorInput {
	pub database_branch_id: DatabaseBranchId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbReclaimerInput {
	pub database_branch_id: DatabaseBranchId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompactionJobKind {
	Hot,
	Reclaim,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactionJobStatus {
	Requested,
	Succeeded,
	Rejected { reason: String },
	Failed { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TxidRange {
	pub min_txid: u64,
	pub max_txid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HotJobInputRange {
	pub txids: TxidRange,
	/// Exclusive page bound within `txids.max_txid`, when that commit was admitted only in part.
	/// `None` means every commit in the range was folded whole.
	///
	/// Install advances the hot watermark to `max_txid` only when this is `None`. A partially folded
	/// commit's shard images are correct and complete for the shards they cover, but the commit still
	/// has pages above the bound in its delta, so trusting its images as a delta-walk floor would hide
	/// them. Holding the watermark below it keeps the partial fold invisible to readers.
	#[serde(default)]
	pub max_pgno_exclusive: Option<u32>,
	pub coverage_txids: Vec<u64>,
	pub max_pages: u32,
	pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColdJobInputRange {
	pub txids: TxidRange,
	pub min_versionstamp: [u8; 16],
	pub max_versionstamp: [u8; 16],
	pub max_bytes: u64,
}

/// Identifies one delta blob: a segment of a commit, or a whole legacy commit blob.
///
/// `first_pgno` is `None` for a legacy single-blob commit and `Some(boundary)` for a segment, which
/// is exactly the discriminator the DELTA key carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DeltaSegmentRef {
	pub txid: u64,
	pub first_pgno: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReclaimJobInputRange {
	pub txids: TxidRange,
	/// Folded delta segments to reclaim: `DELTA/{txid}/*` rows whose pages a materialized shard now
	/// carries. A large commit is stored as several shard-aligned segments whose pages can become
	/// droppable at different times, so each is classified on its own rather than the commit being
	/// held or dropped whole.
	#[serde(default)]
	pub delta_reclaim_segments: Vec<DeltaSegmentRef>,
	/// Non-fold commit metadata to reclaim (C6): `COMMITS/{txid}` + `VTX/{vs}` rows below the
	/// cold-watermark-capped delete bound.
	#[serde(default)]
	pub commit_reclaim_txids: Vec<u64>,
	pub cold_objects: Vec<ReclaimColdObjectRef>,
	#[serde(default)]
	pub shard_cache_evictions: Vec<ShardCacheEvictionRef>,
	/// Hot compaction jobs whose staged shards must be cleaned up after a failed install. The
	/// reclaimer scans the FDB staging area under each job id to find and clear the orphan blobs and
	/// ref rows, so the ref set is never carried through workflow state.
	#[serde(default)]
	pub stale_hot_job_ids: Vec<Id>,
	/// Abandoned staged commits to clear: their DELTA chunk range, their quota charge, and their
	/// `CSTAGE` row. Absent on history written before staged commits existed.
	#[serde(default)]
	pub stale_commit_stage_txids: Vec<u64>,
	/// Cold compaction jobs whose uploaded S3 objects must be cleaned up after a failed publish. The
	/// reclaimer scans the FDB staging area under each job id to find the orphan cold refs and delete
	/// the exact S3 objects, so the ref set is never carried through workflow state.
	#[serde(default)]
	pub stale_cold_job_ids: Vec<Id>,
	/// The commit/delta lane is not this slice's to touch: it was planned by the v2 drain, where
	/// `SweepCommitDeltaChunk` derives and clears that history itself. The delete must then skip
	/// deriving it too, or it would rebuild a non-empty set, compare it against the empty planned one,
	/// and reject every slice. Absent on jobs planned by v1, which carry the lane as before.
	#[serde(default)]
	pub skip_commit_delta: bool,
	/// Exclusive lower bound `(shard_id, as_of_txid)` the cold-object reclaim scan started from when this
	/// slice was planned (R5). The shard-major `CMP/cold_shard` prefix is scanned in
	/// `CMP_FDB_BATCH_MAX_KEYS`-row windows so a long cold history cannot age out the transaction; the
	/// delete activity re-derives `cold_objects` from this same cursor under `Serializable` so OCC still
	/// fences a racing cold-ref change. `None` starts at the prefix beginning.
	#[serde(default)]
	pub cold_scan_cursor: Option<ColdScanCursor>,
	/// Inclusive lower bound the commit/delta reclaim scan started from when this slice was planned.
	/// The `COMMITS` range is walked in budget-bounded windows that advance across drain passes, so a
	/// branch whose leading history is all retained rows cannot wedge the sweep. The delete activity
	/// re-derives the classification from this same cursor under `Serializable` so OCC still fences a
	/// racing commit.
	pub commit_scan_cursor: u64,
	/// Where to resume inside `commit_scan_cursor`'s txid when an earlier pass stopped between
	/// its segments. Absent on history written before commits were segmented, which reads as
	/// "start of the txid".
	#[serde(default)]
	pub cursor_segment_pgno: Option<u32>,
	pub max_keys: u32,
	pub max_bytes: u64,
}

/// Exclusive lower bound for the shard-major cold-object reclaim scan: scanning resumes at the first
/// `CMP/cold_shard` row strictly after `(shard_id, as_of_txid)`.
pub type ColdScanCursor = (u32, u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReclaimColdObjectRef {
	pub object_key: String,
	pub object_generation_id: Id,
	pub content_hash: [u8; 32],
	pub expected_publish_generation: u64,
	pub shard_id: u32,
	pub as_of_txid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShardCacheEvictionRef {
	pub shard_id: u32,
	pub as_of_txid: u64,
	pub size_bytes: u64,
	pub content_hash: [u8; 32],
}

/// Stable identity of a dead `SHARD` version planned for deletion. The delete tx re-validates each
/// candidate locally: `superseded_by_txid` is the fold that superseded this version, so the delete
/// confirms that fold still lists `shard_id` and that no coverage landed in
/// `[as_of_txid, superseded_by_txid)` instead of re-walking the whole fold prefix. The delete rejects
/// the job when the re-validated set changes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeadShardVersionRef {
	pub shard_id: u32,
	pub as_of_txid: u64,
	/// The fold txid (`> as_of_txid`) whose `CMP/fold` entry lists `shard_id`, marking this version
	/// superseded. The delete path looks this fold up directly to re-validate deadness in bounded reads.
	#[serde(default)]
	pub superseded_by_txid: u64,
}

/// Walk state for the bounded dead-shard sweep, held in activity-local memory only.
///
/// The sweep walks the txid-major `CMP/fold` index ascending in `CMP_FDB_BATCH_MAX_KEYS`-row chunks.
/// `fold_cursor` is the exclusive lower-bound fold txid to resume after, `None` starting at the prefix
/// beginning, and it carries across chunks within one activity invocation only.
///
/// `prev` is the last fold txid seen per shard. It is what makes a version identifiable as dead at
/// all: `SHARD/{s}/X` is dead only once a later fold lists `s` again, so the walk has to remember
/// where it last saw each shard. A shard absent from `prev` yields no candidate.
///
/// That is why the sweep activity takes no resume cursor, unlike [`SweepStalePidxInput`]. Resuming at
/// a fold cursor with an empty `prev` is not a slower walk, it is a wrong one: every shard's first
/// appearance after the resume point would record `prev` and emit nothing, so every dead version
/// below the cursor is silently never reclaimed. The failure is an under-delete, so nothing breaks
/// visibly, storage just stops being freed.
///
/// `prev` cannot simply be made durable to fix that. It is O(distinct shards walked), so persisting
/// it in gasoline loop state is the unbounded-loop-state failure that aborts the workflow
/// transaction. Giving this sweep a cursor means either persisting `prev` in FDB behind its own
/// cursor, or replacing it with a per-shard reverse lookup so classification stops needing carried
/// state. `SweepStalePidx` gets a cursor for free because its classification is per-page and needs no
/// cross-row context at all.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeadShardScanState {
	#[serde(default)]
	pub prev: BTreeMap<u32, u64>,
	#[serde(default)]
	pub fold_cursor: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HotShardOutputRef {
	pub shard_id: u32,
	pub as_of_txid: u64,
	pub min_txid: u64,
	pub max_txid: u64,
	pub size_bytes: u64,
	pub content_hash: [u8; 32],
}

/// Rows of one FDB row family that a reclaim pass walked and cleared.
///
/// `scanned_key_count` counts every row the window read, retained or not. Reclaim charges read
/// budget for all of them, so scanned against cleared is what says whether a pass is making progress
/// or burning budget walking history something still pins.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimRowVolume {
	pub scanned_key_count: u32,
	pub key_count: u32,
	/// Key plus value bytes, matching what clearing the row frees and what the quota credit for a
	/// billable row is computed from.
	pub byte_count: u64,
}

impl ReclaimRowVolume {
	pub fn scan(&mut self, count: usize) {
		self.scanned_key_count = self
			.scanned_key_count
			.saturating_add(u32::try_from(count).unwrap_or(u32::MAX));
	}

	pub fn clear(&mut self, key_len: usize, value_len: usize) {
		self.key_count = self.key_count.saturating_add(1);
		self.byte_count = self
			.byte_count
			.saturating_add(u64::try_from(key_len.saturating_add(value_len)).unwrap_or(u64::MAX));
	}

	/// Folds another volume in, for a pass that accumulates across several committed transactions.
	pub fn merge(&mut self, other: &ReclaimRowVolume) {
		self.scanned_key_count = self
			.scanned_key_count
			.saturating_add(other.scanned_key_count);
		self.key_count = self.key_count.saturating_add(other.key_count);
		self.byte_count = self.byte_count.saturating_add(other.byte_count);
	}
}

/// Per-row-family reclaim volume for one pass, so a branch reclaiming plenty of one family while
/// starving another is visible without a per-branch metric label.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimRowStats {
	pub commit: ReclaimRowVolume,
	pub vtx: ReclaimRowVolume,
	pub delta: ReclaimRowVolume,
	pub pitr_interval: ReclaimRowVolume,
	/// Live shard rows demoted to the cold tier, not deleted. Distinct from `shard`, which is the
	/// version-retention sweep deleting dead versions outright.
	pub shard_evict: ReclaimRowVolume,
	pub shard: ReclaimRowVolume,
	pub staging: ReclaimRowVolume,
	/// Shard image bytes the staging cleanup freed, taken from each staged ref's recorded
	/// `size_bytes` rather than summed from the chunk rows. That keeps it directly comparable with
	/// the bytes the staging write path reports, which are also image bytes; `staging.byte_count`
	/// counts the underlying FDB rows including their keys and is not comparable.
	pub staging_blob_bytes_cleared: u64,
}

impl ReclaimRowStats {
	pub fn by_kind(&self) -> [(&'static str, &ReclaimRowVolume); 7] {
		[
			(metrics::RECLAIM_ROW_KIND_COMMIT, &self.commit),
			(metrics::RECLAIM_ROW_KIND_VTX, &self.vtx),
			(metrics::RECLAIM_ROW_KIND_DELTA, &self.delta),
			(metrics::RECLAIM_ROW_KIND_PITR_INTERVAL, &self.pitr_interval),
			(metrics::RECLAIM_ROW_KIND_SHARD_EVICT, &self.shard_evict),
			(metrics::RECLAIM_ROW_KIND_SHARD, &self.shard),
			(metrics::RECLAIM_ROW_KIND_STAGING, &self.staging),
		]
	}

	pub fn total_key_count(&self) -> u32 {
		self.by_kind()
			.iter()
			.fold(0, |acc, (_, volume)| acc.saturating_add(volume.key_count))
	}

	pub fn total_byte_count(&self) -> u64 {
		self.by_kind()
			.iter()
			.fold(0, |acc, (_, volume)| acc.saturating_add(volume.byte_count))
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimOutputRef {
	pub key_count: u32,
	pub byte_count: u64,
	pub min_txid: u64,
	pub max_txid: u64,
	/// Absent on history written before per-row-kind accounting existed, so an in-flight workflow
	/// replaying older activity output reports no per-kind volume for that tail rather than
	/// mis-attributing it. `key_count`/`byte_count` stay authoritative for the aggregate.
	#[serde(default)]
	pub row_stats: ReclaimRowStats,
}

/// One staged hot slice reported by `StageHotSlice`. The staged shard refs are persisted to the FDB
/// staging area, not returned here, so the companion drain loop carries only the cursor. The manager
/// bulk install re-derives the refs from FDB per chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotSliceOutput {
	pub input_range: HotJobInputRange,
	pub input_fingerprint: CompactionInputFingerprint,
	// Staged shard bytes written by this activity call, summed across its write transactions.
	pub staged_bytes: u64,
}

/// Resume position inside one hot slice's shard staging. Staging walks the slice's coverage txids
/// ascending and, within each, its shards ascending, so this pair is the last `(as_of_txid, shard_id)`
/// image a transaction wrote. The next transaction resumes at the first pair strictly after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct HotStageCursor {
	pub as_of_txid: u64,
	pub shard_id: u32,
}

/// Resume position inside one install chunk's shard copies. The staged ref rows are keyed
/// `(shard_id, as_of_txid)`, so the install copies them in that order and this pair is the last ref a
/// transaction copied. The next transaction resumes at the first pair strictly after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub struct HotInstallShardCursor {
	pub shard_id: u32,
	pub as_of_txid: u64,
}

/// One uploaded cold slice (single hot-fold boundary) reported by `StageColdSlice`. The uploaded
/// cold refs are persisted to the FDB staging area, not returned here, so the companion drain loop
/// carries only the cursor. The manager bulk publish re-derives the refs per boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColdSliceOutput {
	pub input_range: ColdJobInputRange,
	pub input_fingerprint: CompactionInputFingerprint,
}

/// A continuation reclaim slice replanned from current FDB state after prior deletes applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedReclaimSlice {
	pub input_range: ReclaimJobInputRange,
	pub input_fingerprint: CompactionInputFingerprint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_deltas_available")]
pub struct DeltasAvailable {
	pub database_branch_id: DatabaseBranchId,
	pub observed_head_txid: u64,
	pub dirty_updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_hot_job_finished")]
pub struct HotJobFinished {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_manifest_generation: u64,
	pub status: CompactionJobStatus,
	// The staged shard refs the companion drained live in the FDB staging area under this `job_id`,
	// not in this signal. On `Succeeded` the manager installs them with a single `InstallHotJob` that
	// re-derives and chunks the FDB writes internally. Otherwise it cleans them up by scanning the
	// staging area for this `job_id`.
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_cold_job_finished")]
pub struct ColdJobFinished {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_manifest_generation: u64,
	pub status: CompactionJobStatus,
	// The uploaded cold refs the companion drained live in the FDB staging area under this `job_id`,
	// not in this signal. On `Succeeded` the manager publishes them with a single `PublishColdJob`
	// that re-derives and chunks the FDB writes internally. Otherwise it cleans up the uploaded S3
	// objects by scanning the staging area for this `job_id`.
	// The last fold boundary the drain advanced its cursor past, and that fold's versionstamp. `None`
	// means the drain found no fold past the cold watermark (nothing to publish). When `Some`, the
	// manager advances the cold watermark to this boundary even when nothing was uploaded, which
	// happens when every drained fold was fully demoted to cold-only (watermark-only advance).
	#[serde(default)]
	pub drained_max_txid: Option<u64>,
	#[serde(default)]
	pub drained_max_versionstamp: [u8; 16],
	// The smallest boundary min-versionstamp the drain uploaded, ascending order so this is the first
	// boundary's. Carried for the publish input range metadata; publish itself re-derives refs.
	#[serde(default)]
	pub drained_min_versionstamp: [u8; 16],
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_reclaim_job_finished")]
pub struct ReclaimJobFinished {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub status: CompactionJobStatus,
	pub output_refs: Vec<ReclaimOutputRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_force_compaction")]
pub struct ForceCompaction {
	pub database_branch_id: DatabaseBranchId,
	pub request_id: Id,
	pub requested_work: ForceCompactionWork,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_destroy_database_branch")]
pub struct DestroyDatabaseBranch {
	pub database_branch_id: DatabaseBranchId,
	pub lifecycle_generation: u64,
	pub requested_at_ms: i64,
	pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_run_hot_job")]
pub struct RunHotJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	// The hot drain stages every slice in `[hot_watermark+1 .. drain_head_txid]`, pinning
	// `drain_head_txid` (H0) and `drain_now_ms` (T0) so all slice fingerprints bind the same head
	// and PITR clock regardless of concurrent commits during the drain.
	pub drain_head_txid: u64,
	pub drain_now_ms: i64,
	/// Whether this job ignores the admission percent for the rest of its drain. Set for a job an
	/// operator forced, since they asked for that specific work regardless of the rollout percent.
	#[serde(default)]
	pub bypass_admission: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_run_cold_job")]
pub struct RunColdJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub status: CompactionJobStatus,
	pub input_range: ColdJobInputRange,
	// Highest fold txid this drain may upload. Zero (a replayed pre-cap signal) drains only the
	// first fold; the next manager refresh replans with a real cap.
	#[serde(default)]
	pub drain_cold_head_txid: u64,
	/// Whether this job ignores the admission percent for the rest of its drain. Set for a job an
	/// operator forced.
	#[serde(default)]
	pub bypass_admission: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_run_reclaim_job")]
pub struct RunReclaimJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub status: CompactionJobStatus,
	pub input_range: ReclaimJobInputRange,
	/// Whether this job ignores the admission percent for the rest of its drain. Set for a job an
	/// operator forced.
	#[serde(default)]
	pub bypass_admission: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbManagerState {
	pub companion_workflow_ids: CompanionWorkflowIds,
	pub active_jobs: ManagerActiveJobs,
	#[serde(default)]
	pub force_compactions: ForceCompactionTracker,
	pub retry_cursors: ManagerRetryCursors,
	#[serde(default)]
	pub next_reclaim_check_at_ms: Option<i64>,
	pub branch_stop_state: BranchStopState,
	pub last_dirty_cursor: Option<DirtyCursor>,
	#[serde(default)]
	pub last_observed_branch_lifecycle_generation: Option<u64>,
	#[serde(default)]
	pub pending_cleanups: PendingCleanupQueue,
	/// Whether the last job dispatched into the single reclaim slot was staging cleanup. Cleanup
	/// yields the slot every other cycle while reclaim work is also ready, so a cleanup that keeps
	/// failing cannot stop the reclaim lanes on this branch.
	#[serde(default)]
	pub last_reclaim_slot_was_cleanup: bool,
	/// How long to hold off re-dispatching the reclaim input that just came back unsuccessful, or
	/// `None` when the last reclaim outcome was a success.
	#[serde(default)]
	pub reclaim_backoff: Option<ReclaimBackoff>,
}

impl DbManagerState {
	pub fn new(companion_workflow_ids: CompanionWorkflowIds) -> Self {
		DbManagerState {
			companion_workflow_ids,
			active_jobs: ManagerActiveJobs::default(),
			force_compactions: ForceCompactionTracker::default(),
			retry_cursors: ManagerRetryCursors::default(),
			next_reclaim_check_at_ms: None,
			branch_stop_state: BranchStopState::Running,
			last_dirty_cursor: None,
			last_observed_branch_lifecycle_generation: None,
			pending_cleanups: PendingCleanupQueue::default(),
			last_reclaim_slot_was_cleanup: false,
			reclaim_backoff: None,
		}
	}
}

/// Holds the reclaim lane off an input that keeps coming back rejected or failed.
///
/// Reclaim dispatches on any wake that finds work, not only on its own timer, and a finished job
/// signals the manager, which is itself a wake. An outcome the input reproduces therefore closes a
/// loop: plan, dispatch, reject, wake, plan the same input again, at whatever the round trip costs.
/// Keying the delay on the input fingerprint is what keeps this from slowing down real work, since a
/// branch whose reclaimable state changed plans a different input and dispatches immediately.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimBackoff {
	/// The input this delay applies to. A planned job with a different fingerprint is unaffected.
	pub input_fingerprint: CompactionInputFingerprint,
	/// Consecutive unsuccessful outcomes for this fingerprint, counted so the delay can grow.
	pub consecutive_outcomes: u32,
	/// When the input may be dispatched again.
	pub retry_at_ms: i64,
}

impl ReclaimBackoff {
	/// The backoff after one more unsuccessful outcome, doubling from the base and clamped to the
	/// ceiling. `previous` is the backoff in force when the outcome arrived, which only contributes
	/// its count when it is for this same input.
	pub(crate) fn next(
		previous: Option<&ReclaimBackoff>,
		input_fingerprint: CompactionInputFingerprint,
		now_ms: i64,
	) -> Self {
		let consecutive_outcomes = previous
			.filter(|backoff| backoff.input_fingerprint == input_fingerprint)
			.map_or(1, |backoff| backoff.consecutive_outcomes.saturating_add(1));
		let doublings = consecutive_outcomes.saturating_sub(1).min(30);
		let delay_ms = MANAGER_RECLAIM_REJECTION_BACKOFF_BASE_MS
			.saturating_mul(1i64 << doublings)
			.min(MANAGER_RECLAIM_REJECTION_BACKOFF_MAX_MS);

		ReclaimBackoff {
			input_fingerprint,
			consecutive_outcomes,
			retry_at_ms: now_ms.saturating_add(delay_ms),
		}
	}

	/// Whether this delay still bars `input_fingerprint` at `now_ms`.
	pub(crate) fn bars(&self, input_fingerprint: CompactionInputFingerprint, now_ms: i64) -> bool {
		self.input_fingerprint == input_fingerprint && now_ms < self.retry_at_ms
	}
}

/// Staging cleanup the manager accepted but could not dispatch yet, held until the single reclaim
/// slot frees up.
///
/// Holds job ids only, never ref lists, so its size is fixed regardless of how much the deferred
/// jobs staged: the reclaimer re-derives each job's staged rows from FDB by job id. The whole queue
/// dispatches as one merged cleanup job, because the requests arrive within a second of each other
/// and one per free slot would leave the later ones waiting behind whatever claims the slot next.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PendingCleanupQueue {
	#[serde(default)]
	pub hot_job_ids: Vec<Id>,
	#[serde(default)]
	pub cold_job_ids: Vec<Id>,
	/// Abandoned staged commits found by the refresh's `CSTAGE` scan. Keyed by txid rather than job
	/// id: these are an actor's partial commit, not a compaction job's staging.
	#[serde(default)]
	pub commit_stage_txids: Vec<u64>,
}

impl PendingCleanupQueue {
	pub fn is_empty(&self) -> bool {
		self.hot_job_ids.is_empty()
			&& self.cold_job_ids.is_empty()
			&& self.commit_stage_txids.is_empty()
	}

	/// Queues an abandoned staged commit's cleanup. Returns false when the lane is at its cap; a
	/// refused txid is recovered by the next refresh's orphan scan, which is idempotent.
	pub fn push_commit_stage(&mut self, txid: u64) -> bool {
		if self.commit_stage_txids.contains(&txid) {
			return true;
		}
		if self.commit_stage_txids.len() >= CMP_MAX_PENDING_CLEANUP_JOB_IDS {
			return false;
		}
		self.commit_stage_txids.push(txid);
		true
	}

	/// Queues a hot job's staging cleanup. Returns false when the lane is at its cap, which the
	/// caller must log: the `CMP/stage/` orphan scan is what recovers a refused id.
	pub fn push_hot(&mut self, job_id: Id) -> bool {
		Self::push(&mut self.hot_job_ids, job_id)
	}

	/// Queues a cold job's staging cleanup. Returns false when the lane is at its cap.
	pub fn push_cold(&mut self, job_id: Id) -> bool {
		Self::push(&mut self.cold_job_ids, job_id)
	}

	/// Removes every queued id for dispatch as one merged cleanup job.
	pub fn take(&mut self) -> PendingCleanupQueue {
		std::mem::take(self)
	}

	fn push(lane: &mut Vec<Id>, job_id: Id) -> bool {
		// Cleanup is idempotent, but a duplicate id would waste a whole staging scan re-deriving rows
		// an earlier pass already cleared.
		if lane.contains(&job_id) {
			return true;
		}
		if lane.len() >= CMP_MAX_PENDING_CLEANUP_JOB_IDS {
			return false;
		}
		lane.push(job_id);
		true
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ManagerActiveJobs {
	pub hot: Option<ActiveHotCompactionJob>,
	pub cold: Option<ActiveColdCompactionJob>,
	pub reclaim: Option<ActiveReclaimCompactionJob>,
}

impl ManagerActiveJobs {
	pub(crate) fn clear(&mut self) {
		*self = Self::default();
	}

	/// Job ids of every lane currently running, in the order hot, cold, reclaim.
	pub(crate) fn job_ids(&self) -> Vec<Id> {
		[
			self.hot.as_ref().map(|job| job.job_id),
			self.cold.as_ref().map(|job| job.job_id),
			self.reclaim.as_ref().map(|job| job.job_id),
		]
		.into_iter()
		.flatten()
		.collect()
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ForceCompactionWork {
	pub hot: bool,
	pub cold: bool,
	pub reclaim: bool,
	pub final_settle: bool,
}

impl ForceCompactionWork {
	pub(crate) fn is_empty(self) -> bool {
		!self.hot && !self.cold && !self.reclaim && !self.final_settle
	}

	pub(crate) fn includes(self, job_kind: CompactionJobKind) -> bool {
		match job_kind {
			CompactionJobKind::Hot => self.hot,
			CompactionJobKind::Reclaim => self.reclaim,
		}
	}

	pub(crate) fn union(self, other: Self) -> Self {
		ForceCompactionWork {
			hot: self.hot || other.hot,
			cold: self.cold || other.cold,
			reclaim: self.reclaim || other.reclaim,
			final_settle: self.final_settle || other.final_settle,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForceCompactionTracker {
	pub pending_requests: Vec<PendingForceCompaction>,
	pub recent_results: Vec<ForceCompactionResult>,
}

impl Default for ForceCompactionTracker {
	fn default() -> Self {
		ForceCompactionTracker {
			pending_requests: Vec::new(),
			recent_results: Vec::new(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingForceCompaction {
	pub request_id: Id,
	pub requested_work: ForceCompactionWork,
	pub attempted_job_kinds: Vec<CompactionJobKind>,
	/// Kinds this request forced that came back rejected. A rejection is deterministic for as long as
	/// the state that produced it holds, so re-forcing the kind re-plans a job that rejects again and
	/// the request never settles. Dropping the kind from the forced set lets the refresh plan nothing
	/// for it, which is what allows the request to complete and report what happened.
	#[serde(default)]
	pub rejected_job_kinds: Vec<CompactionJobKind>,
	pub completed_job_ids: Vec<Id>,
	pub skipped_noop_reasons: Vec<String>,
	pub terminal_error: Option<String>,
	pub requested_at_ms: i64,
}

impl PendingForceCompaction {
	/// The kinds still worth forcing for this request: what it asked for, minus what already came
	/// back rejected.
	fn forceable_work(&self) -> ForceCompactionWork {
		ForceCompactionWork {
			hot: self.requested_work.hot
				&& !self.rejected_job_kinds.contains(&CompactionJobKind::Hot),
			cold: false,
			reclaim: self.requested_work.reclaim
				&& !self
					.rejected_job_kinds
					.contains(&CompactionJobKind::Reclaim),
			final_settle: self.requested_work.final_settle,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForceCompactionResult {
	pub request_id: Id,
	pub requested_work: ForceCompactionWork,
	pub attempted_job_kinds: Vec<CompactionJobKind>,
	pub completed_job_ids: Vec<Id>,
	pub skipped_noop_reasons: Vec<String>,
	pub terminal_error: Option<String>,
	pub completed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanionWorkflowIds {
	pub hot_compactor_workflow_id: Id,
	pub reclaimer_workflow_id: Id,
}

impl CompanionWorkflowIds {
	pub fn new(hot_compactor_workflow_id: Id, reclaimer_workflow_id: Id) -> Self {
		CompanionWorkflowIds {
			hot_compactor_workflow_id,
			reclaimer_workflow_id,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedHotCompactionJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub input_range: HotJobInputRange,
	// Head txid (H0) and now_ms (T0) captured when the drain was planned. The companion drains up to
	// H0 and every slice + the bulk install bind these so the fingerprint stays strict under drift.
	pub drain_head_txid: u64,
	pub drain_now_ms: i64,
	pub planned_at_ms: i64,
	pub attempt: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedColdCompactionJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub input_range: ColdJobInputRange,
	// Highest fold txid this drain may upload. Capped to a bounded window past the cold watermark
	// (raised to the first fold when that fold sits past the window) so the drain's accumulated
	// output refs and upload volume stay bounded per job.
	#[serde(default)]
	pub drain_cold_head_txid: u64,
	pub planned_at_ms: i64,
	pub attempt: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedReclaimCompactionJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub input_range: ReclaimJobInputRange,
	pub planned_at_ms: i64,
	pub attempt: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveHotCompactionJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub input_range: HotJobInputRange,
	pub drain_head_txid: u64,
	pub drain_now_ms: i64,
	pub planned_at_ms: i64,
	pub attempt: u32,
}

impl ActiveHotCompactionJob {
	pub(crate) fn from_planned(planned_job: PlannedHotCompactionJob) -> Self {
		ActiveHotCompactionJob {
			database_branch_id: planned_job.database_branch_id,
			job_id: planned_job.job_id,
			base_lifecycle_generation: planned_job.base_lifecycle_generation,
			base_manifest_generation: planned_job.base_manifest_generation,
			input_fingerprint: planned_job.input_fingerprint,
			input_range: planned_job.input_range,
			drain_head_txid: planned_job.drain_head_txid,
			drain_now_ms: planned_job.drain_now_ms,
			planned_at_ms: planned_job.planned_at_ms,
			attempt: planned_job.attempt,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveColdCompactionJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub input_range: ColdJobInputRange,
	#[serde(default)]
	pub drain_cold_head_txid: u64,
	pub planned_at_ms: i64,
	pub attempt: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveReclaimCompactionJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub input_range: ReclaimJobInputRange,
	pub planned_at_ms: i64,
	pub attempt: u32,
}

impl ActiveReclaimCompactionJob {
	pub(crate) fn from_planned(planned_job: PlannedReclaimCompactionJob) -> Self {
		ActiveReclaimCompactionJob {
			database_branch_id: planned_job.database_branch_id,
			job_id: planned_job.job_id,
			base_lifecycle_generation: planned_job.base_lifecycle_generation,
			base_manifest_generation: planned_job.base_manifest_generation,
			input_fingerprint: planned_job.input_fingerprint,
			input_range: planned_job.input_range,
			planned_at_ms: planned_job.planned_at_ms,
			attempt: planned_job.attempt,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagerRetryCursors {
	pub hot: RetryCursor,
	pub cold: RetryCursor,
	pub reclaim: RetryCursor,
}

impl Default for ManagerRetryCursors {
	fn default() -> Self {
		ManagerRetryCursors {
			hot: RetryCursor::default(),
			cold: RetryCursor::default(),
			reclaim: RetryCursor::default(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryCursor {
	pub attempt: u32,
	pub next_attempt_at_ms: Option<i64>,
	pub last_error: Option<String>,
}

impl Default for RetryCursor {
	fn default() -> Self {
		RetryCursor {
			attempt: 0,
			next_attempt_at_ms: None,
			last_error: None,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchStopState {
	Running,
	StopRequested {
		lifecycle_generation: u64,
		requested_at_ms: i64,
		reason: ManagerStopReason,
	},
	Stopped {
		stopped_at_ms: i64,
	},
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManagerStopReason {
	ExplicitDestroy { reason: String },
	BranchNotLive,
}

impl ManagerStopReason {
	pub(crate) fn companion_reason(&self) -> String {
		match self {
			ManagerStopReason::ExplicitDestroy { reason } => reason.clone(),
			ManagerStopReason::BranchNotLive => "database branch is not live".to_string(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagerStopRequest {
	pub database_branch_id: DatabaseBranchId,
	pub lifecycle_generation: u64,
	pub requested_at_ms: i64,
	pub reason: ManagerStopReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtyCursor {
	pub observed_head_txid: u64,
	pub dirty_updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompanionWorkflowState {
	Idle,
	Running(CompanionRunningJob),
	Stopping {
		active_job: Option<CompanionRunningJob>,
		lifecycle_generation: u64,
		requested_at_ms: i64,
		reason: String,
	},
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanionRunningJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub started_at_ms: i64,
	pub attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct StageHotJobInput {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub input_range: HotJobInputRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageHotJobOutput {
	pub status: CompactionJobStatus,
	pub output_refs: Vec<HotShardOutputRef>,
}

/// Installs the entire hot drain. `input_range` spans `[hot_watermark+1 .. H0]` and `output_refs`
/// is the merged staged-shard set the companion drained across every chunk. The install re-derives
/// the same budget chunks via an internal cursor, copying staged shards and clearing PIDX per FDB
/// transaction, then advances `hot_watermark_txid` to `H0` and bumps `manifest_generation` once in
/// the final transaction. One activity call installs as many chunks as fit inside
/// `CMP_BULK_ACTIVITY_EARLY_TIMEOUT`, then hands the manager a resume cursor for the next call.
#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct InstallHotJobInput {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_range: HotJobInputRange,
	// Head txid (H0) and now_ms (T0) captured at drain start. The install caps head and PITR clock to
	// these so each chunk's recomputed coverage matches the staged shards regardless of later commits.
	pub drain_head_txid: u64,
	pub drain_now_ms: i64,
	// Chunk cursor a previous activity call returned when it crossed
	// `CMP_BULK_ACTIVITY_EARLY_TIMEOUT`. The install resumes here instead of the drain's `min_txid`.
	#[serde(default)]
	pub resume_cursor: Option<u64>,
	/// Resume position within `resume_cursor`, carried when a previous chunk installed only part of
	/// that commit's pages. `None` starts the commit at its first page.
	#[serde(default)]
	pub resume_cursor_segment_pgno: Option<u32>,
	/// Resume position within `resume_cursor`'s chunk, carried when a previous transaction stopped at
	/// the copy cap. `None` copies the chunk from its first staged ref.
	#[serde(default)]
	pub resume_shard_cursor: Option<HotInstallShardCursor>,
	/// Shards already copied by previous activity calls for this job. Accumulated across calls so
	/// finalize can report the shards-installed metric without scanning the whole staging area.
	#[serde(default)]
	pub installed_shard_count_before: u64,
	/// Shard image bytes already copied by previous activity calls for this job, carried for the same
	/// reason as `installed_shard_count_before`.
	#[serde(default)]
	pub installed_shard_bytes_before: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallHotJobOutput {
	pub status: CompactionJobStatus,
	// Set when the activity crossed `CMP_BULK_ACTIVITY_EARLY_TIMEOUT` with chunks left to install.
	// The manager immediately re-dispatches the activity from this cursor.
	#[serde(default)]
	pub resume_cursor: Option<u64>,
	/// Resume position within `resume_cursor` when the call installed only part of that commit's
	/// pages. The manager re-dispatches from this pair so the commit is not re-installed from its
	/// first page.
	#[serde(default)]
	pub resume_cursor_segment_pgno: Option<u32>,
	// Set when the compaction write-throttle budget was exhausted with chunks left to install. The
	// manager backs off before re-dispatching from `resume_cursor` (which is also set) rather than
	// spinning against the budget.
	#[serde(default)]
	pub throttled: bool,
	/// Resume position within `resume_cursor`'s chunk when the call stopped with shard copies left in
	/// that chunk. The manager re-dispatches from this pair so the chunk is not re-copied from its start.
	#[serde(default)]
	pub resume_shard_cursor: Option<HotInstallShardCursor>,
	/// Image bytes this install actually rewrote, as opposed to adopted where the fold already put
	/// them. Zero for a drain folded directly into the shard tier, which is the point: the gap between
	/// this and `installed_shard_bytes` is hot compaction's write amplification, and it collapsing to
	/// zero is what direct folds are for.
	#[serde(default)]
	pub copied_shard_bytes: u64,
	// Staged shards this job has installed, accumulated across activity calls rather than counted from
	// the staging area, because a job's ref rows scale with its whole drain and do not fit one
	// transaction. An activity call that copies chunks and then dies re-copies them on retry, so this
	// can overcount; it feeds a volume histogram, not correctness. Zero on rejected/failed returns.
	#[serde(default)]
	pub installed_shard_count: u64,
	// Shard image bytes this job installed, whether it copied them out of the staging area or adopted
	// them where a direct fold already wrote them. Accumulated the same way and with the same retry
	// overcount caveat as `installed_shard_count`. This is installed volume, not write amplification:
	// `copied_shard_bytes` is the half that was written to FDB a second time.
	#[serde(default)]
	pub installed_shard_bytes: u64,
}

/// Publishes the entire cold drain. `input_range` spans every boundary the companion drained and
/// `output_refs` is the merged uploaded cold-shard set. The publish writes the cold refs in
/// internally chunked FDB transactions, then advances `cold_watermark_txid`/versionstamp to the
/// final boundary and bumps `manifest_generation` once in the final transaction. One activity call
/// publishes as many boundaries as fit inside `CMP_BULK_ACTIVITY_EARLY_TIMEOUT`, then hands the
/// manager a resume cursor for the next call.
#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct PublishColdJobInput {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_range: ColdJobInputRange,
	// Cold boundary cursor a previous activity call returned when it crossed
	// `CMP_BULK_ACTIVITY_EARLY_TIMEOUT`. The publish resumes here instead of the drain start.
	#[serde(default)]
	pub resume_cursor: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ReclaimFdbJobInput {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub input_range: ReclaimJobInputRange,
	/// Carried from the job's `bypass_admission`, so a forced job keeps deleting regardless of the
	/// admission percent.
	#[serde(default)]
	pub bypass_admission: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReclaimFdbJobOutput {
	pub status: CompactionJobStatus,
	pub output_refs: Vec<ReclaimOutputRef>,
	// Set when a compaction throttle budget (write or read) was exhausted before the slice ran. No
	// deletes were issued; the reclaimer backs off and replans in a later window.
	#[serde(default)]
	pub throttled: bool,
	// Set when this slice hit its per-transaction batch budget with work still pending. The slice
	// that ran committed, so the companion immediately re-dispatches the same input to drain the
	// remainder in another bounded transaction.
	#[serde(default)]
	pub has_more: bool,
	// Set when the branch fell outside the admission percent. No deletes were issued; the companion
	// ends the job so the manager frees the reclaim slot.
	#[serde(default)]
	pub admission_blocked: bool,
}

/// Plans and stages one hot slice from the drain cursor in a single FDB transaction. `None` cursor
/// starts at the installed hot watermark. The drain pins `drain_head_txid` (H0) and `drain_now_ms`
/// (T0) captured at drain start so every slice's fingerprint binds the same head and PITR clock.
#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct StageHotSliceInput {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub cursor_min_txid: Option<u64>,
	/// Resume position within `cursor_min_txid`, carried when a previous slice admitted only part of
	/// that commit's pages. `None` starts the commit at its first page.
	#[serde(default)]
	pub cursor_min_segment_pgno: Option<u32>,
	/// Resume position within the slice's shard staging, carried when a previous transaction stopped
	/// at the write cap. `None` stages the slice from its first coverage txid and shard.
	#[serde(default)]
	pub stage_cursor: Option<HotStageCursor>,
	pub drain_head_txid: u64,
	pub drain_now_ms: i64,
	/// Carried from the job's `bypass_admission`, so a forced job keeps draining regardless of the
	/// admission percent.
	#[serde(default)]
	pub bypass_admission: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageHotSliceOutput {
	pub status: CompactionJobStatus,
	/// `Some` when the slice was staged in full; `None` when the drain is complete, the branch went
	/// stale, or the call stopped with shard images still to stage.
	pub slice: Option<HotSliceOutput>,
	/// Set when either compaction throttle budget was exhausted (read budget before the input read, or
	/// write budget before a shard write transaction). The drain backs off and re-dispatches from
	/// `next_stage_cursor`.
	#[serde(default)]
	pub throttled: bool,
	/// Set when the slice is not staged in full yet, either because the call crossed
	/// `CMP_BULK_ACTIVITY_EARLY_TIMEOUT` or because it was throttled partway through. Whatever was
	/// staged is committed, so the drain re-dispatches the same slice from this cursor and holds its
	/// txid cursor until staging reports `None`.
	#[serde(default)]
	pub next_stage_cursor: Option<HotStageCursor>,
	/// The branch fell outside the compaction admission percent currently in effect. Nothing was read
	/// or written, and the cursors did not move. The drain parks until the percent is raised again.
	#[serde(default)]
	pub admission_blocked: bool,
	/// Shard image bytes this call staged, summed across its write transactions. Reported separately
	/// from `slice` because a call that stops at the early timeout leaves `slice` unset while its
	/// staged rows are already durable, and those are exactly the widest slices.
	#[serde(default)]
	pub staged_bytes: u64,
	/// The txid of a commit the slice budget could not admit even on an empty budget, when that is
	/// why nothing was selected. `slice` and `next_stage_cursor` are both unset here, exactly as they
	/// are for a real drain, so without this field the two are indistinguishable and a branch whose
	/// hot lane can never advance reads as idle. No later slice can admit the commit either, so this
	/// is a stall, not a pause.
	#[serde(default)]
	pub stalled_at_txid: Option<u64>,
}

/// Plans and uploads one cold boundary from the drain cursor. `None` cursor starts at the published
/// cold watermark. Cold needs no clock capture: its `hot_watermark` upper bound is pinned by the
/// base `manifest_generation` check, `drain_cold_head_txid` bounds how far past the cold watermark
/// this drain may go, and the cold watermark advance is deferred to finalize.
#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct StageColdSliceInput {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub cursor_cold_txid: Option<u64>,
	// Highest fold txid this drain may upload. The first slice (no cursor) always proceeds so an
	// oversized first fold or a replayed pre-cap zero still drains one boundary.
	#[serde(default)]
	pub drain_cold_head_txid: u64,
	/// Carried from the job's `bypass_admission`, so a forced job keeps draining regardless of the
	/// admission percent.
	#[serde(default)]
	pub bypass_admission: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageColdSliceOutput {
	pub status: CompactionJobStatus,
	pub slice: Option<ColdSliceOutput>,
	/// The cluster-wide compaction read budget was spent, so this call read no shard images and staged
	/// no refs. The drain backs off and retries the same boundary rather than treating the empty slice
	/// as a completed drain.
	#[serde(default)]
	pub throttled: bool,
	/// The branch fell outside the compaction admission percent currently in effect. Nothing was read
	/// or written, and the cursors did not move. The drain parks until the percent is raised again.
	#[serde(default)]
	pub admission_blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct PlanReclaimSliceInput {
	pub database_branch_id: DatabaseBranchId,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	/// Cold-object reclaim scan cursor carried across drain passes (R5). The reclaim drain advances this
	/// from each pass's `next_cold_scan_cursor` so the cold-shard prefix drains in bounded windows.
	#[serde(default)]
	pub cold_scan_cursor: Option<ColdScanCursor>,
	/// Commit/delta reclaim scan cursor carried across drain passes. The reclaim drain advances this
	/// from each pass's `next_commit_scan_cursor` so the `COMMITS` range is swept in budget-bounded
	/// windows instead of every pass restarting at txid 0.
	pub commit_scan_cursor: u64,
	/// Where to resume inside `commit_scan_cursor`'s txid when an earlier pass stopped between
	/// its segments. Absent on history written before commits were segmented, which reads as
	/// "start of the txid".
	#[serde(default)]
	pub cursor_segment_pgno: Option<u32>,
	/// Skip the commit/delta derivation entirely. Set from the v2 drain, where `SweepCommitDeltaChunk`
	/// derives and clears that lane in one transaction and reports its own cursor, so deriving it here
	/// too would scan the same `COMMITS` window twice per drain pass. The v1 drain leaves this unset
	/// and keeps deriving both lanes.
	#[serde(default)]
	pub skip_commit_delta: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanReclaimSliceOutput {
	pub planned: Option<PlannedReclaimSlice>,
	/// Where the next drain pass should resume the cold-object reclaim scan (R5). `Some` while the cold
	/// prefix has more rows past this pass's window; `None` once the scan reached the end. The drain
	/// keeps going while this advances even if `planned` is `None`, so ineligible (pinned / above
	/// watermark) cold refs cannot stall reclaim of eligible refs sitting behind them.
	#[serde(default)]
	pub next_cold_scan_cursor: Option<ColdScanCursor>,
	/// Where the next drain pass should resume the commit/delta reclaim scan.
	pub next_commit_scan_cursor: u64,
	/// Where to resume inside `next_commit_scan_cursor`'s txid when its segments did not all fit.
	/// Absent on history written before commits were segmented, which reads as "start of the txid".
	#[serde(default)]
	pub next_segment_pgno: Option<u32>,
	/// Whether the commit scan reached the end of the reclaimable `COMMITS` range. `planned: None` on
	/// its own only means this window held nothing reclaimable, which is the common case for a branch
	/// whose leading history is retained rows; the drain must keep sweeping until this is true.
	///
	/// Also true on the abort paths (branch not live, manifest generation moved), so the drain breaks
	/// instead of replanning a window that can never produce work.
	pub commit_scan_complete: bool,
	/// The cluster-wide compaction read budget was spent, so this pass ran no scan. Neither cursor
	/// moved and `commit_scan_complete` is meaningless here, so the drain must back off and replan the
	/// same window instead of reading the empty plan as an exhausted range.
	#[serde(default)]
	pub throttled: bool,
}

/// Input for one chunk of the commit/delta history sweep (v2 reclaim drain).
///
/// The chunk derives its own window of `COMMITS`/`DELTA` candidates and clears them in the same
/// transaction, so there is no planned set to fence against and no re-derive. The cursor lives in the
/// drain's durable loop state, which is where the commit window's position has always been kept.
#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct SweepCommitDeltaChunkInput {
	pub database_branch_id: DatabaseBranchId,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	/// Inclusive txid the window starts at.
	pub commit_scan_cursor: u64,
	/// Where to resume inside `commit_scan_cursor`'s txid when an earlier pass stopped between
	/// its segments. Absent on history written before commits were segmented, which reads as
	/// "start of the txid".
	#[serde(default)]
	pub cursor_segment_pgno: Option<u32>,
	/// Carried from the job's `bypass_admission`, so a forced job keeps draining regardless of the
	/// admission percent.
	#[serde(default)]
	pub bypass_admission: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepCommitDeltaChunkOutput {
	/// `Succeeded` for a chunk that ran, whether or not it found anything to clear; `Rejected` on a
	/// lifecycle or generation change. A chunk is never `Requested`: it commits what it derived, so
	/// there is no partial state to resume.
	pub status: CompactionJobStatus,
	/// The cluster-wide compaction read budget was spent, so this chunk read nothing and the cursor did
	/// not move. The drain backs off and retries the same window.
	#[serde(default)]
	pub throttled: bool,
	/// Where the next chunk resumes. Advances past every txid this window scanned, retained or not, so
	/// a run of retained history cannot stall the reclaimable rows behind it.
	pub next_commit_scan_cursor: u64,
	/// Where to resume inside `next_commit_scan_cursor`'s txid when its segments did not all fit this
	/// chunk. Absent on history written before commits were segmented, which reads as "start of the
	/// txid" and so replays as the pre-segmentation whole-txid behaviour.
	#[serde(default)]
	pub next_segment_pgno: Option<u32>,
	/// Whether the scan reached the end of the reclaimable `COMMITS` range.
	pub commit_scan_complete: bool,
	pub key_count: u32,
	pub byte_count: u64,
	/// Absent on history written before per-row-kind accounting existed, so an in-flight workflow
	/// replaying older activity output reports no per-kind volume for that tail rather than
	/// mis-attributing it. `key_count`/`byte_count` stay authoritative for the aggregate.
	#[serde(default)]
	pub row_stats: ReclaimRowStats,
	/// Txids `next_commit_scan_cursor` moved past this chunk. A chunk that keeps advancing a little
	/// against a deep backlog is walking retained history rather than stalling, and the two look
	/// identical from the cleared-row counts alone.
	#[serde(default)]
	pub cursor_advance_txids: u64,
	/// The branch fell outside the compaction admission percent currently in effect. Nothing was read
	/// or cleared and the cursor did not move. Reclaim ends the job rather than parking, so its slot
	/// is freed for the staging cleanups that share it.
	#[serde(default)]
	pub admission_blocked: bool,
}

/// Input for the standalone dead-shard version sweep. The sweep walks the whole `CMP/fold` index in
/// bounded FDB transactions inside one activity, holding the cross-chunk `prev` supersession context
/// in local memory instead of persisting it through workflow state, and deletes dead versions as it
/// goes. It carries no scan cursor: on an early-timeout re-dispatch it re-walks from the start, and
/// already-deleted versions are gone from the fold index so the re-walk simply continues.
///
/// The missing cursor is a correctness constraint, not an omission: adding one without also solving
/// the carried supersession context silently stops reclaiming dead versions below it. See
/// [`DeadShardScanState`] for what the walk needs and why the fix is not a cursor field.
#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct SweepDeadShardVersionsInput {
	pub database_branch_id: DatabaseBranchId,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepDeadShardVersionsOutput {
	/// `Succeeded` once the whole fold index has been walked; `Requested` when the activity crossed
	/// `CMP_BULK_ACTIVITY_EARLY_TIMEOUT` with folds left to walk (the companion re-dispatches);
	/// `Rejected` on a lifecycle/generation change.
	pub status: CompactionJobStatus,
	/// Dead shard-version rows this dispatch deleted, summed over its committed chunks. A `Requested`
	/// dispatch still reports what it freed before the timeout, so the metric follows a long sweep as
	/// it runs instead of only crediting the dispatch that happens to finish the walk.
	pub row_volume: ReclaimRowVolume,
}

/// Drives the one-time stale-PIDX repair walk for a branch. Unlike the dead-shard sweep this carries
/// a scan cursor: a PIDX row the walk decides to keep stays in place, so a re-dispatch that restarted
/// from the start would re-read every live row of the branch before reaching the next stale one, and
/// on a healthy branch nearly every row is live. The cursor is a single scalar, so it is safe to
/// persist through workflow state.
#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct SweepStalePidxInput {
	pub database_branch_id: DatabaseBranchId,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	/// Inclusive page number to resume the walk at. `None` starts at the beginning of the prefix.
	pub pgno_cursor: Option<u32>,
	/// Whether an earlier window of this walk retained a stale row it could not confirm against a
	/// shard image. Carried across activity calls so the window that finishes the walk knows the
	/// whole walk was clean before it retires the branch.
	#[serde(default)]
	pub retained_unconfirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepStalePidxOutput {
	/// `Succeeded` once the whole PIDX prefix has been walked and the branch marked repaired;
	/// `Requested` when the activity crossed `CMP_BULK_ACTIVITY_EARLY_TIMEOUT` with rows left (the
	/// companion re-dispatches from `next_pgno_cursor`); `Rejected` on a lifecycle/generation change.
	pub status: CompactionJobStatus,
	pub next_pgno_cursor: Option<u32>,
	pub cleared_count: u64,
	/// Whether any window of the walk so far retained a row it could not confirm.
	#[serde(default)]
	pub retained_unconfirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct RetireColdObjectsInput {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub cold_objects: Vec<ReclaimColdObjectRef>,
	pub retired_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetireColdObjectsOutput {
	pub status: CompactionJobStatus,
	pub retired_objects: Vec<RetiredColdObject>,
	pub delete_after_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct DeleteRetiredColdObjectsInput {
	pub database_branch_id: DatabaseBranchId,
	pub cold_objects: Vec<ReclaimColdObjectRef>,
	pub now_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRetiredColdObjectsOutput {
	pub status: CompactionJobStatus,
	pub deleted_object_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct CleanupRetiredColdObjectsInput {
	pub database_branch_id: DatabaseBranchId,
	pub cold_objects: Vec<ReclaimColdObjectRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupRetiredColdObjectsOutput {
	pub status: CompactionJobStatus,
	pub cleaned_object_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct DeleteOrphanColdObjectsInput {
	pub database_branch_id: DatabaseBranchId,
	pub base_lifecycle_generation: u64,
	// Cold jobs whose staging area is scanned to re-derive orphan cold refs for deletion, so the ref
	// list is never carried through workflow state.
	pub stale_cold_job_ids: Vec<Id>,
	/// Exclusive lower bound this pass resumes the staged cold-ref scan from. `None` starts at the
	/// first ref of the first stale job.
	#[serde(default)]
	pub cursor: Option<StagedColdRefCursor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteOrphanColdObjectsOutput {
	pub status: CompactionJobStatus,
	pub deleted_object_keys: Vec<String>,
	/// Resume position for the next pass, or `None` once every stale job's staging area has been
	/// walked to its end. A pass never clears the staged ref rows it reads (the objects they point at
	/// are deleted after the planning transaction commits, so dropping the pointers first would strand
	/// them), which makes the scan not self-advancing: the cursor is what keeps the walk moving past a
	/// full window. The rows are cleared in one pass at the end of the walk instead.
	#[serde(default)]
	pub next_cursor: Option<StagedColdRefCursor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ClearStagedColdRefsInput {
	pub database_branch_id: DatabaseBranchId,
	pub base_lifecycle_generation: u64,
	/// Cold jobs whose orphan walk has reached the end of their staging area.
	pub stale_cold_job_ids: Vec<Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearStagedColdRefsOutput {
	pub status: CompactionJobStatus,
}

/// Exclusive lower bound for the staged cold-ref orphan scan: `(job_index, as_of_txid, shard_id)`.
/// `job_index` indexes `DeleteOrphanColdObjectsInput::stale_cold_job_ids`, and each job's
/// `CMP/stage/{job_id}/cold_ref` rows are keyed by `(as_of_txid, shard_id)`, so scanning resumes at
/// the first staged ref of that job strictly after that pair and then walks the remaining jobs.
pub type StagedColdRefCursor = (u32, u64, u32);

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ValidateReclaimColdObjectsInput {
	pub database_branch_id: DatabaseBranchId,
	pub cold_objects: Vec<ReclaimColdObjectRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateReclaimColdObjectsOutput {
	pub status: CompactionJobStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct RefreshManagerInput {
	pub database_branch_id: DatabaseBranchId,
	pub force: ForceCompactionWork,
	/// Jobs whose staging area is in use right now, so the refresh's `CMP/stage/` orphan scan does
	/// not report a live drain's staging as abandoned. A job id enters this list before the companion
	/// stages anything under it and leaves only after the manager has finished the job.
	#[serde(default)]
	pub active_job_ids: Vec<Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct MintCleanupJobIdInput {
	pub database_branch_id: DatabaseBranchId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshManagerOutput {
	#[serde(default)]
	pub refreshed_at_ms: i64,
	pub planned_hot_job: Option<PlannedHotCompactionJob>,
	pub planned_cold_job: Option<PlannedColdCompactionJob>,
	pub planned_reclaim_job: Option<PlannedReclaimCompactionJob>,
	pub observed_dirty: Option<SqliteCmpDirty>,
	pub head_txid: Option<u64>,
	pub branch_is_live: bool,
	pub branch_lifecycle_generation: Option<u64>,
	pub db_pin_count: usize,
	/// Why this refresh planned no reclaim job. `None` means a job was planned, so a value here is
	/// never evidence about a lane that is dispatching: read it together with `planned_reclaim_job`,
	/// which is `None` whenever this is `Some`.
	#[serde(default)]
	pub reclaim_noop_reason: Option<String>,
	/// Whether this branch is admitted to run compaction under the configured percentage-based
	/// admission gate (`compaction_admission_percent`), decided in the refresh activity from a stable
	/// hash of the branch id. Defaults to `true` so replays of pre-field history and default config
	/// behave exactly as before (every branch admitted).
	#[serde(default = "default_true")]
	pub compaction_admitted: bool,
	/// Job ids holding staging under `CMP/stage/` that belong to no in-flight job, found by a bounded
	/// scan of the prefix. This is the backstop for staging whose cleanup request never made it into
	/// the pending queue: an id refused by the queue cap, residue left by a manager that restarted
	/// with queued ids in a lost generation, or anything stranded before the queue existed. The
	/// manager feeds them back through the same repair cleanup lane, which deletes the staged FDB rows
	/// and retires the cold objects the staged refs point at.
	#[serde(default)]
	pub orphan_stage_hot_job_ids: Vec<Id>,
	#[serde(default)]
	pub orphan_stage_cold_job_ids: Vec<Id>,
	/// Staged commits (`CSTAGE`) abandoned by an actor that never came back. The primary cleaner is
	/// the next `StageBegin`, which reuses the abandoned txid and clears it inline; this is the
	/// backstop for a branch where that never happens.
	#[serde(default)]
	pub orphan_commit_stage_txids: Vec<u64>,
}

fn default_true() -> bool {
	true
}

impl ForceCompactionTracker {
	pub(crate) fn record_request(
		&mut self,
		signal: ForceCompaction,
		requested_at_ms: i64,
		active_jobs: &ManagerActiveJobs,
	) {
		if signal.requested_work.is_empty()
			|| self
				.pending_requests
				.iter()
				.any(|request| request.request_id == signal.request_id)
			|| self
				.recent_results
				.iter()
				.any(|result| result.request_id == signal.request_id)
		{
			return;
		}

		let mut request = PendingForceCompaction {
			request_id: signal.request_id,
			requested_work: signal.requested_work,
			attempted_job_kinds: Vec::new(),
			rejected_job_kinds: Vec::new(),
			completed_job_ids: Vec::new(),
			skipped_noop_reasons: Vec::new(),
			terminal_error: None,
			requested_at_ms,
		};
		for job_kind in active_jobs.matching_job_kinds(signal.requested_work) {
			Self::push_unique_job_kind(&mut request.attempted_job_kinds, job_kind);
		}
		self.pending_requests.push(request);
	}

	/// The work still worth forcing: every pending request's requested kinds, minus the kinds that
	/// request has already seen rejected.
	pub(crate) fn pending_work(&self) -> ForceCompactionWork {
		self.pending_requests
			.iter()
			.fold(ForceCompactionWork::default(), |acc, request| {
				acc.union(request.forceable_work())
			})
	}

	pub(crate) fn record_job_attempted(&mut self, job_kind: CompactionJobKind) {
		for request in &mut self.pending_requests {
			if request.requested_work.includes(job_kind) {
				Self::push_unique_job_kind(&mut request.attempted_job_kinds, job_kind);
			}
		}
	}

	pub(crate) fn record_job_finished(
		&mut self,
		job_kind: CompactionJobKind,
		job_id: Id,
		status: &CompactionJobStatus,
	) {
		for request in &mut self.pending_requests {
			if request.requested_work.includes(job_kind) {
				Self::push_unique_job_kind(&mut request.attempted_job_kinds, job_kind);
				Self::push_unique_id(&mut request.completed_job_ids, job_id);
				match status {
					CompactionJobStatus::Failed { error } => {
						request.terminal_error = Some(error.clone());
					}
					// Stop forcing this kind. It rejected on state the request cannot change, so the
					// next refresh would plan the same job and reject it again.
					CompactionJobStatus::Rejected { reason } => {
						Self::push_unique_job_kind(&mut request.rejected_job_kinds, job_kind);
						request
							.skipped_noop_reasons
							.push(format!("{job_kind:?} job rejected: {reason}"));
					}
					CompactionJobStatus::Requested | CompactionJobStatus::Succeeded => {}
				}
			}
		}
	}

	pub(crate) fn complete_ready_requests(
		&mut self,
		active_jobs: &ManagerActiveJobs,
		refresh: &RefreshManagerOutput,
		completed_at_ms: i64,
	) {
		let mut remaining_requests = Vec::new();
		let pending_requests = std::mem::take(&mut self.pending_requests);
		for mut request in pending_requests {
			let forceable_work = request.forceable_work();
			if Self::request_has_active_work(active_jobs, forceable_work) {
				remaining_requests.push(request);
				continue;
			}

			if Self::request_has_planned_work(refresh, forceable_work) {
				remaining_requests.push(request);
				continue;
			}

			request
				.skipped_noop_reasons
				.extend(Self::noop_reasons(&request, refresh));
			self.recent_results.push(ForceCompactionResult {
				request_id: request.request_id,
				requested_work: request.requested_work,
				attempted_job_kinds: request.attempted_job_kinds,
				completed_job_ids: request.completed_job_ids,
				skipped_noop_reasons: request.skipped_noop_reasons,
				terminal_error: request.terminal_error,
				completed_at_ms,
			});
			if self.recent_results.len() > 16 {
				self.recent_results.remove(0);
			}
		}
		self.pending_requests = remaining_requests;
	}

	fn request_has_active_work(
		active_jobs: &ManagerActiveJobs,
		requested_work: ForceCompactionWork,
	) -> bool {
		(requested_work.hot && active_jobs.hot.is_some())
			|| (requested_work.cold && active_jobs.cold.is_some())
			|| (requested_work.reclaim && active_jobs.reclaim.is_some())
	}

	fn request_has_planned_work(
		refresh: &RefreshManagerOutput,
		requested_work: ForceCompactionWork,
	) -> bool {
		(requested_work.hot && refresh.planned_hot_job.is_some())
			|| (requested_work.cold && refresh.planned_cold_job.is_some())
			|| (requested_work.reclaim && refresh.planned_reclaim_job.is_some())
	}

	fn noop_reasons(
		request: &PendingForceCompaction,
		refresh: &RefreshManagerOutput,
	) -> Vec<String> {
		let mut reasons = Vec::new();
		if !refresh.branch_is_live {
			reasons.push("branch:not-live".to_string());
			return reasons;
		}
		if request.requested_work.hot
			&& !request
				.attempted_job_kinds
				.contains(&CompactionJobKind::Hot)
		{
			reasons.push("hot:no-actionable-lag".to_string());
		}
		if request.requested_work.reclaim
			&& !request
				.attempted_job_kinds
				.contains(&CompactionJobKind::Reclaim)
		{
			// A request only reaches here once the refresh has no planned reclaim job, so the refresh
			// carries a reason. The fallback covers replays of history written before the reason was
			// recorded.
			reasons.push(
				refresh
					.reclaim_noop_reason
					.clone()
					.unwrap_or_else(|| "reclaim:no-actionable-work".to_string()),
			);
		}
		if request.requested_work.final_settle {
			reasons.push("final-settle:refreshed".to_string());
		}
		reasons
	}

	fn push_unique_job_kind(job_kinds: &mut Vec<CompactionJobKind>, job_kind: CompactionJobKind) {
		if !job_kinds.contains(&job_kind) {
			job_kinds.push(job_kind);
		}
	}

	fn push_unique_id(ids: &mut Vec<Id>, id: Id) {
		if !ids.contains(&id) {
			ids.push(id);
		}
	}
}

impl ManagerActiveJobs {
	fn matching_job_kinds(&self, requested_work: ForceCompactionWork) -> Vec<CompactionJobKind> {
		let mut job_kinds = Vec::new();
		if requested_work.hot && self.hot.is_some() {
			job_kinds.push(CompactionJobKind::Hot);
		}
		if requested_work.reclaim && self.reclaim.is_some() {
			job_kinds.push(CompactionJobKind::Reclaim);
		}
		job_kinds
	}
}

#[derive(Debug)]
pub(crate) struct ManagerFdbSnapshot {
	pub(crate) branch_record: Option<DatabaseBranchRecord>,
	pub(crate) head: Option<DBHead>,
	pub(crate) root: CompactionRoot,
	pub(crate) dirty: Option<SqliteCmpDirty>,
	pub(crate) db_pins: Vec<DbHistoryPin>,
	pub(crate) hot_inputs: HotInputSnapshot,
	pub(crate) reclaim_inputs: ReclaimInputSnapshot,
	pub(crate) bucket_proof_blocked_reclaim: bool,
	pub(crate) cleared_dirty: bool,
}

#[derive(Debug, Default)]
pub(crate) struct HotInputSnapshot {
	pub(crate) commits: Vec<(u64, CommitRow)>,
	pub(crate) pitr_interval_coverage: Vec<PitrIntervalSelection>,
	pub(crate) delta_chunks: Vec<(Vec<u8>, Vec<u8>)>,
	pub(crate) pidx_entries: Vec<(Vec<u8>, Vec<u8>)>,
	pub(crate) total_value_bytes: u64,
	pub(crate) selected_max_txid: Option<u64>,
	/// Exclusive page bound of the slice within `selected_max_txid`, when that commit was admitted
	/// only in part. `None` means the commit was folded whole.
	///
	/// A partial commit is always the slice's last, so this bounds only its own pages. It is also the
	/// resume cursor: the next slice starts at this page of the same txid. Install must not advance
	/// the hot watermark to a partially folded txid, or a reader would take its shard images as a
	/// delta-walk floor and miss the pages above the bound.
	pub(crate) selected_max_pgno_exclusive: Option<u32>,
	/// The first commit in the window that did not fit the slice budget while nothing before it had
	/// been admitted. A slice that selects nothing for this reason is not drained: the commit is
	/// still there and no smaller slice will ever fit it, so the caller must fail instead of treating
	/// the window as empty and advancing the watermark past unfolded history.
	pub(crate) oversized_commit_txid: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PitrIntervalSelection {
	pub(crate) bucket_start_ms: i64,
	pub(crate) coverage: PitrIntervalCoverage,
}

#[derive(Debug, Default)]
pub(crate) struct ReclaimInputSnapshot {
	pub(crate) cold_object_refs: Vec<ReclaimColdObjectRef>,
	pub(crate) shard_cache_evictions: Vec<ShardCacheEvictionCandidate>,
	/// Expired `SHARD_LRU` rows discovered this pass that are not part of a genuine demote. These are
	/// best-effort index cleanup (stale recency entries for still-active shards plus the rows of
	/// demoted idle shards) and are cleared unconditionally in the delete tx, not fenced.
	pub(crate) shard_lru_cleanup_keys: Vec<Vec<u8>>,
	pub(crate) expired_pitr_interval_rows: Vec<(i64, Vec<u8>, Vec<u8>, PitrIntervalCoverage)>,
	pub(crate) commits: Vec<(u64, Vec<u8>, Vec<u8>, CommitRow)>,
	pub(crate) delta_chunks: Vec<(Vec<u8>, Vec<u8>)>,
	/// Folded delta segments to reclaim (C6): segments whose pages are no longer owned by any live
	/// PIDX entry and whose shards are materialized at every needed coverage fold (the
	/// shard-materialization gate). Their `DELTA` rows are cleared and the freed bytes credited to
	/// quota.
	pub(crate) delta_reclaim_segments: Vec<DeltaSegmentRef>,
	/// Commit metadata to reclaim (C6): non-fold txids at or below the cold-watermark-capped delete
	/// bound. Their `COMMITS/{txid}` + `VTX/{vs}` rows are cleared. COMMITS/VTX are not billable keys,
	/// so no quota credit is taken.
	pub(crate) commit_reclaim_txids: Vec<u64>,
	pub(crate) total_value_bytes: u64,
	/// Cold-object reclaim scan cursor used to derive `cold_object_refs` (R5). The planner copies this
	/// into the slice's `ReclaimJobInputRange.cold_scan_cursor` so the delete re-derives the same window.
	pub(crate) cold_scan_cursor: Option<ColdScanCursor>,
	/// Where the cold-object reclaim scan stopped (R5). `Some` while the cold prefix has more rows past
	/// this window; `None` once the scan reached the end.
	pub(crate) next_cold_scan_cursor: Option<ColdScanCursor>,
	/// Inclusive lower bound the commit/delta scan started from. The planner copies this into the
	/// slice's `ReclaimJobInputRange.commit_scan_cursor` so the delete re-derives the same window.
	pub(crate) commit_scan_cursor: u64,
	/// The segment cursor this window was derived from, carried into the plan so the delete
	/// re-derives the identical window.
	pub(crate) cursor_segment_pgno: Option<u32>,
	/// Where the commit/delta scan stopped, i.e. the cursor the next drain pass resumes from.
	pub(crate) next_commit_scan_cursor: u64,
	pub(crate) next_segment_pgno: Option<u32>,
	/// Whether the commit scan reached the end of the reclaimable `COMMITS` range this pass.
	pub(crate) commit_scan_complete: bool,
	/// Whether the commit scan gave up partway through on its elapsed bound. The candidate sets are
	/// partial when this is set, so they classify nothing: a caller must not plan from them and must
	/// not compare them against a planned set, because a partial set can only ever miscompare. The
	/// cursors are left where the scan started so the next pass re-derives the same window.
	pub(crate) scan_truncated: bool,
	/// Whether the standalone dead-shard sweep should run: true when the manager's bounded first-chunk
	/// walk found any dead version, so the manager dispatches a reclaim job whose companion runs the
	/// full `SweepDeadShardVersions` walk. The sweep itself holds its cross-chunk `prev` context in
	/// local memory, so no fold-walk state is carried through workflow state.
	pub(crate) dead_shard_sweep_needed: bool,
	/// Unexpired PITR interval coverage read for this pass. Exposed so the dead-shard sweep (plan-side
	/// chunk walk and delete-side re-validation) can build its coverage set without re-reading.
	pub(crate) pitr_interval_retention: Vec<PitrIntervalSelection>,
}

#[derive(Debug, Clone)]
pub(crate) struct ShardCacheEvictionCandidate {
	pub(crate) reference: ShardCacheEvictionRef,
	/// The version's live FDB rows (legacy single value or chunk rows), each `compare_and_clear`ed
	/// by the delete tx.
	pub(crate) shard_rows: Vec<(Vec<u8>, Vec<u8>)>,
	pub(crate) cold_ref_key: Vec<u8>,
	pub(crate) cold_ref_bytes: Vec<u8>,
}

/// One bounded chunk of the dead-shard version-retention walk: the candidates found, the scan state to
/// carry into the next pass, and whether more fold chunks remain to read.
#[derive(Debug)]
pub(crate) struct DeadShardScanChunk {
	pub(crate) candidates: Vec<DeadShardVersionCandidate>,
	pub(crate) next_scan: DeadShardScanState,
	pub(crate) has_more: bool,
}

/// One bounded window of the stale-PIDX repair walk. `candidates` are the rows this window proved
/// clearable, `next_pgno_cursor` resumes strictly after the last row scanned (not the last row
/// cleared, so a window full of live rows still advances), and `has_more` is true while the walk
/// stopped on a window or budget boundary rather than the end of the prefix.
pub(crate) struct StalePidxScanChunk {
	pub(crate) candidates: Vec<(Vec<u8>, Vec<u8>)>,
	pub(crate) next_pgno_cursor: Option<u32>,
	pub(crate) has_more: bool,
	/// A row this window classified stale but could not confirm against a shard image carrying its
	/// page, so it was retained. The walk is only allowed to retire the branch when it confirmed
	/// every row it looked at; retiring past one of these strands the row and its delta forever.
	pub(crate) retained_unconfirmed: bool,
}

/// A dead `SHARD` version planned for deletion by the version-retention walk (C4). `shard_rows`
/// holds the version's live FDB rows (always present in C4's `(cold_wm, hot_wm]` walk range), so
/// the delete tx `compare_and_clear`s each row and credits the summed key+value bytes back to
/// quota.
#[derive(Debug, Clone)]
pub(crate) struct DeadShardVersionCandidate {
	pub(crate) reference: DeadShardVersionRef,
	pub(crate) shard_rows: Vec<(Vec<u8>, Vec<u8>)>,
}

gas::prelude::join_signal!(pub DbManagerSignal {
	DeltasAvailable,
	HotJobFinished,
	ReclaimJobFinished,
	ForceCompaction,
	DestroyDatabaseBranch,
});

gas::prelude::join_signal!(pub DbHotCompactorSignal {
	RunHotJob,
	DestroyDatabaseBranch,
});

gas::prelude::join_signal!(pub DbColdCompactorSignal {
	RunColdJob,
	DestroyDatabaseBranch,
});

gas::prelude::join_signal!(pub DbReclaimerSignal {
	RunReclaimJob,
	DestroyDatabaseBranch,
});

impl DbManagerSignal {
	pub fn database_branch_id(&self) -> DatabaseBranchId {
		match self {
			DbManagerSignal::DeltasAvailable(signal) => signal.database_branch_id,
			DbManagerSignal::HotJobFinished(signal) => signal.database_branch_id,
			DbManagerSignal::ReclaimJobFinished(signal) => signal.database_branch_id,
			DbManagerSignal::ForceCompaction(signal) => signal.database_branch_id,
			DbManagerSignal::DestroyDatabaseBranch(signal) => signal.database_branch_id,
		}
	}
}

impl DbHotCompactorSignal {
	pub fn database_branch_id(&self) -> DatabaseBranchId {
		match self {
			DbHotCompactorSignal::RunHotJob(signal) => signal.database_branch_id,
			DbHotCompactorSignal::DestroyDatabaseBranch(signal) => signal.database_branch_id,
		}
	}
}

impl DbColdCompactorSignal {
	pub fn database_branch_id(&self) -> DatabaseBranchId {
		match self {
			DbColdCompactorSignal::RunColdJob(signal) => signal.database_branch_id,
			DbColdCompactorSignal::DestroyDatabaseBranch(signal) => signal.database_branch_id,
		}
	}
}

impl DbReclaimerSignal {
	pub fn database_branch_id(&self) -> DatabaseBranchId {
		match self {
			DbReclaimerSignal::RunReclaimJob(signal) => signal.database_branch_id,
			DbReclaimerSignal::DestroyDatabaseBranch(signal) => signal.database_branch_id,
		}
	}
}

pub fn database_branch_tag_value(database_branch_id: DatabaseBranchId) -> String {
	database_branch_id.as_uuid().to_string()
}
