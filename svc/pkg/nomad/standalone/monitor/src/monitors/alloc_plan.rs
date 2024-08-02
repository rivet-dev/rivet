use chirp_workflow::prelude::*;
use rivet_operation::prelude::proto::backend::pkg::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlanResult {
	allocation: nomad_client::models::Allocation,
}

pub async fn handle(
	ctx: StandaloneCtx,
	PlanResult { allocation: alloc }: &PlanResult,
	payload_json: String,
) -> GlobalResult<()> {
	let job_id = unwrap_ref!(alloc.job_id, "alloc has no job id");

	if !util_job::is_nomad_job_run(job_id) {
		tracing::info!(%job_id, "disregarding event");
		return Ok(());
	}

	msg!([ctx] nomad::msg::monitor_alloc_plan(job_id) {
		dispatched_job_id: job_id.clone(),
		payload_json: payload_json,
	})
	.await?;

	Ok(())
}
