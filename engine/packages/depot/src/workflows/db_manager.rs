use crate::compaction::{shared::*, *};

#[cfg(feature = "test-faults")]
use crate::compaction::test_hooks;
#[cfg(feature = "test-faults")]
use crate::fault::ReclaimFaultPoint;

#[workflow(DbManagerWorkflow)]
pub async fn depot_db_manager(ctx: &mut WorkflowCtx, input: &DbManagerInput) -> Result<()> {
	let companion_workflow_ids =
		dispatch_companion_workflows(ctx, input.database_branch_id).await?;
	let initial_state = DbManagerState::new(companion_workflow_ids);

	ctx.lupe()
		.commit_interval(1)
		.with_state(initial_state)
		.run(|ctx, state| {
			let input = input.clone();
			async move { run_manager_iteration(ctx, state, &input).await }.boxed()
		})
		.await
}

#[derive(Copy, Clone, Debug, Default)]
pub(super) struct WakeTriggers {
	pub hot: bool,
	pub reclaim: bool,
}

async fn run_manager_iteration(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	input: &DbManagerInput,
) -> Result<Loop<()>> {
	let signals = listen_for_manager_signals(ctx, input, state).await?;
	let signal_received = !signals.is_empty();

	let effects = manager_effects_for_signals(state, input, signals, ctx.create_ts());
	execute_manager_effects(ctx, state, input, effects).await?;

	if let Some(effect) = manager_effect_for_requested_stop(state, input) {
		execute_manager_effects(ctx, state, input, vec![effect]).await?;
		return Ok(Loop::Break(()));
	}

	let forced_work = state.force_compactions.pending_work();
	let refresh = execute_manager_refresh(ctx, state, input, forced_work).await?;
	let now_ms = refresh.refreshed_at_ms;

	let triggers = WakeTriggers {
		hot: signal_received,
		reclaim: state.next_reclaim_check_at_ms.is_some_and(|d| now_ms >= d) || forced_work.reclaim,
	};

	let effects = manager_effects_after_refresh(state, input, &refresh, now_ms, triggers);
	let should_stop = effects
		.iter()
		.any(|effect| matches!(effect, ManagerEffect::StopCompanions { .. }));
	execute_manager_effects(ctx, state, input, effects).await?;
	if should_stop {
		return Ok(Loop::Break(()));
	}

	schedule_next_wake(state, input, &refresh, now_ms, signal_received, triggers);

	Ok(Loop::Continue)
}

fn schedule_next_wake(
	state: &mut DbManagerState,
	input: &DbManagerInput,
	refresh: &RefreshManagerOutput,
	now_ms: i64,
	signal_received: bool,
	triggers: WakeTriggers,
) {
	use crate::conveyer::constants::MANAGER_RECLAIM_INTERVAL_MS;

	if manager_planning_timers_disabled(input) {
		state.next_reclaim_check_at_ms = None;
		return;
	}

	if triggers.reclaim {
		state.next_reclaim_check_at_ms = None;
	}

	if signal_received && state.next_reclaim_check_at_ms.is_none() {
		state.next_reclaim_check_at_ms = Some(now_ms + MANAGER_RECLAIM_INTERVAL_MS);
	}

	// Retained interval rows expire by wall clock, so an idle database must
	// keep a wake armed or expired rows sit unreclaimed until the next commit.
	if state.next_reclaim_check_at_ms.is_none() && refresh.has_retained_pitr_intervals {
		state.next_reclaim_check_at_ms = Some(now_ms + MANAGER_RECLAIM_INTERVAL_MS);
	}
}

