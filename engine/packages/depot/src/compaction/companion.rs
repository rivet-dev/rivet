use super::{test_hooks, *};
use crate::workflows::db_manager::DbManagerWorkflow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompanionKind {
	Hot,
	Reclaim,
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
				for signal in ctx.listen_n::<DbHotCompacterSignal>(256).await? {
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
	signal: DbHotCompacterSignal,
) -> Result<()> {
	match signal {
		DbHotCompacterSignal::RunHotJob(signal) => {
			run_hot_compaction_job(ctx, state, database_branch_id, signal).await
		}
		DbHotCompacterSignal::DestroyDatabaseBranch(signal) => {
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
	test_hooks::maybe_pause_after_hot_stage(database_branch_id).await;
	#[cfg(feature = "test-faults")]
	let output = match test_hooks::maybe_fire_hot_compaction_fault(
		database_branch_id,
		crate::fault::HotCompactionFaultPoint::AfterStageBeforeFinishSignal,
	)
	.await
	{
		Ok(Some(_)) | Ok(None) => output,
		Err(err) => StageHotJobOutput {
			status: CompactionJobStatus::Failed {
				error: err.to_string(),
			},
			output_refs: Vec::new(),
		},
	};

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

	let status = output.status;
	let output_refs = output.output_refs;

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
