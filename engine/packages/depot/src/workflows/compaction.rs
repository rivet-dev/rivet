use std::{
	collections::{BTreeMap, BTreeSet},
	env,
	sync::Arc,
};

use anyhow::{Context, Result, bail, ensure};
use futures_util::{FutureExt, TryStreamExt};
use gas::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::{
		IsolationLevel::{Serializable, Snapshot},
		end_of_key_range,
	},
};
use vbare::OwnedVersionedData;

use crate::{
	CMP_COLD_OBJECT_DELETE_GRACE_MS, CMP_FDB_BATCH_MAX_KEYS, CMP_FDB_BATCH_MAX_VALUE_BYTES,
	CMP_S3_DELETE_MAX_OBJECTS, CMP_S3_UPLOAD_LIMIT_BYTES, CMP_S3_UPLOAD_MAX_OBJECTS,
	MAX_NAMESPACE_DEPTH,
	HOT_BURST_COLD_LAG_THRESHOLD_TXIDS,
	conveyer::{
		history_pin, keys, quota,
		ltx::{DecodedLtx, LtxHeader, decode_ltx_v3, encode_ltx_v3},
		types::{
			BranchState, ColdShardRef, CommitRow, CompactionRoot, DBHead, DatabaseBranchId,
			DatabaseBranchRecord, DbHistoryPin, DirtyPage, NamespaceCatalogDbFact,
			NamespaceForkFact, RetiredColdObject, RetiredColdObjectDeleteState, SqliteCmpDirty,
			decode_cold_shard_ref, decode_commit_row,
			decode_compaction_root, decode_database_branch_record, decode_db_head,
			decode_namespace_catalog_db_fact, decode_namespace_fork_fact,
			decode_retired_cold_object, decode_sqlite_cmp_dirty, encode_cold_shard_ref,
			encode_compaction_root, encode_retired_cold_object,
		},
		udb,
	},
	cold_tier::{ColdTier, DisabledColdTier, FilesystemColdTier, S3ColdTier},
};

pub const SQLITE_COMPACTION_WORKFLOW_PAYLOAD_VERSION: u16 = 1;
pub const DATABASE_BRANCH_ID_TAG: &str = "database_branch_id";

pub type CompactionInputFingerprint = [u8; 32];

