use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlanResult {
	evaluation: nomad_client::models::Evaluation,
}

pub async fn handle(
	client: chirp_client::Client,
	PlanResult { evaluation: eval }: &PlanResult,
	payload_json: String,
) -> GlobalResult<()> {
	let job_id = unwrap_ref!(eval.job_id, "eval has no job id");
	let triggered_by = unwrap_ref!(eval.triggered_by).as_str();
	let eval_status_raw = unwrap_ref!(eval.status).as_str();

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

	msg!([client] nomad::msg::monitor_eval_update(job_id) {
		dispatched_job_id: job_id.clone(),
		payload_json: payload_json,
	})
	.await?;

	Ok(())
}
