use rivet_config::config::DEPOT_COMPACTION_THROTTLE;
use serde::Deserializer;
use universaldb::prelude::{Priority, ThrottleCharge};

use crate::compaction::{shared::*, *};
use crate::metrics;

#[cfg(feature = "test-faults")]
use crate::compaction::test_hooks;
#[cfg(feature = "test-faults")]
use crate::fault::ReclaimFaultPoint;

#[workflow(DbManagerWorkflow)]
pub async fn depot_db_manager3(ctx: &mut WorkflowCtx, input: &DbManagerInput) -> Result<()> {
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

	schedule_next_wake(
		state,
		input,
		now_ms,
		signal_received,
		triggers,
		ManagerWakeIntervals::from_config(ctx.config()),
	);

	Ok(Loop::Continue)
}

/// Wake deadlines the manager arms itself on.
///
/// Read from config once per iteration and passed in so `schedule_next_wake` stays a pure function
/// of its arguments, which is what lets the tests drive it directly.
#[derive(Debug, Clone, Copy)]
pub(super) struct ManagerWakeIntervals {
	/// See `sqlite.manager_idle_poll_interval_ms`.
	pub idle_poll_ms: i64,
	/// See `sqlite.manager_reclaim_interval_ms`.
	pub reclaim_ms: i64,
}

impl ManagerWakeIntervals {
	pub(super) fn from_config(config: &rivet_config::Config) -> Self {
		let sqlite = config.sqlite();

		Self {
			idle_poll_ms: sqlite.manager_idle_poll_interval_ms(),
			reclaim_ms: sqlite.manager_reclaim_interval_ms(),
		}
	}
}

/// Arms the manager's next wake deadline.
///
/// This must always leave a deadline armed (unless planning timers are disabled for tests): reads
/// never signal the manager, so a branch that stops being written would otherwise park on a
/// deadline-less listen and strand every reclaim lane. An iteration that fires on a timer and
/// receives no signal falls back to the idle poll.
pub(super) fn schedule_next_wake(
	state: &mut DbManagerState,
	input: &DbManagerInput,
	now_ms: i64,
	signal_received: bool,
	triggers: WakeTriggers,
	intervals: ManagerWakeIntervals,
) {
	if manager_planning_timers_disabled(input) {
		state.next_reclaim_check_at_ms = None;
		return;
	}

	if triggers.reclaim {
		state.next_reclaim_check_at_ms = None;
	}

	if signal_received {
		if state.next_reclaim_check_at_ms.is_none() {
			state.next_reclaim_check_at_ms = Some(now_ms + intervals.reclaim_ms);
		}
	} else if !state.pending_cleanups.is_empty() && state.next_reclaim_check_at_ms.is_none() {
		// Cleanup is queued but could not dispatch this iteration, so come back for it on the idle
		// poll rather than parking until the next commit. An unarmed manager here would hold the
		// staging area resident for as long as the branch stays quiet.
		state.next_reclaim_check_at_ms =
			Some(now_ms + idle_poll_delay_ms(input.database_branch_id, intervals.idle_poll_ms));
	} else if state.next_reclaim_check_at_ms.is_none() {
		// Nothing to do and nothing armed: poll on the idle interval so an idle branch still
		// reclaims. One reclaim job drains the whole backlog, so a single wake is enough.
		state.next_reclaim_check_at_ms =
			Some(now_ms + idle_poll_delay_ms(input.database_branch_id, intervals.idle_poll_ms));
	}

	// A held-off input is only re-planned on a wake, and an idle branch's next wake is the idle poll
	// twelve hours out. Pull the deadline in to when the delay expires so the retry happens on the
	// cadence the backoff chose rather than whenever the branch is next written.
	if let Some(backoff) = state.reclaim_backoff.as_ref() {
		state.next_reclaim_check_at_ms = Some(
			state
				.next_reclaim_check_at_ms
				.map_or(backoff.retry_at_ms, |armed| armed.min(backoff.retry_at_ms)),
		);
	}
}

