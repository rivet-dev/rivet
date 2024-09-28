use chirp_workflow::prelude::*;
use futures_util::StreamExt;
use rivet_operation::prelude::proto::backend::pkg::*;

use crate::{
	util::{signal_allocation, NOMAD_REGION},
	workers::NOMAD_CONFIG,
};

// In ms, a small amount of time to separate the completion of the drain to the deletion of the
// cluster server. We want the drain to complete first.
const DRAIN_PADDING: u64 = 10000;

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub nomad_node_id: String,
	pub drain_timeout: u64,
}

#[workflow]
pub async fn job_run_drain_all(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	// We fetch here so that when we kill allocs later, we don't refetch new job runs that might be on the
	// nomad node. Only allocs fetched at this time will be killed.
	let job_runs = ctx
		.activity(FetchJobRunsInput {
			nomad_node_id: input.nomad_node_id.clone(),
		})
		.await?;

	ctx.activity(StopJobRunsInput {
		run_ids: job_runs.iter().map(|jr| jr.run_id).collect(),
	})
	.await?;

	ctx.sleep(input.drain_timeout.saturating_sub(DRAIN_PADDING))
		.await?;

	ctx.activity(KillAllocsInput {
		nomad_node_id: input.nomad_node_id.clone(),
		alloc_ids: job_runs.into_iter().filter_map(|jr| jr.alloc_id).collect(),
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Input2 {
	pub nomad_node_id: String,
	pub drain_timeout: u64,
}

#[workflow(Workflow2)]
pub async fn job_run_drain_all2(ctx: &mut WorkflowCtx, input: &Input2) -> GlobalResult<()> {
	// We fetch here so that when we kill allocs later, we don't refetch new job runs that might be on the
	// nomad node. Only allocs fetched at this time will be killed.
	let job_runs = ctx
		.activity(FetchJobRunsInput {
			nomad_node_id: input.nomad_node_id.clone(),
		})
		.await?;

	ctx.sleep(input.drain_timeout.saturating_sub(DRAIN_PADDING))
		.await?;

	ctx.activity(StopJobRuns2Input {
		run_ids: job_runs.iter().map(|jr| jr.run_id).collect(),
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct FetchJobRunsInput {
	nomad_node_id: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct RunMetaNomadRow {
	run_id: Uuid,
	alloc_id: Option<String>,
}

#[activity(FetchJobRuns)]
async fn fetch_job_runs(
	ctx: &ActivityCtx,
	input: &FetchJobRunsInput,
) -> GlobalResult<Vec<RunMetaNomadRow>> {
	sql_fetch_all!(
		[ctx, RunMetaNomadRow]
		"
		SELECT r.run_id, rn.alloc_id
		FROM db_job_state.runs AS r
		JOIN db_job_state.run_meta_nomad AS rn
		ON r.run_id = rn.run_id
		WHERE
			rn.node_id = $1 AND
			r.stop_ts IS NULL
		",
		&input.nomad_node_id,
	)
	.await
	.map_err(Into::into)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct StopJobRunsInput {
	run_ids: Vec<Uuid>,
}

#[activity(StopJobRuns)]
async fn stop_job_runs(ctx: &ActivityCtx, input: &StopJobRunsInput) -> GlobalResult<()> {
	for run_id in &input.run_ids {
		msg!([ctx] job_run::msg::stop(run_id) {
			run_id: Some((*run_id).into()),
			skip_kill_alloc: true,
		})
		.await?;
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct StopJobRuns2Input {
	run_ids: Vec<Uuid>,
}

#[activity(StopJobRuns2)]
async fn stop_job_runs2(ctx: &ActivityCtx, input: &StopJobRuns2Input) -> GlobalResult<()> {
	for run_id in &input.run_ids {
		msg!([ctx] job_run::msg::stop(run_id) {
			run_id: Some((*run_id).into()),
			skip_kill_alloc: false,
		})
		.await?;
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct KillAllocsInput {
	nomad_node_id: String,
	alloc_ids: Vec<String>,
}

#[activity(KillAllocs)]
async fn kill_allocs(ctx: &ActivityCtx, input: &KillAllocsInput) -> GlobalResult<()> {
	futures_util::stream::iter(input.alloc_ids.iter().cloned())
		.map(|alloc_id| async move {
			if let Err(err) = signal_allocation(
				&NOMAD_CONFIG,
				&alloc_id,
				None,
				Some(NOMAD_REGION),
				None,
				None,
				Some(nomad_client::models::AllocSignalRequest {
					task: None,
					signal: Some("SIGKILL".to_string()),
				}),
			)
			.await
			{
				tracing::warn!(
					?err,
					?alloc_id,
					"error while trying to manually kill allocation"
				);
			}
		})
		.buffer_unordered(16)
		.collect::<Vec<_>>()
		.await;

	Ok(())
}
