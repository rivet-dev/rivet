// This service catches two edge cases:
//
// # Case A: Nomad outage
//
// In the situation that nomad-monitor fails to receive a Nomad event (i.e.
// node migration, the job failed, or Nomad failed), there will be jobs
// where the Nomad job did not dispatch a stop event, causing the job to be
// orphaned.
//
// # Case B: Matchmaker inconsistency
//
// If the matchmaker lobbies are stopped but fail to stop the Nomad job, this will detect that and
// stop the stop automatically.

use std::collections::HashSet;

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

pub async fn start() -> GlobalResult<()> {
	// TODO: Handle ctrl-c

	let pools = rivet_pools::from_env().await?;

	let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 15));
	loop {
		interval.tick().await;

		run_from_env(util::timestamp::now(), pools.clone()).await?;
	}
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(ts: i64, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let check_orphaned_ts = ts - CHECK_ORPHANED_JOB_THRESHOLD;

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
	);

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
	let runs = sql_fetch_all!(
		[ctx, (Uuid, Option<String>), &crdb]
		"
		SELECT runs.run_id, run_meta_nomad.dispatched_job_id
		FROM db_job_state.runs
		INNER JOIN db_job_state.run_meta_nomad ON run_meta_nomad.run_id = runs.run_id
		AS OF SYSTEM TIME '-5s'
		WHERE stop_ts IS NULL AND create_ts < $1
		",
		check_orphaned_ts,
	)
	.await?;

	// List of all run IDs that are still running by the time the gc finishes
	let mut running_run_ids = runs.iter().map(|x| x.0).collect::<HashSet<Uuid>>();

	let total_potentially_orphaned = runs.len();
	let mut orphaned_nomad_job = 0;
	let mut orphaned_mm_lobby = 0;
	let mut oprhaned_mm_lobby_not_found = 0;
	let mut no_dispatched_job_id = 0;

	// Check for orphaned Nomad jobs
	for (run_id, dispatched_job_id) in runs {
		let dispatched_job_id = if let Some(x) = dispatched_job_id {
			x
		} else {
			tracing::warn!(%run_id, "dispatched job id not found");
			no_dispatched_job_id += 1;
			continue;
		};

		if !job_ids_from_nomad.contains(&dispatched_job_id) {
			orphaned_nomad_job += 1;
			running_run_ids.remove(&run_id);

			tracing::warn!(%run_id, "stopping orphaned run from nomad job");
			msg!([ctx] @wait job_run::msg::stop(run_id) {
				run_id: Some(run_id.into()),
				..Default::default()
			})
			.await?;
		}
	}

	// Check for matchmaker orphans
	let run_lobbies = op!([ctx] mm_lobby_for_run_id {
		run_ids: running_run_ids.iter().cloned().map(Into::into).collect(),
	})
	.await?;
	let lobbies = op!([ctx] mm_lobby_get {
		lobby_ids: run_lobbies.lobbies.iter().flat_map(|x| x.lobby_id).collect(),
		include_stopped: true,
	})
	.await?;
	for run_id in running_run_ids.clone() {
		let lobby = lobbies
			.lobbies
			.iter()
			.find(|x| x.run_id == Some(run_id.into()));

		if let Some(lobby) = lobby {
			if lobby.stop_ts.is_some() {
				orphaned_mm_lobby += 1;
				running_run_ids.remove(&run_id);

				tracing::warn!(%run_id, "stopping orphaned run from matchmaker lobby stopped");
				msg!([ctx] @wait job_run::msg::stop(run_id) {
					run_id: Some(run_id.into()),
					..Default::default()
				})
				.await?;
			}
		} else {
			oprhaned_mm_lobby_not_found += 1;
			running_run_ids.remove(&run_id);

			// HACK: This is only true in production. This will have false positives with tests.
			tracing::warn!(%run_id, "stopping orphaned run from matchmaker lobby not found");
			msg!([ctx] @wait job_run::msg::stop(run_id) {
				run_id: Some(run_id.into()),
				..Default::default()
			})
			.await?;
		}
	}

	tracing::info!(
		?total_potentially_orphaned,
		?orphaned_nomad_job,
		?orphaned_mm_lobby,
		?oprhaned_mm_lobby_not_found,
		?no_dispatched_job_id,
		still_running = ?running_run_ids.len(),
		"finished"
	);

	Ok(())
}