#[cfg(debug_assertions)]
lazy_static::lazy_static! {
	static ref WORKFLOW_TEST_COLD_TIER: parking_lot::Mutex<Option<Arc<dyn ColdTier>>> =
		parking_lot::Mutex::new(None);
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbManagerInput {
	pub database_branch_id: DatabaseBranchId,
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
pub struct StagedHotShardCleanupRef {
	pub job_id: Id,
	pub output_ref: HotShardOutputRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlannedInputRange {
	Hot(HotJobInputRange),
	Cold(ColdJobInputRange),
	Reclaim(ReclaimJobInputRange),
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
	pub active_hot_job: Option<ActiveCompactionJob>,
	pub active_cold_job: Option<ActiveCompactionJob>,
	pub active_reclaim_job: Option<ActiveCompactionJob>,
	pub retry_cursors: ManagerRetryCursors,
	pub planning_deadlines: ManagerPlanningDeadlines,
	pub branch_stop_state: BranchStopState,
	pub last_dirty_cursor: Option<DirtyCursor>,
}

impl DbManagerState {
	pub fn new(companion_workflow_ids: CompanionWorkflowIds) -> Self {
		DbManagerState {
			companion_workflow_ids,
			active_hot_job: None,
			active_cold_job: None,
			active_reclaim_job: None,
			retry_cursors: ManagerRetryCursors::default(),
			planning_deadlines: ManagerPlanningDeadlines::default(),
			branch_stop_state: BranchStopState::Running,
			last_dirty_cursor: None,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanionWorkflowIds {
	pub hot_compacter_workflow_id: Option<Id>,
	pub cold_compacter_workflow_id: Option<Id>,
	pub reclaimer_workflow_id: Option<Id>,
}

impl CompanionWorkflowIds {
	pub fn new(
		hot_compacter_workflow_id: Id,
		cold_compacter_workflow_id: Id,
		reclaimer_workflow_id: Id,
	) -> Self {
		CompanionWorkflowIds {
			hot_compacter_workflow_id: Some(hot_compacter_workflow_id),
			cold_compacter_workflow_id: Some(cold_compacter_workflow_id),
			reclaimer_workflow_id: Some(reclaimer_workflow_id),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveCompactionJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_lifecycle_generation: u64,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub input_range: PlannedInputRange,
	pub planned_at_ms: i64,
	pub attempt: u32,
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
	fn after_refresh(now_ms: i64) -> Self {
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
	DestroyRequested {
		lifecycle_generation: u64,
		requested_at_ms: i64,
		reason: String,
	},
	Stopping {
		lifecycle_generation: u64,
		requested_at_ms: i64,
		reason: String,
	},
	Stopped { stopped_at_ms: i64 },
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshManagerOutput {
	pub planning_deadlines: ManagerPlanningDeadlines,
	pub planned_hot_job: Option<ActiveCompactionJob>,
	pub planned_cold_job: Option<ActiveCompactionJob>,
	pub planned_reclaim_job: Option<ActiveCompactionJob>,
	pub observed_dirty: Option<SqliteCmpDirty>,
	pub head_txid: Option<u64>,
	pub branch_is_live: bool,
	pub branch_lifecycle_generation: Option<u64>,
	pub db_pin_count: usize,
}

#[derive(Debug)]
struct ManagerFdbSnapshot {
	branch_record: Option<DatabaseBranchRecord>,
	head: Option<DBHead>,
	root: CompactionRoot,
	dirty: Option<SqliteCmpDirty>,
	db_pins: Vec<DbHistoryPin>,
	hot_inputs: HotInputSnapshot,
	cold_inputs: ColdInputSnapshot,
	reclaim_inputs: ReclaimInputSnapshot,
	namespace_proof_blocked_reclaim: bool,
	cleared_dirty: bool,
}

#[derive(Debug, Default)]
struct HotInputSnapshot {
	commits: Vec<(u64, CommitRow)>,
	delta_chunks: Vec<(Vec<u8>, Vec<u8>)>,
	pidx_entries: Vec<(Vec<u8>, Vec<u8>)>,
	total_value_bytes: u64,
}

#[derive(Debug, Default)]
struct ColdInputSnapshot {
	commits: Vec<(u64, CommitRow)>,
	shard_blobs: Vec<ColdShardBlob>,
	total_value_bytes: u64,
	min_versionstamp: [u8; 16],
	max_versionstamp: [u8; 16],
}

#[derive(Debug, Clone)]
struct ColdShardBlob {
	shard_id: u32,
	as_of_txid: u64,
	key: Vec<u8>,
	bytes: Vec<u8>,
}

#[derive(Debug, Default)]
struct ReclaimInputSnapshot {
	txid_refs: Vec<ReclaimTxidRef>,
	cold_object_refs: Vec<ReclaimColdObjectRef>,
	commits: Vec<(u64, Vec<u8>, Vec<u8>, CommitRow)>,
	delta_chunks: Vec<(Vec<u8>, Vec<u8>)>,
	pidx_entries: Vec<(Vec<u8>, Vec<u8>)>,
	coverage_shards: Vec<(Vec<u8>, Vec<u8>)>,
	required_coverage_shard_count: usize,
	total_value_bytes: u64,
}

gas::prelude::join_signal!(pub DbManagerSignal {
	DeltasAvailable,
	HotJobFinished,
	ColdJobFinished,
	ReclaimJobFinished,
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

pub fn database_branch_tag_value(database_branch_id: DatabaseBranchId) -> String {
	database_branch_id.as_uuid().to_string()
}

#[workflow(DbManagerWorkflow)]
pub async fn db_manager(ctx: &mut WorkflowCtx, input: &DbManagerInput) -> Result<()> {
	let companion_workflow_ids = dispatch_companion_workflows(ctx, input.database_branch_id).await?;
	let initial_deadline_ms = ctx.create_ts();

	ctx.lupe()
		.commit_interval(1)
		.with_state(DbManagerState::new_with_initial_deadline(
			companion_workflow_ids,
			initial_deadline_ms,
		))
		.run(|ctx, state| {
			let input = input.clone();
			async move {
				let signals = listen_for_manager_signals(ctx, &state.planning_deadlines).await?;

				for signal in signals {
					match signal {
						DbManagerSignal::DeltasAvailable(signal) => {
							if signal.database_branch_id == input.database_branch_id {
								state.last_dirty_cursor = Some(DirtyCursor {
									observed_head_txid: signal.observed_head_txid,
									dirty_updated_at_ms: signal.dirty_updated_at_ms,
								});
							}
						}
						DbManagerSignal::HotJobFinished(signal) => {
							if signal.database_branch_id == input.database_branch_id {
								let active_job = state.active_hot_job.clone();
								if let Some(active_job) = active_job.as_ref()
									&& hot_job_finished_matches_active(&signal, active_job)
								{
									match &signal.status {
										CompactionJobStatus::Requested => {}
										CompactionJobStatus::Succeeded => {
											let input_range = match active_job.input_range.clone() {
												PlannedInputRange::Hot(input_range) => input_range,
												PlannedInputRange::Cold(_)
												| PlannedInputRange::Reclaim(_) => {
													bail!("active hot job carried non-hot input range")
												}
											};
											let install = ctx
												.activity(InstallHotJobInput {
													database_branch_id: signal.database_branch_id,
													job_id: signal.job_id,
													job_kind: signal.job_kind,
													base_lifecycle_generation: active_job
														.base_lifecycle_generation,
													base_manifest_generation: signal
														.base_manifest_generation,
													input_fingerprint: signal.input_fingerprint,
													input_range,
													output_refs: signal.output_refs.clone(),
												})
												.await?;
											match install.status {
												CompactionJobStatus::Requested => {}
												CompactionJobStatus::Succeeded
												| CompactionJobStatus::Rejected { .. }
												| CompactionJobStatus::Failed { .. } => {
													state.active_hot_job = None
												}
											}
										}
										CompactionJobStatus::Rejected { .. }
										| CompactionJobStatus::Failed { .. } => {
											state.active_hot_job = None
										}
									}
								} else {
									schedule_stale_hot_output_cleanup(ctx, state, &signal).await?;
								}
							}
						}
						DbManagerSignal::ColdJobFinished(signal) => {
							if signal.database_branch_id == input.database_branch_id {
								let active_job = state.active_cold_job.clone();
								if let Some(active_job) = active_job.as_ref()
									&& cold_job_finished_matches_active(&signal, active_job)
								{
									match &signal.status {
										CompactionJobStatus::Requested => {}
										CompactionJobStatus::Succeeded => {
											let input_range = match active_job.input_range.clone() {
												PlannedInputRange::Cold(input_range) => input_range,
												PlannedInputRange::Hot(_)
												| PlannedInputRange::Reclaim(_) => {
													bail!("active cold job carried non-cold input range")
												}
											};
											let publish = ctx
												.activity(PublishColdJobInput {
													database_branch_id: signal.database_branch_id,
													job_id: signal.job_id,
													job_kind: signal.job_kind,
													base_lifecycle_generation: active_job
														.base_lifecycle_generation,
													base_manifest_generation: signal
														.base_manifest_generation,
													input_fingerprint: signal.input_fingerprint,
													input_range,
													output_refs: signal.output_refs.clone(),
												})
												.await?;
											match publish.status {
												CompactionJobStatus::Requested => {}
												CompactionJobStatus::Succeeded
												| CompactionJobStatus::Rejected { .. }
												| CompactionJobStatus::Failed { .. } => {
													state.active_cold_job = None
												}
											}
										}
										CompactionJobStatus::Rejected { .. }
										| CompactionJobStatus::Failed { .. } => {
											state.active_cold_job = None
										}
									}
								} else {
									schedule_stale_cold_output_cleanup(ctx, state, &signal).await?;
								}
							}
						}
						DbManagerSignal::ReclaimJobFinished(signal) => {
							if signal.database_branch_id == input.database_branch_id {
								if let Some(active_job) = state.active_reclaim_job.as_ref() {
									if reclaim_job_finished_matches_active(&signal, active_job) {
										match signal.status {
											CompactionJobStatus::Requested => {}
											CompactionJobStatus::Succeeded
											| CompactionJobStatus::Rejected { .. }
											| CompactionJobStatus::Failed { .. } => {
												state.active_reclaim_job = None
											}
										}
									}
								}
							}
						}
						DbManagerSignal::DestroyDatabaseBranch(signal) => {
							if signal.database_branch_id == input.database_branch_id {
								state.branch_stop_state = BranchStopState::DestroyRequested {
									lifecycle_generation: signal.lifecycle_generation,
									requested_at_ms: signal.requested_at_ms,
									reason: signal.reason,
								};
							}
						}
					}
				}

				if let BranchStopState::DestroyRequested {
					lifecycle_generation,
					requested_at_ms,
					reason,
				} = state.branch_stop_state.clone()
				{
					signal_companions_destroy(
						ctx,
						input.database_branch_id,
						&state.companion_workflow_ids,
						lifecycle_generation,
						requested_at_ms,
						reason.clone(),
					)
					.await?;
					state.active_hot_job = None;
					state.active_cold_job = None;
					state.active_reclaim_job = None;
					state.branch_stop_state = BranchStopState::Stopped {
						stopped_at_ms: ctx.create_ts(),
					};
					return Ok(Loop::Break(()));
				}

				let refresh = ctx
					.activity(RefreshManagerInput {
						database_branch_id: input.database_branch_id,
					})
					.await?;
				state.planning_deadlines = refresh.planning_deadlines;
				if state.last_dirty_cursor.is_none()
					&& let Some(dirty) = refresh.observed_dirty
				{
					state.last_dirty_cursor = Some(DirtyCursor {
						observed_head_txid: dirty.observed_head_txid,
						dirty_updated_at_ms: dirty.updated_at_ms,
					});
				}
				if !refresh.branch_is_live
					&& matches!(state.branch_stop_state, BranchStopState::Running)
				{
					let lifecycle_generation =
						refresh.branch_lifecycle_generation.unwrap_or_default();
					signal_companions_destroy(
						ctx,
						input.database_branch_id,
						&state.companion_workflow_ids,
						lifecycle_generation,
						ctx.create_ts(),
						"database branch is not live".to_string(),
					)
					.await?;
					state.active_hot_job = None;
					state.active_cold_job = None;
					state.active_reclaim_job = None;
					state.branch_stop_state = BranchStopState::Stopped {
						stopped_at_ms: ctx.create_ts(),
					};
					return Ok(Loop::Break(()));
				}

				if state.active_hot_job.is_none()
					&& matches!(state.branch_stop_state, BranchStopState::Running)
				{
					if let Some(active_job) = refresh.planned_hot_job {
						let hot_compacter_workflow_id = state
							.companion_workflow_ids
							.hot_compacter_workflow_id
							.context("hot compacter workflow id missing from manager state")?;
						let input_range = match active_job.input_range.clone() {
							PlannedInputRange::Hot(input_range) => input_range,
							PlannedInputRange::Cold(_) | PlannedInputRange::Reclaim(_) => {
								bail!("planned hot job carried non-hot input range")
							}
						};

						ctx.signal(RunHotJob {
							database_branch_id: active_job.database_branch_id,
							job_id: active_job.job_id,
							job_kind: CompactionJobKind::Hot,
							base_lifecycle_generation: active_job.base_lifecycle_generation,
							base_manifest_generation: active_job.base_manifest_generation,
							input_fingerprint: active_job.input_fingerprint,
							status: CompactionJobStatus::Requested,
							input_range,
						})
						.to_workflow_id(hot_compacter_workflow_id)
						.send()
						.await?;

						state.active_hot_job = Some(active_job);
					}
				}

				if state.active_cold_job.is_none()
					&& matches!(state.branch_stop_state, BranchStopState::Running)
				{
					if let Some(active_job) = refresh.planned_cold_job {
						let cold_compacter_workflow_id = state
							.companion_workflow_ids
							.cold_compacter_workflow_id
							.context("cold compacter workflow id missing from manager state")?;
						let input_range = match active_job.input_range.clone() {
							PlannedInputRange::Cold(input_range) => input_range,
							PlannedInputRange::Hot(_) | PlannedInputRange::Reclaim(_) => {
								bail!("planned cold job carried non-cold input range")
							}
						};

						ctx.signal(RunColdJob {
							database_branch_id: active_job.database_branch_id,
							job_id: active_job.job_id,
							job_kind: CompactionJobKind::Cold,
							base_lifecycle_generation: active_job.base_lifecycle_generation,
							base_manifest_generation: active_job.base_manifest_generation,
							input_fingerprint: active_job.input_fingerprint,
							status: CompactionJobStatus::Requested,
							input_range,
						})
						.to_workflow_id(cold_compacter_workflow_id)
						.send()
						.await?;

						state.active_cold_job = Some(active_job);
					}
				}

				if state.active_reclaim_job.is_none()
					&& state.active_cold_job.is_none()
					&& matches!(state.branch_stop_state, BranchStopState::Running)
				{
					if let Some(active_job) = refresh.planned_reclaim_job {
						let reclaimer_workflow_id = state
							.companion_workflow_ids
							.reclaimer_workflow_id
							.context("reclaimer workflow id missing from manager state")?;
						let input_range = match active_job.input_range.clone() {
							PlannedInputRange::Reclaim(input_range) => input_range,
							PlannedInputRange::Hot(_) | PlannedInputRange::Cold(_) => {
								bail!("planned reclaim job carried non-reclaim input range")
							}
						};

						ctx.signal(RunReclaimJob {
							database_branch_id: active_job.database_branch_id,
							job_id: active_job.job_id,
							job_kind: CompactionJobKind::Reclaim,
							base_lifecycle_generation: active_job.base_lifecycle_generation,
							base_manifest_generation: active_job.base_manifest_generation,
							input_fingerprint: active_job.input_fingerprint,
							status: CompactionJobStatus::Requested,
							input_range,
						})
						.to_workflow_id(reclaimer_workflow_id)
						.send()
						.await?;

						state.active_reclaim_job = Some(active_job);
					}
				}

				Ok(Loop::Continue)
			}
			.boxed()
		})
		.await
}

#[workflow(DbHotCompacterWorkflow)]
pub async fn db_hot_compacter(ctx: &mut WorkflowCtx, input: &DbHotCompacterInput) -> Result<()> {
	run_companion_loop(ctx, input.database_branch_id, CompanionKind::Hot).await
}

#[workflow(DbColdCompacterWorkflow)]
pub async fn db_cold_compacter(ctx: &mut WorkflowCtx, input: &DbColdCompacterInput) -> Result<()> {
	run_companion_loop(ctx, input.database_branch_id, CompanionKind::Cold).await
}

#[workflow(DbReclaimerWorkflow)]
pub async fn db_reclaimer(ctx: &mut WorkflowCtx, input: &DbReclaimerInput) -> Result<()> {
	run_companion_loop(ctx, input.database_branch_id, CompanionKind::Reclaim).await
}

async fn dispatch_companion_workflows(
	ctx: &mut WorkflowCtx,
	database_branch_id: DatabaseBranchId,
) -> Result<CompanionWorkflowIds> {
	let tag_value = database_branch_tag_value(database_branch_id);

	let hot_compacter_workflow_id = ctx
		.workflow(DbHotCompacterInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let cold_compacter_workflow_id = ctx
		.workflow(DbColdCompacterInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let reclaimer_workflow_id = ctx
		.workflow(DbReclaimerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;

	Ok(CompanionWorkflowIds::new(
		hot_compacter_workflow_id,
		cold_compacter_workflow_id,
		reclaimer_workflow_id,
	))
}

async fn signal_companions_destroy(
	ctx: &mut WorkflowCtx,
	database_branch_id: DatabaseBranchId,
	companion_workflow_ids: &CompanionWorkflowIds,
	lifecycle_generation: u64,
	requested_at_ms: i64,
	reason: String,
) -> Result<()> {
	let destroy = DestroyDatabaseBranch {
		database_branch_id,
		lifecycle_generation,
		requested_at_ms,
		reason,
	};

	let hot_compacter_workflow_id = companion_workflow_ids
		.hot_compacter_workflow_id
		.context("hot compacter workflow id missing from manager state")?;
	ctx.signal(destroy.clone())
		.to_workflow_id(hot_compacter_workflow_id)
		.send()
		.await?;

	let cold_compacter_workflow_id = companion_workflow_ids
		.cold_compacter_workflow_id
		.context("cold compacter workflow id missing from manager state")?;
	ctx.signal(destroy.clone())
		.to_workflow_id(cold_compacter_workflow_id)
		.send()
		.await?;

	let reclaimer_workflow_id = companion_workflow_ids
		.reclaimer_workflow_id
		.context("reclaimer workflow id missing from manager state")?;
	ctx.signal(destroy)
		.to_workflow_id(reclaimer_workflow_id)
		.send()
		.await?;

	Ok(())
}

async fn listen_for_manager_signals(
	ctx: &mut WorkflowCtx,
	planning_deadlines: &ManagerPlanningDeadlines,
) -> Result<Vec<DbManagerSignal>> {
	if let Some(deadline) = nearest_planning_deadline(planning_deadlines) {
		ctx.listen_n_until::<DbManagerSignal>(deadline, 256).await
	} else {
		ctx.listen_n::<DbManagerSignal>(256).await
	}
}

fn nearest_planning_deadline(planning_deadlines: &ManagerPlanningDeadlines) -> Option<i64> {
	[
		planning_deadlines.next_hot_check_at_ms,
		planning_deadlines.next_cold_check_at_ms,
		planning_deadlines.next_reclaim_check_at_ms,
		planning_deadlines.final_settle_check_at_ms,
	]
	.into_iter()
	.flatten()
	.min()
}

#[activity(RefreshManager)]
pub async fn refresh_manager(
	ctx: &ActivityCtx,
	input: &RefreshManagerInput,
) -> Result<RefreshManagerOutput> {
	let now_ms = ctx.ts();
	let database_branch_id = input.database_branch_id;
	let snapshot = ctx
		.udb()?
		.run(move |tx| async move { read_manager_fdb_snapshot(&tx, database_branch_id).await })
		.await?;
	let branch_is_live = snapshot
		.branch_record
		.as_ref()
		.is_some_and(|record| record.state == BranchState::Live);
	let branch_lifecycle_generation = snapshot
		.branch_record
		.as_ref()
		.map(|record| record.lifecycle_generation);
	let head_txid = snapshot.head.as_ref().map(|head| head.head_txid);
	let planned_hot_job = if branch_is_live {
		plan_hot_job(
			database_branch_id,
			&snapshot,
			Id::new_v1(ctx.config().dc_label()),
			now_ms,
		)
	} else {
		None
	};
	let planned_cold_job = if branch_is_live {
		plan_cold_job(
			database_branch_id,
			&snapshot,
			Id::new_v1(ctx.config().dc_label()),
			now_ms,
		)
	} else {
		None
	};
	let planned_reclaim_job = if branch_is_live {
		plan_reclaim_job(
			database_branch_id,
			&snapshot,
			Id::new_v1(ctx.config().dc_label()),
			now_ms,
		)
	} else {
		None
	};

	Ok(RefreshManagerOutput {
		planning_deadlines: ManagerPlanningDeadlines::after_refresh(now_ms),
		planned_hot_job,
		planned_cold_job,
		planned_reclaim_job,
		observed_dirty: if snapshot.cleared_dirty {
			None
		} else {
			snapshot.dirty
		},
		head_txid,
		branch_is_live,
		branch_lifecycle_generation,
		db_pin_count: snapshot.db_pins.len(),
	})
}

fn hot_job_finished_matches_active(
	signal: &HotJobFinished,
	active_job: &ActiveCompactionJob,
) -> bool {
	signal.job_id == active_job.job_id
		&& signal.job_kind == active_job.job_kind
		&& signal.base_manifest_generation == active_job.base_manifest_generation
		&& signal.input_fingerprint == active_job.input_fingerprint
}

fn cold_job_finished_matches_active(
	signal: &ColdJobFinished,
	active_job: &ActiveCompactionJob,
) -> bool {
	signal.job_id == active_job.job_id
		&& signal.job_kind == active_job.job_kind
		&& signal.base_manifest_generation == active_job.base_manifest_generation
		&& signal.input_fingerprint == active_job.input_fingerprint
}

fn reclaim_job_finished_matches_active(
	signal: &ReclaimJobFinished,
	active_job: &ActiveCompactionJob,
) -> bool {
	signal.job_id == active_job.job_id
		&& signal.job_kind == active_job.job_kind
		&& signal.base_manifest_generation == active_job.base_manifest_generation
		&& signal.input_fingerprint == active_job.input_fingerprint
}

async fn schedule_stale_hot_output_cleanup(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	signal: &HotJobFinished,
) -> Result<()> {
	if !matches!(signal.status, CompactionJobStatus::Succeeded) || signal.output_refs.is_empty() {
		return Ok(());
	}

	let staged_hot_shards = signal
		.output_refs
		.iter()
		.cloned()
		.map(|output_ref| StagedHotShardCleanupRef {
			job_id: signal.job_id,
			output_ref,
		})
		.collect::<Vec<_>>();
	let input_range = repair_reclaim_input_range(
		staged_hot_shards,
		Vec::new(),
		signal.output_refs.iter().map(|output_ref| output_ref.as_of_txid),
	);

	schedule_repair_reclaim_job(
		ctx,
		state,
		signal.database_branch_id,
		signal.base_manifest_generation,
		input_range,
		signal.job_id,
		"cleanup_stale_hot_output",
	)
	.await
}

async fn schedule_stale_cold_output_cleanup(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	signal: &ColdJobFinished,
) -> Result<()> {
	if !matches!(signal.status, CompactionJobStatus::Succeeded) || signal.output_refs.is_empty() {
		return Ok(());
	}

	let input_range = repair_reclaim_input_range(
		Vec::new(),
		signal.output_refs.clone(),
		signal.output_refs.iter().map(|output_ref| output_ref.as_of_txid),
	);
	schedule_repair_reclaim_job(
		ctx,
		state,
		signal.database_branch_id,
		signal.base_manifest_generation,
		input_range,
		signal.job_id,
		"delete_stale_cold_output",
	)
	.await
}

fn repair_reclaim_input_range(
	staged_hot_shards: Vec<StagedHotShardCleanupRef>,
	orphan_cold_objects: Vec<ColdShardRef>,
	txids: impl Iterator<Item = u64>,
) -> ReclaimJobInputRange {
	let mut min_txid = u64::MAX;
	let mut max_txid = 0_u64;
	for txid in txids {
		min_txid = min_txid.min(txid);
		max_txid = max_txid.max(txid);
	}
	if min_txid == u64::MAX {
		min_txid = 0;
	}

	ReclaimJobInputRange {
		txids: TxidRange { min_txid, max_txid },
		txid_refs: Vec::new(),
		cold_objects: Vec::new(),
		staged_hot_shards,
		orphan_cold_objects,
		max_keys: CMP_FDB_BATCH_MAX_KEYS as u32,
		max_bytes: CMP_FDB_BATCH_MAX_VALUE_BYTES as u64,
	}
}

async fn schedule_repair_reclaim_job(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	database_branch_id: DatabaseBranchId,
	base_manifest_generation: u64,
	input_range: ReclaimJobInputRange,
	source_job_id: Id,
	repair_action: &'static str,
) -> Result<()> {
	if state.active_reclaim_job.is_some() {
		tracing::warn!(
			?database_branch_id,
			manifest_generation = base_manifest_generation,
			?source_job_id,
			repair_action,
			"stale compaction output cleanup deferred because reclaimer is busy"
		);
		return Ok(());
	}

	let reclaimer_workflow_id = state
		.companion_workflow_ids
		.reclaimer_workflow_id
		.context("reclaimer workflow id missing from manager state")?;
	let cleanup_job_id = Id::new_v1(ctx.config().dc_label());
	let input_fingerprint =
		fingerprint_repair_reclaim_range(database_branch_id, &input_range);
	tracing::warn!(
		?database_branch_id,
		manifest_generation = base_manifest_generation,
		?source_job_id,
		?cleanup_job_id,
		repair_action,
		staged_hot_shard_count = input_range.staged_hot_shards.len(),
		orphan_cold_object_count = input_range.orphan_cold_objects.len(),
		"scheduled stale compaction output cleanup"
	);

	ctx.signal(RunReclaimJob {
		database_branch_id,
		job_id: cleanup_job_id,
		job_kind: CompactionJobKind::Reclaim,
		base_lifecycle_generation: 0,
		base_manifest_generation,
		input_fingerprint,
		status: CompactionJobStatus::Requested,
		input_range: input_range.clone(),
	})
	.to_workflow_id(reclaimer_workflow_id)
	.send()
	.await?;

	state.active_reclaim_job = Some(ActiveCompactionJob {
		database_branch_id,
		job_id: cleanup_job_id,
		job_kind: CompactionJobKind::Reclaim,
		base_lifecycle_generation: 0,
		base_manifest_generation,
		input_fingerprint,
		input_range: PlannedInputRange::Reclaim(input_range),
		planned_at_ms: ctx.create_ts(),
		attempt: 0,
	});

	Ok(())
}

fn branch_record_is_live_at_generation(
	branch_record: Option<&DatabaseBranchRecord>,
	lifecycle_generation: u64,
) -> bool {
	branch_record.is_some_and(|record| {
		record.state == BranchState::Live && record.lifecycle_generation == lifecycle_generation
	})
}

#[activity(StageHotJob)]
pub async fn stage_hot_job(
	ctx: &ActivityCtx,
	input: &StageHotJobInput,
) -> Result<StageHotJobOutput> {
	let input = input.clone();

	ctx.udb()?
		.run(move |tx| {
			let input = input.clone();
			async move { stage_hot_job_tx(&tx, &input).await }
		})
		.await
}

async fn stage_hot_job_tx(
	tx: &universaldb::Transaction,
	input: &StageHotJobInput,
) -> Result<StageHotJobOutput> {
	if input.job_kind != CompactionJobKind::Hot {
		return Ok(rejected_hot_job("hot compacter received a non-hot job"));
	}

	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for hot compaction")?;
	if !branch_record_is_live_at_generation(
		branch_record.as_ref(),
		input.base_lifecycle_generation,
	) {
		return Ok(rejected_hot_job("database branch lifecycle changed"));
	}

	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for hot compaction")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	});
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(rejected_hot_job("base manifest generation changed"));
	}

	let Some(head) = tx_get_value(
		tx,
		&keys::branch_meta_head_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_db_head)
	.transpose()
	.context("decode sqlite head for hot compaction")?
	else {
		return Ok(rejected_hot_job("database branch head is missing"));
	};

	let db_pins =
		history_pin::read_db_history_pins(tx, input.database_branch_id, Snapshot).await?;
	let coverage_txids = selected_hot_coverage_txids(&root, &head, &db_pins);
	if coverage_txids != input.input_range.coverage_txids {
		return Ok(rejected_hot_job("hot compaction coverage targets changed"));
	}

	let hot_inputs =
		read_hot_input_snapshot(tx, input.database_branch_id, Some(&head), &root, Snapshot)
			.await?;
	let input_fingerprint = fingerprint_hot_inputs(
		input.database_branch_id,
		&root,
		&head,
		&coverage_txids,
		&hot_inputs,
	);
	if input_fingerprint != input.input_fingerprint {
		return Ok(rejected_hot_job("hot compaction input fingerprint changed"));
	}

	let output_refs = write_staged_hot_shards(tx, input, &head, &hot_inputs).await?;

	Ok(StageHotJobOutput {
		status: CompactionJobStatus::Succeeded,
		output_refs,
	})
}

fn rejected_hot_job(reason: impl Into<String>) -> StageHotJobOutput {
	StageHotJobOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		output_refs: Vec::new(),
	}
}

#[activity(InstallHotJob)]
pub async fn install_hot_job(
	ctx: &ActivityCtx,
	input: &InstallHotJobInput,
) -> Result<InstallHotJobOutput> {
	let input = input.clone();

	ctx.udb()?
		.run(move |tx| {
			let input = input.clone();
			async move { install_hot_job_tx(&tx, &input).await }
		})
		.await
}

async fn install_hot_job_tx(
	tx: &universaldb::Transaction,
	input: &InstallHotJobInput,
) -> Result<InstallHotJobOutput> {
	if input.job_kind != CompactionJobKind::Hot {
		return Ok(rejected_hot_install("manager received a non-hot job"));
	}

	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for hot install")?;
	if !branch_record_is_live_at_generation(
		branch_record.as_ref(),
		input.base_lifecycle_generation,
	) {
		return Ok(rejected_hot_install("database branch lifecycle changed"));
	}

	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for hot install")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	});
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(rejected_hot_install("base manifest generation changed"));
	}

	let Some(head) = tx_get_value(
		tx,
		&keys::branch_meta_head_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_db_head)
	.transpose()
	.context("decode sqlite head for hot install")?
	else {
		return Ok(rejected_hot_install("database branch head is missing"));
	};

	let mut db_pins =
		history_pin::read_db_history_pins(tx, input.database_branch_id, Serializable).await?;
	if resolve_namespace_fork_pins(tx, input.database_branch_id, &mut db_pins).await? {
		return Ok(rejected_hot_install("namespace fork proof is ambiguous"));
	}
	let coverage_txids = selected_hot_coverage_txids(&root, &head, &db_pins);
	if coverage_txids != input.input_range.coverage_txids {
		return Ok(rejected_hot_install("hot compaction coverage targets changed"));
	}

	let hot_inputs = read_hot_input_snapshot(
		tx,
		input.database_branch_id,
		Some(&head),
		&root,
		Serializable,
	)
	.await?;
	let input_fingerprint = fingerprint_hot_inputs(
		input.database_branch_id,
		&root,
		&head,
		&coverage_txids,
		&hot_inputs,
	);
	if input_fingerprint != input.input_fingerprint {
		return Ok(rejected_hot_install("hot compaction input fingerprint changed"));
	}

	let mut staged_blobs = Vec::with_capacity(input.output_refs.len());
	let mut staged_outputs = BTreeSet::new();
	let mut latest_staged_shards = BTreeSet::new();
	let coverage_txids = input
		.input_range
		.coverage_txids
		.iter()
		.copied()
		.collect::<BTreeSet<_>>();
	for output_ref in &input.output_refs {
		if !coverage_txids.contains(&output_ref.as_of_txid)
			|| output_ref.min_txid != input.input_range.txids.min_txid
			|| output_ref.max_txid != output_ref.as_of_txid
		{
			return Ok(rejected_hot_install("hot output ref does not match planned txid range"));
		}
		if !staged_outputs.insert((output_ref.shard_id, output_ref.as_of_txid)) {
			return Ok(rejected_hot_install("duplicate staged hot shard output ref"));
		}
		if output_ref.as_of_txid == input.input_range.txids.max_txid
			&& !latest_staged_shards.insert(output_ref.shard_id)
		{
			return Ok(rejected_hot_install("duplicate latest hot shard output ref"));
		}

		let stage_key = keys::branch_compaction_stage_hot_shard_key(
			input.database_branch_id,
			input.job_id,
			output_ref.shard_id,
			output_ref.as_of_txid,
			0,
		);
		let Some(staged_blob) = tx_get_value(tx, &stage_key, Serializable).await? else {
			return Ok(rejected_hot_install("staged hot shard is missing"));
		};
		if output_ref.size_bytes != u64::try_from(staged_blob.len()).unwrap_or(u64::MAX)
			|| output_ref.content_hash != content_hash(&staged_blob)
		{
			return Ok(rejected_hot_install("staged hot shard checksum mismatch"));
		}
		staged_blobs.push((output_ref.clone(), staged_blob));
	}

	for (output_ref, staged_blob) in &staged_blobs {
		tx.informal().set(
			&keys::branch_shard_key(
				input.database_branch_id,
				output_ref.shard_id,
				output_ref.as_of_txid,
			),
			staged_blob,
		);
	}

	for (key, value) in &hot_inputs.pidx_entries {
		let pgno = decode_branch_pidx_pgno(input.database_branch_id, key)?;
		let shard_id = pgno / keys::SHARD_SIZE;
		if !latest_staged_shards.contains(&shard_id) {
			return Ok(rejected_hot_install("missing staged hot shard for PIDX row"));
		}
		decode_pidx_txid(value)?;
	}

	for (key, value) in &hot_inputs.pidx_entries {
		udb::compare_and_clear(tx, key, value);
	}

	let next_root = CompactionRoot {
		schema_version: root.schema_version,
		manifest_generation: root.manifest_generation.saturating_add(1),
		hot_watermark_txid: root.hot_watermark_txid.max(input.input_range.txids.max_txid),
		cold_watermark_txid: root.cold_watermark_txid,
		cold_watermark_versionstamp: root.cold_watermark_versionstamp,
	};
	tx.informal().set(
		&keys::branch_compaction_root_key(input.database_branch_id),
		&encode_compaction_root(next_root).context("encode sqlite compaction root for hot install")?,
	);

	Ok(InstallHotJobOutput {
		status: CompactionJobStatus::Succeeded,
		output_refs: input.output_refs.clone(),
	})
}

fn rejected_hot_install(reason: impl Into<String>) -> InstallHotJobOutput {
	InstallHotJobOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		output_refs: Vec::new(),
	}
}

#[activity(UploadColdJob)]
pub async fn upload_cold_job(
	ctx: &ActivityCtx,
	input: &UploadColdJobInput,
) -> Result<UploadColdJobOutput> {
	let input = input.clone();
	let upload = ctx
		.udb()?
		.run(move |tx| {
			let input = input.clone();
			async move { prepare_cold_upload_tx(&tx, &input).await }
		})
		.await?;

	let PreparedColdUpload {
		status,
		output_refs,
		objects,
	} = upload;
	if !matches!(status, CompactionJobStatus::Succeeded) {
		return Ok(UploadColdJobOutput {
			status,
			output_refs: Vec::new(),
		});
	}

	let cold_tier = workflow_cold_tier().await?;
	for object in objects {
		cold_tier
			.put_object(&object.object_key, &object.bytes)
			.await
			.with_context(|| format!("put sqlite workflow cold shard {}", object.object_key))?;
	}

	Ok(UploadColdJobOutput {
		status,
		output_refs,
	})
}

#[derive(Debug)]
struct PreparedColdUpload {
	status: CompactionJobStatus,
	output_refs: Vec<ColdShardRef>,
	objects: Vec<ColdUploadObject>,
}

#[derive(Debug)]
struct ColdUploadObject {
	object_key: String,
	bytes: Vec<u8>,
}

async fn prepare_cold_upload_tx(
	tx: &universaldb::Transaction,
	input: &UploadColdJobInput,
) -> Result<PreparedColdUpload> {
	if input.job_kind != CompactionJobKind::Cold {
		return Ok(rejected_cold_upload("cold compacter received a non-cold job"));
	}

	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for cold upload")?;
	if !branch_record_is_live_at_generation(
		branch_record.as_ref(),
		input.base_lifecycle_generation,
	) {
		return Ok(rejected_cold_upload("database branch lifecycle changed"));
	}

	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for cold upload")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	});
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(rejected_cold_upload("base manifest generation changed"));
	}

	let cold_inputs =
		read_cold_input_snapshot(tx, input.database_branch_id, &root, Serializable).await?;
	if cold_inputs.shard_blobs.is_empty() {
		return Ok(rejected_cold_upload("no cold shard input is available"));
	}
	if cold_inputs.min_versionstamp != input.input_range.min_versionstamp
		|| cold_inputs.max_versionstamp != input.input_range.max_versionstamp
	{
		return Ok(rejected_cold_upload("cold compaction versionstamp bounds changed"));
	}
	let input_fingerprint =
		fingerprint_cold_inputs(input.database_branch_id, &root, &cold_inputs);
	if input_fingerprint != input.input_fingerprint {
		return Ok(rejected_cold_upload("cold compaction input fingerprint changed"));
	}

	let mut output_refs = Vec::with_capacity(cold_inputs.shard_blobs.len());
	let mut objects = Vec::with_capacity(cold_inputs.shard_blobs.len());
	let publish_generation = root.manifest_generation.saturating_add(1);
	for blob in cold_inputs.shard_blobs {
		if blob.as_of_txid != input.input_range.txids.max_txid {
			return Ok(rejected_cold_upload("cold shard txid does not match planned range"));
		}
		let content_hash = content_hash(&blob.bytes);
		let object_key = cold_shard_object_key(
			input.database_branch_id,
			blob.shard_id,
			blob.as_of_txid,
			input.job_id,
			content_hash,
		);
		output_refs.push(ColdShardRef {
			object_key: object_key.clone(),
			object_generation_id: input.job_id,
			shard_id: blob.shard_id,
			as_of_txid: blob.as_of_txid,
			min_txid: input.input_range.txids.min_txid,
			max_txid: blob.as_of_txid,
			min_versionstamp: input.input_range.min_versionstamp,
			max_versionstamp: input.input_range.max_versionstamp,
			size_bytes: u64::try_from(blob.bytes.len()).unwrap_or(u64::MAX),
			content_hash,
			publish_generation,
		});
		objects.push(ColdUploadObject {
			object_key,
			bytes: blob.bytes,
		});
	}

	Ok(PreparedColdUpload {
		status: CompactionJobStatus::Succeeded,
		output_refs,
		objects,
	})
}

fn rejected_cold_upload(reason: impl Into<String>) -> PreparedColdUpload {
	PreparedColdUpload {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		output_refs: Vec::new(),
		objects: Vec::new(),
	}
}

#[activity(PublishColdJob)]
pub async fn publish_cold_job(
	ctx: &ActivityCtx,
	input: &PublishColdJobInput,
) -> Result<PublishColdJobOutput> {
	let input = input.clone();

	ctx.udb()?
		.run(move |tx| {
			let input = input.clone();
			async move { publish_cold_job_tx(&tx, &input).await }
		})
		.await
}

async fn publish_cold_job_tx(
	tx: &universaldb::Transaction,
	input: &PublishColdJobInput,
) -> Result<PublishColdJobOutput> {
	if input.job_kind != CompactionJobKind::Cold {
		return Ok(rejected_cold_publish("manager received a non-cold job"));
	}

	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for cold publish")?;
	if !branch_record_is_live_at_generation(
		branch_record.as_ref(),
		input.base_lifecycle_generation,
	) {
		return Ok(rejected_cold_publish("database branch lifecycle changed"));
	}

	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for cold publish")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	});
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(rejected_cold_publish("base manifest generation changed"));
	}

	let mut db_pins =
		history_pin::read_db_history_pins(tx, input.database_branch_id, Serializable).await?;
	if resolve_namespace_fork_pins(tx, input.database_branch_id, &mut db_pins).await? {
		return Ok(rejected_cold_publish("namespace fork proof is ambiguous"));
	}

	let cold_inputs =
		read_cold_input_snapshot(tx, input.database_branch_id, &root, Serializable).await?;
	if cold_inputs.shard_blobs.is_empty() {
		return Ok(rejected_cold_publish("no cold shard input is available"));
	}
	if cold_inputs.min_versionstamp != input.input_range.min_versionstamp
		|| cold_inputs.max_versionstamp != input.input_range.max_versionstamp
	{
		return Ok(rejected_cold_publish("cold compaction versionstamp bounds changed"));
	}
	let input_fingerprint =
		fingerprint_cold_inputs(input.database_branch_id, &root, &cold_inputs);
	if input_fingerprint != input.input_fingerprint {
		return Ok(rejected_cold_publish("cold compaction input fingerprint changed"));
	}

	let publish_generation = root.manifest_generation.saturating_add(1);
	let expected_outputs = expected_cold_output_refs(input, &cold_inputs, publish_generation);
	if expected_outputs != input.output_refs {
		return Ok(rejected_cold_publish("cold output refs do not match planned inputs"));
	}

	let mut seen_outputs = BTreeSet::new();
	for output_ref in &input.output_refs {
		if !seen_outputs.insert((output_ref.shard_id, output_ref.as_of_txid)) {
			return Ok(rejected_cold_publish("duplicate cold shard output ref"));
		}
		if tx_get_value(
			tx,
			&keys::branch_compaction_retired_cold_object_key(
				input.database_branch_id,
				content_hash(output_ref.object_key.as_bytes()),
			),
			Serializable,
		)
		.await?
		.is_some()
		{
			return Ok(rejected_cold_publish("cold object was already retired"));
		}
	}

	for output_ref in &input.output_refs {
		tx.informal().set(
			&keys::branch_compaction_cold_shard_key(
				input.database_branch_id,
				output_ref.shard_id,
				output_ref.as_of_txid,
			),
			&encode_cold_shard_ref(output_ref.clone())
				.context("encode sqlite cold shard ref for cold publish")?,
		);
	}

	let next_root = CompactionRoot {
		schema_version: root.schema_version,
		manifest_generation: publish_generation,
		hot_watermark_txid: root.hot_watermark_txid,
		cold_watermark_txid: root
			.cold_watermark_txid
			.max(input.input_range.txids.max_txid),
		cold_watermark_versionstamp: root
			.cold_watermark_versionstamp
			.max(input.input_range.max_versionstamp),
	};
	tx.informal().set(
		&keys::branch_compaction_root_key(input.database_branch_id),
		&encode_compaction_root(next_root)
			.context("encode sqlite compaction root for cold publish")?,
	);

	Ok(PublishColdJobOutput {
		status: CompactionJobStatus::Succeeded,
		output_refs: input.output_refs.clone(),
	})
}

fn rejected_cold_publish(reason: impl Into<String>) -> PublishColdJobOutput {
	PublishColdJobOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		output_refs: Vec::new(),
	}
}

#[activity(ReclaimFdbJob)]
pub async fn reclaim_fdb_job(
	ctx: &ActivityCtx,
	input: &ReclaimFdbJobInput,
) -> Result<ReclaimFdbJobOutput> {
	let input = input.clone();

	ctx.udb()?
		.run(move |tx| {
			let input = input.clone();
			async move { reclaim_fdb_job_tx(&tx, &input).await }
		})
		.await
}

async fn reclaim_fdb_job_tx(
	tx: &universaldb::Transaction,
	input: &ReclaimFdbJobInput,
) -> Result<ReclaimFdbJobOutput> {
	if input.job_kind != CompactionJobKind::Reclaim {
		return Ok(rejected_reclaim_job("reclaimer received a non-reclaim job"));
	}
	if input.input_range.txid_refs.is_empty()
		&& input.input_range.cold_objects.is_empty()
		&& (!input.input_range.staged_hot_shards.is_empty()
			|| !input.input_range.orphan_cold_objects.is_empty())
	{
		return cleanup_repair_fdb_outputs_tx(tx, input).await;
	}

	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for FDB reclaim")?;
	if !branch_record_is_live_at_generation(
		branch_record.as_ref(),
		input.base_lifecycle_generation,
	) {
		return Ok(rejected_reclaim_job("database branch lifecycle changed"));
	}

	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for FDB reclaim")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	});
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(rejected_reclaim_job("base manifest generation changed"));
	}

	let mut db_pins =
		history_pin::read_db_history_pins(tx, input.database_branch_id, Serializable).await?;
	if resolve_namespace_fork_pins(tx, input.database_branch_id, &mut db_pins).await? {
		return Ok(rejected_reclaim_job("namespace fork proof is ambiguous"));
	}
	let snapshot =
		read_reclaim_input_snapshot(tx, input.database_branch_id, &root, &db_pins, Serializable)
			.await?;
	if !input.input_range.txid_refs.is_empty() && snapshot.txid_refs != input.input_range.txid_refs {
		return Ok(rejected_reclaim_job("reclaim txid set changed"));
	}
	if snapshot.cold_object_refs != input.input_range.cold_objects {
		return Ok(rejected_reclaim_job("cold object reclaim set changed"));
	}
	if !input.input_range.txid_refs.is_empty() {
		if !snapshot.pidx_entries.is_empty() {
			return Ok(rejected_reclaim_job("PIDX still references reclaim txids"));
		}
		if !reclaim_coverage_is_complete(&snapshot) {
			return Ok(rejected_reclaim_job("replacement SHARD coverage is missing"));
		}
	}

	let input_fingerprint =
		fingerprint_reclaim_inputs(input.database_branch_id, &root, &snapshot);
	if input_fingerprint != input.input_fingerprint {
		return Ok(rejected_reclaim_job("reclaim input fingerprint changed"));
	}

	let selected_reclaim_txids = input
		.input_range
		.txid_refs
		.iter()
		.map(|txid_ref| txid_ref.txid)
		.collect::<BTreeSet<_>>();
	let mut key_count = 0_u32;
	let mut byte_count = 0_u64;
	for (txid, key, value, commit) in &snapshot.commits {
		if !selected_reclaim_txids.contains(txid) {
			continue;
		}
		udb::compare_and_clear(tx, key, value);
		key_count = key_count.saturating_add(1);
		byte_count = byte_count.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));

		let vtx_key = keys::branch_vtx_key(input.database_branch_id, commit.versionstamp);
		if let Some(vtx_value) = tx_get_value(tx, &vtx_key, Serializable).await? {
			if vtx_value == txid.to_be_bytes() {
				udb::compare_and_clear(tx, &vtx_key, &vtx_value);
				key_count = key_count.saturating_add(1);
				byte_count =
					byte_count.saturating_add(u64::try_from(vtx_value.len()).unwrap_or(u64::MAX));
			} else {
				return Ok(rejected_reclaim_job("VTX row changed for reclaim txid"));
			}
		}
	}
	for (key, value) in &snapshot.delta_chunks {
		let txid = keys::decode_branch_delta_chunk_txid(input.database_branch_id, key)?;
		if !selected_reclaim_txids.contains(&txid) {
			continue;
		}
		udb::compare_and_clear(tx, key, value);
		key_count = key_count.saturating_add(1);
		byte_count = byte_count.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
	}

	Ok(ReclaimFdbJobOutput {
		status: CompactionJobStatus::Succeeded,
		output_refs: vec![ReclaimOutputRef {
			key_count,
			byte_count,
			min_txid: input.input_range.txids.min_txid,
			max_txid: input.input_range.txids.max_txid,
		}],
	})
}

async fn cleanup_repair_fdb_outputs_tx(
	tx: &universaldb::Transaction,
	input: &ReclaimFdbJobInput,
) -> Result<ReclaimFdbJobOutput> {
	let input_fingerprint =
		fingerprint_repair_reclaim_range(input.database_branch_id, &input.input_range);
	if input_fingerprint != input.input_fingerprint {
		return Ok(rejected_reclaim_job("repair cleanup input fingerprint changed"));
	}

	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Snapshot,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for repair cleanup")?;
	let manifest_generation = root
		.as_ref()
		.map(|root| root.manifest_generation)
		.unwrap_or(input.base_manifest_generation);

	let mut key_count = 0_u32;
	let mut byte_count = 0_u64;
	for staged in &input.input_range.staged_hot_shards {
		let stage_key = keys::branch_compaction_stage_hot_shard_key(
			input.database_branch_id,
			staged.job_id,
			staged.output_ref.shard_id,
			staged.output_ref.as_of_txid,
			0,
		);
		let Some(stage_value) = tx_get_value(tx, &stage_key, Serializable).await? else {
			continue;
		};
		if staged.output_ref.size_bytes != u64::try_from(stage_value.len()).unwrap_or(u64::MAX)
			|| staged.output_ref.content_hash != content_hash(&stage_value)
		{
			tracing::error!(
				?input.database_branch_id,
				manifest_generation,
				?staged.job_id,
				shard_id = staged.output_ref.shard_id,
				as_of_txid = staged.output_ref.as_of_txid,
				repair_action = "retain_staged_hot_output",
				"staged hot shard cleanup found mismatched bytes"
			);
			return Ok(rejected_reclaim_job("staged hot shard cleanup bytes changed"));
		}

		tracing::warn!(
			?input.database_branch_id,
			manifest_generation,
			?staged.job_id,
			shard_id = staged.output_ref.shard_id,
			as_of_txid = staged.output_ref.as_of_txid,
			repair_action = "clear_staged_hot_output",
			"clearing orphan staged hot shard output"
		);
		udb::compare_and_clear(tx, &stage_key, &stage_value);
		key_count = key_count.saturating_add(1);
		byte_count = byte_count.saturating_add(u64::try_from(stage_value.len()).unwrap_or(u64::MAX));
	}

	Ok(ReclaimFdbJobOutput {
		status: CompactionJobStatus::Succeeded,
		output_refs: vec![ReclaimOutputRef {
			key_count,
			byte_count,
			min_txid: input.input_range.txids.min_txid,
			max_txid: input.input_range.txids.max_txid,
		}],
	})
}

fn rejected_reclaim_job(reason: impl Into<String>) -> ReclaimFdbJobOutput {
	ReclaimFdbJobOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		output_refs: Vec::new(),
	}
}

#[activity(RetireColdObjects)]
pub async fn retire_cold_objects(
	ctx: &ActivityCtx,
	input: &RetireColdObjectsInput,
) -> Result<RetireColdObjectsOutput> {
	let mut input = input.clone();
	input.retired_at_ms = ctx.ts();

	ctx.udb()?
		.run(move |tx| {
			let input = input.clone();
			async move { retire_cold_objects_tx(&tx, &input).await }
		})
		.await
}

async fn retire_cold_objects_tx(
	tx: &universaldb::Transaction,
	input: &RetireColdObjectsInput,
) -> Result<RetireColdObjectsOutput> {
	if input.job_kind != CompactionJobKind::Reclaim {
		return Ok(rejected_cold_object_retire("reclaimer received a non-reclaim job"));
	}
	if input.cold_objects.is_empty() {
		return Ok(RetireColdObjectsOutput {
			status: CompactionJobStatus::Succeeded,
			retired_objects: Vec::new(),
			delete_after_ms: None,
		});
	}

	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for cold retire")?;
	if !branch_record_is_live_at_generation(
		branch_record.as_ref(),
		input.base_lifecycle_generation,
	) {
		return Ok(rejected_cold_object_retire("database branch lifecycle changed"));
	}

	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for cold retire")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	});
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(rejected_cold_object_retire("base manifest generation changed"));
	}

	let delete_after_ms = input
		.retired_at_ms
		.saturating_add(CMP_COLD_OBJECT_DELETE_GRACE_MS);
	let retired_manifest_generation = root.manifest_generation.saturating_add(1);
	let mut retired_objects = Vec::with_capacity(input.cold_objects.len());

	for cold_object in &input.cold_objects {
		let live_key = keys::branch_compaction_cold_shard_key(
			input.database_branch_id,
			cold_object.shard_id,
			cold_object.as_of_txid,
		);
		let Some(live_value) = tx_get_value(tx, &live_key, Serializable).await? else {
			return Ok(rejected_cold_object_retire("cold shard ref is already absent"));
		};
		let live_ref = decode_cold_shard_ref(&live_value)
			.context("decode sqlite cold shard ref for cold retire")?;
		if reclaim_cold_object_ref(&live_ref) != *cold_object {
			return Ok(rejected_cold_object_retire("cold shard ref changed"));
		}

		let retired_key = keys::branch_compaction_retired_cold_object_key(
			input.database_branch_id,
			content_hash(cold_object.object_key.as_bytes()),
		);
		if tx_get_value(tx, &retired_key, Serializable).await?.is_some() {
			return Ok(rejected_cold_object_retire("cold object is already retired"));
		}

		udb::compare_and_clear(tx, &live_key, &live_value);
		let retired = RetiredColdObject {
			object_key: cold_object.object_key.clone(),
			object_generation_id: cold_object.object_generation_id,
			content_hash: cold_object.content_hash,
			retired_manifest_generation,
			retired_at_ms: input.retired_at_ms,
			delete_after_ms,
			delete_state: RetiredColdObjectDeleteState::Retired,
		};
		tx.informal().set(
			&retired_key,
			&encode_retired_cold_object(retired.clone())
				.context("encode sqlite retired cold object")?,
		);
		retired_objects.push(retired);
	}

	let next_root = CompactionRoot {
		schema_version: root.schema_version,
		manifest_generation: retired_manifest_generation,
		hot_watermark_txid: root.hot_watermark_txid,
		cold_watermark_txid: root.cold_watermark_txid,
		cold_watermark_versionstamp: root.cold_watermark_versionstamp,
	};
	tx.informal().set(
		&keys::branch_compaction_root_key(input.database_branch_id),
		&encode_compaction_root(next_root).context("encode sqlite compaction root for cold retire")?,
	);

	Ok(RetireColdObjectsOutput {
		status: CompactionJobStatus::Succeeded,
		retired_objects,
		delete_after_ms: Some(delete_after_ms),
	})
}

fn rejected_cold_object_retire(reason: impl Into<String>) -> RetireColdObjectsOutput {
	RetireColdObjectsOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		retired_objects: Vec::new(),
		delete_after_ms: None,
	}
}

#[activity(DeleteRetiredColdObjects)]
pub async fn delete_retired_cold_objects(
	ctx: &ActivityCtx,
	input: &DeleteRetiredColdObjectsInput,
) -> Result<DeleteRetiredColdObjectsOutput> {
	let input = input.clone();
	let cold_tier = workflow_cold_tier().await?;

	let marked = ctx
		.udb()?
		.run({
			let input = input.clone();
			move |tx| {
				let input = input.clone();
				async move { mark_retired_cold_objects_delete_issued_tx(&tx, &input).await }
			}
		})
		.await?;
	if !matches!(marked.status, CompactionJobStatus::Succeeded) {
		return Ok(marked);
	}

	cold_tier
		.delete_objects(&marked.deleted_object_keys)
		.await
		.context("delete retired sqlite cold objects")?;

	Ok(marked)
}

async fn mark_retired_cold_objects_delete_issued_tx(
	tx: &universaldb::Transaction,
	input: &DeleteRetiredColdObjectsInput,
) -> Result<DeleteRetiredColdObjectsOutput> {
	let mut object_keys = Vec::with_capacity(input.cold_objects.len());

	for cold_object in &input.cold_objects {
		let retired_key = keys::branch_compaction_retired_cold_object_key(
			input.database_branch_id,
			content_hash(cold_object.object_key.as_bytes()),
		);
		let Some(retired_value) = tx_get_value(tx, &retired_key, Serializable).await? else {
			return Ok(rejected_cold_object_delete("retired cold object is missing"));
		};
		let mut retired = decode_retired_cold_object(&retired_value)
			.context("decode sqlite retired cold object for S3 delete")?;
		if !retired_matches_cold_object(&retired, cold_object) {
			return Ok(rejected_cold_object_delete("retired cold object changed"));
		}
		if retired.delete_after_ms > input.now_ms {
			return Ok(rejected_cold_object_delete("retired cold object is still in grace window"));
		}

		let live_key = keys::branch_compaction_cold_shard_key(
			input.database_branch_id,
			cold_object.shard_id,
			cold_object.as_of_txid,
		);
		if tx_get_value(tx, &live_key, Serializable).await?.is_some() {
			tracing::error!(
				?input.database_branch_id,
				object_key = %cold_object.object_key,
				publish_generation = cold_object.expected_publish_generation,
				"live cold ref exists for retired object before S3 delete"
			);
			return Ok(rejected_cold_object_delete("live cold ref exists for retired object"));
		}

		if retired.delete_state == RetiredColdObjectDeleteState::Retired {
			retired.delete_state = RetiredColdObjectDeleteState::DeleteIssued;
			tx.informal().set(
				&retired_key,
				&encode_retired_cold_object(retired)
					.context("encode sqlite retired cold object delete state")?,
			);
		}
		object_keys.push(cold_object.object_key.clone());
	}

	Ok(DeleteRetiredColdObjectsOutput {
		status: CompactionJobStatus::Succeeded,
		deleted_object_keys: object_keys,
	})
}

fn rejected_cold_object_delete(reason: impl Into<String>) -> DeleteRetiredColdObjectsOutput {
	DeleteRetiredColdObjectsOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		deleted_object_keys: Vec::new(),
	}
}

#[activity(CleanupRetiredColdObjects)]
pub async fn cleanup_retired_cold_objects(
	ctx: &ActivityCtx,
	input: &CleanupRetiredColdObjectsInput,
) -> Result<CleanupRetiredColdObjectsOutput> {
	let input = input.clone();

	ctx.udb()?
		.run(move |tx| {
			let input = input.clone();
			async move { cleanup_retired_cold_objects_tx(&tx, &input).await }
		})
		.await
}

async fn cleanup_retired_cold_objects_tx(
	tx: &universaldb::Transaction,
	input: &CleanupRetiredColdObjectsInput,
) -> Result<CleanupRetiredColdObjectsOutput> {
	let mut cleaned = Vec::with_capacity(input.cold_objects.len());

	for cold_object in &input.cold_objects {
		let live_key = keys::branch_compaction_cold_shard_key(
			input.database_branch_id,
			cold_object.shard_id,
			cold_object.as_of_txid,
		);
		if tx_get_value(tx, &live_key, Serializable).await?.is_some() {
			tracing::error!(
				?input.database_branch_id,
				object_key = %cold_object.object_key,
				publish_generation = cold_object.expected_publish_generation,
				"live cold ref exists for delete-issued retired object"
			);
			return Ok(rejected_cold_object_cleanup(
				"live cold ref exists for delete-issued retired object",
			));
		}

		let retired_key = keys::branch_compaction_retired_cold_object_key(
			input.database_branch_id,
			content_hash(cold_object.object_key.as_bytes()),
		);
		let Some(retired_value) = tx_get_value(tx, &retired_key, Serializable).await? else {
			continue;
		};
		let retired = decode_retired_cold_object(&retired_value)
			.context("decode sqlite retired cold object for cleanup")?;
		if !retired_matches_cold_object(&retired, cold_object) {
			return Ok(rejected_cold_object_cleanup("retired cold object changed"));
		}
		if retired.delete_state != RetiredColdObjectDeleteState::DeleteIssued {
			return Ok(rejected_cold_object_cleanup("retired cold object delete was not issued"));
		}

		let completed = RetiredColdObject {
			delete_state: RetiredColdObjectDeleteState::Deleted,
			..retired
		};
		tx.informal().set(
			&retired_key,
			&encode_retired_cold_object(completed)
				.context("encode completed sqlite retired cold object")?,
		);
		cleaned.push(cold_object.object_key.clone());
	}

	Ok(CleanupRetiredColdObjectsOutput {
		status: CompactionJobStatus::Succeeded,
		cleaned_object_keys: cleaned,
	})
}

fn rejected_cold_object_cleanup(reason: impl Into<String>) -> CleanupRetiredColdObjectsOutput {
	CleanupRetiredColdObjectsOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		cleaned_object_keys: Vec::new(),
	}
}

