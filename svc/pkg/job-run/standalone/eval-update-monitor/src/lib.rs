use rivet_operation::prelude::*;
use serde::Deserialize;

use proto::backend::pkg::*;

lazy_static::lazy_static! {
	static ref NOMAD_CONFIG: nomad_client::apis::configuration::Configuration =
		nomad_util::config_from_env().unwrap();
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PlanResult {
	evaluation: nomad_client::models::Evaluation,
}

#[tracing::instrument(skip_all)]
pub async fn start(
	shared_client: chirp_client::SharedClientHandle,
	redis_job: RedisPool,
) -> GlobalResult<()> {
	let redis_index_key = format!(
		"nomad:monitor_index:job_run_eval_update_monitor:{}",
		shared_client.region()
	);

	let configuration = nomad_util::config_from_env().unwrap();
	nomad_util::monitor::Monitor::run(
		configuration,
		redis_job,
		&redis_index_key,
		&["Evaluation"],
		move |event| {
			let client = shared_client
				.clone()
				.wrap_new("job-run-eval-update-monitor");
			async move {
				if let Some(payload) = event
					.decode::<PlanResult>("Evaluation", "EvaluationUpdated")
					.unwrap()
				{
					// We can't decode this with serde, so manually deserialize the response
					let spawn_res = tokio::task::Builder::new()
						.name("job_run_eval_update_monitor::handle_event")
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
	PlanResult { evaluation: eval }: &PlanResult,
	payload_json: String,
) -> GlobalResult<()> {
	let job_id = internal_unwrap!(eval.job_id, "eval has no job id");
	let triggered_by = internal_unwrap!(eval.triggered_by).as_str();
	let eval_status_raw = internal_unwrap!(eval.status).as_str();

	// Ignore jobs we don't care about
	if !util_job::is_nomad_job_run(job_id) || triggered_by != "job-register" {
		tracing::info!(%job_id, "disregarding event");
		return Ok(());
	}

	// Ignore statuses we don't care about
	if eval_status_raw != "complete" {
		tracing::info!(
			%job_id,
			?eval_status_raw,
			"ignoring status"
		);
		return Ok(());
	}

	msg!([client] job_run::msg::nomad_monitor_eval_update(job_id) {
		dispatched_job_id: job_id.clone(),
		payload_json: payload_json,
	})
	.await?;

	Ok(())
}
