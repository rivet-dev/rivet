use anyhow::{Context, Result, bail};
use futures_util::FutureExt;
use gas::prelude::*;
use serde::{Deserialize, Serialize};
use vbare::OwnedVersionedData;

use crate::conveyer::types::{ColdShardRef, DatabaseBranchId};

pub const SQLITE_COMPACTION_WORKFLOW_PAYLOAD_VERSION: u16 = 1;
pub const DATABASE_BRANCH_ID_TAG: &str = "database_branch_id";

pub type CompactionInputFingerprint = [u8; 32];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxidRange {
	pub min_txid: u64,
	pub max_txid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotJobInputRange {
	pub txids: TxidRange,
	pub max_pages: u32,
	pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColdJobInputRange {
	pub txids: TxidRange,
	pub min_versionstamp: [u8; 16],
	pub max_versionstamp: [u8; 16],
	pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimJobInputRange {
	pub txids: TxidRange,
	pub max_keys: u32,
	pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlannedInputRange {
	Hot(HotJobInputRange),
	Cold(ColdJobInputRange),
	Reclaim(ReclaimJobInputRange),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
	pub requested_at_ms: i64,
	pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_run_hot_job")]
pub struct RunHotJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchStopState {
	Running,
	DestroyRequested { requested_at_ms: i64, reason: String },
	Stopping { requested_at_ms: i64, reason: String },
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
		requested_at_ms: i64,
		reason: String,
	},
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanionRunningJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub started_at_ms: i64,
	pub attempt: u32,
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

	ctx.lupe()
		.commit_interval(1)
		.with_state(DbManagerState::new(companion_workflow_ids))
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
								state.active_hot_job = None;
							}
						}
						DbManagerSignal::ColdJobFinished(signal) => {
							if signal.database_branch_id == input.database_branch_id {
								state.active_cold_job = None;
							}
						}
						DbManagerSignal::ReclaimJobFinished(signal) => {
							if signal.database_branch_id == input.database_branch_id {
								state.active_reclaim_job = None;
							}
						}
						DbManagerSignal::DestroyDatabaseBranch(signal) => {
							if signal.database_branch_id == input.database_branch_id {
								state.branch_stop_state = BranchStopState::DestroyRequested {
									requested_at_ms: signal.requested_at_ms,
									reason: signal.reason,
								};
							}
						}
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
										record_companion_job(
											state,
											database_branch_id,
											CompactionJobKind::Hot,
											signal.job_id,
											signal.base_manifest_generation,
											signal.input_fingerprint,
										);
									}
								}
								DbHotCompacterSignal::DestroyDatabaseBranch(signal) => {
									if signal.database_branch_id == database_branch_id {
										record_companion_stop(
											state,
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
										record_companion_job(
											state,
											database_branch_id,
											CompactionJobKind::Cold,
											signal.job_id,
											signal.base_manifest_generation,
											signal.input_fingerprint,
										);
									}
								}
								DbColdCompacterSignal::DestroyDatabaseBranch(signal) => {
									if signal.database_branch_id == database_branch_id {
										record_companion_stop(
											state,
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
										record_companion_job(
											state,
											database_branch_id,
											CompactionJobKind::Reclaim,
											signal.job_id,
											signal.base_manifest_generation,
											signal.input_fingerprint,
										);
									}
								}
								DbReclaimerSignal::DestroyDatabaseBranch(signal) => {
									if signal.database_branch_id == database_branch_id {
										record_companion_stop(
											state,
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

fn record_companion_job(
	state: &mut CompanionWorkflowState,
	database_branch_id: DatabaseBranchId,
	job_kind: CompactionJobKind,
	job_id: Id,
	base_manifest_generation: u64,
	input_fingerprint: CompactionInputFingerprint,
) {
	*state = CompanionWorkflowState::Running(CompanionRunningJob {
		database_branch_id,
		job_id,
		job_kind,
		base_manifest_generation,
		input_fingerprint,
		started_at_ms: 0,
		attempt: 0,
	});
	*state = CompanionWorkflowState::Idle;
}

fn record_companion_stop(
	state: &mut CompanionWorkflowState,
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