#[derive(Debug)]
pub(super) enum ManagerEffect {
	Refresh {
		force: ForceCompactionWork,
	},
	InstallHotOutput {
		signal: HotJobFinished,
		active_job: ActiveHotCompactionJob,
	},
	FinishHotJob {
		job_id: Id,
		status: CompactionJobStatus,
	},
	FinishReclaimJob {
		job_id: Id,
		status: CompactionJobStatus,
	},
	ScheduleStaleHotOutputCleanup {
		signal: HotJobFinished,
		actor_id: Option<String>,
	},
	RunHotJob {
		active_job: PlannedHotCompactionJob,
	},
	RunReclaimJob {
		active_job: PlannedReclaimCompactionJob,
	},
	StopCompanions {
		request: ManagerStopRequest,
	},
	CompleteReadyForceCompactions {
		refresh: RefreshManagerOutput,
	},
}

pub(super) fn manager_effects_for_signals(
	state: &mut DbManagerState,
	input: &DbManagerInput,
	signals: Vec<DbManagerSignal>,
	now_ms: i64,
) -> Vec<ManagerEffect> {
	let mut effects = Vec::new();
	for signal in signals {
		if signal.database_branch_id() != input.database_branch_id {
			continue;
		}

		match signal {
			DbManagerSignal::DeltasAvailable(signal) => {
				record_deltas_available(state, signal);
			}
			DbManagerSignal::ForceCompaction(signal) => {
				state
					.force_compactions
					.record_request(signal, now_ms, &state.active_jobs);
			}
			DbManagerSignal::HotJobFinished(signal) => {
				effects.extend(manager_effects_for_hot_job_finished(state, input, signal));
			}
			DbManagerSignal::ReclaimJobFinished(signal) => {
				effects.extend(manager_effects_for_reclaim_job_finished(state, signal));
			}
			DbManagerSignal::DestroyDatabaseBranch(signal) => {
				record_destroy_database_branch(state, signal);
			}
		}
	}

	effects
}

fn record_deltas_available(state: &mut DbManagerState, signal: DeltasAvailable) {
	state.last_dirty_cursor = Some(DirtyCursor {
		observed_head_txid: signal.observed_head_txid,
		dirty_updated_at_ms: signal.dirty_updated_at_ms,
	});
}

fn record_destroy_database_branch(state: &mut DbManagerState, signal: DestroyDatabaseBranch) {
	state.branch_stop_state = BranchStopState::StopRequested {
		lifecycle_generation: signal.lifecycle_generation,
		requested_at_ms: signal.requested_at_ms,
		reason: ManagerStopReason::ExplicitDestroy {
			reason: signal.reason,
		},
	};
}

pub(super) fn manager_effects_for_hot_job_finished(
	state: &mut DbManagerState,
	input: &DbManagerInput,
	signal: HotJobFinished,
) -> Vec<ManagerEffect> {
	let active_job = state.active_jobs.hot.clone();
	if let Some(active_job) = active_job.as_ref()
		&& hot_job_finished_matches_active(&signal, active_job)
	{
		return match &signal.status {
			CompactionJobStatus::Requested => Vec::new(),
			CompactionJobStatus::Succeeded => vec![ManagerEffect::InstallHotOutput {
				signal,
				active_job: active_job.clone(),
			}],
			CompactionJobStatus::Rejected { .. } | CompactionJobStatus::Failed { .. } => {
				vec![ManagerEffect::FinishHotJob {
					job_id: signal.job_id,
					status: signal.status.clone(),
				}]
			}
		};
	}

	vec![ManagerEffect::ScheduleStaleHotOutputCleanup {
		signal,
		actor_id: input.actor_id.clone(),
	}]
}

pub(super) fn manager_effects_for_reclaim_job_finished(
	state: &mut DbManagerState,
	signal: ReclaimJobFinished,
) -> Vec<ManagerEffect> {
	if let Some(active_job) = state.active_jobs.reclaim.as_ref()
		&& reclaim_job_finished_matches_active(&signal, active_job)
	{
		return match signal.status {
			CompactionJobStatus::Requested => Vec::new(),
			CompactionJobStatus::Succeeded
			| CompactionJobStatus::Rejected { .. }
			| CompactionJobStatus::Failed { .. } => {
				vec![ManagerEffect::FinishReclaimJob {
					job_id: signal.job_id,
					status: signal.status,
				}]
			}
		};
	}

	Vec::new()
}

