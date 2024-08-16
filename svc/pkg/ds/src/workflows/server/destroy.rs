use chirp_workflow::prelude::*;
use futures_util::FutureExt;
use serde_json::json;

use crate::util::NEW_NOMAD_CONFIG;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Input {
	pub server_id: Uuid,
	pub override_kill_timeout_ms: i64,
}

#[workflow]
pub(crate) async fn ds_server_destroy(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let dynamic_server = ctx
		.activity(UpdateDbInput {
			server_id: input.server_id,
		})
		.await?;

	ctx.activity(DeleteJobInput {
		job_id: dynamic_server.dispatched_job_id.clone(),
	})
	.await?;

	ctx.msg(
		json!({
			"server_id": input.server_id,
		}),
		DestroyComplete {},
	)
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateDbInput {
	server_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, Hash, sqlx::FromRow)]
struct UpdateDbOutput {
	server_id: Uuid,
	datacenter_id: Uuid,
	dispatched_job_id: String,
	alloc_id: String,
}

#[activity(UpdateDb)]
async fn update_db(ctx: &ActivityCtx, input: &UpdateDbInput) -> GlobalResult<UpdateDbOutput> {
	// Run in transaction for internal retryability
	rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();
		let server_id = input.server_id;

		async move {
			sql_fetch_one!(
				[ctx, UpdateDbOutput, @tx tx]
				"
				UPDATE db_ds.servers AS s1
				SET destroy_ts = $2
				FROM db_ds.servers AS s2
				JOIN db_ds.server_nomad AS sn
				ON s2.server_id = sn.server_id
				WHERE
					s1.server_id = $1 AND
					s1.server_id = s2.server_id AND
					s2.destroy_ts IS NULL
				RETURNING
					s1.server_id,
					s1.datacenter_id,
					sn.nomad_dispatched_job_id AS dispatched_job_id,
					sn.nomad_alloc_id AS alloc_id
				",
				server_id,
				ctx.ts(),
			)
			.await
		}
		.boxed()
	})
	.await
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct DeleteJobInput {
	job_id: String,
}

#[activity(DeleteJob)]
async fn delete_job(ctx: &ActivityCtx, input: &DeleteJobInput) -> GlobalResult<()> {
	// TODO: Handle 404 safely. See RIV-179
	// Stop the job.
	//
	// Setting purge to false will change the behavior of the create poll
	// functionality if the job dies immediately. You can set it to false to
	// debug lobbies, but it's preferred to extract metadata from the
	// job-run-stop lifecycle event.

	match nomad_client::apis::jobs_api::delete_job(
		&NEW_NOMAD_CONFIG,
		&input.job_id,
		Some(super::NOMAD_REGION),
		None,
		None,
		None,
		Some(false), // TODO: Maybe change back to true for performance?
		None,
	)
	.await
	{
		Ok(_) => {
			tracing::info!("job stopped");

			// tokio::task::spawn(async move {

			// });
		}
		Err(err) => {
			tracing::warn!(?err, "error thrown while stopping job");
		}
	}

	Ok(())
}

#[message("ds_server_destroy_complete")]
pub struct DestroyComplete {}
