// TODO: Migrate to CRDB

// use crate::proto::backend::svc::*;
// use chirp_client::prelude::*;
// use futures_util::StreamExt;
// // // use std::collections::HashSet;
// use uuid::Uuid;

// #[derive(Debug, scylla::macros::FromRow)]
// struct LobbyHistoryRow {
// 	lobby_id: Uuid,
// 	namespace_id: Uuid,
// 	create_ts: chrono::Duration,
// 	stop_ts: Option<chrono::Duration>,
// }

// #[derive(Debug, scylla::macros::FromRow)]
// struct LobbyRow {
// 	lobby_id: Uuid,
// 	stop_ts: Option<chrono::Duration>,
// }

// #[tracing::instrument(skip_all)]
// pub async fn run(pools: rivet_pools::Pools, client: chirp_client::Client) -> GlobalResult<()> {
// 	// TODO: Iterate over all lobbies in history
// 	// TODO: Check if mm lobby is stopped
// 	// TODO: Check if job run is stopped
// 	// TODO: update stop ts to match
// 	// TODO: Log total running lobbies

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

// 	// Count running lobby history
// 	tracing::info!("counting run meta");
// 	let mut running_lobby_history = HashSet::new();
// 	let mut running_lobby_history_old = HashSet::new();
// 	let mut running_lobby_history_rows = Vec::new();
// 	let mut lobby_history_iter = mm_scylla
// 		.query_iter(
// 			"SELECT lobby_id, namespace_id, create_ts, stop_ts FROM lobby_history ALLOW FILTERING",
// 			(),
// 		)
// 		.await?
// 		.into_typed::<LobbyHistoryRow>();
// 	while let Some(lobby) = lobby_history_iter.next().await {
// 		let lobby = lobby?;
// 		if lobby.stop_ts.is_none() {
// 			running_lobby_history.insert(lobby.lobby_id);
// 			if lobby.create_ts.num_milliseconds()
// 				< util::timestamp::now() - util::duration::days(1)
// 			{
// 				running_lobby_history_old.insert(lobby.lobby_id);
// 			}
// 			running_lobby_history_rows.push(lobby);
// 		}
// 	}
// 	tracing::info!(len = ?running_lobby_history.len(), len_old = ?running_lobby_history_old.len(), "running lobby history");

// 	// Count running lobbies
// 	tracing::info!("counting run meta");
// 	let mut running_lobbies = HashSet::new();
// 	let mut lobbies_iter = mm_scylla
// 		.query_iter(
// 			"SELECT lobby_id, stop_ts FROM lobbies WHERE lobby_id IN ?",
// 			(running_lobby_history.iter().cloned().collect::<Vec<_>>(),),
// 		)
// 		.await?
// 		.into_typed::<LobbyRow>();
// 	let mut fixed_history = 0usize;
// 	while let Some(lobby) = lobbies_iter.next().await {
// 		let lobby = lobby?;
// 		if lobby.stop_ts.is_none() {
// 			running_lobbies.insert(lobby.lobby_id);
// 		}

// 		// Fix the history row
// 		if let Some(history_row) = running_lobby_history_rows
// 			.iter()
// 			.find(|x| x.lobby_id == lobby.lobby_id)
// 		{
// 			if let Some(stop_ts) = lobby.stop_ts {
// 				fixed_history += 1;
// 				tracing::info!(lobby_id = %lobby.lobby_id, "fixing history");
// 				let mut lobbies_iter = mm_scylla
// 					.query(
// 						"UPDATE lobby_history SET stop_ts = ? WHERE namespace_id = ? AND create_ts = ? AND lobby_id = ?",
// 						(Timestamp(stop_ts), history_row.namespace_id, Timestamp(history_row.create_ts), history_row.lobby_id),
// 					)
// 					.await?;
// 			}
// 		}
// 	}
// 	tracing::info!(len = ?running_lobbies.len(), "running lobbies");
// 	tracing::info!(?fixed_history, "fixed history");

// 	Ok(())
// }