#[activity(ValidateReclaimColdObjects)]
pub async fn validate_reclaim_cold_objects(
	ctx: &ActivityCtx,
	input: &ValidateReclaimColdObjectsInput,
) -> Result<ValidateReclaimColdObjectsOutput> {
	let input = input.clone();
	if input.cold_objects.is_empty() {
		return Ok(ValidateReclaimColdObjectsOutput {
			status: CompactionJobStatus::Succeeded,
		});
	}

	let validated = ctx
		.udb()?
		.run({
			let input = input.clone();
			move |tx| {
				let input = input.clone();
				async move { validate_reclaim_cold_objects_tx(&tx, &input).await }
			}
		})
		.await?;
	if !matches!(validated.status, CompactionJobStatus::Succeeded) {
		return Ok(validated);
	}

	let cold_tier = workflow_cold_tier().await?;
	for cold_object in &input.cold_objects {
		if cold_tier
			.get_object(&cold_object.object_key)
			.await
			.with_context(|| format!("get sqlite workflow cold object {}", cold_object.object_key))?
			.is_none()
		{
			tracing::error!(
				?input.database_branch_id,
				manifest_generation = cold_object.expected_publish_generation,
				object_key = %cold_object.object_key,
				?cold_object.object_generation_id,
				shard_id = cold_object.shard_id,
				as_of_txid = cold_object.as_of_txid,
				repair_action = "retain_live_cold_ref",
				"live cold ref points at missing S3 object"
			);
			return Ok(rejected_validate_reclaim_cold_objects(
				"live cold ref points at missing S3 object",
			));
		}
	}

	Ok(ValidateReclaimColdObjectsOutput {
		status: CompactionJobStatus::Succeeded,
	})
}

