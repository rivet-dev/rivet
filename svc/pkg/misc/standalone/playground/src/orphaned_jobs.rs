// TODO: Migrate to CRDB

// use crate::proto::backend::svc::*;
// use chirp_client::prelude::*;
// use futures_util::StreamExt;
// // use std::collections::HashSet;
// use uuid::Uuid;

// use crate::NOMAD_CONFIG;

// pub const CHECK_ORPHANED_JOB_THRESHOLD: i64 = util::duration::minutes(5);

// #[derive(Debug, scylla::macros::FromRow)]
// struct Lobby {
// 	lobby_id: Option<Uuid>,
// 	run_id: Option<Uuid>,
// 	stop_ts: Option<chrono::Duration>,
// }

// #[derive(Debug, scylla::macros::FromRow)]
// struct Player {
// 	lobby_id: Option<Uuid>,
// 	remove_ts: Option<chrono::Duration>,
// }

// #[derive(Debug, scylla::macros::FromRow)]
// struct Run {
// 	run_id: Option<Uuid>,
// 	stop_ts: Option<chrono::Duration>,
// }

// #[derive(Debug, scylla::macros::FromRow)]
// struct RunKindMmLobby {
// 	run_id: Uuid,
// 	lobby_id: Uuid,
// }

// #[tracing::instrument(skip_all)]
// pub async fn run(pools: rivet_pools::Pools, client: chirp_client::Client) -> GlobalResult<()> {
// 	let check_orphaned_ts: i64 = util::timestamp::now() - CHECK_ORPHANED_JOB_THRESHOLD;

// 	let region_list_res = op!([client] region_list {
// 		..Default::default()
// 	})
// 	.await?;
// 	let regions_res = op!([client] region_get {
// 		region_ids: region_list_res.region_ids.clone(),
// 	})
// 	.await?;

// 	let mm_scylla = pools.scylla("db-mm")?;
// 	let job_scylla = pools.scylla("db-job-state")?;

// 	// Count lobbies
// 	tracing::info!("counting lobbies");
// 	let mut running_lobbies = HashSet::new();
// 	let mut running_lobby_runs = HashSet::new();
// 	let mut lobbies_iter = mm_scylla
// 		.query_iter(
// 			"SELECT lobby_id, run_id, stop_ts FROM lobbies ALLOW FILTERING",
// 			(),
// 		)
// 		.await?
// 		.into_typed::<Lobby>();
// 	while let Some(lobby) = lobbies_iter.next().await {
// 		let lobby = lobby?;
// 		if lobby.stop_ts.is_none() {
// 			if let (Some(lobby_id), Some(run_id)) = (lobby.lobby_id, lobby.run_id) {
// 				running_lobbies.insert(lobby_id);
// 				running_lobby_runs.insert(run_id);
// 			} else {
// 				tracing::warn!(?lobby, "malformed lobby");
// 			}
// 		}
// 	}
// 	tracing::info!(len = ?running_lobbies.len(), "running lobbies");

// 	// Count lobbies from active players
// 	// let mut running_player_lobbies = HashSet::new();
// 	// let mut players_iter = mm_scylla
// 	// 	.query_iter(
// 	// 		"SELECT lobby_id, remove_ts FROM players ALLOW FILTERING",
// 	// 		(),
// 	// 	)
// 	// 	.await?
// 	// 	.into_typed::<Player>();
// 	// while let Some(player) = players_iter.next().await {
// 	// 	let player = player?;
// 	// 	if player.remove_ts.is_none() {
// 	// 		running_player_lobbies.insert(player.lobby_id);
// 	// 	}
// 	// }
// 	// tracing::info!(len = ?running_player_lobbies.len(), "running player lobbies");

// 	// Count runs
// 	tracing::info!("counting runs");
// 	let mut all_runs = HashSet::new();
// 	let mut running_runs = HashSet::new();
// 	let mut runs_iter = job_scylla
// 		.query_iter("SELECT run_id, stop_ts FROM runs ALLOW FILTERING", ())
// 		.await?
// 		.into_typed::<Run>();
// 	while let Some(run) = runs_iter.next().await {
// 		let run = run?;
// 		all_runs.insert(run.run_id);
// 		if run.stop_ts.is_none() {
// 			if let Some(run_id) = run.run_id {
// 				running_runs.insert(run_id);
// 			} else {
// 				tracing::warn!(?run, "malformed run");
// 			}
// 		}
// 	}
// 	tracing::info!(len = ?running_runs.len(), "running runs");

