use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use redis::AsyncCommands;
use tokio::task;

#[derive(Debug, sqlx::FromRow)]
struct RunRow {
	region_id: Uuid,
	create_ts: i64,
	stop_ts: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct RunMetaNomadRow {
	alloc_id: Option<String>,
	dispatched_job_id: Option<String>,
}

use crate::{
	util::signal_allocation,
	workers::{NEW_NOMAD_CONFIG, NOMAD_CONFIG},
};

// Update timeout to give time for the timeout in `kill_allocation`
#[worker(name = "job-run-stop", timeout = 90)]
async fn worker(ctx: &OperationContext<job_run::msg::stop::Message>) -> GlobalResult<()> {
	// NOTE: Idempotent

	let run_id = unwrap_ref!(ctx.run_id).as_uuid();

	// Cleanup the job ASAP.
	//
	// This will also be called in `job-run-cleanup`, but this is idempotent.
	msg!([ctx] job_run::msg::cleanup(run_id) {
		run_id: Some(run_id.into()),
		..Default::default()
	})
	.await?;

	let Some((run_row, run_meta_nomad_row)) =
		rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
			Box::pin(update_db(ctx.clone(), ctx.ts(), run_id, tx))
		})
		.await?
	else {
		if ctx.req_dt() > util::duration::minutes(5) {
			tracing::error!("discarding stale message");
			return Ok(());
		} else {
			retry_bail!("run not found, may be race condition with insertion");
		}
	};

	// HACK: Remove from proxied ports early. This also gets removed in job-run-cleanup, but that
	// may not run correclty if the dispatched job id is not set correctly.
	ctx.redis_job()
		.await?
		.hdel(
			util_job::key::proxied_ports(run_row.region_id),
			run_id.to_string(),
		)
		.await?;

	// Get the region
	let region_res = op!([ctx] region_get {
		region_ids: vec![run_row.region_id.into()],
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
	if let Some(RunMetaNomadRow {
		alloc_id,
		dispatched_job_id: Some(dispatched_job_id),
	}) = &run_meta_nomad_row
	{
		match nomad_client_new::apis::jobs_api::delete_job(
			&NEW_NOMAD_CONFIG,
			dispatched_job_id,
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

				if !ctx.skip_kill_alloc {
					if let Some(alloc_id) = alloc_id {
						kill_allocation(region.nomad_region.clone(), alloc_id.clone());
					}
				}
			}
			Err(err) => {
				tracing::warn!(?err, "error thrown while stopping job, probably a 404, will continue as if stopped normally");
			}
		}
	}

	Ok(())
}

#[tracing::instrument(skip_all)]
async fn update_db(
	ctx: OperationContext<job_run::msg::stop::Message>,
	now: i64,
	run_id: Uuid,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> GlobalResult<Option<(RunRow, Option<RunMetaNomadRow>)>> {
	let run_row = sql_fetch_optional!(
		[ctx,  RunRow, @tx tx]
		"
		SELECT region_id, create_ts, stop_ts
		FROM db_job_state.runs
		WHERE run_id = $1
		FOR UPDATE
		",
		run_id,
	)
	.await?;
	tracing::info!(?run_row, "fetched run");

	let Some(run_row) = run_row else {
		return Ok(None);
	};

	let run_meta_nomad_row = sql_fetch_optional!(
		[ctx, RunMetaNomadRow, @tx tx]
		"
		SELECT alloc_id, dispatched_job_id
		FROM db_job_state.run_meta_nomad
		WHERE run_id = $1
		FOR UPDATE
		",
		run_id,
	)
	.await?;
	tracing::info!(?run_meta_nomad_row, "fetched run meta nomad");

	// Check if job has been dispatched already
	if let Some(run_meta_nomad) = &run_meta_nomad_row {
		if run_meta_nomad.dispatched_job_id.is_none()
			&& now - run_row.create_ts < util::duration::seconds(75)
		{
			// If the job is new, then there may be a race condition with
			// submitting the job to Nomad and writing the dispatched job ID to
			// the database.
			//
			// In this case, we'll fail and retry this later.
			//
			// There is a situation where the Nomad API returns an error and the
			// job ID is never written to the database.
			retry_bail!("potential race condition with starting nomad job")
		}
	}

	// We can't assume that started has been called here, so we can't fetch the alloc ID.

	if run_row.stop_ts.is_none() {
		sql_execute!(
			[ctx, @tx tx]
			"UPDATE db_job_state.runs SET stop_ts = $2 WHERE run_id = $1",
			run_id,
			now,
		)
		.await?;
	}

	Ok(Some((run_row, run_meta_nomad_row)))
}

/// Kills the allocation after 30 seconds
///
/// See `docs/packages/job/JOB_DRAINING_AND_KILL_TIMEOUTS.md`
fn kill_allocation(nomad_region: String, alloc_id: String) {
	task::spawn(async move {
		tokio::time::sleep(util_job::JOB_STOP_TIMEOUT).await;

		tracing::info!(?alloc_id, "manually killing allocation");

		if let Err(err) = signal_allocation(
			&NOMAD_CONFIG,
			&alloc_id,
			None,
			Some(&nomad_region),
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
	});
}