async fn execute_manager_refresh(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	input: &DbManagerInput,
	force: ForceCompactionWork,
) -> Result<RefreshManagerOutput> {
	let executions =
		execute_manager_effects(ctx, state, input, vec![ManagerEffect::Refresh { force }]).await?;
	let [ManagerExecution::Refresh(refresh)] = executions.as_slice() else {
		bail!("refresh effect did not return refresh output");
	};

	Ok(refresh.clone())
}

#[derive(Debug)]
enum ManagerExecution {
	Refresh(RefreshManagerOutput),
}

async fn execute_manager_effects(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	input: &DbManagerInput,
	effects: Vec<ManagerEffect>,
) -> Result<Vec<ManagerExecution>> {
	let mut executions = Vec::new();
	for effect in effects {
		match effect {
			ManagerEffect::Refresh { force } => {
				let refresh = execute_refresh_effect(ctx, state, input, force).await?;
				executions.push(ManagerExecution::Refresh(refresh));
			}
			ManagerEffect::InstallHotOutput { signal, active_job } => {
				execute_install_hot_output_effect(ctx, state, input, signal, active_job).await?;
			}
			ManagerEffect::FinishHotJob { job_id, status } => {
				state.force_compactions.record_job_finished(
					CompactionJobKind::Hot,
					job_id,
					&status,
				);
				state.active_jobs.hot = None;
			}
			ManagerEffect::FinishReclaimJob { job_id, status } => {
				state.force_compactions.record_job_finished(
					CompactionJobKind::Reclaim,
					job_id,
					&status,
				);
				state.active_jobs.reclaim = None;
				// Passes are batch budgeted, so draining a backlog needs
				// back-to-back passes rather than one per reclaim interval.
				// Rejections are plan/execute races; retry them after a short
				// backoff. Failures wait for the normal interval.
				use crate::conveyer::constants::MANAGER_RECLAIM_RETRY_MS;
				match &status {
					CompactionJobStatus::Succeeded => {
						state.next_reclaim_check_at_ms = Some(ctx.create_ts());
					}
					CompactionJobStatus::Rejected { .. } => {
						state.next_reclaim_check_at_ms =
							Some(ctx.create_ts() + MANAGER_RECLAIM_RETRY_MS);
					}
					CompactionJobStatus::Requested | CompactionJobStatus::Failed { .. } => {}
				}
				// The reclaimer just freed: run any queued stale-output cleanup.
				if !state.pending_stage_cleanups.is_empty() {
					let pending = state.pending_stage_cleanups.remove(0);
					schedule_repair_reclaim_job(
						ctx,
						state,
						input.database_branch_id,
						pending.base_lifecycle_generation,
						pending.base_manifest_generation,
						pending.input_range,
						job_id,
						"cleanup_stale_hot_output_queued",
						input.actor_id.as_deref(),
					)
					.await?;
				}
			}
			ManagerEffect::ScheduleStaleHotOutputCleanup { signal, actor_id } => {
				schedule_stale_hot_output_cleanup(ctx, state, &signal, actor_id.as_deref()).await?;
			}
			ManagerEffect::RunHotJob { active_job } => {
				execute_run_hot_job_effect(ctx, state, active_job).await?;
			}
			ManagerEffect::RunReclaimJob { active_job } => {
				execute_run_reclaim_job_effect(ctx, state, active_job).await?;
			}
			ManagerEffect::StopCompanions { request } => {
				signal_companions_destroy(ctx, &state.companion_workflow_ids, &request).await?;
				state.active_jobs.clear();
				state.branch_stop_state = BranchStopState::Stopped {
					stopped_at_ms: ctx.create_ts(),
				};
			}
			ManagerEffect::CompleteReadyForceCompactions { refresh } => {
				state.force_compactions.complete_ready_requests(
					&state.active_jobs,
					&refresh,
					ctx.create_ts(),
				);
			}
		}
	}

	Ok(executions)
}

