use std::collections::HashSet;

use futures_util::stream::StreamExt;
use indoc::indoc;
use proto::backend::pkg::*;
use rivet_operation::prelude::*;

lazy_static::lazy_static! {
	static ref NOMAD_CONFIG: nomad_client::apis::configuration::Configuration =
		nomad_util::config_from_env().unwrap();
}

/// How long after a job is submitted before we begin checking it against the
/// known jobs.
pub const CHECK_ORPHANED_JOB_THRESHOLD: i64 = util::duration::hours(1);

#[tracing::instrument(skip_all)]
pub async fn run_from_env(ts: i64, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let check_orphaned_ts = ts - CHECK_ORPHANED_JOB_THRESHOLD;

	let pools = rivet_pools::from_env("job-gc").await?;
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("job-gc");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"job-gc".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		ts,
		ts,
		(),
		Vec::new(),
	);

	// In the situation that nomad-monitor fails to receive a Nomad event (i.e.
	// node migration, the job failed, or Nomad failed), there will be jobs
	// where the Nomad job did not dispatch a stop event, causing the job to be
	// orphaned.

	// Find jobs to stop.
	let job_stubs =
		nomad_client::apis::jobs_api::get_jobs(&NOMAD_CONFIG, None, None, None, None, Some("job-"))
			.await?;
	let job_ids_from_nomad = job_stubs
		.into_iter()
		.filter(|job| {
			// Validate that this is a dispatched job
			job.parent_id.is_some() &&
				job.parameterized_job == Some(false) &&
				// Job is running
				job.status.as_deref() == Some("running") &&
				// Check if job is beyond the threshold
				job.submit_time.map_or(true, |x| {
					(x / 1_000_000) < check_orphaned_ts
			   })
		})
		.filter_map(|job| job.ID)
		.collect::<HashSet<String>>();
	tracing::info!(count = ?job_ids_from_nomad.len(), "fetched jobs");

	// Paginate over all runs to look for orphans
	//
	// We use stale reads without locks since job-run-stop is idempotent.
	let crdb = ctx.crdb().await?;
	let mut runs_iter = sql_fetch!(
		[ctx, (Uuid, Option<i64>, Option<String>), &crdb]
		"
		SELECT runs.run_id, runs.stop_ts, run_meta_nomad.dispatched_job_id
		FROM db_job_state.runs
		INNER JOIN db_job_state.run_meta_nomad ON run_meta_nomad.run_id = runs.run_id
		AS OF SYSTEM TIME '-5s'
		WHERE stop_ts IS NULL AND start_ts < $1
		",
		check_orphaned_ts,
	);
	while let Some(res) = runs_iter.next().await {
		let (run_id, stop_ts, dispatched_job_id) = res?;

		if stop_ts.is_some() {
			continue;
		}

		tracing::info!(%run_id, "checking for orphaned runs");

		let dispatched_job_id = if let Some(x) = dispatched_job_id {
			x
		} else {
			tracing::warn!(%run_id, "dispatched job id not found");
			continue;
		};

		if !job_ids_from_nomad.contains(&dispatched_job_id) {
			tracing::warn!(%run_id, "stopping orphaned run");
			msg!([ctx] @wait job_run::msg::stop(run_id) {
				run_id: Some(run_id.into()),
				..Default::default()
			})
			.await?;
		}
	}

	tracing::info!("finished");

	Ok(())
}
