use super::{test_hooks, *};
use crate::workflows::db_manager::DbManagerWorkflow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompanionKind {
	Hot,
	Reclaim,
}

/// Durable loop state for the hot companion's internal drain. Only the cursor is checkpointed by
/// `loope` so a workflow restart resumes mid-drain instead of re-staging from the start. Each staged
/// slice writes its shard refs to the FDB staging area under the job id, so the manager can install
/// the whole drain with a single `InstallHotJob` without the companion accumulating refs in state.
#[derive(Serialize, Deserialize)]
struct HotDrainState {
	cursor: Option<u64>,
	/// Resume position inside `cursor`'s own commit. A commit whose pages do not all fit one slice is
	/// admitted across several, and `cursor` holds until the commit reports itself fully folded.
	#[serde(default)]
	segment_pgno: Option<u32>,
	/// Resume position inside the current slice's shard staging. A slice whose folded shard images do
	/// not fit one FDB transaction is staged across several, and `cursor` holds until the slice reports
	/// itself fully staged.
	#[serde(default)]
	stage_cursor: Option<HotStageCursor>,
}

/// Durable loop state for the reclaimer's internal replan-and-drain. Reclaim deletes apply immediately,
/// so most lanes drain by replanning from current FDB state. The cold-object and commit/delta reclaim
/// scans are read in bounded windows, so their cursors are carried here and advanced from each pass so
/// a long history drains across bounded passes within one job. The dead-shard fold walk is not here: it
/// runs in the standalone `SweepDeadShardVersions` activity, which holds its cross-chunk context in
/// local memory.
///
/// Both cursors are per-job, not per-branch: one reclaim job sweeps the range once and stops, so
/// reclaim keeps trailing compaction rather than running on its own schedule.
#[derive(Serialize, Deserialize, Default)]
struct ReclaimDrainState {
	cold_scan_cursor: Option<ColdScanCursor>,
	commit_scan_cursor: u64,
	/// Where to resume inside `commit_scan_cursor`'s txid when a pass stopped between its segments. A
	/// large commit is many shard-aligned segments and need not fit one window, so the sweep can stop
	/// mid-commit and pick up where it left off. `serde(default)` so a state serialized before
	/// segmentation deserializes and resumes as a whole-txid scan.
	#[serde(default)]
	segment_pgno: Option<u32>,
}

pub(crate) async fn run_companion_loop(
	ctx: &mut WorkflowCtx,
	database_branch_id: DatabaseBranchId,
	kind: CompanionKind,
) -> Result<()> {
	match kind {
		CompanionKind::Hot => run_hot_companion_loop(ctx, database_branch_id).await,
		CompanionKind::Reclaim => run_reclaim_companion_loop(ctx, database_branch_id).await,
	}
}

async fn run_hot_companion_loop(
	ctx: &mut WorkflowCtx,
	database_branch_id: DatabaseBranchId,
) -> Result<()> {
	ctx.lupe()
		.commit_interval(1)
		.with_state(CompanionWorkflowState::Idle)
		.run(|ctx, state| {
			async move {
				for signal in ctx.listen_n::<DbHotCompactorSignal>(256).await? {
					if signal.database_branch_id() != database_branch_id {
						continue;
					}

					handle_hot_companion_signal(ctx, state, database_branch_id, signal).await?;
				}

				Ok(companion_loop_after_signals(state))
			}
			.boxed()
		})
		.await
}

async fn run_reclaim_companion_loop(
	ctx: &mut WorkflowCtx,
	database_branch_id: DatabaseBranchId,
) -> Result<()> {
	ctx.lupe()
		.commit_interval(1)
		.with_state(CompanionWorkflowState::Idle)
		.run(|ctx, state| {
			async move {
				for signal in ctx.listen_n::<DbReclaimerSignal>(256).await? {
					if signal.database_branch_id() != database_branch_id {
						continue;
					}

					handle_reclaim_companion_signal(ctx, state, database_branch_id, signal).await?;
				}

				Ok(companion_loop_after_signals(state))
			}
			.boxed()
		})
		.await
}