async fn execute_refresh_effect(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	input: &DbManagerInput,
	force: ForceCompactionWork,
) -> Result<RefreshManagerOutput> {
	let refresh = ctx
		.activity(RefreshManagerInput {
			database_branch_id: input.database_branch_id,
			force,
			shard_gc_cursor: state.next_shard_gc_cursor.clone(),
		})
		.await?;

	state.last_observed_branch_lifecycle_generation = refresh.branch_lifecycle_generation;
	// Rotate the shard GC window only when a pass is actually planned, so an
	// unplanned window is rescanned instead of skipped.
	if refresh.planned_reclaim_job.is_some() {
		state.next_shard_gc_cursor = refresh.shard_gc_next_cursor.clone();
	}
	if state.last_dirty_cursor.is_none()
		&& let Some(dirty) = refresh.observed_dirty.as_ref()
	{
		state.last_dirty_cursor = Some(DirtyCursor {
			observed_head_txid: dirty.observed_head_txid,
			dirty_updated_at_ms: dirty.updated_at_ms,
		});
	}

	Ok(refresh)
}

async fn execute_install_hot_output_effect(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	input: &DbManagerInput,
	signal: HotJobFinished,
	active_job: ActiveHotCompactionJob,
) -> Result<()> {
	let install = ctx
		.activity(InstallHotJobInput {
			database_branch_id: signal.database_branch_id,
			job_id: signal.job_id,
			job_kind: signal.job_kind,
			base_lifecycle_generation: active_job.base_lifecycle_generation,
			base_manifest_generation: signal.base_manifest_generation,
			input_fingerprint: signal.input_fingerprint,
			input_range: active_job.input_range,
			output_refs: signal.output_refs.clone(),
		})
		.await?;
	match install.status {
		CompactionJobStatus::Requested => {}
		CompactionJobStatus::Succeeded => {
			state.force_compactions.record_job_finished(
				CompactionJobKind::Hot,
				signal.job_id,
				&install.status,
			);
			state.active_jobs.hot = None;
		}
		CompactionJobStatus::Rejected { .. } | CompactionJobStatus::Failed { .. } => {
			state.force_compactions.record_job_finished(
				CompactionJobKind::Hot,
				signal.job_id,
				&install.status,
			);
			state.active_jobs.hot = None;
			// The stage succeeded but the install did not publish, so the
			// staged shard rows are orphaned; schedule the repair cleanup.
			schedule_stale_hot_output_cleanup(ctx, state, &signal, input.actor_id.as_deref())
				.await?;
		}
	}

	Ok(())
}

async fn execute_run_hot_job_effect(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	active_job: PlannedHotCompactionJob,
) -> Result<()> {
	ctx.signal(RunHotJob {
		database_branch_id: active_job.database_branch_id,
		job_id: active_job.job_id,
		job_kind: CompactionJobKind::Hot,
		base_lifecycle_generation: active_job.base_lifecycle_generation,
		base_manifest_generation: active_job.base_manifest_generation,
		input_fingerprint: active_job.input_fingerprint,
		status: CompactionJobStatus::Requested,
		input_range: active_job.input_range.clone(),
	})
	.to_workflow_id(state.companion_workflow_ids.hot_compacter_workflow_id)
	.send()
	.await?;

	state
		.force_compactions
		.record_job_attempted(CompactionJobKind::Hot);
	state.active_jobs.hot = Some(ActiveHotCompactionJob::from_planned(active_job));

	Ok(())
}

