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
	ACCESS_TOUCH_THROTTLE_MS, CMP_COLD_OBJECT_DELETE_GRACE_MS, CMP_FDB_BATCH_MAX_KEYS,
	CMP_FDB_BATCH_MAX_VALUE_BYTES, CMP_S3_DELETE_MAX_OBJECTS, CMP_S3_UPLOAD_LIMIT_BYTES,
	CMP_S3_UPLOAD_MAX_OBJECTS, HOT_BURST_COLD_LAG_THRESHOLD_TXIDS, MAX_BUCKET_DEPTH,
	cold_tier::{ColdTier, cold_tier_from_config},
	conveyer::{
		history_pin, keys,
		ltx::{DecodedLtx, LtxHeader, decode_ltx_v3, encode_ltx_v3},
		quota,
		types::{
			BranchState, BucketCatalogDbFact, BucketForkFact, BucketId, ColdShardRef, CommitRow,
			CompactionRoot, DBHead, DatabaseBranchId, DatabaseBranchRecord, DbHistoryPin,
			DirtyPage, PitrIntervalCoverage, PitrPolicy, RetiredColdObject,
			RetiredColdObjectDeleteState, ShardCachePolicy, SqliteCmpDirty,
			decode_bucket_catalog_db_fact, decode_bucket_fork_fact, decode_bucket_pointer,
			decode_cold_shard_ref, decode_commit_row, decode_compaction_root,
			decode_database_branch_record, decode_database_pointer, decode_db_head,
			decode_pitr_interval_coverage, decode_pitr_policy, decode_retired_cold_object,
			decode_shard_cache_policy, decode_sqlite_cmp_dirty, encode_cold_shard_ref,
			encode_compaction_root, encode_pitr_interval_coverage, encode_retired_cold_object,
		},
		udb,
	},
};

pub const DATABASE_BRANCH_ID_TAG: &str = "database_branch_id";

pub type CompactionInputFingerprint = [u8; 32];