async fn validate_reclaim_cold_objects_tx(
	tx: &universaldb::Transaction,
	input: &ValidateReclaimColdObjectsInput,
) -> Result<ValidateReclaimColdObjectsOutput> {
	for cold_object in &input.cold_objects {
		let live_key = keys::branch_compaction_cold_shard_key(
			input.database_branch_id,
			cold_object.shard_id,
			cold_object.as_of_txid,
		);
		let Some(live_value) = tx_get_value(tx, &live_key, Serializable).await? else {
			return Ok(rejected_validate_reclaim_cold_objects(
				"cold shard ref is missing before validation",
			));
		};
		let live_ref = decode_cold_shard_ref(&live_value)
			.context("decode sqlite cold shard ref for validation")?;
		if reclaim_cold_object_ref(&live_ref) != *cold_object {
			return Ok(rejected_validate_reclaim_cold_objects(
				"cold shard ref changed before validation",
			));
		}

		if let Some(retired) =
			read_retired_cold_object_by_object_key(tx, input.database_branch_id, &cold_object.object_key)
				.await?
			&& retired.delete_state == RetiredColdObjectDeleteState::DeleteIssued
		{
			tracing::error!(
				?input.database_branch_id,
				manifest_generation = cold_object.expected_publish_generation,
				object_key = %cold_object.object_key,
				?cold_object.object_generation_id,
				shard_id = cold_object.shard_id,
				as_of_txid = cold_object.as_of_txid,
				repair_action = "retain_live_cold_ref",
				"live cold ref points at delete-issued retired object"
			);
			return Ok(rejected_validate_reclaim_cold_objects(
				"live cold ref points at delete-issued retired object",
			));
		}
	}

	Ok(ValidateReclaimColdObjectsOutput {
		status: CompactionJobStatus::Succeeded,
	})
}