async fn execute_run_reclaim_job_effect(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	active_job: PlannedReclaimCompactionJob,
) -> Result<()> {
	ctx.signal(RunReclaimJob {
		database_branch_id: active_job.database_branch_id,
		job_id: active_job.job_id,
		job_kind: CompactionJobKind::Reclaim,
		base_lifecycle_generation: active_job.base_lifecycle_generation,
		base_manifest_generation: active_job.base_manifest_generation,
		input_fingerprint: active_job.input_fingerprint,
		status: CompactionJobStatus::Requested,
		input_range: active_job.input_range.clone(),
	})
	.to_workflow_id(state.companion_workflow_ids.reclaimer_workflow_id)
	.send()
	.await?;

	state
		.force_compactions
		.record_job_attempted(CompactionJobKind::Reclaim);
	state.active_jobs.reclaim = Some(ActiveReclaimCompactionJob::from_planned(active_job));

	Ok(())
}

pub(super) fn manager_effect_for_requested_stop(
	state: &DbManagerState,
	input: &DbManagerInput,
) -> Option<ManagerEffect> {
	if let BranchStopState::StopRequested {
		lifecycle_generation,
		requested_at_ms,
		reason,
	} = state.branch_stop_state.clone()
	{
		return Some(stop_companions_effect(ManagerStopRequest {
			database_branch_id: input.database_branch_id,
			lifecycle_generation,
			requested_at_ms,
			reason,
		}));
	}

	None
}

pub(super) fn manager_effects_after_refresh(
	state: &DbManagerState,
	input: &DbManagerInput,
	refresh: &RefreshManagerOutput,
	now_ms: i64,
	triggers: WakeTriggers,
) -> Vec<ManagerEffect> {
	if !refresh.branch_is_live && matches!(state.branch_stop_state, BranchStopState::Running) {
		return vec![stop_companions_effect(ManagerStopRequest {
			database_branch_id: input.database_branch_id,
			lifecycle_generation: refresh.branch_lifecycle_generation.unwrap_or_default(),
			requested_at_ms: now_ms,
			reason: ManagerStopReason::BranchNotLive,
		})];
	}

	let mut effects = Vec::new();
	if matches!(state.branch_stop_state, BranchStopState::Running) {
		if triggers.hot
			&& state.active_jobs.hot.is_none()
			&& let Some(active_job) = refresh.planned_hot_job.clone()
		{
			effects.push(ManagerEffect::RunHotJob { active_job });
		}
		if triggers.reclaim
			&& state.active_jobs.reclaim.is_none()
			&& let Some(active_job) = refresh.planned_reclaim_job.clone()
		{
			effects.push(ManagerEffect::RunReclaimJob { active_job });
		}
	}
	effects.push(ManagerEffect::CompleteReadyForceCompactions {
		refresh: refresh.clone(),
	});
	effects
}