// 	// Count run meta
// 	tracing::info!("counting run meta");
// 	let mut running_runs_for_lobby = HashSet::new();
// 	let mut runs_mm_lobby_iter = job_scylla
// 		.query_iter(
// 			"SELECT run_id, lobby_id FROM run_kind_mm_lobby ALLOW FILTERING",
// 			(),
// 		)
// 		.await?
// 		.into_typed::<RunKindMmLobby>();
// 	while let Some(run) = runs_mm_lobby_iter.next().await {
// 		let run = run?;
// 		if running_runs.contains(&run.run_id) {
// 			running_runs_for_lobby.insert(run.run_id);
// 		}
// 	}
// 	tracing::info!(len = ?running_runs_for_lobby.len(), "running runs for lobby");

// 	tracing::info!(len = ?(&running_lobby_runs - &running_runs_for_lobby).len(), "running lobbies without running run");
// 	tracing::info!(len = ?(&running_runs_for_lobby - &running_lobby_runs).len(), "running runs without running lobby");

// 	// Fetch all jobs
// 	tracing::info!("fetching all jobs");
// 	let mut valid_jobs = HashSet::new();
// 	let mut invalid_jobs = HashSet::new();
// 	for region in &regions_res.regions {
// 		let mut region_invalid_jobs = HashSet::new();

// 		let job_stubs = nomad_client::apis::jobs_api::get_jobs(
// 			&*NOMAD_CONFIG,
// 			None,
// 			Some(&region.nomad_region),
// 			None,
// 			None,
// 			Some("job-"),
// 		)
// 		.await?;
// 		let job_ids_from_nomad = job_stubs
// 			.into_iter()
// 			.filter(|job| {
// 				// Validate that this is a dispatched job
// 				job.parent_id.is_some() &&
// 				job.parameterized_job == Some(false) &&
// 				// Job is running
// 				job.status.as_ref().map(String::as_str) == Some("running") &&
// 				// Check if job is beyond the threshold
// 				job.submit_time.map_or(true, |x| {
// 					(x / 1_000_000) < check_orphaned_ts
// 			   })
// 			})
// 			.filter_map(|job| job.ID)
// 			.collect::<HashSet<String>>();

// 		// Compare with the database to see which runs don't exist
// 		for dispatched_job_id in &job_ids_from_nomad {
// 			let row = job_scylla
// 			.query(
// 				"SELECT run_id FROM run_meta_nomad_by_dispatched_job WHERE dispatched_job_id = ?",
// 				(&dispatched_job_id,),
// 			)
// 			.await?
// 			.maybe_first_row_typed::<(Uuid,)>()?;
// 			if let Some((run_id,)) = row {
// 				if running_runs.contains(&run_id) {
// 					valid_jobs.insert(dispatched_job_id.clone());
// 				} else {
// 					tracing::warn!(?dispatched_job_id, "job not running");
// 					invalid_jobs.insert(dispatched_job_id.clone());
// 					region_invalid_jobs.insert(dispatched_job_id.clone());
// 				}
// 			} else {
// 				tracing::warn!(?dispatched_job_id, "job not in database");
// 				invalid_jobs.insert(dispatched_job_id.clone());
// 				region_invalid_jobs.insert(dispatched_job_id.clone());
// 			}
// 		}

// 		tracing::info!(region = ?region.nomad_region, len = ?region_invalid_jobs.len(), "stopping jobs");
// 		for dispatched_job_id in &region_invalid_jobs {
// 			tracing::info!(?dispatched_job_id, "stopping");
// 			match nomad_client::apis::jobs_api::stop_job(
// 				&*NOMAD_CONFIG,
// 				&dispatched_job_id,
// 				None,
// 				Some(&region.nomad_region),
// 				None,
// 				None,
// 				Some(false),
// 			)
// 			.await
// 			{
// 				Ok(_) => tracing::info!("job stopped"),
// 				Err(err) => {
// 					tracing::warn!(?err, "error thrown while stopping job, probably a 404, will continue as if stopped normally");
// 				}
// 			}
// 		}
// 	}
// 	tracing::info!(invalid_len = ?invalid_jobs.len(), valid_len = ?valid_jobs.len(), "compared jobs");

// 	Ok(())
// }
