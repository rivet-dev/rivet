use chirp_worker::prelude::*;
use futures_util::FutureExt;
use proto::backend::pkg::*;
use tokio::task;

#[derive(Debug, sqlx::FromRow)]
struct UpdatedServer {
	ds_server_id: Uuid,
	ds_datacenter_id: Uuid,
	alloc_id: String,
	dispatched_job_id: String,
}

#[operation(name = "ds-server-delete")]
pub async fn handle(
	ctx: OperationContext<dynamic_servers::server_delete::Request>,
) -> GlobalResult<dynamic_servers::server_delete::Response> {
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();

	let dynamic_server = rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();

		async move {
			let dynamic_server = sql_fetch_one!(
				[ctx, UpdatedServer, @tx tx]
				"
				UPDATE db_ds.servers
				SET delete_ts = $2
				WHERE
					server_id = $1
					AND delete_ts IS NULL
				RETURNING
					server_id,
					datacenter_id
					server_nomad.nomad_dispatched_job_id,
					server_nomad.nomad_alloc_id,
				FROM
					db_ds.servers
				JOIN
					db_ds.server_nomad
				ON
					db_ds.servers.server_id = db_ds.server_nomad.server_id
				",
				server_id,
				ctx.ts(),
			)
			.await?;

			Ok(dynamic_server)
		}
		.boxed()
	})
	.await?;

	// // NOTE: Idempotent

	// let run_id = unwrap_ref!(ctx.run_id).as_uuid();

	// // Cleanup the job ASAP.
	// //
	// // This will also be called in `job-run-cleanup`, but this is idempotent.
	// // msg!([ctx] job_run::msg::cleanup(run_id) {
	// // 	run_id: Some(run_id.into()),
	// // 	..Default::default()
	// // })
	// // .await?;

	// let run_id = unwrap_ref!(ctx.run_id).as_uuid();

	// #[derive(Debug, sqlx::FromRow)]
	// struct RunRow {
	// 	region_id: Uuid,
	// 	create_ts: i64,
	// 	cleanup_ts: Option<i64>,
	// }

	// let Some((run_row, run_meta_nomad_row)) =
	// 	rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
	// 		let run_row = sql_fetch_optional!(
	// 			[ctx, RunRow, @tx tx]
	// 			"
	// 			SELECT region_id, create_ts, cleanup_ts
	// 			FROM db_job_state.runs
	// 			WHERE run_id = $1
	// 			FOR UPDATE
	// 			",
	// 			run_id,
	// 		)
	// 		.await?;
	// 		tracing::info!(?run_row, "run row");

	// 		let Some(run_row) = run_row else {
	// 			return Ok(None);
	// 		};

	// 		let run_meta_nomad_row = sql_fetch_optional!(
	// 			[ctx, RunMetaNomadRow, @tx tx]
	// 			"
	// 			SELECT dispatched_job_id, node_id
	// 			FROM db_job_state.run_meta_nomad
	// 			WHERE run_id = $1
	// 			FOR UPDATE
	// 			",
	// 			run_id,
	// 		)
	// 		.await?;
	// 		tracing::info!(?run_meta_nomad_row, "run meta row");

	// 		// Check if job has been dispatched already
	// 		if let Some(run_meta_nomad) = &run_meta_nomad_row {
	// 			if run_meta_nomad.dispatched_job_id.is_none()
	// 				&& now - run_row.create_ts < util::duration::seconds(75)
	// 			{
	// 				// If the job is new, then there may be a race condition with
	// 				// submitting the job to Nomad and writing the dispatched job ID to
	// 				// the database.
	// 				//
	// 				// In this case, we'll fail and retry this later.
	// 				//
	// 				// There is a situation where the Nomad API returns an error and the
	// 				// job ID is never written to the database.
	// 				retry_bail!("potential race condition with starting nomad job")
	// 			}
	// 		}

	// 		tracing::info!("deleting run");
	// 		if run_row.cleanup_ts.is_none() {
	// 			sql_execute!(
	// 				[ctx, @tx tx]
	// 				"UPDATE db_job_state.runs SET cleanup_ts = $2 WHERE run_id = $1",
	// 				run_id,
	// 				now,
	// 			)
	// 			.await?;
	// 		}
	// 	})
	// 	.await?
	// else {
	// 	if ctx.req_dt() > util::duration::minutes(5) {
	// 		tracing::error!("discarding stale message");
	// 		return Ok(());
	// 	} else {
	// 		retry_bail!("run not found, may be race condition with insertion");
	// 	}
	// };

	// tracing::info!("removing from cache");
	// if matches!(
	// 	run_meta_nomad_row,
	// 	Some(RunMetaNomadRow {
	// 		node_id: Some(_),
	// 		..
	// 	})
	// ) {
	// 	ctx.redis_job()
	// 		.await?
	// 		.hdel(
	// 			util_job::key::proxied_ports(run_row.region_id),
	// 			run_id.to_string(),
	// 		)
	// 		.await?;
	// }

	// let Some((run_row, run_meta_nomad_row)) =
	// 	rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
	// 		let run_row = sql_fetch_optional!(
	// 			[ctx,  RunRow, @tx tx]
	// 			"
	// 			SELECT region_id, create_ts, stop_ts
	// 			FROM db_job_state.runs
	// 			WHERE run_id = $1
	// 			FOR UPDATE
	// 			",
	// 			run_id,
	// 		)
	// 		.await?;
	// 		tracing::info!(?run_row, "fetched run");

	// 		let Some(run_row) = run_row else {
	// 			return Ok(None);
	// 		};

	// 		let run_meta_nomad_row = sql_fetch_optional!(
	// 			[ctx, RunMetaNomadRow, @tx tx]
	// 			"
	// 			SELECT alloc_id, dispatched_job_id
	// 			FROM db_job_state.run_meta_nomad
	// 			WHERE run_id = $1
	// 			FOR UPDATE
	// 			",
	// 			run_id,
	// 		)
	// 		.await?;
	// 		tracing::info!(?run_meta_nomad_row, "fetched run meta nomad");

	// 		// Check if job has been dispatched already
	// 		if let Some(run_meta_nomad) = &run_meta_nomad_row {
	// 			if run_meta_nomad.dispatched_job_id.is_none()
	// 				&& now - run_row.create_ts < util::duration::seconds(75)
	// 			{
	// 				// If the job is new, then there may be a race condition with
	// 				// submitting the job to Nomad and writing the dispatched job ID to
	// 				// the database.
	// 				//
	// 				// In this case, we'll fail and retry this later.
	// 				//
	// 				// There is a situation where the Nomad API returns an error and the
	// 				// job ID is never written to the database.
	// 				retry_bail!("potential race condition with starting nomad job")
	// 			}
	// 		}

	// 		// We can't assume that started has been called here, so we can't fetch the alloc ID.

	// 		if run_row.stop_ts.is_none() {
	// 			sql_execute!(
	// 				[ctx, @tx tx]
	// 				"UPDATE db_job_state.runs SET stop_ts = $2 WHERE run_id = $1",
	// 				run_id,
	// 				now,
	// 			)
	// 			.await?;
	// 		}
	// 	})
	// 	.await?
	// else {
	// 	if ctx.req_dt() > util::duration::minutes(5) {
	// 		tracing::error!("discarding stale message");
	// 		return Ok(());
	// 	} else {
	// 		retry_bail!("run not found, may be race condition with insertion");
	// 	}
	// };

	// // HACK: Remove from proxied ports early. This also gets removed in job-run-cleanup, but that
	// // may not run correclty if the dispatched job id is not set correctly.
	// ctx.redis_job()
	// 	.await?
	// 	.hdel(
	// 		util_job::key::proxied_ports(run_row.region_id),
	// 		run_id.to_string(),
	// 	)
	// 	.await?;

	// Get the region
	let region_res = op!([ctx] region_get {
		region_ids: vec![dynamic_server.ds_datacenter_id.into()],
	})
	.await?;
	let region = unwrap!(region_res.regions.first());

	// TODO: Handle 404 safely. See RIV-179
	// Stop the job.
	//
	// Setting purge to false will change the behavior of the create poll
	// functionality if the job dies immediately. You can set it to false to
	// debug lobbies, but it's preferred to extract metadata from the
	// job-run-stop lifecycle event.

	match nomad_client::apis::jobs_api::delete_job(
		&nomad_util::new_config_from_env().unwrap(),
		&dynamic_server.dispatched_job_id,
		Some(&region.nomad_region),
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

			task::spawn(async move {
				// tokio::time::sleep(util_job::JOB_STOP_TIMEOUT).await;

				// tracing::info!(?dynamic_server.alloc_id, "manually killing allocation");

				// if let Err(err) = {
				// 	let local_var_client = &configuration.client;

				// 	let local_var_uri_str = format!(
				// 		"{}/client/allocation/{alloc_id}/signal",
				// 		configuration.base_path,
				// 		alloc_id = nomad_client::apis::urlencode(dynamic_server.alloc_id),
				// 	);
				// 	let mut local_var_req_builder =
				// 		local_var_client.post(local_var_uri_str.as_str());

				// 	if let Some(ref local_var_str) = namespace {
				// 		local_var_req_builder = local_var_req_builder
				// 			.query(&[("namespace", &local_var_str.to_string())]);
				// 	}
				// 	if let Some(ref local_var_str) = region {
				// 		local_var_req_builder = local_var_req_builder
				// 			.query(&[("region", &local_var_str.to_string())]);
				// 	}
				// 	if let Some(ref local_var_str) = index {
				// 		local_var_req_builder = local_var_req_builder
				// 			.query(&[("index", &local_var_str.to_string())]);
				// 	}
				// 	if let Some(ref local_var_str) = wait {
				// 		local_var_req_builder = local_var_req_builder
				// 			.query(&[("wait", &local_var_str.to_string())]);
				// 	}
				// 	if let Some(ref local_var_user_agent) = configuration.user_agent {
				// 		local_var_req_builder = local_var_req_builder
				// 			.header(reqwest::header::USER_AGENT, local_var_user_agent.clone());
				// 	}
				// 	local_var_req_builder = local_var_req_builder.json(&alloc_signal_request);

				// 	let local_var_req = local_var_req_builder.build()?;
				// 	let local_var_resp = local_var_client.execute(local_var_req).await?;

				// 	let local_var_status = local_var_resp.status();
				// 	let local_var_content = local_var_resp.text().await?;

				// 	if !local_var_status.is_client_error()
				// 		&& !local_var_status.is_server_error()
				// 	{
				// 		Ok(())
				// 	} else {
				// 		let local_var_entity: Option<
				// 			nomad_client::apis::allocations_api::SignalAllocationError,
				// 		> = serde_json::from_str(&local_var_content).ok();
				// 		let local_var_error = nomad_client::apis::ResponseContent {
				// 			status: local_var_status,
				// 			content: local_var_content,
				// 			entity: local_var_entity,
				// 		};
				// 		Err(nomad_client::apis::Error::ResponseError(local_var_error))
				// 	}
				// } {
				// 	tracing::warn!(
				// 		?err,
				// 		?alloc_id,
				// 		"error while trying to manually kill allocation"
				// 	);
				// }
			});
		}
		Err(err) => {
			tracing::warn!(?err, "error thrown while stopping job, probably a 404, will continue as if stopped normally");
		}
	}

	Ok(dynamic_servers::server_delete::Response {
		server_id: Some(server_id.into()),
	})
}