fn stop_companions_effect(request: ManagerStopRequest) -> ManagerEffect {
	ManagerEffect::StopCompanions { request }
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
	let reclaimer_workflow_id = ctx
		.workflow(DbReclaimerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;

	Ok(CompanionWorkflowIds::new(
		hot_compacter_workflow_id,
		reclaimer_workflow_id,
	))
}

async fn signal_companions_destroy(
	ctx: &mut WorkflowCtx,
	companion_workflow_ids: &CompanionWorkflowIds,
	request: &ManagerStopRequest,
) -> Result<()> {
	let destroy = DestroyDatabaseBranch {
		database_branch_id: request.database_branch_id,
		lifecycle_generation: request.lifecycle_generation,
		requested_at_ms: request.requested_at_ms,
		reason: request.reason.companion_reason(),
	};

	ctx.signal(destroy.clone())
		.to_workflow_id(companion_workflow_ids.hot_compacter_workflow_id)
		.send()
		.await?;

	ctx.signal(destroy)
		.to_workflow_id(companion_workflow_ids.reclaimer_workflow_id)
		.send()
		.await?;

	Ok(())
}

async fn listen_for_manager_signals(
	ctx: &mut WorkflowCtx,
	input: &DbManagerInput,
	state: &DbManagerState,
) -> Result<Vec<DbManagerSignal>> {
	if manager_planning_timers_disabled(input) {
		return ctx.listen_n::<DbManagerSignal>(256).await;
	}

	let deadline = state.next_reclaim_check_at_ms;

	if let Some(deadline) = deadline {
		ctx.listen_n_until::<DbManagerSignal>(deadline, 256).await
	} else {
		ctx.listen_n::<DbManagerSignal>(256).await
	}
}

#[cfg(feature = "test-faults")]
fn manager_planning_timers_disabled(input: &DbManagerInput) -> bool {
	input.disable_planning_timers
}

#[cfg(not(feature = "test-faults"))]
fn manager_planning_timers_disabled(_input: &DbManagerInput) -> bool {
	false
}

#[activity(RefreshManager)]
pub async fn refresh_manager(
	ctx: &ActivityCtx,
	input: &RefreshManagerInput,
) -> Result<RefreshManagerOutput> {
	let now_ms = ctx.ts();
	let database_branch_id = input.database_branch_id;
	#[cfg(feature = "test-faults")]
	test_hooks::maybe_fire_reclaim_fault(database_branch_id, ReclaimFaultPoint::PlanBeforeSnapshot)
		.await?;
	let shard_gc_cursor = input.shard_gc_cursor.clone();
	let snapshot = ctx
		.udb()?
		.txn("depot_manager_refresh", move |tx| {
			let shard_gc_cursor = shard_gc_cursor.clone();
			async move {
				read_manager_fdb_snapshot(&tx, database_branch_id, &shard_gc_cursor, now_ms).await
			}
		})
		.await?;
	#[cfg(feature = "test-faults")]
	test_hooks::maybe_fire_reclaim_fault(database_branch_id, ReclaimFaultPoint::PlanAfterSnapshot)
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
			input.force.hot,
		)
	} else {
		None
	};
	let planned_reclaim_job = if branch_is_live {
		plan_reclaim_job(
			database_branch_id,
			&snapshot,
			Id::new_v1(ctx.config().dc_label()),
			&input.shard_gc_cursor,
			now_ms,
		)
	} else {
		None
	};
	let reclaim_noop_reason = if branch_is_live {
		Some(reclaim_noop_reason(&snapshot).to_string())
	} else {
		None
	};

	Ok(RefreshManagerOutput {
		refreshed_at_ms: now_ms,
		planned_hot_job,
		planned_reclaim_job,
		has_retained_pitr_intervals: snapshot.reclaim_inputs.has_retained_pitr_intervals,
		shard_gc_next_cursor: snapshot.reclaim_inputs.shard_gc_next_cursor.clone(),
		observed_dirty: if snapshot.cleared_dirty {
			None
		} else {
			snapshot.dirty
		},
		head_txid,
		branch_is_live,
		branch_lifecycle_generation,
		db_pin_count: snapshot.db_pins.len(),
		reclaim_noop_reason,
	})
}

fn hot_job_finished_matches_active(
	signal: &HotJobFinished,
	active_job: &ActiveHotCompactionJob,
) -> bool {
	signal.job_id == active_job.job_id
		&& signal.job_kind == CompactionJobKind::Hot
		&& signal.base_manifest_generation == active_job.base_manifest_generation
		&& signal.input_fingerprint == active_job.input_fingerprint
}

fn reclaim_job_finished_matches_active(
	signal: &ReclaimJobFinished,
	active_job: &ActiveReclaimCompactionJob,
) -> bool {
	signal.job_id == active_job.job_id
		&& signal.job_kind == CompactionJobKind::Reclaim
		&& signal.base_manifest_generation == active_job.base_manifest_generation
		&& signal.input_fingerprint == active_job.input_fingerprint
}

fn log_actor_id(actor_id: Option<&str>) -> &str {
	actor_id.unwrap_or("unknown")
}