async fn handle_hot_companion_signal(
	ctx: &mut WorkflowCtx,
	state: &mut CompanionWorkflowState,
	database_branch_id: DatabaseBranchId,
	signal: DbHotCompactorSignal,
) -> Result<()> {
	match signal {
		DbHotCompactorSignal::RunHotJob(signal) => {
			run_hot_compaction_job(ctx, state, database_branch_id, signal).await
		}
		DbHotCompactorSignal::DestroyDatabaseBranch(signal) => {
			record_companion_stop_signal(state, signal);
			Ok(())
		}
	}
}

async fn handle_reclaim_companion_signal(
	ctx: &mut WorkflowCtx,
	state: &mut CompanionWorkflowState,
	database_branch_id: DatabaseBranchId,
	signal: DbReclaimerSignal,
) -> Result<()> {
	match signal {
		DbReclaimerSignal::RunReclaimJob(signal) => {
			run_reclaim_job(ctx, state, database_branch_id, signal).await
		}
		DbReclaimerSignal::DestroyDatabaseBranch(signal) => {
			record_companion_stop_signal(state, signal);
			Ok(())
		}
	}
}

fn companion_loop_after_signals(state: &CompanionWorkflowState) -> Loop<()> {
	if matches!(state, CompanionWorkflowState::Stopping { .. }) {
		Loop::Break(())
	} else {
		Loop::Continue
	}
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
		[0u8; 32],
		ctx.create_ts(),
	);

	// Drain every hot slice in `[hot_watermark+1 .. drain_head_txid]` internally via a durable loop.
	// Each slice stages its shard refs into the FDB staging area under `job_id`; the loop carries only
	// the cursor. The manager installs the whole staged set at once when we report `Succeeded`, and on
	// any rejection or failure it cleans up the staged shards by scanning the staging area instead.
	let job_id = signal.job_id;
	let base_lifecycle_generation = signal.base_lifecycle_generation;
	let base_manifest_generation = signal.base_manifest_generation;
	let drain_head_txid = signal.drain_head_txid;
	let drain_now_ms = signal.drain_now_ms;
	let bypass_admission = signal.bypass_admission;
	let status = ctx
		.loope(
			HotDrainState {
				cursor: None,
				segment_pgno: None,
				stage_cursor: None,
			},
			move |ctx, drain| {
				async move {
					let output = ctx
						.activity(StageHotSliceInput {
							database_branch_id,
							job_id,
							base_lifecycle_generation,
							base_manifest_generation,
							cursor_min_txid: drain.cursor,
							cursor_min_segment_pgno: drain.segment_pgno,
							stage_cursor: drain.stage_cursor,
							drain_head_txid,
							drain_now_ms,
							bypass_admission,
						})
						.await?;
					test_hooks::maybe_pause_after_hot_stage(database_branch_id).await;
					// The branch fell outside the admission percent. Park on the cursors as they
					// stand: staged shards stay in the staging area and the drain resumes exactly
					// here once the percent is raised. The sleep is longer than the worker poll
					// interval, so gasoline parks the workflow in the database and frees the worker
					// slot rather than holding the lease in memory.
					if output.admission_blocked {
						drain.stage_cursor = output.next_stage_cursor;
						ctx.sleep(crate::ADMISSION_PARK_MS).await?;
						return Ok(Loop::Continue);
					}
					// A compaction throttle budget was spent this window. Back off and resume the
					// same slice; whatever was staged is committed and the cursors track it, so no
					// work is lost. A slice that staged a duplicate backs off far longer than the
					// lanes it yields to, so its retries stop competing for the budget install and
					// reclaim are working under; a direct fold made no duplicate and backs off on the
					// ordinary terms. Reading the backoff here is replay-safe: gasoline records the
					// sleep deadline and replays that recorded value rather than recomputing it.
					if output.throttled {
						drain.stage_cursor = output.next_stage_cursor;
						ctx.sleep(throttle::hot_slice_backoff_ms(ctx.config()))
							.await?;
						return Ok(Loop::Continue);
					}
					// The slice's shard images did not all fit this call. Resume it from the staging
					// cursor without advancing the txid cursor.
					if let Some(stage_cursor) = output.next_stage_cursor {
						drain.stage_cursor = Some(stage_cursor);
						return Ok(Loop::Continue);
					}
					match output.status {
						CompactionJobStatus::Succeeded => match output.slice {
							Some(slice) => {
								drain.stage_cursor = None;
								// A commit staged only in part holds the txid cursor and advances the
								// page cursor, so the next slice resumes inside it. Only a commit
								// staged whole moves the drain on to the next txid.
								if let Some(next_pgno) = slice.input_range.max_pgno_exclusive {
									drain.segment_pgno = Some(next_pgno);
									return Ok(Loop::Continue);
								}
								let next_cursor =
									slice.input_range.txids.max_txid.saturating_add(1);
								drain.cursor = Some(next_cursor);
								drain.segment_pgno = None;
								if next_cursor > drain_head_txid {
									Ok(Loop::Break(CompactionJobStatus::Succeeded))
								} else {
									Ok(Loop::Continue)
								}
							}
							None => Ok(Loop::Break(CompactionJobStatus::Succeeded)),
						},
						other => Ok(Loop::Break(other)),
					}
				}
				.boxed()
			},
		)
		.await?;

	#[cfg(feature = "test-faults")]
	let status = match test_hooks::maybe_fire_hot_compaction_fault(
		database_branch_id,
		crate::fault::HotCompactionFaultPoint::AfterStageBeforeFinishSignal,
	)
	.await
	{
		Ok(Some(_)) | Ok(None) => status,
		Err(err) => CompactionJobStatus::Failed {
			error: err.to_string(),
		},
	};

	let tag_value = database_branch_tag_value(database_branch_id);
	ctx.signal(HotJobFinished {
		database_branch_id,
		job_id: signal.job_id,
		job_kind: CompactionJobKind::Hot,
		base_manifest_generation: signal.base_manifest_generation,
		status,
	})
	.to_workflow::<DbManagerWorkflow>()
	.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
	.send()
	.await?;

	*state = CompanionWorkflowState::Idle;

	Ok(())
}