/// Idle poll delay for a branch, jittered by +/-10% of the interval from a stable hash of the branch
/// id.
///
/// Branches that go idle at different times self-spread on their own, but an engine restart or a
/// compaction backfill pass re-aligns every manager into the same window. Must stay a pure function
/// of the branch id because it is computed in a workflow body that gets replayed.
fn idle_poll_delay_ms(database_branch_id: DatabaseBranchId, idle_poll_interval_ms: i64) -> i64 {
	let jitter_span_ms = idle_poll_interval_ms / 5;
	if jitter_span_ms <= 0 {
		return idle_poll_interval_ms;
	}

	let jitter_ms = (database_branch_id.as_uuid().as_u128() % jitter_span_ms as u128) as i64;

	idle_poll_interval_ms - jitter_span_ms / 2 + jitter_ms
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
		bypass_admission: bool,
	},
	RunReclaimJob {
		active_job: PlannedReclaimCompactionJob,
		bypass_admission: bool,
	},
	DispatchPendingCleanups,
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
				effects.extend(manager_effects_for_reclaim_job_finished(
					state, signal, now_ms,
				));
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
				// The drain may have staged some slices before failing; clean those staged shards up
				// then finish so the next refresh re-drains from the unchanged watermark.
				vec![
					ManagerEffect::ScheduleStaleHotOutputCleanup {
						signal: signal.clone(),
						actor_id: input.actor_id.clone(),
					},
					ManagerEffect::FinishHotJob {
						job_id: signal.job_id,
						status: signal.status.clone(),
					},
				]
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
	now_ms: i64,
) -> Vec<ManagerEffect> {
	if let Some(active_job) = state.active_jobs.reclaim.as_ref()
		&& reclaim_job_finished_matches_active(&signal, active_job)
	{
		record_reclaim_backoff(state, &signal, now_ms);

		return match signal.status {
			CompactionJobStatus::Requested => Vec::new(),
			CompactionJobStatus::Succeeded
			| CompactionJobStatus::Rejected { .. }
			| CompactionJobStatus::Failed { .. } => {
				// A repair job's ids are not re-queued here. Once the job finishes they stop being
				// excluded from the refresh's `CMP/stage/` scan, so the next refresh reports whatever
				// staging the job failed to clear. Re-queuing instead would retry a deterministic
				// rejection on every iteration with nothing rate-limiting it.
				vec![ManagerEffect::FinishReclaimJob {
					job_id: signal.job_id,
					status: signal.status,
				}]
			}
		};
	}

	Vec::new()
}

/// Records how long the reclaim lane holds off the input this job just finished.
///
/// A success clears the delay outright: the input it ran against is gone, so nothing is left to hold
/// off. Anything else arms or extends the delay, because reclaim re-plans from current FDB state and
/// an unchanged state re-plans the same input.
fn record_reclaim_backoff(state: &mut DbManagerState, signal: &ReclaimJobFinished, now_ms: i64) {
	match &signal.status {
		CompactionJobStatus::Requested => {}
		CompactionJobStatus::Succeeded => {
			state.reclaim_backoff = None;
		}
		CompactionJobStatus::Rejected { .. } | CompactionJobStatus::Failed { .. } => {
			let backoff = ReclaimBackoff::next(
				state.reclaim_backoff.as_ref(),
				signal.input_fingerprint,
				now_ms,
			);
			let status = match &signal.status {
				CompactionJobStatus::Rejected { reason } => reason.as_str(),
				CompactionJobStatus::Failed { error } => error.as_str(),
				CompactionJobStatus::Requested | CompactionJobStatus::Succeeded => "",
			};
			tracing::warn!(
				database_branch_id = ?signal.database_branch_id,
				job_id = %signal.job_id,
				reason = status,
				consecutive_outcomes = backoff.consecutive_outcomes,
				retry_in_ms = backoff.retry_at_ms.saturating_sub(now_ms),
				"reclaim job did not succeed; holding off its input"
			);
			state.reclaim_backoff = Some(backoff);
		}
	}
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
			}
			ManagerEffect::ScheduleStaleHotOutputCleanup { signal, actor_id } => {
				schedule_stale_hot_output_cleanup(ctx, state, &signal, actor_id.as_deref()).await?;
			}
			ManagerEffect::RunHotJob {
				active_job,
				bypass_admission,
			} => {
				execute_run_hot_job_effect(ctx, state, active_job, bypass_admission).await?;
			}
			ManagerEffect::RunReclaimJob {
				active_job,
				bypass_admission,
			} => {
				state.last_reclaim_slot_was_cleanup = false;
				execute_run_reclaim_job_effect(ctx, state, active_job, bypass_admission).await?;
			}
			ManagerEffect::DispatchPendingCleanups => {
				dispatch_pending_cleanups(ctx, state, input).await?;
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
			active_job_ids: staging_jobs_accounted_for(state),
		})
		.await?;

	state.last_observed_branch_lifecycle_generation = refresh.branch_lifecycle_generation;
	record_orphan_stage_jobs(state, input, &refresh);
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