async fn schedule_stale_hot_output_cleanup(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	signal: &HotJobFinished,
	actor_id: Option<&str>,
) -> Result<()> {
	// Cleanup applies whenever staged output exists, regardless of how the job
	// or its install ended.
	if signal.output_refs.is_empty() {
		return Ok(());
	}
	let Some(base_lifecycle_generation) = state.last_observed_branch_lifecycle_generation else {
		tracing::warn!(
			actor_id = log_actor_id(actor_id),
			?signal.database_branch_id,
			manifest_generation = signal.base_manifest_generation,
			?signal.job_id,
			repair_action = "defer_stale_hot_output_cleanup",
			"stale hot output cleanup deferred until branch lifecycle is observed"
		);
		return Ok(());
	};

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
		signal
			.output_refs
			.iter()
			.map(|output_ref| output_ref.as_of_txid),
	);

	schedule_repair_reclaim_job(
		ctx,
		state,
		signal.database_branch_id,
		base_lifecycle_generation,
		signal.base_manifest_generation,
		input_range,
		signal.job_id,
		"cleanup_stale_hot_output",
		actor_id,
	)
	.await
}

pub(super) fn repair_reclaim_input_range(
	staged_hot_shards: Vec<StagedHotShardCleanupRef>,
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
		staged_hot_shards,
		max_keys: CMP_FDB_BATCH_MAX_KEYS as u32,
		max_bytes: CMP_FDB_BATCH_MAX_VALUE_BYTES as u64,
		shard_gc_cursor: Vec::new(),
	}
}

async fn schedule_repair_reclaim_job(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	database_branch_id: DatabaseBranchId,
	base_lifecycle_generation: u64,
	base_manifest_generation: u64,
	input_range: ReclaimJobInputRange,
	source_job_id: Id,
	repair_action: &'static str,
	actor_id: Option<&str>,
) -> Result<()> {
	if state.active_jobs.reclaim.is_some() {
		// Queue the cleanup so it actually runs when the reclaimer frees;
		// dropping it here would leak the staged blobs permanently.
		tracing::warn!(
			actor_id = log_actor_id(actor_id),
			?database_branch_id,
			manifest_generation = base_manifest_generation,
			?source_job_id,
			repair_action,
			"stale compaction output cleanup queued because reclaimer is busy"
		);
		state.pending_stage_cleanups.push(PendingStageCleanup {
			base_lifecycle_generation,
			base_manifest_generation,
			input_range,
		});
		return Ok(());
	}

	let cleanup_job_id = Id::new_v1(ctx.config().dc_label());
	let input_fingerprint = fingerprint_repair_reclaim_range(database_branch_id, &input_range);
	tracing::warn!(
		actor_id = log_actor_id(actor_id),
		?database_branch_id,
		manifest_generation = base_manifest_generation,
		?source_job_id,
		?cleanup_job_id,
		repair_action,
		staged_hot_shard_count = input_range.staged_hot_shards.len(),
		"scheduled stale compaction output cleanup"
	);

	ctx.signal(RunReclaimJob {
		database_branch_id,
		job_id: cleanup_job_id,
		job_kind: CompactionJobKind::Reclaim,
		base_lifecycle_generation,
		base_manifest_generation,
		input_fingerprint,
		status: CompactionJobStatus::Requested,
		input_range: input_range.clone(),
	})
	.to_workflow_id(state.companion_workflow_ids.reclaimer_workflow_id)
	.send()
	.await?;

	state.active_jobs.reclaim = Some(ActiveReclaimCompactionJob {
		database_branch_id,
		job_id: cleanup_job_id,
		base_lifecycle_generation,
		base_manifest_generation,
		input_fingerprint,
		input_range,
		planned_at_ms: ctx.create_ts(),
		attempt: 0,
	});

	Ok(())
}

pub(crate) fn branch_record_is_live_at_generation(
	branch_record: Option<&DatabaseBranchRecord>,
	lifecycle_generation: u64,
) -> bool {
	branch_record.is_some_and(|record| {
		record.state == BranchState::Live && record.lifecycle_generation == lifecycle_generation
	})
}