/// How the v2 reclaim drain reports that it stopped early because the branch fell outside the
/// admission percent.
///
/// Encoded as a status rather than a richer return type because the drain's break value is persisted
/// in workflow history: a reclaim job that is mid-job across a deploy replays a recorded
/// `CompactionJobStatus`, so the type has to stay as it is. `Requested` is free for this because no
/// other break path in the drain produces it. `run_reclaim_job` translates it to `Succeeded` before
/// anything else sees it.
const RECLAIM_DRAIN_ADMISSION_BLOCKED: CompactionJobStatus = CompactionJobStatus::Requested;

/// The v2 reclaim drain. Each pass derives and clears one window of commit/delta history in a single
/// transaction, then plans and executes the cold lanes, which still need a durable planned set because
/// their S3 retirement runs past the transaction that selected them.
///
/// The loop is the chunk loop: `ReclaimDrainState.commit_scan_cursor` has always been where the commit
/// window's position lives, and the sweep advances it by committing rather than by planning. A
/// conflict therefore costs one window instead of a whole pass, and the reads that produced a cleared
/// row are the reads that justified clearing it.
async fn run_reclaim_drain_v2(
	ctx: &mut WorkflowCtx,
	database_branch_id: DatabaseBranchId,
	job_id: Id,
	base_lifecycle_generation: u64,
	base_manifest_generation: u64,
	bypass_admission: bool,
) -> Result<CompactionJobStatus> {
	ctx.loope(ReclaimDrainState::default(), move |ctx, state| {
		async move {
			let sweep = ctx
				.activity(SweepCommitDeltaChunkInput {
					database_branch_id,
					base_lifecycle_generation,
					base_manifest_generation,
					commit_scan_cursor: state.commit_scan_cursor,
					cursor_segment_pgno: state.segment_pgno,
					bypass_admission,
				})
				.await?;
			// The branch fell outside the admission percent. End the job rather than parking on it:
			// the reclaim slot is shared with the staging cleanups, which are deliberately not gated
			// on the percent, and a parked drain would hold that slot for as long as the percent
			// stayed down. Nothing is lost by stopping here, because reclaim's cursors are per-job
			// and its deletes apply as it goes, so the next job replans from current FDB state.
			if sweep.admission_blocked {
				return Ok(Loop::Break(RECLAIM_DRAIN_ADMISSION_BLOCKED));
			}
			if sweep.throttled {
				ctx.sleep(crate::THROTTLE_BACKOFF_MS).await?;
				return Ok(Loop::Continue);
			}
			if !matches!(sweep.status, CompactionJobStatus::Succeeded) {
				return Ok(Loop::Break(sweep.status));
			}
			// Advances past retained history too, so a pinned prefix cannot stall the reclaimable rows
			// behind it.
			state.commit_scan_cursor = sweep.next_commit_scan_cursor;
			state.segment_pgno = sweep.next_segment_pgno;

			// The cold lanes keep plan-then-execute: `cold_objects` drives retirement and S3 deletes in
			// later activities, so that set has to survive the transaction that derived it.
			let output = ctx
				.activity(PlanReclaimSliceInput {
					database_branch_id,
					base_lifecycle_generation,
					base_manifest_generation,
					cold_scan_cursor: state.cold_scan_cursor,
					commit_scan_cursor: state.commit_scan_cursor,
					// The sweep owns this lane now, so planning it here would scan the same window
					// twice per pass, and there is no segment cursor to carry for a lane it skips.
					cursor_segment_pgno: None,
					skip_commit_delta: true,
				})
				.await?;
			if output.throttled {
				ctx.sleep(crate::THROTTLE_BACKOFF_MS).await?;
				return Ok(Loop::Continue);
			}
			state.cold_scan_cursor = output.next_cold_scan_cursor;

			let Some(planned) = output.planned else {
				// Completion is the sweep's to report for commit/delta; the plan only speaks for the
				// cold scan.
				if sweep.commit_scan_complete && output.next_cold_scan_cursor.is_none() {
					return Ok(Loop::Break(CompactionJobStatus::Succeeded));
				}
				return Ok(Loop::Continue);
			};
			let outcome = execute_reclaim_slice(
				ctx,
				database_branch_id,
				job_id,
				CompactionJobKind::Reclaim,
				base_lifecycle_generation,
				base_manifest_generation,
				planned.input_fingerprint,
				planned.input_range,
				bypass_admission,
			)
			.await?;
			// The percent dropped between the sweep that admitted this pass and the slice that
			// executes it. End the drain on the same terms the sweep's own gate does.
			if outcome.admission_blocked {
				return Ok(Loop::Break(RECLAIM_DRAIN_ADMISSION_BLOCKED));
			}
			if matches!(outcome.status, CompactionJobStatus::Succeeded) {
				Ok(Loop::Continue)
			} else {
				Ok(Loop::Break(outcome.status))
			}
		}
		.boxed()
	})
	.await
}

