use anyhow::{Context, Result, bail};
use futures_util::{FutureExt, TryStreamExt};
use gas::prelude::*;
use serde::{Deserialize, Serialize};
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::IsolationLevel::{Serializable, Snapshot},
};
use vbare::OwnedVersionedData;

use crate::{
	CMP_FDB_BATCH_MAX_KEYS, CMP_FDB_BATCH_MAX_VALUE_BYTES,
	HOT_BURST_COLD_LAG_THRESHOLD_TXIDS,
	conveyer::{
		keys, quota,
		types::{
			BranchState, ColdShardRef, CommitRow, CompactionRoot, DBHead, DatabaseBranchId,
			DatabaseBranchRecord, DbHistoryPin, SqliteCmpDirty, decode_commit_row,
			decode_compaction_root, decode_database_branch_record, decode_db_head,
			decode_db_history_pin, decode_sqlite_cmp_dirty,
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
	let hot_inputs = read_hot_input_snapshot(tx, branch_id, head.as_ref(), &root).await?;
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

	for (key, value) in tx_scan_prefix_values(tx, &keys::branch_commit_prefix(branch_id), Snapshot)
		.await?
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

	for (key, value) in tx_scan_prefix_values(tx, &keys::branch_delta_prefix(branch_id), Snapshot)
		.await?
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

	for (key, value) in tx_scan_prefix_values(tx, &keys::branch_pidx_prefix(branch_id), Snapshot)
		.await?
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
