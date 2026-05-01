use std::collections::{BTreeMap, BTreeSet};

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
	CMP_FDB_BATCH_MAX_KEYS, CMP_FDB_BATCH_MAX_VALUE_BYTES,
	HOT_BURST_COLD_LAG_THRESHOLD_TXIDS,
	conveyer::{
		keys, quota,
		ltx::{DecodedLtx, LtxHeader, decode_ltx_v3, encode_ltx_v3},
		types::{
			BranchState, ColdShardRef, CommitRow, CompactionRoot, DBHead, DatabaseBranchId,
			DatabaseBranchRecord, DbHistoryPin, DirtyPage, SqliteCmpDirty, decode_commit_row,
			decode_compaction_root, decode_database_branch_record, decode_db_head,
			decode_db_history_pin, decode_sqlite_cmp_dirty, encode_compaction_root,
		},
		udb,
	},
};

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

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct StageHotJobInput {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
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
pub struct RefreshManagerInput {
	pub database_branch_id: DatabaseBranchId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshManagerOutput {
	pub planning_deadlines: ManagerPlanningDeadlines,
	pub planned_hot_job: Option<ActiveCompactionJob>,
	pub observed_dirty: Option<SqliteCmpDirty>,
	pub head_txid: Option<u64>,
	pub branch_is_live: bool,
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
	cleared_dirty: bool,
}

#[derive(Debug, Default)]
struct HotInputSnapshot {
	commits: Vec<(u64, CommitRow)>,
	delta_chunks: Vec<(Vec<u8>, Vec<u8>)>,
	pidx_entries: Vec<(Vec<u8>, Vec<u8>)>,
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
								if let Some(active_job) = state.active_hot_job.as_ref() {
									if hot_job_finished_matches_active(&signal, active_job) {
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
									}
								}
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

	Ok(RefreshManagerOutput {
		planning_deadlines: ManagerPlanningDeadlines::after_refresh(now_ms),
		planned_hot_job,
		observed_dirty: if snapshot.cleared_dirty {
			None
		} else {
			snapshot.dirty
		},
		head_txid,
		branch_is_live,
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
	if !branch_record
		.as_ref()
		.is_some_and(|record| record.state == BranchState::Live)
	{
		return Ok(rejected_hot_job("database branch is not live"));
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

	let hot_inputs =
		read_hot_input_snapshot(tx, input.database_branch_id, Some(&head), &root, Snapshot)
			.await?;
	let input_fingerprint =
		fingerprint_hot_inputs(input.database_branch_id, &root, &head, &hot_inputs);
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
	if !branch_record
		.as_ref()
		.is_some_and(|record| record.state == BranchState::Live)
	{
		return Ok(rejected_hot_install("database branch is not live"));
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

	let hot_inputs = read_hot_input_snapshot(
		tx,
		input.database_branch_id,
		Some(&head),
		&root,
		Serializable,
	)
	.await?;
	let input_fingerprint =
		fingerprint_hot_inputs(input.database_branch_id, &root, &head, &hot_inputs);
	if input_fingerprint != input.input_fingerprint {
		return Ok(rejected_hot_install("hot compaction input fingerprint changed"));
	}

	let mut staged_blobs = Vec::with_capacity(input.output_refs.len());
	let mut staged_shards = BTreeSet::new();
	for output_ref in &input.output_refs {
		if output_ref.as_of_txid != input.input_range.txids.max_txid
			|| output_ref.min_txid != input.input_range.txids.min_txid
			|| output_ref.max_txid != input.input_range.txids.max_txid
		{
			return Ok(rejected_hot_install("hot output ref does not match planned txid range"));
		}
		if !staged_shards.insert(output_ref.shard_id) {
			return Ok(rejected_hot_install("duplicate staged hot shard output ref"));
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
		if !staged_shards.contains(&shard_id) {
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
	let db_pins = read_db_history_pins(tx, branch_id).await?;
	let hot_inputs = read_hot_input_snapshot(tx, branch_id, head.as_ref(), &root, Snapshot).await?;
	let hot_lag = head
		.as_ref()
		.map_or(0, |head| head.head_txid.saturating_sub(root.hot_watermark_txid));
	let cold_lag = head
		.as_ref()
		.map_or(0, |head| head.head_txid.saturating_sub(root.cold_watermark_txid));
	let has_actionable_lag = hot_lag >= quota::COMPACTION_DELTA_THRESHOLD
		|| cold_lag >= HOT_BURST_COLD_LAG_THRESHOLD_TXIDS;
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
		cleared_dirty,
	})
}

async fn read_db_history_pins(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
) -> Result<Vec<DbHistoryPin>> {
	let rows = tx_scan_prefix_values(tx, &keys::db_pin_prefix(branch_id), Serializable).await?;

	rows.into_iter()
		.map(|(_, value)| decode_db_history_pin(&value))
		.collect::<Result<Vec<_>>>()
		.context("decode sqlite db history pins for compaction manager")
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
		if snapshot.commits.len() >= CMP_FDB_BATCH_MAX_KEYS
			|| snapshot.total_value_bytes >= CMP_FDB_BATCH_MAX_VALUE_BYTES as u64
		{
			break;
		}
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

fn plan_hot_job(
	database_branch_id: DatabaseBranchId,
	snapshot: &ManagerFdbSnapshot,
	job_id: Id,
	now_ms: i64,
) -> Option<ActiveCompactionJob> {
	let head = snapshot.head.as_ref()?;
	let hot_lag = head.head_txid.saturating_sub(snapshot.root.hot_watermark_txid);
	if hot_lag < quota::COMPACTION_DELTA_THRESHOLD {
		return None;
	}

	let input_range = HotJobInputRange {
		txids: TxidRange {
			min_txid: snapshot.root.hot_watermark_txid.saturating_add(1),
			max_txid: head.head_txid,
		},
		max_pages: u32::try_from(snapshot.hot_inputs.pidx_entries.len()).unwrap_or(u32::MAX),
		max_bytes: snapshot.hot_inputs.total_value_bytes,
	};
	let input_fingerprint =
		fingerprint_hot_inputs(database_branch_id, &snapshot.root, head, &snapshot.hot_inputs);

	Some(ActiveCompactionJob {
		database_branch_id,
		job_id,
		job_kind: CompactionJobKind::Hot,
		base_manifest_generation: snapshot.root.manifest_generation,
		input_fingerprint,
		input_range: PlannedInputRange::Hot(input_range),
		planned_at_ms: now_ms,
		attempt: 0,
	})
}

fn fingerprint_hot_inputs(
	database_branch_id: DatabaseBranchId,
	root: &CompactionRoot,
	head: &DBHead,
	hot_inputs: &HotInputSnapshot,
) -> CompactionInputFingerprint {
	let mut fingerprint = [0_u8; 32];
	mix_fingerprint(&mut fingerprint, database_branch_id.as_uuid().as_bytes());
	mix_fingerprint(&mut fingerprint, &root.manifest_generation.to_be_bytes());
	mix_fingerprint(&mut fingerprint, &root.hot_watermark_txid.to_be_bytes());
	mix_fingerprint(&mut fingerprint, &head.head_txid.to_be_bytes());
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
	let pages_by_shard = collect_hot_pages_by_shard(
		input.database_branch_id,
		head,
		&deltas,
		&hot_inputs.pidx_entries,
	)?;
	let mut output_refs = Vec::with_capacity(pages_by_shard.len());

	for (shard_id, page_updates) in pages_by_shard {
		let encoded = build_staged_hot_shard_blob(
			tx,
			input.database_branch_id,
			shard_id,
			input.input_range.txids.max_txid,
			page_updates,
		)
		.await?;
		let key = keys::branch_compaction_stage_hot_shard_key(
			input.database_branch_id,
			input.job_id,
			shard_id,
			input.input_range.txids.max_txid,
			0,
		);
		let content_hash = content_hash(&encoded);

		tx.informal().set(&key, &encoded);
		output_refs.push(HotShardOutputRef {
			shard_id,
			as_of_txid: input.input_range.txids.max_txid,
			min_txid: input.input_range.txids.min_txid,
			max_txid: input.input_range.txids.max_txid,
			size_bytes: u64::try_from(encoded.len()).unwrap_or(u64::MAX),
			content_hash,
		});
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
	branch_id: DatabaseBranchId,
	head: &DBHead,
	deltas: &BTreeMap<u64, DecodedLtx>,
	pidx_entries: &[(Vec<u8>, Vec<u8>)],
) -> Result<BTreeMap<u32, Vec<(u32, Vec<u8>)>>> {
	let mut pages_by_shard = BTreeMap::<u32, Vec<(u32, Vec<u8>)>>::new();

	for (key, value) in pidx_entries {
		let txid = decode_pidx_txid(value)?;
		let Some(delta) = deltas.get(&txid) else {
			continue;
		};
		let pgno = decode_branch_pidx_pgno(branch_id, key)?;
		if pgno > head.db_size_pages {
			continue;
		}
		let bytes = delta
			.get_page(pgno)
			.with_context(|| {
				format!("PIDX row for page {pgno} pointed at delta {txid} without the page")
			})?
			.to_vec();

		pages_by_shard
			.entry(pgno / keys::SHARD_SIZE)
			.or_default()
			.push((pgno, bytes));
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
											ctx.create_ts(),
										);
										*state = CompanionWorkflowState::Idle;
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
											ctx.create_ts(),
										);
										*state = CompanionWorkflowState::Idle;
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

async fn run_hot_compaction_job(
	ctx: &mut WorkflowCtx,
	state: &mut CompanionWorkflowState,
	database_branch_id: DatabaseBranchId,
	signal: RunHotJob,
) -> Result<()> {
	record_companion_job(
		state,
		database_branch_id,
		CompactionJobKind::Hot,
		signal.job_id,
		signal.base_manifest_generation,
		signal.input_fingerprint,
		ctx.create_ts(),
	);

	let output = ctx
		.activity(StageHotJobInput {
			database_branch_id,
			job_id: signal.job_id,
			job_kind: signal.job_kind,
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

fn record_companion_job(
	state: &mut CompanionWorkflowState,
	database_branch_id: DatabaseBranchId,
	job_kind: CompactionJobKind,
	job_id: Id,
	base_manifest_generation: u64,
	input_fingerprint: CompactionInputFingerprint,
	started_at_ms: i64,
) {
	*state = CompanionWorkflowState::Running(CompanionRunningJob {
		database_branch_id,
		job_id,
		job_kind,
		base_manifest_generation,
		input_fingerprint,
		started_at_ms,
		attempt: 0,
	});
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
