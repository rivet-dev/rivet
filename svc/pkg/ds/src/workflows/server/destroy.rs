use chirp_workflow::prelude::*;
use futures_util::FutureExt;
use serde_json::json;

use crate::util::{signal_allocation, NOMAD_CONFIG};

#[message("ds_server_destroy_started")]
pub struct DestroyStarted {}

#[message("ds_server_destroy_complete")]
pub struct DestroyComplete {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Input {
	pub server_id: Uuid,
	pub override_kill_timeout_ms: Option<i64>,
}

#[workflow]
pub(crate) async fn ds_server_destroy(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let dynamic_server = ctx
		.activity(UpdateDbInput {
			server_id: input.server_id,
		})
		.await?;

	ctx.msg(
		json!({
			"server_id": input.server_id,
		}),
		DestroyStarted {},
	)
	.await?;

	if let Some(job_id) = &dynamic_server.dispatched_job_id {
		let delete_output = ctx
			.activity(DeleteJobInput {
				job_id: job_id.clone(),
			})
			.await?;

		if delete_output.job_exists {
			if let Some(alloc_id) = &dynamic_server.alloc_id {
				ctx.sleep(
					input
						.override_kill_timeout_ms
						.unwrap_or(dynamic_server.kill_timeout_ms),
				)
				.await?;

				ctx.activity(KillAllocInput {
					alloc_id: alloc_id.clone(),
				})
				.await?;
			}
		}
	}

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
	datacenter_id: Uuid,
	kill_timeout_ms: i64,
	dispatched_job_id: Option<String>,
	alloc_id: Option<String>,
}

#[activity(UpdateDb)]
async fn update_db(ctx: &ActivityCtx, input: &UpdateDbInput) -> GlobalResult<UpdateDbOutput> {
	// Run in transaction for internal retryability
	let db_output = rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
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
					s1.datacenter_id,
					s1.kill_timeout_ms,
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
	.await?;

	// NOTE: This call is infallible because redis is infallible. If it was not, it would be put in its own
	// workflow step
	// Invalidate cache when server is destroyed
	ctx.cache()
		.purge("servers_ports", [db_output.datacenter_id])
		.await?;

	Ok(db_output)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct DeleteJobInput {
	job_id: String,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct DeleteJobOutput {
	job_exists: bool,
}

#[activity(DeleteJob)]
async fn delete_job(ctx: &ActivityCtx, input: &DeleteJobInput) -> GlobalResult<DeleteJobOutput> {
	// TODO: Handle 404 safely. See RVTEE-498
	// Stop the job.
	//
	// Setting purge to false will change the behavior of the create poll
	// functionality if the job dies immediately. You can set it to false to
	// debug lobbies, but it's preferred to extract metadata from the
	// job-run-stop lifecycle event.

	match nomad_client::apis::jobs_api::delete_job(
		&NOMAD_CONFIG,
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
			Ok(DeleteJobOutput { job_exists: true })
		}
		Err(err) => {
			tracing::warn!(?err, "error thrown while stopping job");
			Ok(DeleteJobOutput { job_exists: false })
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct KillAllocInput {
	alloc_id: String,
}

/// Kills the server's allocation after 30 seconds
///
/// See `docs/packages/job/JOB_DRAINING_AND_KILL_TIMEOUTS.md`
#[activity(KillAlloc)]
async fn kill_alloc(ctx: &ActivityCtx, input: &KillAllocInput) -> GlobalResult<()> {
	// TODO: Handle 404 safely. See RVTEE-498
	if let Err(err) = signal_allocation(
		&NOMAD_CONFIG,
		&input.alloc_id,
		None,
		Some(super::NOMAD_REGION),
		None,
		None,
		Some(nomad_client_old::models::AllocSignalRequest {
			task: None,
			signal: Some("SIGKILL".to_string()),
		}),
	)
	.await
	{
		tracing::warn!(
			?err,
			"error while trying to manually kill allocation, ignoring"
		);
	}

	Ok(())
}
