use std::convert::TryInto;

use chirp_workflow::prelude::*;
use futures_util::FutureExt;
use serde_json::json;

use crate::util::NEW_NOMAD_CONFIG;

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
				ctx.activity(KillAllocInput {
					alloc_id: alloc_id.clone(),
					kill_timeout_ms: input
						.override_kill_timeout_ms
						.unwrap_or(dynamic_server.kill_timeout_ms),
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
	server_id: Uuid,
	datacenter_id: Uuid,
	kill_timeout_ms: i64,
	dispatched_job_id: Option<String>,
	alloc_id: Option<String>,
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
	.await
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
	kill_timeout_ms: i64,
}

/// Kills the server's allocation after 30 seconds
///
/// See `docs/packages/job/JOB_DRAINING_AND_KILL_TIMEOUTS.md`
#[activity(KillAlloc)]
#[timeout = 45]
async fn kill_alloc(ctx: &ActivityCtx, input: &KillAllocInput) -> GlobalResult<()> {
	// TODO: Move this to a workflow sleep RVTEE-497
	tokio::time::sleep(std::time::Duration::from_millis(
		input.kill_timeout_ms.try_into()?,
	))
	.await;

	// TODO: Handle 404 safely. See RVTEE-498
	if let Err(err) = signal_allocation(
		&NEW_NOMAD_CONFIG,
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

// Have to patch `nomad_client::apis::allocations_api::signal_allocation` because it uses `/allocation`
// instead of `/client/allocation`
async fn signal_allocation(
	configuration: &nomad_client::apis::configuration::Configuration,
	alloc_id: &str,
	namespace: Option<&str>,
	region: Option<&str>,
	index: Option<i64>,
	wait: Option<&str>,
	alloc_signal_request: Option<nomad_client_old::models::AllocSignalRequest>,
) -> Result<
	(),
	nomad_client::apis::Error<nomad_client_old::apis::allocations_api::SignalAllocationError>,
> {
	let local_var_client = &configuration.client;

	let local_var_uri_str = format!(
		"{}/client/allocation/{alloc_id}/signal",
		configuration.base_path,
		alloc_id = nomad_client::apis::urlencode(alloc_id),
	);
	let mut local_var_req_builder = local_var_client.post(local_var_uri_str.as_str());

	if let Some(ref local_var_str) = namespace {
		local_var_req_builder =
			local_var_req_builder.query(&[("namespace", &local_var_str.to_string())]);
	}
	if let Some(ref local_var_str) = region {
		local_var_req_builder =
			local_var_req_builder.query(&[("region", &local_var_str.to_string())]);
	}
	if let Some(ref local_var_str) = index {
		local_var_req_builder =
			local_var_req_builder.query(&[("index", &local_var_str.to_string())]);
	}
	if let Some(ref local_var_str) = wait {
		local_var_req_builder =
			local_var_req_builder.query(&[("wait", &local_var_str.to_string())]);
	}
	if let Some(ref local_var_user_agent) = configuration.user_agent {
		local_var_req_builder =
			local_var_req_builder.header(http::header::USER_AGENT, local_var_user_agent.clone());
	}
	local_var_req_builder = local_var_req_builder.json(&alloc_signal_request);

	let local_var_req = local_var_req_builder.build()?;
	let local_var_resp = local_var_client.execute(local_var_req).await?;

	let local_var_status = local_var_resp.status();
	let local_var_content = local_var_resp.text().await?;

	if !local_var_status.is_client_error() && !local_var_status.is_server_error() {
		Ok(())
	} else {
		let local_var_entity: Option<
			nomad_client_old::apis::allocations_api::SignalAllocationError,
		> = serde_json::from_str(&local_var_content).ok();
		let local_var_error = nomad_client::apis::ResponseContent {
			status: local_var_status,
			content: local_var_content,
			entity: local_var_entity,
		};
		Err(nomad_client::apis::Error::ResponseError(local_var_error))
	}
}