/// Durable resume state for the hot install loop. The state is `None` until a call actually stops
/// early, which is the only case that serializes it at all.
///
/// This state was once a bare cursor integer rather than a struct, so deserialization accepts that
/// scalar form too. A workflow that stopped early before the struct shipped has an integer persisted
/// as its loop state, and rejecting it wedges that branch's manager forever. A legacy state resumes
/// at the same cursor with no shard cursor, which is exactly where the scalar left off, and at a zero
/// `installed_shard_count`, which only under-reports one sample of the shards-installed histogram
/// because nothing else reads that count. The same applies to `installed_shard_bytes`.
#[derive(Debug, Clone, Copy, Serialize)]
pub(super) struct HotInstallResume {
	pub(super) cursor: u64,
	/// Resume position inside `cursor`'s own commit, when a chunk installed only part of its pages.
	#[serde(default)]
	pub(super) cursor_segment_pgno: Option<u32>,
	pub(super) shard_cursor: Option<HotInstallShardCursor>,
	pub(super) installed_shard_count: u64,
	pub(super) installed_shard_bytes: u64,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum HotInstallResumeRepr {
	LegacyCursor(u64),
	Current {
		cursor: u64,
		#[serde(default)]
		cursor_segment_pgno: Option<u32>,
		#[serde(default)]
		shard_cursor: Option<HotInstallShardCursor>,
		#[serde(default)]
		installed_shard_count: u64,
		#[serde(default)]
		installed_shard_bytes: u64,
	},
}

impl<'de> Deserialize<'de> for HotInstallResume {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		Ok(match HotInstallResumeRepr::deserialize(deserializer)? {
			HotInstallResumeRepr::LegacyCursor(cursor) => HotInstallResume {
				cursor,
				cursor_segment_pgno: None,
				shard_cursor: None,
				installed_shard_count: 0,
				installed_shard_bytes: 0,
			},
			HotInstallResumeRepr::Current {
				cursor,
				cursor_segment_pgno,
				shard_cursor,
				installed_shard_count,
				installed_shard_bytes,
			} => HotInstallResume {
				cursor,
				cursor_segment_pgno,
				shard_cursor,
				installed_shard_count,
				installed_shard_bytes,
			},
		})
	}
}

async fn execute_install_hot_output_effect(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	input: &DbManagerInput,
	signal: HotJobFinished,
	active_job: ActiveHotCompactionJob,
) -> Result<()> {
	// The companion drained one logical compaction up to `H0` and merged every chunk's staged refs.
	// Install the whole set by chunking the FDB writes across activity calls: each call installs
	// chunks until `CMP_BULK_ACTIVITY_EARLY_TIMEOUT` and returns a resume cursor, and the final call
	// advances the watermark to `H0` plus bumps the manifest generation exactly once.
	// Use a durable loop so completed resume-cursor iterations move to forgotten history instead of
	// growing live workflow history unboundedly.
	let database_branch_id = signal.database_branch_id;
	let job_id = signal.job_id;
	let job_kind = signal.job_kind;
	let base_lifecycle_generation = active_job.base_lifecycle_generation;
	let base_manifest_generation = signal.base_manifest_generation;
	let min_txid = active_job.input_range.txids.min_txid;
	let max_txid = active_job.drain_head_txid;
	let drain_head_txid = active_job.drain_head_txid;
	let drain_now_ms = active_job.drain_now_ms;
	let install = ctx
		.lupe()
		.commit_interval(1)
		.with_state(None::<HotInstallResume>)
		.run(|ctx, resume| {
			async move {
				let resume_position = *resume;
				let install = ctx
					.activity(InstallHotJobInput {
						database_branch_id,
						job_id,
						job_kind,
						base_lifecycle_generation,
						base_manifest_generation,
						input_range: HotJobInputRange {
							txids: TxidRange { min_txid, max_txid },
							// Install re-derives every slice from FDB, so the range it is handed
							// carries only the txid bounds; the page bound comes from each
							// re-derived slice.
							max_pgno_exclusive: None,
							coverage_txids: Vec::new(),
							max_pages: 0,
							max_bytes: 0,
						},
						drain_head_txid,
						drain_now_ms,
						resume_cursor: resume_position.map(|position| position.cursor),
						resume_cursor_segment_pgno: resume_position
							.and_then(|position| position.cursor_segment_pgno),
						resume_shard_cursor: resume_position
							.and_then(|position| position.shard_cursor),
						installed_shard_count_before: resume_position
							.map(|position| position.installed_shard_count)
							.unwrap_or_default(),
						installed_shard_bytes_before: resume_position
							.map(|position| position.installed_shard_bytes)
							.unwrap_or_default(),
					})
					.await?;
				let Some(cursor) = install.resume_cursor else {
					return Ok(Loop::Break(install));
				};
				// The install stopped early because the cluster-wide compaction write budget was spent
				// this window. Back off before re-dispatching from the cursor so we do not spin against
				// the budget.
				if install.throttled {
					ctx.sleep(crate::THROTTLE_BACKOFF_MS).await?;
				}
				*resume = Some(HotInstallResume {
					cursor,
					cursor_segment_pgno: install.resume_cursor_segment_pgno,
					shard_cursor: install.resume_shard_cursor,
					installed_shard_count: install.installed_shard_count,
					installed_shard_bytes: install.installed_shard_bytes,
				});
				Ok(Loop::Continue)
			}
			.boxed()
		})
		.await?;
	let final_status = install.status;

	match &final_status {
		// The install asked to retry; keep the active job so the next refresh retries it.
		CompactionJobStatus::Requested => {}
		CompactionJobStatus::Succeeded => {
			// The install copied every staged shard into the live SHARD tier and advanced the
			// watermark, so this job's staging area is now redundant scratch. Reads never touch
			// staging and it carries no PITR dependency, so schedule its cleanup immediately rather
			// than leaking it. The reclaimer clears the staging under this job id in bounded chunks.
			schedule_stale_hot_output_cleanup(ctx, state, &signal, input.actor_id.as_deref())
				.await?;
			state.force_compactions.record_job_finished(
				CompactionJobKind::Hot,
				signal.job_id,
				&final_status,
			);
			state.active_jobs.hot = None;
		}
		CompactionJobStatus::Rejected { .. } | CompactionJobStatus::Failed { .. } => {
			// The install may have copied some chunks without finalizing. Clean up the staged shards and
			// finish so the next refresh re-drains from the unchanged watermark.
			schedule_stale_hot_output_cleanup(ctx, state, &signal, input.actor_id.as_deref())
				.await?;
			state.force_compactions.record_job_finished(
				CompactionJobKind::Hot,
				signal.job_id,
				&final_status,
			);
			state.active_jobs.hot = None;
		}
	}

	Ok(())
}