/// The v1 reclaim drain. Frozen: it is reached only by jobs whose version check recorded `1`, i.e.
/// jobs already in flight when the v2 drain shipped. Every lane here still works the way it did, and
/// nothing new should be added to it.
///
/// Each pass plans a slice and then executes it, and the executing side re-derives the planner's whole
/// window under `Serializable` to fence it. That re-derive is what v2 removes.
async fn run_reclaim_drain_v1(
	ctx: &mut WorkflowCtx,
	database_branch_id: DatabaseBranchId,
	job_id: Id,
	base_lifecycle_generation: u64,
	base_manifest_generation: u64,
	bypass_admission: bool,
) -> Result<CompactionJobStatus> {
	ctx.loope(ReclaimDrainState::default(), move |ctx, state| {
		async move {
			let output = ctx
				.activity(PlanReclaimSliceInput {
					database_branch_id,
					base_lifecycle_generation,
					base_manifest_generation,
					cold_scan_cursor: state.cold_scan_cursor,
					commit_scan_cursor: state.commit_scan_cursor,
					cursor_segment_pgno: state.segment_pgno,
					skip_commit_delta: false,
				})
				.await?;
			// The cluster-wide compaction read budget was spent this window, so the pass ran
			// no scan. Back off and replan the same window; the cursors it handed back are the
			// ones it was given.
			if output.throttled {
				ctx.sleep(crate::THROTTLE_BACKOFF_MS).await?;
				return Ok(Loop::Continue);
			}
			// Advance both bounded scans even when this pass planned no work, so ineligible
			// rows filling a window cannot stall the reclaimable rows sitting behind them.
			state.cold_scan_cursor = output.next_cold_scan_cursor;
			state.commit_scan_cursor = output.next_commit_scan_cursor;
			state.segment_pgno = output.next_segment_pgno;
			let Some(planned) = output.planned else {
				// An empty plan only means this window held nothing reclaimable. The job is
				// done once both windowed scans have reached the end of their ranges.
				if output.commit_scan_complete && output.next_cold_scan_cursor.is_none() {
					return Ok(Loop::Break(CompactionJobStatus::Succeeded));
				}
				return Ok(Loop::Continue);
			};
			let outcome = execute_reclaim_slice(
				ctx,
				database_branch_id,
				job_id,
				CompactionJobKind::Reclaim,
				base_lifecycle_generation,
				base_manifest_generation,
				planned.input_fingerprint,
				planned.input_range,
				bypass_admission,
			)
			.await?;
			// The percent dropped between the sweep that admitted this pass and the slice that
			// executes it. End the drain on the same terms the sweep's own gate does.
			if outcome.admission_blocked {
				return Ok(Loop::Break(RECLAIM_DRAIN_ADMISSION_BLOCKED));
			}
			if matches!(outcome.status, CompactionJobStatus::Succeeded) {
				Ok(Loop::Continue)
			} else {
				Ok(Loop::Break(outcome.status))
			}
		}
		.boxed()
	})
	.await
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

	// Repair cleanup jobs carry an explicit one-shot input set and must not replan.
	let is_repair = !signal.input_range.stale_hot_job_ids.is_empty()
		|| !signal.input_range.stale_cold_job_ids.is_empty();

	let outcome = execute_reclaim_slice(
		ctx,
		database_branch_id,
		signal.job_id,
		signal.job_kind,
		signal.base_lifecycle_generation,
		signal.base_manifest_generation,
		signal.input_fingerprint,
		signal.input_range.clone(),
		signal.bypass_admission,
	)
	.await?;
	let mut status = outcome.status;

	// Set when the drain stops early because the branch fell outside the admission percent. The
	// sweeps after it are more reclaim work on the same branch, so they are skipped too.
	//
	// The first slice sets it too, which is what stops a staging cleanup on a de-admitted branch.
	// Cleanup reaches this activity directly rather than through the drain, so the drain's gate never
	// sees it, and it is the one reclaim lane the manager dispatches without consulting the percent.
	let mut drain_admission_blocked = outcome.admission_blocked;
	if drain_admission_blocked {
		// The job did everything it was allowed to do. Report a clean finish so the manager frees the
		// reclaim slot instead of re-dispatching an input the percent will refuse again.
		status = CompactionJobStatus::Succeeded;
	}

	// Reclaim applies its deletes immediately, so a normal reclaim drains by replanning from current
	// FDB state until the whole reclaimable range has been swept. A durable loop checkpoints progress
	// so a restart resumes from current state rather than redoing all prior passes.
	//
	// The version check is per job, not per workflow: this companion is persistent, so a check at the
	// top of the workflow would pin every branch that already has one to v1 for the life of the branch
	// and the new drain would only ever reach branches created after it shipped. Here, a job already
	// mid-drain finishes on v1 and the branch's next job records v2.
	if !is_repair && !drain_admission_blocked && matches!(status, CompactionJobStatus::Succeeded) {
		let job_id = signal.job_id;
		let base_lifecycle_generation = signal.base_lifecycle_generation;
		let base_manifest_generation = signal.base_manifest_generation;
		status = match ctx.check_version(2).await? {
			1 => {
				run_reclaim_drain_v1(
					ctx,
					database_branch_id,
					job_id,
					base_lifecycle_generation,
					base_manifest_generation,
					signal.bypass_admission,
				)
				.await?
			}
			_latest => {
				run_reclaim_drain_v2(
					ctx,
					database_branch_id,
					job_id,
					base_lifecycle_generation,
					base_manifest_generation,
					signal.bypass_admission,
				)
				.await?
			}
		};
		// Translate the drain's de-admitted marker before anything downstream reads the status. The
		// job did everything it was allowed to do, so the manager sees a clean finish and frees the
		// reclaim slot.
		if matches!(status, RECLAIM_DRAIN_ADMISSION_BLOCKED) {
			drain_admission_blocked = true;
			status = CompactionJobStatus::Succeeded;
		}
	}
	// Run the standalone dead-shard version-retention sweep (C4). It walks the whole `CMP/fold` index
	// in one activity, holding its cross-chunk supersession context in local memory, and re-dispatches
	// on an early timeout. A single forward walk detects every current supersession, so this is one
	// activity call in the common case.
	if !is_repair && !drain_admission_blocked && matches!(status, CompactionJobStatus::Succeeded) {
		let base_lifecycle_generation = signal.base_lifecycle_generation;
		let base_manifest_generation = signal.base_manifest_generation;
		status = ctx
			.loope(0u32, move |ctx, _attempt| {
				async move {
					let output = ctx
						.activity(SweepDeadShardVersionsInput {
							database_branch_id,
							base_lifecycle_generation,
							base_manifest_generation,
						})
						.await?;
					match output.status {
						// The activity crossed the early timeout with folds left; re-dispatch. The
						// re-walk from the start skips already-deleted versions.
						CompactionJobStatus::Requested => Ok(Loop::Continue),
						other => Ok(Loop::Break(other)),
					}
				}
				.boxed()
			})
			.await?;
	}

	// Run the one-time stale-PIDX repair walk. It clears rows left behind by hot slices that folded a
	// page without clearing its PIDX row, which nothing else in the system can free. The activity
	// no-ops once the branch carries the repair marker, so the steady-state cost is one point read per
	// reclaim job rather than a walk of the whole PIDX prefix.
	if !is_repair && !drain_admission_blocked && matches!(status, CompactionJobStatus::Succeeded) {
		let base_lifecycle_generation = signal.base_lifecycle_generation;
		let base_manifest_generation = signal.base_manifest_generation;
		status = ctx
			.loope(StalePidxSweepState::default(), move |ctx, state| {
				async move {
					let output = ctx
						.activity(SweepStalePidxInput {
							database_branch_id,
							base_lifecycle_generation,
							base_manifest_generation,
							pgno_cursor: state.pgno_cursor(),
							retained_unconfirmed: state.retained_unconfirmed(),
						})
						.await?;
					match output.status {
						// The activity crossed the early timeout with rows left. Resume from the
						// cursor it reached; restarting the walk would re-read every live row before
						// it reached the next stale one.
						CompactionJobStatus::Requested => {
							*state = StalePidxSweepState::Walk {
								pgno_cursor: output.next_pgno_cursor,
								retained_unconfirmed: output.retained_unconfirmed,
							};
							Ok(Loop::Continue)
						}
						other => Ok(Loop::Break(other)),
					}
				}
				.boxed()
			})
			.await?;
	}

	let tag_value = database_branch_tag_value(database_branch_id);
	ctx.signal(ReclaimJobFinished {
		database_branch_id,
		job_id: signal.job_id,
		job_kind: CompactionJobKind::Reclaim,
		base_manifest_generation: signal.base_manifest_generation,
		input_fingerprint: signal.input_fingerprint,
		status,
		output_refs: Vec::new(),
	})
	.to_workflow::<DbManagerWorkflow>()
	.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
	.send()
	.await?;

	*state = CompanionWorkflowState::Idle;

	Ok(())
}

