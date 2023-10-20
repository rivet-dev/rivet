use proto::backend::pkg::*;

use rivet_operation::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PlanResult {
	allocation: nomad_client::models::Allocation,
}

#[tracing::instrument(skip_all)]
pub async fn start(
	shared_client: chirp_client::SharedClientHandle,
	redis_job: RedisPool,
) -> GlobalResult<()> {
	let redis_index_key = format!(
		"nomad:monitor_index:job_run_alloc_plan_monitor:{}",
		shared_client.region()
	);

	let configuration = nomad_util::config_from_env()?;
	nomad_util::monitor::Monitor::run(
		configuration,
		redis_job.clone(),
		&redis_index_key,
		&["Allocation"],
		move |event| {
			let client = shared_client.clone().wrap_new("job-run-alloc-plan-monitor");
			async move {
				if let Some(payload) = event
					.decode::<PlanResult>("Allocation", "PlanResult")
					.unwrap()
				{
					let spawn_res = tokio::task::Builder::new()
						.name("job_run_alloc_plan_monitor::handle_event")
						.spawn(handle(client, payload, event.payload.to_string()));
					if let Err(err) = spawn_res {
						tracing::error!(?err, "failed to spawn handle_event task");
					}
				}
			}
		},
	)
	.await?;

	Ok(())
}

#[tracing::instrument(skip_all)]
async fn handle(client: chirp_client::Client, payload: PlanResult, payload_json: String) {
	match handle_inner(client, &payload, payload_json).await {
		Ok(_) => {}
		Err(err) => {
			tracing::error!(?err, ?payload, "error handling event");
		}
	}
}
async fn handle_inner(
	client: chirp_client::Client,
	PlanResult { allocation: alloc }: &PlanResult,
	payload_json: String,
) -> GlobalResult<()> {
	let job_id = unwrap_ref!(alloc.job_id, "alloc has no job id");

	if !util_job::is_nomad_job_run(job_id) {
		tracing::info!(%job_id, "disregarding event");
		return Ok(());
	}

	msg!([client] job_run::msg::nomad_monitor_alloc_plan(job_id) {
		dispatched_job_id: job_id.clone(),
		payload_json: payload_json,
	})
	.await?;

	Ok(())
}