async fn execute_run_hot_job_effect(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	active_job: PlannedHotCompactionJob,
	bypass_admission: bool,
) -> Result<()> {
	ctx.signal(RunHotJob {
		database_branch_id: active_job.database_branch_id,
		job_id: active_job.job_id,
		job_kind: CompactionJobKind::Hot,
		base_lifecycle_generation: active_job.base_lifecycle_generation,
		base_manifest_generation: active_job.base_manifest_generation,
		drain_head_txid: active_job.drain_head_txid,
		drain_now_ms: active_job.drain_now_ms,
		bypass_admission,
	})
	.to_workflow_id(state.companion_workflow_ids.hot_compactor_workflow_id)
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
	bypass_admission: bool,
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
		bypass_admission,
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

	// Percentage-based admission: a branch outside the admitted fraction does not start any compaction
	// job, so a rollout can bound how many databases compact at once. An explicit force-compaction
	// request bypasses the gate per lane, since the operator asked for that specific work regardless of
	// the rollout percent.
	let forced = state.force_compactions.pending_work();
	let mut effects = Vec::new();
	if matches!(state.branch_stop_state, BranchStopState::Running) {
		if triggers.hot
			&& (refresh.compaction_admitted || forced.hot)
			&& state.active_jobs.hot.is_none()
			&& let Some(active_job) = refresh.planned_hot_job.clone()
		{
			effects.push(ManagerEffect::RunHotJob {
				active_job,
				bypass_admission: forced.hot,
			});
		}
		// Staging cleanup shares the reclaim slot and is gated on the admission percent like every
		// other lane, so setting the percent to zero stops all compaction work on the branch rather
		// than all of it except this.
		//
		// Cleanup is pure-delete work, but it is not free: it reads every staged chunk row back to
		// validate it against its ref before clearing it, so it spends read and write budget in
		// proportion to the staged volume. On a de-admitted branch it was the only lane still
		// running, which left it the whole compaction budget to itself.
		//
		// Nothing is stranded by holding it. The queue is not drained here, so it stays intact for
		// the next admitted wake, and `schedule_next_wake` arms the idle poll whenever cleanups are
		// queued and undispatched. Even a queue lost to a restart is rebuilt, because the refresh
		// activity's staging orphan scan re-derives the job ids from FDB and `record_orphan_stage_jobs`
		// re-queues them.
		let cleanup_admitted = refresh.compaction_admitted || forced.reclaim;
		if state.active_jobs.reclaim.is_none() {
			// Dispatch on any wake that finds work, not only on a reclaim-timer wake. Hot dispatch
			// is driven by `triggers.hot`, which is just `signal_received`, so hot runs on every
			// commit and every job completion while reclaim used to run only when its own timer
			// happened to elapse. That asymmetry let the fold outrun reclaim by the ratio of signal
			// rate to timer rate, and the gap between what hot had folded and what reclaim had
			// retired is exactly the branch's peak footprint. Worse, a refresh that discovered
			// reclaim work on a non-timer wake threw the planned job away: it was neither
			// dispatched nor queued (unlike `pending_cleanups`, which are explicitly never dropped)
			// and it did not shorten the next wake, so the branch fell to the idle poll holding
			// everything it had just planned to free.
			//
			// The reclaim interval keeps its real job, which is to wake an otherwise idle branch so
			// it reclaims at all. It is a fallback cadence, not an admission gate.
			let planned_reclaim = (refresh.compaction_admitted || forced.reclaim)
				.then(|| refresh.planned_reclaim_job.clone())
				.flatten()
				// An input that just came back rejected or failed re-plans identically for as long as
				// the state behind it holds, and a finished job wakes the manager, so dispatching it
				// again here closes a loop that runs at the job's round-trip latency. The fingerprint
				// is what the delay is keyed on, so reclaimable work that appears meanwhile still
				// dispatches on the next wake.
				.filter(|active_job| {
					state
						.reclaim_backoff
						.as_ref()
						.is_none_or(|backoff| !backoff.bars(active_job.input_fingerprint, now_ms))
				});
			// Cleanup does not wait for the reclaim timer, because every cycle it waits is another
			// compaction generation of staging left resident. It does yield the slot every other
			// cycle when reclaim work is also ready: a cleanup that keeps failing is re-reported by
			// the staging scan indefinitely, and unconditional priority would let that one job stop
			// commit, delta, and cold-object reclaim on the branch for good.
			let cleanup_yields = planned_reclaim.is_some() && state.last_reclaim_slot_was_cleanup;
			if cleanup_admitted && !state.pending_cleanups.is_empty() && !cleanup_yields {
				effects.push(ManagerEffect::DispatchPendingCleanups);
			} else if let Some(active_job) = planned_reclaim {
				effects.push(ManagerEffect::RunReclaimJob {
					active_job,
					bypass_admission: forced.reclaim,
				});
			}
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

	let hot_compactor_workflow_id = ctx
		.workflow(DbHotCompactorInput { database_branch_id })
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
		hot_compactor_workflow_id,
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
		.to_workflow_id(companion_workflow_ids.hot_compactor_workflow_id)
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
	// Read the admission percent and the hot drain span through the dynamic config so an operator
	// can change the rollout without restarting the engine. One snapshot decides every lane, so a
	// change landing mid-refresh cannot plan a hot job against a span the admission decision never
	// saw.
	let dynamic_config = ctx.config().dynamic();
	let compaction_admitted = compaction_admitted(
		dynamic_config.sqlite().compaction_admission_fraction(),
		database_branch_id,
	);
	#[cfg(feature = "test-faults")]
	test_hooks::maybe_fire_reclaim_fault(database_branch_id, ReclaimFaultPoint::PlanBeforeSnapshot)
		.await?;
	let default_pitr_policy = PitrPolicy::from_config(ctx.config().sqlite());
	let active_job_ids = input.active_job_ids.clone();
	let (snapshot, orphan_stage_jobs, orphan_commit_stage_txids) = ctx
		.udb()?
		.txn("depot_manager_refresh", move |tx| {
			let active_job_ids = active_job_ids.clone();
			async move {
				// The refresh charges the read axis but never checks it. Its reads are real FDB load
				// and scale with fleet write traffic (an iteration runs per manager wake), so leaving
				// them unmetered lets the gated paths absorb pressure this transaction created.
				// Gating it instead would only blind the manager: the snapshot carries no resume
				// cursor, so a denied refresh discards everything it read, and the dirty marker it
				// could not clear wakes the manager to read it all again.
				tx.charge_throttle(DEPOT_COMPACTION_THROTTLE, ThrottleCharge::Read)?;
				tx.priority(Priority::Low)?;
				let snapshot =
					read_manager_fdb_snapshot(&tx, database_branch_id, default_pitr_policy, now_ms)
						.await?;
				// Costs one empty range read on a branch holding no staging, which is the settled
				// state, so it runs every refresh instead of behind a trigger.
				let orphan_stage_jobs = scan_staged_job_subspaces(
					&tx,
					database_branch_id,
					&active_job_ids,
					CMP_STAGE_ORPHAN_SCAN_MAX_JOBS,
				)
				.await?;
				// Same shape as the staging scan above: one bounded read per refresh, on the branch's
				// own small `CSTAGE` prefix.
				let orphan_commit_stage_txids = scan_orphan_commit_stages(
					&tx,
					database_branch_id,
					snapshot.head.as_ref().map_or(0, |head| head.head_txid),
					now_ms,
					COMMIT_STAGE_ORPHAN_SCAN_MAX_TXIDS,
				)
				.await?;
				Ok((snapshot, orphan_stage_jobs, orphan_commit_stage_txids))
			}
		})
		.await?;
	let mut orphan_stage_hot_job_ids = Vec::new();
	let mut orphan_stage_cold_job_ids = Vec::new();
	for staged in orphan_stage_jobs {
		match staged.lane {
			keys::StagedJobLane::Hot => orphan_stage_hot_job_ids.push(staged.job_id),
			keys::StagedJobLane::Cold => orphan_stage_cold_job_ids.push(staged.job_id),
		}
	}
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
			dynamic_config
				.sqlite()
				.compaction_max_hot_drain_span_txids(),
		)
	} else {
		None
	};
	let planned_cold_job = None;
	let planned_reclaim_job = if branch_is_live {
		plan_reclaim_job(
			database_branch_id,
			&snapshot,
			Id::new_v1(ctx.config().dc_label()),
			now_ms,
			// The entry slice runs before the drain's version check, so it must not claim the
			// commit/delta lane: it would re-derive the whole window from txid 0, which is the most
			// expensive scan on the branch and exactly what the sweep replaces. The refresh still
			// derives the lane to decide whether dispatching a reclaim job is worth it at all.
			true,
		)
	} else {
		None
	};
	// Set only when this refresh planned no reclaim job, so presence is exactly "the reclaim lane sat
	// out this cycle" and absence is "a job was planned". Reporting a reason alongside a planned job
	// makes a lane that dispatches every cycle read as idle.
	let reclaim_noop_reason = if branch_is_live && planned_reclaim_job.is_none() {
		Some(reclaim_noop_reason(&snapshot).to_string())
	} else {
		None
	};

	// Observe the compaction backlog for live branches: hot lag is the txid span awaiting hot
	// install, cold lag is the hot-installed but not-yet-cold-published span.
	if branch_is_live {
		if let Some(head_txid) = head_txid {
			let hot_lag = head_txid.saturating_sub(snapshot.root.hot_watermark_txid);
			let cold_lag = snapshot
				.root
				.hot_watermark_txid
				.saturating_sub(snapshot.root.cold_watermark_txid);
			metrics::observe_lag(hot_lag, cold_lag);
		}
	}

	Ok(RefreshManagerOutput {
		refreshed_at_ms: now_ms,
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
		reclaim_noop_reason,
		compaction_admitted,
		orphan_stage_hot_job_ids,
		orphan_stage_cold_job_ids,
		orphan_commit_stage_txids,
	})
}