fn rejected_validate_reclaim_cold_objects(
	reason: impl Into<String>,
) -> ValidateReclaimColdObjectsOutput {
	ValidateReclaimColdObjectsOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
	}
}

#[activity(DeleteOrphanColdObjects)]
pub async fn delete_orphan_cold_objects(
	ctx: &ActivityCtx,
	input: &DeleteOrphanColdObjectsInput,
) -> Result<DeleteOrphanColdObjectsOutput> {
	let input = input.clone();
	if input.orphan_cold_objects.is_empty() {
		return Ok(DeleteOrphanColdObjectsOutput {
			status: CompactionJobStatus::Succeeded,
			deleted_object_keys: Vec::new(),
		});
	}

	let planned = ctx
		.udb()?
		.run({
			let input = input.clone();
			move |tx| {
				let input = input.clone();
				async move { plan_orphan_cold_object_deletes_tx(&tx, &input).await }
			}
		})
		.await?;
	if !matches!(planned.status, CompactionJobStatus::Succeeded)
		|| planned.deleted_object_keys.is_empty()
	{
		return Ok(planned);
	}

	let cold_tier = workflow_cold_tier().await?;
	cold_tier
		.delete_objects(&planned.deleted_object_keys)
		.await
		.context("delete orphan sqlite workflow cold objects")?;

	Ok(planned)
}

async fn plan_orphan_cold_object_deletes_tx(
	tx: &universaldb::Transaction,
	input: &DeleteOrphanColdObjectsInput,
) -> Result<DeleteOrphanColdObjectsOutput> {
	let live_refs = tx_scan_prefix_values(
		tx,
		&keys::branch_compaction_cold_shard_prefix(input.database_branch_id),
		Serializable,
	)
	.await?
	.into_iter()
	.map(|(_, value)| decode_cold_shard_ref(&value))
	.collect::<Result<Vec<_>>>()
	.context("decode sqlite cold shard refs for orphan cleanup")?;
	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Snapshot,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for orphan cleanup")?;
	let manifest_generation = root
		.as_ref()
		.map(|root| root.manifest_generation)
		.unwrap_or_default();
	let mut delete_keys = Vec::new();

	for orphan in &input.orphan_cold_objects {
		let retired =
			read_retired_cold_object_by_object_key(tx, input.database_branch_id, &orphan.object_key)
				.await?;
		let live_ref = live_refs
			.iter()
			.find(|live_ref| live_ref.object_key == orphan.object_key);
		if let Some(live_ref) = live_ref {
			if retired
				.as_ref()
				.is_some_and(|retired| retired.delete_state == RetiredColdObjectDeleteState::DeleteIssued)
			{
				tracing::error!(
					?input.database_branch_id,
					manifest_generation,
					object_key = %orphan.object_key,
					?orphan.object_generation_id,
					shard_id = live_ref.shard_id,
					as_of_txid = live_ref.as_of_txid,
					repair_action = "retain_live_cold_ref",
					"live cold ref points at delete-issued retired object"
				);
				return Ok(rejected_orphan_cold_object_delete(
					"live cold ref points at delete-issued retired object",
				));
			}
			continue;
		}
		if retired.is_some() {
			continue;
		}

		tracing::warn!(
			?input.database_branch_id,
			manifest_generation,
			object_key = %orphan.object_key,
			?orphan.object_generation_id,
			shard_id = orphan.shard_id,
			as_of_txid = orphan.as_of_txid,
			repair_action = "delete_orphan_cold_object",
			"deleting orphan cold object"
		);
		delete_keys.push(orphan.object_key.clone());
		if delete_keys.len() >= CMP_S3_DELETE_MAX_OBJECTS {
			break;
		}
	}

	Ok(DeleteOrphanColdObjectsOutput {
		status: CompactionJobStatus::Succeeded,
		deleted_object_keys: delete_keys,
	})
}

fn rejected_orphan_cold_object_delete(
	reason: impl Into<String>,
) -> DeleteOrphanColdObjectsOutput {
	DeleteOrphanColdObjectsOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		deleted_object_keys: Vec::new(),
	}
}

async fn read_retired_cold_object_by_object_key(
	tx: &universaldb::Transaction,
	database_branch_id: DatabaseBranchId,
	object_key: &str,
) -> Result<Option<RetiredColdObject>> {
	tx_get_value(
		tx,
		&keys::branch_compaction_retired_cold_object_key(
			database_branch_id,
			content_hash(object_key.as_bytes()),
		),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_retired_cold_object)
	.transpose()
	.context("decode sqlite retired cold object for repair")
}

async fn read_manager_fdb_snapshot(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
) -> Result<ManagerFdbSnapshot> {
	let branch_record = tx_get_value(tx, &keys::branches_list_key(branch_id), Serializable)
		.await?
		.as_deref()
		.map(decode_database_branch_record)
		.transpose()
		.context("decode sqlite database branch record for compaction manager")?;
	let head = tx_get_value(tx, &keys::branch_meta_head_key(branch_id), Serializable)
		.await?
		.as_deref()
		.map(decode_db_head)
		.transpose()
		.context("decode sqlite head for compaction manager")?;
	let root = tx_get_value(tx, &keys::branch_compaction_root_key(branch_id), Serializable)
		.await?
		.as_deref()
		.map(decode_compaction_root)
		.transpose()
		.context("decode sqlite compaction root for manager refresh")?
		.unwrap_or(CompactionRoot {
			schema_version: 1,
			manifest_generation: 0,
			hot_watermark_txid: 0,
			cold_watermark_txid: 0,
			cold_watermark_versionstamp: [0; 16],
		});
	let dirty_key = keys::sqlite_cmp_dirty_key(branch_id);
	let dirty_bytes = tx_get_value(tx, &dirty_key, Serializable).await?;
	let dirty = dirty_bytes
		.as_deref()
		.map(decode_sqlite_cmp_dirty)
		.transpose()
		.context("decode sqlite dirty marker for compaction manager")?;
	let mut db_pins = history_pin::read_db_history_pins(tx, branch_id, Serializable).await?;
	let namespace_proof_blocked_reclaim =
		resolve_namespace_fork_pins(tx, branch_id, &mut db_pins).await?;
	let hot_inputs = read_hot_input_snapshot(tx, branch_id, head.as_ref(), &root, Snapshot).await?;
	let cold_inputs = read_cold_input_snapshot(tx, branch_id, &root, Snapshot).await?;
	let reclaim_inputs =
		read_reclaim_input_snapshot(tx, branch_id, &root, &db_pins, Snapshot).await?;
	let hot_lag = head
		.as_ref()
		.map_or(0, |head| head.head_txid.saturating_sub(root.hot_watermark_txid));
	let cold_lag = head
		.as_ref()
		.map_or(0, |head| head.head_txid.saturating_sub(root.cold_watermark_txid));
	let has_actionable_lag = hot_lag >= quota::COMPACTION_DELTA_THRESHOLD
		|| (cold_lag >= HOT_BURST_COLD_LAG_THRESHOLD_TXIDS && !cold_inputs.shard_blobs.is_empty())
		|| reclaim_coverage_is_complete(&reclaim_inputs);
	let cleared_dirty = if !has_actionable_lag {
		if let Some(expected_dirty) = dirty_bytes {
			udb::compare_and_clear(tx, &dirty_key, &expected_dirty);
			true
		} else {
			false
		}
	} else {
		false
	};

	Ok(ManagerFdbSnapshot {
		branch_record,
		head,
		root,
		dirty,
		db_pins,
		hot_inputs,
		cold_inputs,
		reclaim_inputs,
		namespace_proof_blocked_reclaim,
		cleared_dirty,
	})
}

async fn resolve_namespace_fork_pins(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	db_pins: &mut Vec<DbHistoryPin>,
) -> Result<bool> {
	let catalog_rows =
		tx_scan_prefix_values(tx, &keys::nscat_by_db_prefix(branch_id), Serializable).await?;
	if catalog_rows.len() >= CMP_FDB_BATCH_MAX_KEYS {
		tracing::warn!(
			?branch_id,
			row_count = catalog_rows.len(),
			"retaining sqlite history because namespace catalog proof is too large"
		);
		return Ok(true);
	}

	for (_, value) in catalog_rows {
		let catalog_fact = decode_namespace_catalog_db_fact(&value)
			.context("decode sqlite namespace catalog proof fact")?;
		if catalog_fact.database_branch_id != branch_id {
			tracing::warn!(
				?branch_id,
				?catalog_fact,
				"retaining sqlite history because namespace catalog proof has wrong branch"
			);
			return Ok(true);
		}
		if resolve_namespace_catalog_forks(tx, branch_id, db_pins, &catalog_fact).await? {
			return Ok(true);
		}
	}

	Ok(false)
}

async fn resolve_namespace_catalog_forks(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	db_pins: &mut Vec<DbHistoryPin>,
	catalog_fact: &NamespaceCatalogDbFact,
) -> Result<bool> {
	let mut queue = vec![catalog_fact.namespace_branch_id];
	let mut visited = BTreeSet::new();
	let mut inspected_rows = 0_usize;

	for depth in 0..=MAX_NAMESPACE_DEPTH {
		let Some(source_namespace_branch_id) = queue.pop() else {
			return Ok(false);
		};
		if !visited.insert(source_namespace_branch_id) {
			continue;
		}

		let child_rows =
			tx_scan_prefix_values(tx, &keys::ns_child_prefix(source_namespace_branch_id), Serializable)
				.await?;
		inspected_rows = inspected_rows.saturating_add(child_rows.len());
		if inspected_rows >= CMP_FDB_BATCH_MAX_KEYS {
			tracing::warn!(
				?branch_id,
				?source_namespace_branch_id,
				row_count = inspected_rows,
				"retaining sqlite history because namespace child proof is too large"
			);
			return Ok(true);
		}

		for (_, value) in child_rows {
			let child_fact =
				decode_namespace_fork_fact(&value).context("decode sqlite namespace child fact")?;
			if child_fact.source_namespace_branch_id != source_namespace_branch_id {
				tracing::warn!(
					?branch_id,
					?child_fact,
					"retaining sqlite history because namespace child proof has wrong source"
				);
				return Ok(true);
			}
			if !namespace_fork_can_inherit_database(&child_fact, catalog_fact) {
				continue;
			}
			if namespace_fork_pin_fact_is_missing_or_changed(tx, &child_fact).await? {
				tracing::warn!(
					?branch_id,
					?child_fact,
					"retaining sqlite history because namespace fork proof is missing"
				);
				return Ok(true);
			}
			if materialize_namespace_fork_pin(tx, branch_id, db_pins, &child_fact).await? {
				return Ok(true);
			}
			queue.push(child_fact.target_namespace_branch_id);
		}

		if depth == MAX_NAMESPACE_DEPTH && !queue.is_empty() {
			tracing::warn!(
				?branch_id,
				"retaining sqlite history because namespace proof exceeded max depth"
			);
			return Ok(true);
		}
	}

	Ok(false)
}

fn namespace_fork_can_inherit_database(
	fork_fact: &NamespaceForkFact,
	catalog_fact: &NamespaceCatalogDbFact,
) -> bool {
	fork_fact.fork_versionstamp >= catalog_fact.catalog_versionstamp
		&& catalog_fact
			.tombstone_versionstamp
			.map_or(true, |tombstone_versionstamp| {
				fork_fact.fork_versionstamp < tombstone_versionstamp
			})
}

async fn namespace_fork_pin_fact_is_missing_or_changed(
	tx: &universaldb::Transaction,
	child_fact: &NamespaceForkFact,
) -> Result<bool> {
	let Some(fork_pin_bytes) = tx_get_value(
		tx,
		&keys::ns_fork_pin_key(
			child_fact.source_namespace_branch_id,
			child_fact.fork_versionstamp,
			child_fact.target_namespace_branch_id,
		),
		Serializable,
	)
	.await?
	else {
		return Ok(true);
	};
	let fork_pin_fact =
		decode_namespace_fork_fact(&fork_pin_bytes).context("decode sqlite namespace fork fact")?;

	Ok(fork_pin_fact != *child_fact)
}

async fn materialize_namespace_fork_pin(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	db_pins: &mut Vec<DbHistoryPin>,
	fork_fact: &NamespaceForkFact,
) -> Result<bool> {
	let Some((at_txid, at_versionstamp, commit)) =
		latest_commit_at_or_before_versionstamp(tx, branch_id, fork_fact.fork_versionstamp).await?
	else {
		tracing::warn!(
			?branch_id,
			?fork_fact,
			"retaining sqlite history because namespace fork versionstamp could not be resolved"
		);
		return Ok(true);
	};

	history_pin::write_namespace_fork_pin(
		tx,
		branch_id,
		fork_fact.target_namespace_branch_id,
		at_versionstamp,
		at_txid,
		commit.wall_clock_ms,
	)?;
	db_pins
		.retain(|pin| pin.owner_namespace_branch_id != Some(fork_fact.target_namespace_branch_id));
	db_pins.push(DbHistoryPin {
		at_versionstamp,
		at_txid,
		kind: crate::types::DbHistoryPinKind::NamespaceFork,
		owner_database_branch_id: None,
		owner_namespace_branch_id: Some(fork_fact.target_namespace_branch_id),
		owner_bookmark: None,
		created_at_ms: commit.wall_clock_ms,
	});

	Ok(false)
}

async fn latest_commit_at_or_before_versionstamp(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	versionstamp_cap: [u8; 16],
) -> Result<Option<(u64, [u8; 16], CommitRow)>> {
	let mut selected = None;
	let mut inspected_rows = 0_usize;

	for (key, value) in
		tx_scan_prefix_values(tx, &keys::branch_vtx_prefix(branch_id), Serializable).await?
	{
		let versionstamp = decode_branch_vtx_versionstamp(branch_id, &key)?;
		if versionstamp > versionstamp_cap {
			break;
		}
		inspected_rows = inspected_rows.saturating_add(1);
		if inspected_rows >= CMP_FDB_BATCH_MAX_KEYS {
			tracing::warn!(
				?branch_id,
				row_count = inspected_rows,
				"retaining sqlite history because namespace VTX proof is too large"
			);
			return Ok(None);
		}
		let txid = decode_txid_value(&value)?;
		selected = Some((txid, versionstamp));
	}

	let Some((txid, versionstamp)) = selected else {
		return Ok(None);
	};
	let Some(commit_bytes) = tx_get_value(tx, &keys::branch_commit_key(branch_id, txid), Serializable).await?
	else {
		return Ok(None);
	};
	let commit = decode_commit_row(&commit_bytes).context("decode sqlite namespace pin commit row")?;

	Ok(Some((txid, versionstamp, commit)))
}

fn decode_branch_vtx_versionstamp(
	branch_id: DatabaseBranchId,
	key: &[u8],
) -> Result<[u8; 16]> {
	let prefix = keys::branch_vtx_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("branch VTX key did not start with expected prefix")?;
	ensure!(
		suffix.len() == 16,
		"branch VTX versionstamp suffix had {} bytes, expected 16",
		suffix.len()
	);

	suffix
		.try_into()
		.context("branch VTX versionstamp suffix should decode as 16 bytes")
}

fn decode_txid_value(value: &[u8]) -> Result<u64> {
	let bytes = <[u8; 8]>::try_from(value)
		.map_err(|_| anyhow::anyhow!("txid value had {} bytes, expected 8", value.len()))?;

	Ok(u64::from_be_bytes(bytes))
}

