use api_helper::ctx::Ctx;
use proto::backend::pkg::*;
use rivet_job_server::models;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: POST /runs/cleanup
pub async fn cleanup(
	ctx: Ctx<Auth>,
	_body: models::CleanupRequest,
) -> GlobalResult<models::CleanupResponse> {
	let job_run_ent = ctx.auth().job_run()?;
	let run_id = job_run_ent.run_id;

	// job-run-cleanup is idempotent, so we call it here in addition to in
	// job-run-alloc-update-monitor in order to ensure the database is updated
	// over hell or high water
	msg!([ctx] job_run::msg::cleanup(run_id) {
		run_id: Some(run_id.into()),
		..Default::default()
	})
	.await?;

	Ok(models::CleanupResponse {})
}