#[cfg(feature = "test-faults")]
lazy_static::lazy_static! {
	pub(crate) static ref WORKFLOW_TEST_COLD_TIERS: parking_lot::Mutex<Vec<(DatabaseBranchId, Arc<dyn ColdTier>)>> =
		parking_lot::Mutex::new(Vec::new());
}

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
pub struct DbHotCompacterInput {
	pub database_branch_id: DatabaseBranchId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbColdCompacterInput {
	pub database_branch_id: DatabaseBranchId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbReclaimerInput {
	pub database_branch_id: DatabaseBranchId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompactionJobKind {
	Hot,
	Cold,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReclaimJobInputRange {
	pub txids: TxidRange,
	pub txid_refs: Vec<ReclaimTxidRef>,
	pub cold_objects: Vec<ReclaimColdObjectRef>,
	#[serde(default)]
	pub shard_cache_evictions: Vec<ShardCacheEvictionRef>,
	pub staged_hot_shards: Vec<StagedHotShardCleanupRef>,
	pub orphan_cold_objects: Vec<ColdShardRef>,
	pub max_keys: u32,
	pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReclaimTxidRef {
	pub txid: u64,
	pub versionstamp: [u8; 16],
}

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StagedHotShardCleanupRef {
	pub job_id: Id,
	pub output_ref: HotShardOutputRef,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimOutputRef {
	pub key_count: u32,
	pub byte_count: u64,
	pub min_txid: u64,
	pub max_txid: u64,
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
	pub input_fingerprint: CompactionInputFingerprint,
	pub status: CompactionJobStatus,
	pub output_refs: Vec<HotShardOutputRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_cold_job_finished")]
pub struct ColdJobFinished {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub status: CompactionJobStatus,
	pub output_refs: Vec<ColdShardRef>,
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
	pub input_fingerprint: CompactionInputFingerprint,
	pub status: CompactionJobStatus,
	pub input_range: HotJobInputRange,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbManagerState {
	pub companion_workflow_ids: CompanionWorkflowIds,
	pub active_jobs: ManagerActiveJobs,
	#[serde(default)]
	pub force_compactions: ForceCompactionTracker,
	pub retry_cursors: ManagerRetryCursors,
	pub planning_deadlines: ManagerPlanningDeadlines,
	pub branch_stop_state: BranchStopState,
	pub last_dirty_cursor: Option<DirtyCursor>,
	#[serde(default)]
	pub last_observed_branch_lifecycle_generation: Option<u64>,
}

impl DbManagerState {
	pub fn new(companion_workflow_ids: CompanionWorkflowIds) -> Self {
		DbManagerState {
			companion_workflow_ids,
			active_jobs: ManagerActiveJobs::default(),
			force_compactions: ForceCompactionTracker::default(),
			retry_cursors: ManagerRetryCursors::default(),
			planning_deadlines: ManagerPlanningDeadlines::default(),
			branch_stop_state: BranchStopState::Running,
			last_dirty_cursor: None,
			last_observed_branch_lifecycle_generation: None,
		}
	}

	pub fn new_with_initial_deadline(
		companion_workflow_ids: CompanionWorkflowIds,
		now_ms: i64,
	) -> Self {
		let mut state = Self::new(companion_workflow_ids);
		state.planning_deadlines = ManagerPlanningDeadlines::after_refresh(now_ms);
		state
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
			CompactionJobKind::Cold => self.cold,
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
	pub completed_job_ids: Vec<Id>,
	pub skipped_noop_reasons: Vec<String>,
	pub terminal_error: Option<String>,
	pub requested_at_ms: i64,
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
	pub hot_compacter_workflow_id: Id,
	pub cold_compacter_workflow_id: Id,
	pub reclaimer_workflow_id: Id,
}

impl CompanionWorkflowIds {
	pub fn new(
		hot_compacter_workflow_id: Id,
		cold_compacter_workflow_id: Id,
		reclaimer_workflow_id: Id,
	) -> Self {
		CompanionWorkflowIds {
			hot_compacter_workflow_id,
			cold_compacter_workflow_id,
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
	pub planned_at_ms: i64,
	pub attempt: u32,
}

impl ActiveColdCompactionJob {
	pub(crate) fn from_planned(planned_job: PlannedColdCompactionJob) -> Self {
		ActiveColdCompactionJob {
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
pub struct ManagerPlanningDeadlines {
	pub next_hot_check_at_ms: Option<i64>,
	pub next_cold_check_at_ms: Option<i64>,
	pub next_reclaim_check_at_ms: Option<i64>,
	pub final_settle_check_at_ms: Option<i64>,
}

impl Default for ManagerPlanningDeadlines {
	fn default() -> Self {
		ManagerPlanningDeadlines {
			next_hot_check_at_ms: None,
			next_cold_check_at_ms: None,
			next_reclaim_check_at_ms: None,
			final_settle_check_at_ms: None,
		}
	}
}

impl ManagerPlanningDeadlines {
	pub(crate) fn after_refresh(now_ms: i64) -> Self {
		ManagerPlanningDeadlines {
			next_hot_check_at_ms: Some(now_ms + 500),
			next_cold_check_at_ms: Some(now_ms + 5_000),
			next_reclaim_check_at_ms: Some(now_ms + 10_000),
			final_settle_check_at_ms: Some(now_ms + 30_000),
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

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct InstallHotJobInput {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub input_range: HotJobInputRange,
	pub output_refs: Vec<HotShardOutputRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallHotJobOutput {
	pub status: CompactionJobStatus,
	pub output_refs: Vec<HotShardOutputRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct UploadColdJobInput {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub input_range: ColdJobInputRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadColdJobOutput {
	pub status: CompactionJobStatus,
	pub output_refs: Vec<ColdShardRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct PublishColdJobInput {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub input_range: ColdJobInputRange,
	pub output_refs: Vec<ColdShardRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishColdJobOutput {
	pub status: CompactionJobStatus,
	pub output_refs: Vec<ColdShardRef>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReclaimFdbJobOutput {
	pub status: CompactionJobStatus,
	pub output_refs: Vec<ReclaimOutputRef>,
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
	pub orphan_cold_objects: Vec<ColdShardRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteOrphanColdObjectsOutput {
	pub status: CompactionJobStatus,
	pub deleted_object_keys: Vec<String>,
}

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshManagerOutput {
	pub planning_deadlines: ManagerPlanningDeadlines,
	pub planned_hot_job: Option<PlannedHotCompactionJob>,
	pub planned_cold_job: Option<PlannedColdCompactionJob>,
	pub planned_reclaim_job: Option<PlannedReclaimCompactionJob>,
	pub observed_dirty: Option<SqliteCmpDirty>,
	pub head_txid: Option<u64>,
	pub branch_is_live: bool,
	pub branch_lifecycle_generation: Option<u64>,
	pub db_pin_count: usize,
	#[serde(default)]
	pub reclaim_noop_reason: Option<String>,
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

	pub(crate) fn pending_work(&self) -> ForceCompactionWork {
		self.pending_requests
			.iter()
			.fold(ForceCompactionWork::default(), |acc, request| {
				acc.union(request.requested_work)
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
				if let CompactionJobStatus::Failed { error } = status {
					request.terminal_error = Some(error.clone());
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
			if Self::request_has_active_work(active_jobs, request.requested_work) {
				remaining_requests.push(request);
				continue;
			}

			if Self::request_has_planned_work(refresh, request.requested_work) {
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
		if request.requested_work.cold
			&& !request
				.attempted_job_kinds
				.contains(&CompactionJobKind::Cold)
		{
			reasons.push("cold:no-actionable-lag".to_string());
		}
		if request.requested_work.reclaim
			&& !request
				.attempted_job_kinds
				.contains(&CompactionJobKind::Reclaim)
		{
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
		if requested_work.cold && self.cold.is_some() {
			job_kinds.push(CompactionJobKind::Cold);
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
	pub(crate) cold_inputs: ColdInputSnapshot,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PitrIntervalSelection {
	pub(crate) bucket_start_ms: i64,
	pub(crate) coverage: PitrIntervalCoverage,
}

#[derive(Debug, Default)]
pub(crate) struct ColdInputSnapshot {
	pub(crate) commits: Vec<(u64, CommitRow)>,
	pub(crate) shard_blobs: Vec<ColdShardBlob>,
	pub(crate) total_value_bytes: u64,
	pub(crate) min_versionstamp: [u8; 16],
	pub(crate) max_versionstamp: [u8; 16],
}

#[derive(Debug, Clone)]
pub(crate) struct ColdShardBlob {
	pub(crate) shard_id: u32,
	pub(crate) as_of_txid: u64,
	pub(crate) key: Vec<u8>,
	pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Default)]
pub(crate) struct ReclaimInputSnapshot {
	pub(crate) txid_refs: Vec<ReclaimTxidRef>,
	pub(crate) cold_object_refs: Vec<ReclaimColdObjectRef>,
	pub(crate) shard_cache_evictions: Vec<ShardCacheEvictionCandidate>,
	pub(crate) expired_pitr_interval_rows: Vec<(i64, Vec<u8>, Vec<u8>, PitrIntervalCoverage)>,
	pub(crate) commits: Vec<(u64, Vec<u8>, Vec<u8>, CommitRow)>,
	pub(crate) delta_chunks: Vec<(Vec<u8>, Vec<u8>)>,
	pub(crate) pidx_entries: Vec<(Vec<u8>, Vec<u8>)>,
	pub(crate) coverage_shards: Vec<(Vec<u8>, Vec<u8>)>,
	pub(crate) required_coverage_shard_count: usize,
	pub(crate) total_value_bytes: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ShardCacheEvictionCandidate {
	pub(crate) reference: ShardCacheEvictionRef,
	pub(crate) shard_key: Vec<u8>,
	pub(crate) shard_bytes: Vec<u8>,
	pub(crate) cold_ref_key: Vec<u8>,
	pub(crate) cold_ref_bytes: Vec<u8>,
}

gas::prelude::join_signal!(pub DbManagerSignal {
	DeltasAvailable,
	HotJobFinished,
	ColdJobFinished,
	ReclaimJobFinished,
	ForceCompaction,
	DestroyDatabaseBranch,
});

gas::prelude::join_signal!(pub DbHotCompacterSignal {
	RunHotJob,
	DestroyDatabaseBranch,
});

gas::prelude::join_signal!(pub DbColdCompacterSignal {
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
			DbManagerSignal::ColdJobFinished(signal) => signal.database_branch_id,
			DbManagerSignal::ReclaimJobFinished(signal) => signal.database_branch_id,
			DbManagerSignal::ForceCompaction(signal) => signal.database_branch_id,
			DbManagerSignal::DestroyDatabaseBranch(signal) => signal.database_branch_id,
		}
	}
}

impl DbHotCompacterSignal {
	pub fn database_branch_id(&self) -> DatabaseBranchId {
		match self {
			DbHotCompacterSignal::RunHotJob(signal) => signal.database_branch_id,
			DbHotCompacterSignal::DestroyDatabaseBranch(signal) => signal.database_branch_id,
		}
	}
}

impl DbColdCompacterSignal {
	pub fn database_branch_id(&self) -> DatabaseBranchId {
		match self {
			DbColdCompacterSignal::RunColdJob(signal) => signal.database_branch_id,
			DbColdCompacterSignal::DestroyDatabaseBranch(signal) => signal.database_branch_id,
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