async fn read_hot_input_snapshot(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	head: Option<&DBHead>,
	root: &CompactionRoot,
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<HotInputSnapshot> {
	let Some(head) = head else {
		return Ok(HotInputSnapshot::default());
	};
	if head.head_txid <= root.hot_watermark_txid {
		return Ok(HotInputSnapshot::default());
	}

	let min_txid = root.hot_watermark_txid.saturating_add(1);
	let max_txid = head.head_txid;
	let mut snapshot = HotInputSnapshot::default();

	for (key, value) in
		tx_scan_prefix_values(tx, &keys::branch_commit_prefix(branch_id), isolation_level).await?
	{
		let txid = decode_branch_commit_txid(branch_id, &key)?;
		if txid < min_txid || txid > max_txid {
			continue;
		}
		snapshot.total_value_bytes = snapshot
			.total_value_bytes
			.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
		snapshot.commits.push((
			txid,
			decode_commit_row(&value).context("decode sqlite commit row for hot planning")?,
		));
	}

	for (key, value) in
		tx_scan_prefix_values(tx, &keys::branch_delta_prefix(branch_id), isolation_level).await?
	{
		let txid = keys::decode_branch_delta_chunk_txid(branch_id, &key)?;
		if txid < min_txid || txid > max_txid {
			continue;
		}
		snapshot.total_value_bytes = snapshot
			.total_value_bytes
			.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
		snapshot.delta_chunks.push((key, value));
		if snapshot.delta_chunks.len() + snapshot.commits.len() >= CMP_FDB_BATCH_MAX_KEYS
			|| snapshot.total_value_bytes >= CMP_FDB_BATCH_MAX_VALUE_BYTES as u64
		{
			break;
		}
	}

	for (key, value) in
		tx_scan_prefix_values(tx, &keys::branch_pidx_prefix(branch_id), isolation_level).await?
	{
		if let Ok(txid) = decode_pidx_txid(&value) {
			if txid < min_txid || txid > max_txid {
				continue;
			}
		}
		snapshot.total_value_bytes = snapshot
			.total_value_bytes
			.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
		snapshot.pidx_entries.push((key, value));
		if snapshot.pidx_entries.len() + snapshot.delta_chunks.len() + snapshot.commits.len()
			>= CMP_FDB_BATCH_MAX_KEYS
			|| snapshot.total_value_bytes >= CMP_FDB_BATCH_MAX_VALUE_BYTES as u64
		{
			break;
		}
	}

	Ok(snapshot)
}

async fn read_reclaim_input_snapshot(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	root: &CompactionRoot,
	db_pins: &[DbHistoryPin],
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<ReclaimInputSnapshot> {
	let cold_object_refs = read_reclaim_cold_object_refs(tx, branch_id, root, isolation_level).await?;
	let Some(max_reclaim_txid) = reclaim_delete_upper_bound(root, db_pins) else {
		return Ok(ReclaimInputSnapshot {
			cold_object_refs,
			..ReclaimInputSnapshot::default()
		});
	};

	let mut snapshot = ReclaimInputSnapshot {
		cold_object_refs,
		..ReclaimInputSnapshot::default()
	};
	for (key, value) in
		tx_scan_prefix_values(tx, &keys::branch_commit_prefix(branch_id), isolation_level).await?
	{
		let txid = decode_branch_commit_txid(branch_id, &key)?;
		if txid > max_reclaim_txid {
			continue;
		}
		let commit = decode_commit_row(&value).context("decode sqlite commit row for reclaim")?;
		snapshot.total_value_bytes = snapshot
			.total_value_bytes
			.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
		snapshot.txid_refs.push(ReclaimTxidRef {
			txid,
			versionstamp: commit.versionstamp,
		});
		snapshot.commits.push((txid, key, value, commit));
	}

	let selected_txids = snapshot
		.txid_refs
		.iter()
		.map(|txid_ref| txid_ref.txid)
		.collect::<BTreeSet<_>>();
	if selected_txids.is_empty() {
		return Ok(snapshot);
	}

	for (key, value) in
		tx_scan_prefix_values(tx, &keys::branch_delta_prefix(branch_id), isolation_level).await?
	{
		let txid = keys::decode_branch_delta_chunk_txid(branch_id, &key)?;
		if !selected_txids.contains(&txid) {
			continue;
		}
		snapshot.total_value_bytes = snapshot
			.total_value_bytes
			.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
		snapshot.delta_chunks.push((key, value));
		if snapshot.txid_refs.len() + snapshot.delta_chunks.len() >= CMP_FDB_BATCH_MAX_KEYS
			|| snapshot.total_value_bytes >= CMP_FDB_BATCH_MAX_VALUE_BYTES as u64
		{
			break;
		}
	}

	for (key, value) in
		tx_scan_prefix_values(tx, &keys::branch_pidx_prefix(branch_id), isolation_level).await?
	{
		if let Ok(txid) = decode_pidx_txid(&value) {
			if selected_txids.contains(&txid) {
				snapshot.pidx_entries.push((key, value));
			}
		}
	}

	let shard_ids = reclaim_delta_shard_ids(branch_id, &snapshot.delta_chunks)?;
	snapshot.required_coverage_shard_count = shard_ids.len();
	for shard_id in shard_ids {
		let key = keys::branch_shard_key(branch_id, shard_id, root.hot_watermark_txid);
		if let Some(value) = tx_get_value(tx, &key, isolation_level).await? {
			snapshot.coverage_shards.push((key, value));
		}
	}

	Ok(snapshot)
}

async fn read_reclaim_cold_object_refs(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	root: &CompactionRoot,
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Vec<ReclaimColdObjectRef>> {
	let mut refs = Vec::new();

	for (_, value) in
		tx_scan_prefix_values(tx, &keys::branch_compaction_cold_shard_prefix(branch_id), isolation_level)
			.await?
	{
		let cold_ref =
			decode_cold_shard_ref(&value).context("decode sqlite cold shard ref for reclaim")?;
		if cold_ref.as_of_txid >= root.cold_watermark_txid {
			continue;
		}
		refs.push(reclaim_cold_object_ref(&cold_ref));
		if refs.len() >= CMP_S3_DELETE_MAX_OBJECTS {
			break;
		}
	}

	Ok(refs)
}

async fn read_cold_input_snapshot(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	root: &CompactionRoot,
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<ColdInputSnapshot> {
	if root.hot_watermark_txid <= root.cold_watermark_txid {
		return Ok(ColdInputSnapshot::default());
	}

	let min_txid = root.cold_watermark_txid.saturating_add(1);
	let max_txid = root.hot_watermark_txid;
	let mut snapshot = ColdInputSnapshot::default();

	for (key, value) in
		tx_scan_prefix_values(tx, &keys::branch_commit_prefix(branch_id), isolation_level).await?
	{
		let txid = decode_branch_commit_txid(branch_id, &key)?;
		if txid < min_txid || txid > max_txid {
			continue;
		}
		let commit = decode_commit_row(&value).context("decode sqlite commit row for cold planning")?;
		if snapshot.commits.is_empty() {
			snapshot.min_versionstamp = commit.versionstamp;
			snapshot.max_versionstamp = commit.versionstamp;
		} else {
			snapshot.min_versionstamp = snapshot.min_versionstamp.min(commit.versionstamp);
			snapshot.max_versionstamp = snapshot.max_versionstamp.max(commit.versionstamp);
		}
		snapshot.total_value_bytes = snapshot
			.total_value_bytes
			.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
		snapshot.commits.push((txid, commit));
	}

	if snapshot.commits.is_empty() {
		return Ok(ColdInputSnapshot::default());
	}

	for (key, value) in
		tx_scan_prefix_values(tx, &keys::branch_shard_prefix(branch_id), isolation_level).await?
	{
		let Some((shard_id, as_of_txid)) = decode_branch_shard_version_key(branch_id, &key)? else {
			continue;
		};
		if as_of_txid != max_txid {
			continue;
		}
		snapshot.total_value_bytes = snapshot
			.total_value_bytes
			.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
		snapshot.shard_blobs.push(ColdShardBlob {
			shard_id,
			as_of_txid,
			key,
			bytes: value,
		});
		if snapshot.shard_blobs.len() >= CMP_S3_UPLOAD_MAX_OBJECTS
			|| snapshot.total_value_bytes >= CMP_S3_UPLOAD_LIMIT_BYTES as u64
		{
			break;
		}
	}

	Ok(snapshot)
}

fn reclaim_delete_upper_bound(
	root: &CompactionRoot,
	db_pins: &[DbHistoryPin],
) -> Option<u64> {
	if root.hot_watermark_txid == 0 {
		return None;
	}

	let pinned_floor = db_pins
		.iter()
		.filter(|pin| pin.at_txid <= root.hot_watermark_txid)
		.map(|pin| pin.at_txid)
		.min();
	let max_reclaim_txid = pinned_floor
		.map(|txid| txid.saturating_sub(1))
		.unwrap_or(root.hot_watermark_txid);

	(max_reclaim_txid > 0).then_some(max_reclaim_txid)
}

fn reclaim_delta_shard_ids(
	branch_id: DatabaseBranchId,
	delta_chunks: &[(Vec<u8>, Vec<u8>)],
) -> Result<BTreeSet<u32>> {
	let deltas = decode_hot_delta_chunks(branch_id, delta_chunks)?;
	let mut shard_ids = BTreeSet::new();
	for delta in deltas.values() {
		for page in &delta.pages {
			shard_ids.insert(page.pgno / keys::SHARD_SIZE);
		}
	}
	Ok(shard_ids)
}

fn reclaim_coverage_is_complete(snapshot: &ReclaimInputSnapshot) -> bool {
	!snapshot.delta_chunks.is_empty()
		&& snapshot.required_coverage_shard_count > 0
		&& snapshot.coverage_shards.len() == snapshot.required_coverage_shard_count
}

fn selected_hot_coverage_txids(
	root: &CompactionRoot,
	head: &DBHead,
	db_pins: &[DbHistoryPin],
) -> Vec<u64> {
	let mut coverage_txids = BTreeSet::new();
	coverage_txids.insert(head.head_txid);

	for pin in db_pins {
		if pin.at_txid > root.hot_watermark_txid && pin.at_txid <= head.head_txid {
			coverage_txids.insert(pin.at_txid);
		}
	}

	coverage_txids.into_iter().collect()
}

fn plan_hot_job(
	database_branch_id: DatabaseBranchId,
	snapshot: &ManagerFdbSnapshot,
	job_id: Id,
	now_ms: i64,
) -> Option<ActiveCompactionJob> {
	let branch_record = snapshot.branch_record.as_ref()?;
	let head = snapshot.head.as_ref()?;
	if head.head_txid <= snapshot.root.hot_watermark_txid {
		return None;
	}
	let hot_lag = head.head_txid.saturating_sub(snapshot.root.hot_watermark_txid);
	let coverage_txids = selected_hot_coverage_txids(&snapshot.root, head, &snapshot.db_pins);
	let has_uncovered_pin = coverage_txids
		.iter()
		.any(|txid| *txid != head.head_txid && *txid > snapshot.root.hot_watermark_txid);
	if hot_lag < quota::COMPACTION_DELTA_THRESHOLD && !has_uncovered_pin {
		return None;
	}

	let input_range = HotJobInputRange {
		txids: TxidRange {
			min_txid: snapshot.root.hot_watermark_txid.saturating_add(1),
			max_txid: head.head_txid,
		},
		coverage_txids: coverage_txids.clone(),
		max_pages: u32::try_from(snapshot.hot_inputs.pidx_entries.len()).unwrap_or(u32::MAX),
		max_bytes: snapshot.hot_inputs.total_value_bytes,
	};
	let input_fingerprint = fingerprint_hot_inputs(
		database_branch_id,
		&snapshot.root,
		head,
		&coverage_txids,
		&snapshot.hot_inputs,
	);

	Some(ActiveCompactionJob {
		database_branch_id,
		job_id,
		job_kind: CompactionJobKind::Hot,
		base_lifecycle_generation: branch_record.lifecycle_generation,
		base_manifest_generation: snapshot.root.manifest_generation,
		input_fingerprint,
		input_range: PlannedInputRange::Hot(input_range),
		planned_at_ms: now_ms,
		attempt: 0,
	})
}

fn plan_cold_job(
	database_branch_id: DatabaseBranchId,
	snapshot: &ManagerFdbSnapshot,
	job_id: Id,
	now_ms: i64,
) -> Option<ActiveCompactionJob> {
	let branch_record = snapshot.branch_record.as_ref()?;
	if snapshot.cold_inputs.shard_blobs.is_empty() {
		return None;
	}
	let cold_lag = snapshot
		.root
		.hot_watermark_txid
		.saturating_sub(snapshot.root.cold_watermark_txid);
	if cold_lag < HOT_BURST_COLD_LAG_THRESHOLD_TXIDS {
		return None;
	}

	let input_range = ColdJobInputRange {
		txids: TxidRange {
			min_txid: snapshot.root.cold_watermark_txid.saturating_add(1),
			max_txid: snapshot.root.hot_watermark_txid,
		},
		min_versionstamp: snapshot.cold_inputs.min_versionstamp,
		max_versionstamp: snapshot.cold_inputs.max_versionstamp,
		max_bytes: snapshot.cold_inputs.total_value_bytes,
	};
	let input_fingerprint =
		fingerprint_cold_inputs(database_branch_id, &snapshot.root, &snapshot.cold_inputs);

	Some(ActiveCompactionJob {
		database_branch_id,
		job_id,
		job_kind: CompactionJobKind::Cold,
		base_lifecycle_generation: branch_record.lifecycle_generation,
		base_manifest_generation: snapshot.root.manifest_generation,
		input_fingerprint,
		input_range: PlannedInputRange::Cold(input_range),
		planned_at_ms: now_ms,
		attempt: 0,
	})
}

fn plan_reclaim_job(
	database_branch_id: DatabaseBranchId,
	snapshot: &ManagerFdbSnapshot,
	job_id: Id,
	now_ms: i64,
) -> Option<ActiveCompactionJob> {
	let branch_record = snapshot.branch_record.as_ref()?;
	if snapshot.namespace_proof_blocked_reclaim {
		return None;
	}
	let has_hot_reclaim = !snapshot.reclaim_inputs.txid_refs.is_empty()
		&& snapshot.reclaim_inputs.pidx_entries.is_empty()
		&& reclaim_coverage_is_complete(&snapshot.reclaim_inputs);
	let has_cold_reclaim = !snapshot.reclaim_inputs.cold_object_refs.is_empty();
	if !has_hot_reclaim && !has_cold_reclaim {
		return None;
	}

	let min_txid = snapshot
		.reclaim_inputs
		.txid_refs
		.first()
		.map(|txid_ref| txid_ref.txid)
		.unwrap_or(snapshot.root.cold_watermark_txid);
	let max_txid = snapshot
		.reclaim_inputs
		.txid_refs
		.last()
		.map(|txid_ref| txid_ref.txid)
		.unwrap_or(snapshot.root.cold_watermark_txid);
	let input_range = ReclaimJobInputRange {
		txids: TxidRange { min_txid, max_txid },
		txid_refs: if has_hot_reclaim {
			snapshot.reclaim_inputs.txid_refs.clone()
		} else {
			Vec::new()
		},
		cold_objects: snapshot.reclaim_inputs.cold_object_refs.clone(),
		staged_hot_shards: Vec::new(),
		orphan_cold_objects: Vec::new(),
		max_keys: CMP_FDB_BATCH_MAX_KEYS as u32,
		max_bytes: CMP_FDB_BATCH_MAX_VALUE_BYTES as u64,
	};
	let input_fingerprint =
		fingerprint_reclaim_inputs(database_branch_id, &snapshot.root, &snapshot.reclaim_inputs);

	Some(ActiveCompactionJob {
		database_branch_id,
		job_id,
		job_kind: CompactionJobKind::Reclaim,
		base_lifecycle_generation: branch_record.lifecycle_generation,
		base_manifest_generation: snapshot.root.manifest_generation,
		input_fingerprint,
		input_range: PlannedInputRange::Reclaim(input_range),
		planned_at_ms: now_ms,
		attempt: 0,
	})
}

fn fingerprint_hot_inputs(
	database_branch_id: DatabaseBranchId,
	root: &CompactionRoot,
	head: &DBHead,
	coverage_txids: &[u64],
	hot_inputs: &HotInputSnapshot,
) -> CompactionInputFingerprint {
	let mut fingerprint = [0_u8; 32];
	mix_fingerprint(&mut fingerprint, database_branch_id.as_uuid().as_bytes());
	mix_fingerprint(&mut fingerprint, &root.manifest_generation.to_be_bytes());
	mix_fingerprint(&mut fingerprint, &root.hot_watermark_txid.to_be_bytes());
	mix_fingerprint(&mut fingerprint, &head.head_txid.to_be_bytes());
	for txid in coverage_txids {
		mix_fingerprint(&mut fingerprint, &txid.to_be_bytes());
	}
	for (txid, commit) in &hot_inputs.commits {
		mix_fingerprint(&mut fingerprint, &txid.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &commit.wall_clock_ms.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &commit.versionstamp);
		mix_fingerprint(&mut fingerprint, &commit.db_size_pages.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &commit.post_apply_checksum.to_be_bytes());
	}
	for (key, value) in &hot_inputs.delta_chunks {
		mix_fingerprint(&mut fingerprint, key);
		mix_fingerprint(&mut fingerprint, value);
	}
	for (key, value) in &hot_inputs.pidx_entries {
		mix_fingerprint(&mut fingerprint, key);
		mix_fingerprint(&mut fingerprint, value);
	}
	fingerprint
}

fn fingerprint_reclaim_inputs(
	database_branch_id: DatabaseBranchId,
	root: &CompactionRoot,
	reclaim_inputs: &ReclaimInputSnapshot,
) -> CompactionInputFingerprint {
	let mut fingerprint = [0_u8; 32];
	mix_fingerprint(&mut fingerprint, database_branch_id.as_uuid().as_bytes());
	mix_fingerprint(&mut fingerprint, &root.manifest_generation.to_be_bytes());
	mix_fingerprint(&mut fingerprint, &root.hot_watermark_txid.to_be_bytes());
	for txid_ref in &reclaim_inputs.txid_refs {
		mix_fingerprint(&mut fingerprint, &txid_ref.txid.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &txid_ref.versionstamp);
	}
	for cold_object in &reclaim_inputs.cold_object_refs {
		mix_fingerprint(&mut fingerprint, cold_object.object_key.as_bytes());
		mix_fingerprint(&mut fingerprint, &cold_object.object_generation_id.as_bytes());
		mix_fingerprint(&mut fingerprint, &cold_object.content_hash);
		mix_fingerprint(&mut fingerprint, &cold_object.expected_publish_generation.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &cold_object.shard_id.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &cold_object.as_of_txid.to_be_bytes());
	}
	for (txid, key, value, commit) in &reclaim_inputs.commits {
		mix_fingerprint(&mut fingerprint, &txid.to_be_bytes());
		mix_fingerprint(&mut fingerprint, key);
		mix_fingerprint(&mut fingerprint, value);
		mix_fingerprint(&mut fingerprint, &commit.versionstamp);
	}
	for (key, value) in &reclaim_inputs.delta_chunks {
		mix_fingerprint(&mut fingerprint, key);
		mix_fingerprint(&mut fingerprint, value);
	}
	for (key, value) in &reclaim_inputs.pidx_entries {
		mix_fingerprint(&mut fingerprint, key);
		mix_fingerprint(&mut fingerprint, value);
	}
	for (key, value) in &reclaim_inputs.coverage_shards {
		mix_fingerprint(&mut fingerprint, key);
		mix_fingerprint(&mut fingerprint, value);
	}
	fingerprint
}

fn fingerprint_repair_reclaim_range(
	database_branch_id: DatabaseBranchId,
	input_range: &ReclaimJobInputRange,
) -> CompactionInputFingerprint {
	let mut fingerprint = [0_u8; 32];
	mix_fingerprint(&mut fingerprint, database_branch_id.as_uuid().as_bytes());
	mix_fingerprint(&mut fingerprint, &input_range.txids.min_txid.to_be_bytes());
	mix_fingerprint(&mut fingerprint, &input_range.txids.max_txid.to_be_bytes());
	for staged in &input_range.staged_hot_shards {
		mix_fingerprint(&mut fingerprint, &staged.job_id.as_bytes());
		mix_fingerprint(&mut fingerprint, &staged.output_ref.shard_id.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &staged.output_ref.as_of_txid.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &staged.output_ref.size_bytes.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &staged.output_ref.content_hash);
	}
	for cold_ref in &input_range.orphan_cold_objects {
		mix_fingerprint(&mut fingerprint, cold_ref.object_key.as_bytes());
		mix_fingerprint(&mut fingerprint, &cold_ref.object_generation_id.as_bytes());
		mix_fingerprint(&mut fingerprint, &cold_ref.content_hash);
		mix_fingerprint(&mut fingerprint, &cold_ref.publish_generation.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &cold_ref.shard_id.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &cold_ref.as_of_txid.to_be_bytes());
	}
	fingerprint
}

fn fingerprint_cold_inputs(
	database_branch_id: DatabaseBranchId,
	root: &CompactionRoot,
	cold_inputs: &ColdInputSnapshot,
) -> CompactionInputFingerprint {
	let mut fingerprint = [0_u8; 32];
	mix_fingerprint(&mut fingerprint, database_branch_id.as_uuid().as_bytes());
	mix_fingerprint(&mut fingerprint, &root.manifest_generation.to_be_bytes());
	mix_fingerprint(&mut fingerprint, &root.hot_watermark_txid.to_be_bytes());
	mix_fingerprint(&mut fingerprint, &root.cold_watermark_txid.to_be_bytes());
	mix_fingerprint(&mut fingerprint, &root.cold_watermark_versionstamp);
	mix_fingerprint(&mut fingerprint, &cold_inputs.min_versionstamp);
	mix_fingerprint(&mut fingerprint, &cold_inputs.max_versionstamp);
	for (txid, commit) in &cold_inputs.commits {
		mix_fingerprint(&mut fingerprint, &txid.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &commit.wall_clock_ms.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &commit.versionstamp);
		mix_fingerprint(&mut fingerprint, &commit.db_size_pages.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &commit.post_apply_checksum.to_be_bytes());
	}
	for blob in &cold_inputs.shard_blobs {
		mix_fingerprint(&mut fingerprint, &blob.shard_id.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &blob.as_of_txid.to_be_bytes());
		mix_fingerprint(&mut fingerprint, &blob.key);
		mix_fingerprint(&mut fingerprint, &blob.bytes);
	}
	fingerprint
}

fn mix_fingerprint(fingerprint: &mut CompactionInputFingerprint, bytes: &[u8]) {
	for (idx, byte) in bytes.iter().enumerate() {
		let slot = idx % fingerprint.len();
		fingerprint[slot] = fingerprint[slot]
			.wrapping_mul(31)
			.wrapping_add(*byte)
			.wrapping_add(slot as u8);
	}
}

async fn write_staged_hot_shards(
	tx: &universaldb::Transaction,
	input: &StageHotJobInput,
	head: &DBHead,
	hot_inputs: &HotInputSnapshot,
) -> Result<Vec<HotShardOutputRef>> {
	let deltas = decode_hot_delta_chunks(input.database_branch_id, &hot_inputs.delta_chunks)?;
	let mut output_refs = Vec::new();

	for as_of_txid in &input.input_range.coverage_txids {
		let pages_by_shard = collect_hot_pages_by_shard(head, &deltas, *as_of_txid)?;

		for (shard_id, page_updates) in pages_by_shard {
			let encoded = build_staged_hot_shard_blob(
				tx,
				input.database_branch_id,
				shard_id,
				*as_of_txid,
				page_updates,
			)
			.await?;
			let key = keys::branch_compaction_stage_hot_shard_key(
				input.database_branch_id,
				input.job_id,
				shard_id,
				*as_of_txid,
				0,
			);
			let content_hash = content_hash(&encoded);

			tx.informal().set(&key, &encoded);
			output_refs.push(HotShardOutputRef {
				shard_id,
				as_of_txid: *as_of_txid,
				min_txid: input.input_range.txids.min_txid,
				max_txid: *as_of_txid,
				size_bytes: u64::try_from(encoded.len()).unwrap_or(u64::MAX),
				content_hash,
			});
		}
	}

	Ok(output_refs)
}

fn decode_hot_delta_chunks(
	branch_id: DatabaseBranchId,
	delta_chunks: &[(Vec<u8>, Vec<u8>)],
) -> Result<BTreeMap<u64, DecodedLtx>> {
	let mut chunks_by_txid = BTreeMap::<u64, BTreeMap<u32, Vec<u8>>>::new();
	for (key, value) in delta_chunks {
		let txid = keys::decode_branch_delta_chunk_txid(branch_id, key)?;
		let chunk_idx = keys::decode_branch_delta_chunk_idx(branch_id, txid, key)?;
		chunks_by_txid
			.entry(txid)
			.or_default()
			.insert(chunk_idx, value.clone());
	}

	chunks_by_txid
		.into_iter()
		.map(|(txid, chunks)| {
			let bytes = chunks
				.into_values()
				.flatten()
				.collect::<Vec<_>>();
			let decoded =
				decode_ltx_v3(&bytes).with_context(|| format!("decode hot delta {txid}"))?;

			Ok((txid, decoded))
		})
		.collect()
}

fn collect_hot_pages_by_shard(
	head: &DBHead,
	deltas: &BTreeMap<u64, DecodedLtx>,
	as_of_txid: u64,
) -> Result<BTreeMap<u32, Vec<(u32, Vec<u8>)>>> {
	let mut pages_by_number = BTreeMap::<u32, Vec<u8>>::new();

	for (txid, delta) in deltas {
		if *txid > as_of_txid {
			continue;
		}
		for page in &delta.pages {
			if page.pgno <= head.db_size_pages {
				pages_by_number.insert(page.pgno, page.bytes.clone());
			}
		}
	}

	let mut pages_by_shard = BTreeMap::<u32, Vec<(u32, Vec<u8>)>>::new();
	for (pgno, bytes) in pages_by_number {
		pages_by_shard.entry(pgno / keys::SHARD_SIZE).or_default().push((pgno, bytes));
	}
	Ok(pages_by_shard)
}

async fn build_staged_hot_shard_blob(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	shard_id: u32,
	as_of_txid: u64,
	page_updates: Vec<(u32, Vec<u8>)>,
) -> Result<Vec<u8>> {
	let existing_blob = load_latest_branch_shard_blob(tx, branch_id, shard_id, as_of_txid).await?;
	let mut merged_pages = BTreeMap::<u32, Vec<u8>>::new();
	let mut timestamp_ms = 0;

	if let Some(existing_blob) = existing_blob {
		let decoded = decode_ltx_v3(&existing_blob).context("decode existing branch shard blob")?;
		timestamp_ms = decoded.header.timestamp_ms;
		for page in decoded.pages {
			if page.pgno / keys::SHARD_SIZE == shard_id {
				ensure!(
					page.bytes.len() == keys::PAGE_SIZE as usize,
					"page {} had {} bytes, expected {}",
					page.pgno,
					page.bytes.len(),
					keys::PAGE_SIZE
				);
				merged_pages.insert(page.pgno, page.bytes);
			}
		}
	}

	for (pgno, bytes) in page_updates {
		ensure!(pgno > 0, "page number must be greater than zero");
		ensure!(
			pgno / keys::SHARD_SIZE == shard_id,
			"page {} does not belong to shard {}",
			pgno,
			shard_id
		);
		ensure!(
			bytes.len() == keys::PAGE_SIZE as usize,
			"page {} had {} bytes, expected {}",
			pgno,
			bytes.len(),
			keys::PAGE_SIZE
		);
		merged_pages.insert(pgno, bytes);
	}

	let pages = merged_pages
		.into_iter()
		.map(|(pgno, bytes)| DirtyPage { pgno, bytes })
		.collect::<Vec<_>>();
	let commit = pages.iter().map(|page| page.pgno).max().unwrap_or(1);
	let header = LtxHeader::delta(as_of_txid, commit, timestamp_ms);

	encode_ltx_v3(header, &pages).context("encode staged hot shard blob")
}

async fn load_latest_branch_shard_blob(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	shard_id: u32,
	as_of_txid: u64,
) -> Result<Option<Vec<u8>>> {
	let prefix = keys::branch_shard_version_prefix(branch_id, shard_id);
	let end = end_of_key_range(&keys::branch_shard_key(branch_id, shard_id, as_of_txid));
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..(prefix.as_slice(), end.as_slice()).into()
		},
		Snapshot,
	);

	let mut latest = None;
	while let Some(entry) = stream.try_next().await? {
		latest = Some(entry.value().to_vec());
	}

	Ok(latest)
}

fn decode_branch_pidx_pgno(branch_id: DatabaseBranchId, key: &[u8]) -> Result<u32> {
	let prefix = keys::branch_pidx_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("branch PIDX key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u32>()] = suffix
		.try_into()
		.map_err(|_| anyhow::anyhow!("branch PIDX key suffix had invalid length"))?;

	Ok(u32::from_be_bytes(bytes))
}

fn content_hash(bytes: &[u8]) -> [u8; 32] {
	let digest = Sha256::digest(bytes);
	let mut hash = [0_u8; 32];
	hash.copy_from_slice(&digest);
	hash
}

fn expected_cold_output_refs(
	input: &PublishColdJobInput,
	cold_inputs: &ColdInputSnapshot,
	publish_generation: u64,
) -> Vec<ColdShardRef> {
	cold_inputs
		.shard_blobs
		.iter()
		.map(|blob| {
			let content_hash = content_hash(&blob.bytes);
			ColdShardRef {
				object_key: cold_shard_object_key(
					input.database_branch_id,
					blob.shard_id,
					blob.as_of_txid,
					input.job_id,
					content_hash,
				),
				object_generation_id: input.job_id,
				shard_id: blob.shard_id,
				as_of_txid: blob.as_of_txid,
				min_txid: input.input_range.txids.min_txid,
				max_txid: blob.as_of_txid,
				min_versionstamp: input.input_range.min_versionstamp,
				max_versionstamp: input.input_range.max_versionstamp,
				size_bytes: u64::try_from(blob.bytes.len()).unwrap_or(u64::MAX),
				content_hash,
				publish_generation,
			}
		})
		.collect()
}

fn reclaim_cold_object_ref(cold_ref: &ColdShardRef) -> ReclaimColdObjectRef {
	ReclaimColdObjectRef {
		object_key: cold_ref.object_key.clone(),
		object_generation_id: cold_ref.object_generation_id,
		content_hash: cold_ref.content_hash,
		expected_publish_generation: cold_ref.publish_generation,
		shard_id: cold_ref.shard_id,
		as_of_txid: cold_ref.as_of_txid,
	}
}

fn retired_matches_cold_object(
	retired: &RetiredColdObject,
	cold_object: &ReclaimColdObjectRef,
) -> bool {
	retired.object_key == cold_object.object_key
		&& retired.object_generation_id == cold_object.object_generation_id
		&& retired.content_hash == cold_object.content_hash
}

fn cold_shard_object_key(
	branch_id: DatabaseBranchId,
	shard_id: u32,
	as_of_txid: u64,
	object_generation_id: Id,
	content_hash: [u8; 32],
) -> String {
	format!(
		"db/{}/shard/{shard_id:08x}/{as_of_txid:016x}-{object_generation_id}-{}.ltx",
		branch_id.as_uuid().simple(),
		hex_lower(&content_hash)
	)
}

fn hex_lower(bytes: &[u8]) -> String {
	const HEX: &[u8; 16] = b"0123456789abcdef";
	let mut out = String::with_capacity(bytes.len() * 2);
	for byte in bytes {
		out.push(HEX[(byte >> 4) as usize] as char);
		out.push(HEX[(byte & 0x0f) as usize] as char);
	}
	out
}

async fn workflow_cold_tier() -> Result<Arc<dyn ColdTier>> {
	#[cfg(debug_assertions)]
	if let Some(cold_tier) = WORKFLOW_TEST_COLD_TIER.lock().clone() {
		return Ok(cold_tier);
	}

	if let Ok(root) = env::var("RIVET_SQLITE_WORKFLOW_COLD_TIER_FS_ROOT") {
		return Ok(Arc::new(FilesystemColdTier::new(root)));
	}

	if let Ok(bucket) = env::var("RIVET_SQLITE_WORKFLOW_COLD_TIER_S3_BUCKET") {
		let prefix =
			env::var("RIVET_SQLITE_WORKFLOW_COLD_TIER_S3_PREFIX").unwrap_or_default();
		let endpoint_url = env::var("RIVET_SQLITE_WORKFLOW_COLD_TIER_S3_ENDPOINT").ok();
		return Ok(Arc::new(
			S3ColdTier::from_env(bucket, prefix, endpoint_url).await?,
		));
	}

	Ok(Arc::new(DisabledColdTier))
}

#[cfg(debug_assertions)]
pub fn set_workflow_test_cold_tier_for_test(cold_tier: Option<Arc<dyn ColdTier>>) {
	*WORKFLOW_TEST_COLD_TIER.lock() = cold_tier;
}

fn decode_branch_shard_version_key(
	branch_id: DatabaseBranchId,
	key: &[u8],
) -> Result<Option<(u32, u64)>> {
	let prefix = keys::branch_shard_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("branch shard key did not start with expected prefix")?;
	if suffix.len() == std::mem::size_of::<u32>() {
		return Ok(None);
	}
	if suffix.len() != std::mem::size_of::<u32>() + 1 + std::mem::size_of::<u64>()
		|| suffix[std::mem::size_of::<u32>()] != b'/'
	{
		bail!("branch shard version key suffix had invalid length");
	}
	let shard_id = u32::from_be_bytes(
		suffix[..std::mem::size_of::<u32>()]
			.try_into()
			.context("decode branch shard id")?,
	);
	let as_of_txid = u64::from_be_bytes(
		suffix[std::mem::size_of::<u32>() + 1..]
			.try_into()
			.context("decode branch shard txid")?,
	);

	Ok(Some((shard_id, as_of_txid)))
}

fn decode_branch_commit_txid(branch_id: DatabaseBranchId, key: &[u8]) -> Result<u64> {
	let prefix = keys::branch_commit_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("branch commit key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u64>()] = suffix
		.try_into()
		.map_err(|_| anyhow::anyhow!("branch commit key suffix had invalid length"))?;

	Ok(u64::from_be_bytes(bytes))
}

fn decode_pidx_txid(value: &[u8]) -> Result<u64> {
	let bytes: [u8; std::mem::size_of::<u64>()] = value
		.try_into()
		.map_err(|_| anyhow::anyhow!("branch pidx value had invalid length"))?;

	Ok(u64::from_be_bytes(bytes))
}

async fn tx_get_value(
	tx: &universaldb::Transaction,
	key: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Option<Vec<u8>>> {
	Ok(tx
		.informal()
		.get(key, isolation_level)
		.await?
		.map(Vec::<u8>::from))
}

async fn tx_scan_prefix_values(
	tx: &universaldb::Transaction,
	prefix: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		isolation_level,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok(rows)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompanionKind {
	Hot,
	Cold,
	Reclaim,
}

async fn run_companion_loop(
	ctx: &mut WorkflowCtx,
	database_branch_id: DatabaseBranchId,
	kind: CompanionKind,
) -> Result<()> {
	ctx.lupe()
		.commit_interval(1)
		.with_state(CompanionWorkflowState::Idle)
		.run(|ctx, state| {
			async move {
				match kind {
					CompanionKind::Hot => {
						for signal in ctx.listen_n::<DbHotCompacterSignal>(256).await? {
							match signal {
								DbHotCompacterSignal::RunHotJob(signal) => {
									if signal.database_branch_id == database_branch_id {
										run_hot_compaction_job(
											ctx,
											state,
											database_branch_id,
											signal,
										)
										.await?;
									}
								}
								DbHotCompacterSignal::DestroyDatabaseBranch(signal) => {
									if signal.database_branch_id == database_branch_id {
										record_companion_stop(
											state,
											signal.lifecycle_generation,
											signal.requested_at_ms,
											signal.reason,
										);
									}
								}
							}
						}
					}
					CompanionKind::Cold => {
						for signal in ctx.listen_n::<DbColdCompacterSignal>(256).await? {
							match signal {
								DbColdCompacterSignal::RunColdJob(signal) => {
									if signal.database_branch_id == database_branch_id {
										run_cold_compaction_job(
											ctx,
											state,
											database_branch_id,
											signal,
										)
										.await?;
									}
								}
								DbColdCompacterSignal::DestroyDatabaseBranch(signal) => {
									if signal.database_branch_id == database_branch_id {
										record_companion_stop(
											state,
											signal.lifecycle_generation,
											signal.requested_at_ms,
											signal.reason,
										);
									}
								}
							}
						}
					}
					CompanionKind::Reclaim => {
						for signal in ctx.listen_n::<DbReclaimerSignal>(256).await? {
							match signal {
								DbReclaimerSignal::RunReclaimJob(signal) => {
									if signal.database_branch_id == database_branch_id {
										run_reclaim_job(ctx, state, database_branch_id, signal)
											.await?;
									}
								}
								DbReclaimerSignal::DestroyDatabaseBranch(signal) => {
									if signal.database_branch_id == database_branch_id {
										record_companion_stop(
											state,
											signal.lifecycle_generation,
											signal.requested_at_ms,
											signal.reason,
										);
									}
								}
							}
						}
					}
				}

				if matches!(state, CompanionWorkflowState::Stopping { .. }) {
					Ok(Loop::Break(()))
				} else {
					Ok(Loop::Continue)
				}
			}
			.boxed()
		})
	.await
}

async fn run_hot_compaction_job(
	ctx: &mut WorkflowCtx,
	state: &mut CompanionWorkflowState,
	database_branch_id: DatabaseBranchId,
	signal: RunHotJob,
) -> Result<()> {
	if matches!(state, CompanionWorkflowState::Stopping { .. }) {
		return Ok(());
	}
	record_companion_job(
		state,
		database_branch_id,
		CompactionJobKind::Hot,
		signal.job_id,
		signal.base_lifecycle_generation,
		signal.base_manifest_generation,
		signal.input_fingerprint,
		ctx.create_ts(),
	);

	let output = ctx
		.activity(StageHotJobInput {
			database_branch_id,
			job_id: signal.job_id,
			job_kind: signal.job_kind,
			base_lifecycle_generation: signal.base_lifecycle_generation,
			base_manifest_generation: signal.base_manifest_generation,
			input_fingerprint: signal.input_fingerprint,
			input_range: signal.input_range,
		})
		.await?;

	let tag_value = database_branch_tag_value(database_branch_id);
	ctx.signal(HotJobFinished {
		database_branch_id,
		job_id: signal.job_id,
		job_kind: CompactionJobKind::Hot,
		base_manifest_generation: signal.base_manifest_generation,
		input_fingerprint: signal.input_fingerprint,
		status: output.status,
		output_refs: output.output_refs,
	})
	.to_workflow::<DbManagerWorkflow>()
	.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
	.send()
	.await?;

	*state = CompanionWorkflowState::Idle;

	Ok(())
}

async fn run_cold_compaction_job(
	ctx: &mut WorkflowCtx,
	state: &mut CompanionWorkflowState,
	database_branch_id: DatabaseBranchId,
	signal: RunColdJob,
) -> Result<()> {
	if matches!(state, CompanionWorkflowState::Stopping { .. }) {
		return Ok(());
	}
	record_companion_job(
		state,
		database_branch_id,
		CompactionJobKind::Cold,
		signal.job_id,
		signal.base_lifecycle_generation,
		signal.base_manifest_generation,
		signal.input_fingerprint,
		ctx.create_ts(),
	);

	let output = ctx
		.activity(UploadColdJobInput {
			database_branch_id,
			job_id: signal.job_id,
			job_kind: signal.job_kind,
			base_lifecycle_generation: signal.base_lifecycle_generation,
			base_manifest_generation: signal.base_manifest_generation,
			input_fingerprint: signal.input_fingerprint,
			input_range: signal.input_range,
		})
		.await?;

	let tag_value = database_branch_tag_value(database_branch_id);
	ctx.signal(ColdJobFinished {
		database_branch_id,
		job_id: signal.job_id,
		job_kind: CompactionJobKind::Cold,
		base_manifest_generation: signal.base_manifest_generation,
		input_fingerprint: signal.input_fingerprint,
		status: output.status,
		output_refs: output.output_refs,
	})
	.to_workflow::<DbManagerWorkflow>()
	.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
	.send()
	.await?;

	*state = CompanionWorkflowState::Idle;

	Ok(())
}

async fn run_reclaim_job(
	ctx: &mut WorkflowCtx,
	state: &mut CompanionWorkflowState,
	database_branch_id: DatabaseBranchId,
	signal: RunReclaimJob,
) -> Result<()> {
	if matches!(state, CompanionWorkflowState::Stopping { .. }) {
		return Ok(());
	}
	record_companion_job(
		state,
		database_branch_id,
		CompactionJobKind::Reclaim,
		signal.job_id,
		signal.base_lifecycle_generation,
		signal.base_manifest_generation,
		signal.input_fingerprint,
		ctx.create_ts(),
	);

	let output = ctx
		.activity(ReclaimFdbJobInput {
			database_branch_id,
			job_id: signal.job_id,
			job_kind: signal.job_kind,
			base_lifecycle_generation: signal.base_lifecycle_generation,
			base_manifest_generation: signal.base_manifest_generation,
			input_fingerprint: signal.input_fingerprint,
			input_range: signal.input_range.clone(),
		})
		.await?;

	let mut status = output.status;
	let output_refs = output.output_refs;

	if matches!(status, CompactionJobStatus::Succeeded)
		&& !signal.input_range.cold_objects.is_empty()
	{
		let validated = ctx
			.activity(ValidateReclaimColdObjectsInput {
				database_branch_id,
				cold_objects: signal.input_range.cold_objects.clone(),
			})
			.await?;
		status = validated.status;
	}

	if matches!(status, CompactionJobStatus::Succeeded)
		&& !signal.input_range.cold_objects.is_empty()
	{
		let retired = ctx
			.activity(RetireColdObjectsInput {
				database_branch_id,
				job_id: signal.job_id,
				job_kind: signal.job_kind,
				base_lifecycle_generation: signal.base_lifecycle_generation,
				base_manifest_generation: signal.base_manifest_generation,
				input_fingerprint: signal.input_fingerprint,
				cold_objects: signal.input_range.cold_objects.clone(),
				retired_at_ms: ctx.create_ts(),
			})
			.await?;
		status = retired.status;

		if matches!(status, CompactionJobStatus::Succeeded) {
			let delete_now_ms = if let Some(delete_after_ms) = retired.delete_after_ms {
				ctx.sleep_until(delete_after_ms).await?;
				delete_after_ms
			} else {
				ctx.create_ts()
			};

			let deleted = ctx
				.activity(DeleteRetiredColdObjectsInput {
					database_branch_id,
					cold_objects: signal.input_range.cold_objects.clone(),
					now_ms: delete_now_ms,
				})
				.await?;
			status = deleted.status;

			if matches!(status, CompactionJobStatus::Succeeded) {
				let cleaned = ctx
					.activity(CleanupRetiredColdObjectsInput {
						database_branch_id,
						cold_objects: signal.input_range.cold_objects.clone(),
					})
					.await?;
				status = cleaned.status;
			}
		}
	}

	if matches!(status, CompactionJobStatus::Succeeded)
		&& !signal.input_range.orphan_cold_objects.is_empty()
	{
		let deleted = ctx
			.activity(DeleteOrphanColdObjectsInput {
				database_branch_id,
				orphan_cold_objects: signal.input_range.orphan_cold_objects.clone(),
			})
			.await?;
		status = deleted.status;
	}

	let tag_value = database_branch_tag_value(database_branch_id);
	ctx.signal(ReclaimJobFinished {
		database_branch_id,
		job_id: signal.job_id,
		job_kind: CompactionJobKind::Reclaim,
		base_manifest_generation: signal.base_manifest_generation,
		input_fingerprint: signal.input_fingerprint,
		status,
		output_refs,
	})
	.to_workflow::<DbManagerWorkflow>()
	.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
	.send()
	.await?;

	*state = CompanionWorkflowState::Idle;

	Ok(())
}

fn record_companion_job(
	state: &mut CompanionWorkflowState,
	database_branch_id: DatabaseBranchId,
	job_kind: CompactionJobKind,
	job_id: Id,
	base_lifecycle_generation: u64,
	base_manifest_generation: u64,
	input_fingerprint: CompactionInputFingerprint,
	started_at_ms: i64,
) {
	*state = CompanionWorkflowState::Running(CompanionRunningJob {
		database_branch_id,
		job_id,
		job_kind,
		base_lifecycle_generation,
		base_manifest_generation,
		input_fingerprint,
		started_at_ms,
		attempt: 0,
	});
}

fn record_companion_stop(
	state: &mut CompanionWorkflowState,
	lifecycle_generation: u64,
	requested_at_ms: i64,
	reason: String,
) {
	let active_job = match std::mem::replace(state, CompanionWorkflowState::Idle) {
		CompanionWorkflowState::Running(job) => Some(job),
		CompanionWorkflowState::Stopping { active_job, .. } => active_job,
		CompanionWorkflowState::Idle => None,
	};

	*state = CompanionWorkflowState::Stopping {
		active_job,
		lifecycle_generation,
		requested_at_ms,
		reason,
	};
}

enum VersionedDeltasAvailable {
	V1(DeltasAvailable),
}

enum VersionedHotJobFinished {
	V1(HotJobFinished),
}

enum VersionedColdJobFinished {
	V1(ColdJobFinished),
}

enum VersionedReclaimJobFinished {
	V1(ReclaimJobFinished),
}

enum VersionedDestroyDatabaseBranch {
	V1(DestroyDatabaseBranch),
}

enum VersionedRunHotJob {
	V1(RunHotJob),
}

enum VersionedRunColdJob {
	V1(RunColdJob),
}

enum VersionedRunReclaimJob {
	V1(RunReclaimJob),
}

enum VersionedDbManagerState {
	V1(DbManagerState),
}

enum VersionedCompanionWorkflowState {
	V1(CompanionWorkflowState),
}

macro_rules! impl_workflow_compaction_versioned_data {
	($versioned:ident, $latest:ty, $name:literal, $encode:ident, $decode:ident) => {
		impl OwnedVersionedData for $versioned {
			type Latest = $latest;

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V1(latest)
			}

			fn unwrap_latest(self) -> Result<Self::Latest> {
				match self {
					Self::V1(data) => Ok(data),
				}
			}

			fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
				match version {
					1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
					_ => bail!("invalid depot workflow compaction {} version: {version}", $name),
				}
			}

			fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
				match self {
					Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
				}
			}
		}

		pub fn $encode(payload: $latest) -> Result<Vec<u8>> {
			$versioned::wrap_latest(payload)
				.serialize_with_embedded_version(SQLITE_COMPACTION_WORKFLOW_PAYLOAD_VERSION)
				.with_context(|| format!("encode sqlite workflow compaction {}", $name))
		}

		pub fn $decode(payload: &[u8]) -> Result<$latest> {
			$versioned::deserialize_with_embedded_version(payload)
				.with_context(|| format!("decode sqlite workflow compaction {}", $name))
		}
	};
}

impl_workflow_compaction_versioned_data!(
	VersionedDeltasAvailable,
	DeltasAvailable,
	"DeltasAvailable",
	encode_deltas_available,
	decode_deltas_available
);
impl_workflow_compaction_versioned_data!(
	VersionedHotJobFinished,
	HotJobFinished,
	"HotJobFinished",
	encode_hot_job_finished,
	decode_hot_job_finished
);
impl_workflow_compaction_versioned_data!(
	VersionedColdJobFinished,
	ColdJobFinished,
	"ColdJobFinished",
	encode_cold_job_finished,
	decode_cold_job_finished
);
impl_workflow_compaction_versioned_data!(
	VersionedReclaimJobFinished,
	ReclaimJobFinished,
	"ReclaimJobFinished",
	encode_reclaim_job_finished,
	decode_reclaim_job_finished
);
impl_workflow_compaction_versioned_data!(
	VersionedDestroyDatabaseBranch,
	DestroyDatabaseBranch,
	"DestroyDatabaseBranch",
	encode_destroy_database_branch,
	decode_destroy_database_branch
);
impl_workflow_compaction_versioned_data!(
	VersionedRunHotJob,
	RunHotJob,
	"RunHotJob",
	encode_run_hot_job,
	decode_run_hot_job
);
impl_workflow_compaction_versioned_data!(
	VersionedRunColdJob,
	RunColdJob,
	"RunColdJob",
	encode_run_cold_job,
	decode_run_cold_job
);
impl_workflow_compaction_versioned_data!(
	VersionedRunReclaimJob,
	RunReclaimJob,
	"RunReclaimJob",
	encode_run_reclaim_job,
	decode_run_reclaim_job
);
impl_workflow_compaction_versioned_data!(
	VersionedDbManagerState,
	DbManagerState,
	"DbManagerState",
	encode_db_manager_state,
	decode_db_manager_state
);
impl_workflow_compaction_versioned_data!(
	VersionedCompanionWorkflowState,
	CompanionWorkflowState,
	"CompanionWorkflowState",
	encode_companion_workflow_state,
	decode_companion_workflow_state
);