/// What one reclaim slice reported back.
///
/// `admission_blocked` is carried beside the status rather than encoded in it because the status a
/// de-admitted slice returns is `Requested`, which every caller here also produces for ordinary
/// reasons. The callers act on it by ending the job, so conflating the two would make a de-admitted
/// branch look like one with work still pending.
struct ReclaimSliceOutcome {
	status: CompactionJobStatus,
	admission_blocked: bool,
}

async fn execute_reclaim_slice(
	ctx: &mut WorkflowCtx,
	database_branch_id: DatabaseBranchId,
	job_id: Id,
	job_kind: CompactionJobKind,
	base_lifecycle_generation: u64,
	base_manifest_generation: u64,
	input_fingerprint: CompactionInputFingerprint,
	input_range: ReclaimJobInputRange,
	bypass_admission: bool,
) -> Result<ReclaimSliceOutcome> {
	// Retry the FDB delete slice while the cluster-wide compaction write budget is spent. A throttled
	// slice issues no deletes and changes nothing, so backing off and re-running the same input is
	// safe and idempotent.
	// Use a durable loop so throttled retries move to forgotten history instead of growing live
	// workflow history unboundedly.
	let reclaim_input_range = input_range.clone();
	let output = ctx
		.repeat(move |ctx| {
			let input_range = reclaim_input_range.clone();
			async move {
				let output = ctx
					.activity(ReclaimFdbJobInput {
						database_branch_id,
						job_id,
						job_kind,
						base_lifecycle_generation,
						base_manifest_generation,
						input_fingerprint,
						input_range,
						bypass_admission,
					})
					.await?;
				// The branch fell outside the admission percent. Stop rather than back off: the
				// percent is an operator switch that can stay down for hours, and a slice parked on
				// it would hold the reclaim slot for that whole time.
				if output.admission_blocked {
					return Ok(Loop::Break(output));
				}
				if output.throttled {
					ctx.sleep(crate::THROTTLE_BACKOFF_MS).await?;
					return Ok(Loop::Continue);
				}
				// The slice filled its batch budget with work still pending. What it did clear is
				// committed, so re-dispatching the same input drains the remainder in another
				// bounded transaction rather than growing one transaction past FDB's limits.
				if output.has_more {
					return Ok(Loop::Continue);
				}
				Ok(Loop::Break(output))
			}
			.boxed()
		})
		.await?;

	Ok(ReclaimSliceOutcome {
		status: output.status,
		admission_blocked: output.admission_blocked,
	})
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

fn record_companion_stop_signal(state: &mut CompanionWorkflowState, signal: DestroyDatabaseBranch) {
	record_companion_stop(
		state,
		signal.lifecycle_generation,
		signal.requested_at_ms,
		signal.reason,
	);
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

/// Durable state of the stale-PIDX repair walk.
///
/// This loop originally carried a bare `Option<u32>` cursor. Widening a durable loop's state
/// normally wedges every workflow already in flight, because their persisted state no longer
/// deserializes. `#[serde(untagged)]` avoids that: loop state is stored as self-describing JSON, so a
/// persisted bare cursor still matches `Cursor` while everything written from now on is a `Walk`.
/// Do not add a tag or reorder the variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum StalePidxSweepState {
	/// Written by this build: the cursor plus whether any window so far retained a row it could not
	/// confirm against a shard image.
	Walk {
		pgno_cursor: Option<u32>,
		retained_unconfirmed: bool,
	},
	/// Persisted by an older build, which carried the cursor alone.
	Cursor(Option<u32>),
}

impl Default for StalePidxSweepState {
	fn default() -> Self {
		Self::Walk {
			pgno_cursor: None,
			retained_unconfirmed: false,
		}
	}
}

impl StalePidxSweepState {
	pub(crate) fn pgno_cursor(&self) -> Option<u32> {
		match self {
			Self::Walk { pgno_cursor, .. } => *pgno_cursor,
			Self::Cursor(pgno_cursor) => *pgno_cursor,
		}
	}

	/// An older build's state cannot say whether its walk retained anything, so treat it as clean.
	/// The walk it is resuming predates the check either way, and a branch it wrongly retires is
	/// still reachable by the same means as before this fix.
	pub(crate) fn retained_unconfirmed(&self) -> bool {
		match self {
			Self::Walk {
				retained_unconfirmed,
				..
			} => *retained_unconfirmed,
			Self::Cursor(_) => false,
		}
	}
}