// Minting the cleanup job id must be recorded in workflow history. Generating it inline in the
// workflow body reads randomness (`Uuid::new_v4`), so a replay would produce a different id, mutate
// `active_jobs.reclaim`, and diverge the emitted `RunReclaimJob` signal sequence.
#[activity(MintCleanupJobId)]
pub async fn mint_cleanup_job_id(ctx: &ActivityCtx, _input: &MintCleanupJobIdInput) -> Result<Id> {
	Ok(Id::new_v1(ctx.config().dc_label()))
}

fn hot_job_finished_matches_active(
	signal: &HotJobFinished,
	active_job: &ActiveHotCompactionJob,
) -> bool {
	// The drain spans many slices, so there is no single input fingerprint to match. Correlation is by
	// job_id (unique per dispatch) plus the base manifest generation the drain was planned against.
	signal.job_id == active_job.job_id
		&& signal.job_kind == CompactionJobKind::Hot
		&& signal.base_manifest_generation == active_job.base_manifest_generation
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

/// Jobs whose staging the orphan scan must not report: the lanes running right now, whatever is
/// queued for cleanup, and whatever the in-flight cleanup job is already collecting.
///
/// Reporting one of these would not be unsafe (cleanup is idempotent and a live job's staging is
/// reported only after that job is finished), but it would spend a whole staging scan re-deriving
/// rows another cleanup is about to clear.
fn staging_jobs_accounted_for(state: &DbManagerState) -> Vec<Id> {
	let mut ids = state.active_jobs.job_ids();
	ids.extend(state.pending_cleanups.hot_job_ids.iter().copied());
	ids.extend(state.pending_cleanups.cold_job_ids.iter().copied());
	if let Some(reclaim) = state.active_jobs.reclaim.as_ref() {
		ids.extend(reclaim.input_range.stale_hot_job_ids.iter().copied());
		ids.extend(reclaim.input_range.stale_cold_job_ids.iter().copied());
	}

	ids
}

/// Queues a job's staging cleanup for the next free reclaim slot.
///
/// Every caller that cannot dispatch right now routes through here instead of returning, because a
/// cleanup request that is not queued is that job's staging area leaked: the job id lives nowhere
/// else once the signal is handled.
fn defer_cleanup_job(
	state: &mut DbManagerState,
	database_branch_id: DatabaseBranchId,
	job_id: Id,
	lane: keys::StagedJobLane,
	repair_action: &'static str,
	reason: &'static str,
	actor_id: Option<&str>,
) {
	let queued = match lane {
		keys::StagedJobLane::Hot => state.pending_cleanups.push_hot(job_id),
		keys::StagedJobLane::Cold => state.pending_cleanups.push_cold(job_id),
	};

	if queued {
		tracing::debug!(
			actor_id = log_actor_id(actor_id),
			?database_branch_id,
			?job_id,
			repair_action,
			reason,
			"queued stale compaction output cleanup for the next free reclaim slot"
		);
	} else {
		// The staging area stays resident until the manager's `CMP/stage/` orphan scan finds it.
		tracing::warn!(
			actor_id = log_actor_id(actor_id),
			?database_branch_id,
			?job_id,
			repair_action,
			reason,
			"pending compaction cleanup queue is full, deferring to the staging orphan scan"
		);
	}
}

/// Feeds the refresh's `CMP/stage/` orphan scan into the pending cleanup queue.
fn record_orphan_stage_jobs(
	state: &mut DbManagerState,
	input: &DbManagerInput,
	refresh: &RefreshManagerOutput,
) {
	for job_id in &refresh.orphan_stage_hot_job_ids {
		defer_cleanup_job(
			state,
			input.database_branch_id,
			*job_id,
			keys::StagedJobLane::Hot,
			"cleanup_orphan_stage_hot_output",
			"found by the staging orphan scan",
			input.actor_id.as_deref(),
		);
	}
	for txid in &refresh.orphan_commit_stage_txids {
		if !state.pending_cleanups.push_commit_stage(*txid) {
			tracing::warn!(
				actor_id = log_actor_id(input.actor_id.as_deref()),
				database_branch_id = ?input.database_branch_id,
				txid,
				repair_action = "cleanup_orphan_commit_stage",
				reason = "pending compaction cleanup queue is full",
				"deferring an abandoned staged commit to a later orphan scan"
			);
		}
	}
	for job_id in &refresh.orphan_stage_cold_job_ids {
		defer_cleanup_job(
			state,
			input.database_branch_id,
			*job_id,
			keys::StagedJobLane::Cold,
			"cleanup_orphan_stage_cold_output",
			"found by the staging orphan scan",
			input.actor_id.as_deref(),
		);
	}
}

/// Dispatches every queued cleanup as one merged repair reclaim job.
///
/// Merging matters as much as queuing does: the requests arrive within a second of each other (a hot
/// install's cleanup, then the cold publish that install just rejected), and dispatching them one per
/// free slot would leave the later ones waiting behind whatever claims the slot next.
async fn dispatch_pending_cleanups(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	input: &DbManagerInput,
) -> Result<()> {
	let Some(base_lifecycle_generation) = state.last_observed_branch_lifecycle_generation else {
		// Nothing observed the branch lifecycle yet, so the reclaimer's lifecycle fence has no value
		// to check against. The queue holds until a refresh observes one.
		return Ok(());
	};
	if state.pending_cleanups.is_empty() {
		return Ok(());
	}

	// `schedule_repair_reclaim_job` re-queues these ids if it cannot dispatch after all, so taking
	// them here cannot drop them.
	let pending = state.pending_cleanups.take();
	let mut input_range = repair_reclaim_input_range(
		pending.hot_job_ids.clone(),
		pending.cold_job_ids.clone(),
		std::iter::empty(),
	);
	input_range.stale_commit_stage_txids = pending.commit_stage_txids.clone();
	let source_job_id = pending
		.hot_job_ids
		.first()
		.or(pending.cold_job_ids.first())
		.copied()
		.unwrap_or_else(Id::nil);

	schedule_repair_reclaim_job(
		ctx,
		state,
		input.database_branch_id,
		base_lifecycle_generation,
		// Repair cleanup does not bind the manifest generation, and the deferred requests span
		// several generations anyway, so there is no single one to carry. This is safe only because
		// `reclaim_fdb_job` routes an input range whose four reclaim lanes are all empty (which
		// `repair_reclaim_input_range` guarantees) straight into the staging cleanup, ahead of its
		// `root.manifest_generation != base_manifest_generation` check. Adding a reclaim lane to a
		// cleanup input range would send it past that check and reject every cleanup on any branch
		// that has ever installed.
		0,
		input_range,
		source_job_id,
		"cleanup_deferred_output",
		input.actor_id.as_deref(),
	)
	.await?;

	// Only a dispatch that actually claimed the slot counts toward the yield. A request re-queued
	// because the slot was taken burned nothing, and recording it would make cleanup stand down for a
	// cycle it never got.
	state.last_reclaim_slot_was_cleanup = state.active_jobs.reclaim.is_some();

	Ok(())
}

async fn schedule_stale_hot_output_cleanup(
	ctx: &mut WorkflowCtx,
	state: &mut DbManagerState,
	signal: &HotJobFinished,
	actor_id: Option<&str>,
) -> Result<()> {
	// The drain staged its shards into the FDB staging area under this job id. This covers a stale
	// (non-matching) finish, a matching drain that staged some chunks before rejecting or failing,
	// and a successful install whose staged shards are now redundant after being copied into the
	// live tier. The reclaimer scans that job's staging area to find and clear the orphan shards, so
	// no ref list is carried here.
	let Some(base_lifecycle_generation) = state.last_observed_branch_lifecycle_generation else {
		defer_cleanup_job(
			state,
			signal.database_branch_id,
			signal.job_id,
			keys::StagedJobLane::Hot,
			"defer_stale_hot_output_cleanup",
			"branch lifecycle not observed yet",
			actor_id,
		);
		return Ok(());
	};

	let input_range =
		repair_reclaim_input_range(vec![signal.job_id], Vec::new(), std::iter::empty());

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
	stale_hot_job_ids: Vec<Id>,
	stale_cold_job_ids: Vec<Id>,
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
		delta_reclaim_segments: Vec::new(),
		cursor_segment_pgno: None,
		commit_reclaim_txids: Vec::new(),
		cold_objects: Vec::new(),
		shard_cache_evictions: Vec::new(),
		stale_hot_job_ids,
		stale_commit_stage_txids: Vec::new(),
		stale_cold_job_ids,
		skip_commit_delta: false,
		// Repair cleanup jobs carry an explicit one-shot input set and never run the cold-object or
		// commit reclaim scans, so they need no scan cursors.
		cold_scan_cursor: None,
		commit_scan_cursor: 0,
		max_keys: CMP_FDB_BATCH_MAX_KEYS as u32,
		max_bytes: CMP_FDB_BATCH_MAX_VALUE_BYTES as u64,
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
		// The reclaimer is almost always busy right after an install, so this is the common path, not
		// the exceptional one. Queue every id the request carried for the next free slot.
		for job_id in &input_range.stale_hot_job_ids {
			defer_cleanup_job(
				state,
				database_branch_id,
				*job_id,
				keys::StagedJobLane::Hot,
				repair_action,
				"reclaimer is busy",
				actor_id,
			);
		}
		for job_id in &input_range.stale_cold_job_ids {
			defer_cleanup_job(
				state,
				database_branch_id,
				*job_id,
				keys::StagedJobLane::Cold,
				repair_action,
				"reclaimer is busy",
				actor_id,
			);
		}
		for txid in &input_range.stale_commit_stage_txids {
			if !state.pending_cleanups.push_commit_stage(*txid) {
				tracing::warn!(
					actor_id = log_actor_id(actor_id),
					?database_branch_id,
					txid,
					repair_action,
					reason = "pending compaction cleanup queue is full",
					"deferring an abandoned staged commit to a later orphan scan"
				);
			}
		}
		return Ok(());
	}

	let cleanup_job_id = ctx
		.v(2)
		.activity(MintCleanupJobIdInput { database_branch_id })
		.await?;
	let input_fingerprint = fingerprint_repair_reclaim_range(database_branch_id, &input_range);
	tracing::warn!(
		actor_id = log_actor_id(actor_id),
		?database_branch_id,
		manifest_generation = base_manifest_generation,
		?source_job_id,
		?cleanup_job_id,
		repair_action,
		stale_hot_job_count = input_range.stale_hot_job_ids.len(),
		stale_cold_job_count = input_range.stale_cold_job_ids.len(),
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
		// Staging cleanup is never gated on the admission percent. The job ids it collects live
		// nowhere else, so a de-admitted branch would leak its staging.
		bypass_admission: true,
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
