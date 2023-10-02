use std::collections::{HashMap, HashSet};

use indoc::indoc;
use proto::backend::pkg::*;
use redis::AsyncCommands;
use rivet_operation::prelude::*;

const OLD_PLAYER_TIMEOUT: i64 = util::duration::hours(12);

#[tracing::instrument]
pub async fn run_from_env(ts: i64) -> GlobalResult<()> {
	let pools = rivet_pools::from_env("mm-gc-full").await?;
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("mm-gc");
	let redis_mm = pools.redis("mm")?;
	let crdb = pools.crdb()?;

	let old_players_ts = ts - OLD_PLAYER_TIMEOUT;

	tracing::info!("fetching all lobbies online");
	let lobbies_online = sqlx::query_as::<_, (Uuid,)>(indoc!(
		"
		SELECT lobby_id
		FROM db_mm_state.lobbies AS OF SYSTEM TIME '-5s'
		WHERE stop_ts IS NULL
		"
	))
	.fetch_all(&crdb)
	.await?
	.into_iter()
	.map(|x| x.0)
	.collect::<HashSet<Uuid>>();
	tracing::info!(len = ?lobbies_online.len(), "cockroach lobbies online");

	tracing::info!("fetching all players online");
	let players_online = sqlx::query_as::<_, (Uuid,)>(indoc!(
		"
		SELECT player_id
		FROM db_mm_state.players AS OF SYSTEM TIME '-5s'
		WHERE remove_ts IS NULL
		"
	))
	.fetch_all(&crdb)
	.await?
	.into_iter()
	.map(|x| x.0)
	.collect::<HashSet<Uuid>>();
	tracing::info!(len = ?players_online.len(), "cockroach players online");

	remove_zombie_players(
		ts,
		old_players_ts,
		redis_mm.clone(),
		crdb.clone(),
		client.clone(),
		&lobbies_online,
		&players_online,
	)
	.await?;

	// Should be called after remove_zombie_players to reducse false positives.
	check_for_zombie_configs(redis_mm.clone(), crdb.clone()).await?;

	Ok(())
}

/// Checks for any players that may have leaked.
///
/// This should no longer catch any instances and will be removed once confirmed
/// that this no longer catches any errors.
#[tracing::instrument(skip_all)]
async fn remove_zombie_players(
	ts: i64,
	old_players_ts: i64,
	mut redis_mm: RedisPool,
	crdb: CrdbPool,
	client: chirp_client::Client,
	lobbies_online: &HashSet<Uuid>,
	players_online: &HashSet<Uuid>,
) -> GlobalResult<()> {
	// Build player index
	let mut all_player_ids = HashSet::<Uuid>::new();
	let mut player_namespace_ids = HashMap::<Uuid, Uuid>::new();
	let mut player_lobby_ids = HashMap::<Uuid, Uuid>::new();

	let ns_player_id_keys = redis_mm
		.keys::<_, Vec<String>>("mm:ns:????????-????-????-????-????????????:player_ids".to_string())
		.await?;
	for (i, key) in ns_player_id_keys.iter().enumerate() {
		tracing::info!(?i, len = ?ns_player_id_keys.len(), "ns player id keys");

		let namespace_id = key.split(':').nth(2).unwrap();
		let namespace_id = util::uuid::parse(namespace_id)?;

		let player_ids = redis_mm
			.zrangebyscore::<_, _, _, Vec<String>>(&key, 0, old_players_ts as isize)
			.await?;
		for player_id in player_ids {
			let player_id = util::uuid::parse(&player_id)?;
			all_player_ids.insert(player_id);
			player_namespace_ids.insert(player_id, namespace_id);
		}
	}

	let lobby_player_id_keys = redis_mm
		.keys::<_, Vec<String>>(
			"mm:lobby:????????-????-????-????-????????????:player_ids".to_string(),
		)
		.await?;
	for (i, key) in lobby_player_id_keys.iter().enumerate() {
		tracing::info!(?i, len = ?lobby_player_id_keys.len(), "lobby player id keys");

		let lobby_id = key.split(':').nth(2).unwrap();
		let lobby_id = util::uuid::parse(lobby_id)?;

		let player_ids = redis_mm
			.zrangebyscore::<_, _, _, Vec<String>>(&key, 0, old_players_ts as isize)
			.await?;
		for player_id in player_ids {
			let player_id = util::uuid::parse(&player_id)?;
			all_player_ids.insert(player_id);
			player_lobby_ids.insert(player_id, lobby_id);
		}
	}

	let lobby_registered_player_id_keys = redis_mm
		.keys::<_, Vec<String>>(
			"mm:lobby:????????-????-????-????-????????????:registered_player_ids".to_string(),
		)
		.await?;
	for (i, key) in lobby_registered_player_id_keys.iter().enumerate() {
		tracing::info!(?i, len = ?lobby_registered_player_id_keys.len(), "lobby registered player id keys");

		let lobby_id = key.split(':').nth(2).unwrap();
		let lobby_id = util::uuid::parse(lobby_id)?;

		let player_ids = redis_mm
			.zrangebyscore::<_, _, _, Vec<String>>(&key, 0, old_players_ts as isize)
			.await?;
		for player_id in player_ids {
			let player_id = util::uuid::parse(&player_id)?;
			all_player_ids.insert(player_id);
			player_lobby_ids.insert(player_id, lobby_id);
		}
	}

	let mut players_still_online = 0;
	for player_id in players_online.iter() {
		if all_player_ids.remove(player_id) {
			players_still_online += 1;
		}
	}
	tracing::info!(
		?players_still_online,
		"skipping removing players because they're still online"
	);

	tracing::info!(len = ?all_player_ids.len(), ns_players = ?player_namespace_ids.len(), lobby_players = ?player_lobby_ids.len(), "collected player ids");

	// Purge all players
	for (i, player_id) in all_player_ids.iter().cloned().enumerate() {
		let namespace_id = player_namespace_ids.get(&player_id).cloned();
		let lobby_id = player_lobby_ids.get(&player_id).cloned();

		tracing::warn!(?i, len = ?all_player_ids.len(), ?player_id, ?lobby_id, ?namespace_id, "removing player");

		msg!([client] mm::msg::player_remove(player_id) {
			player_id: Some(player_id.into()),
			lobby_id: None,
			from_lobby_destroy: false,
		})
		.await?;

		// Remove from Redis in case mm-player-remove doesn't work (i.e. when
		// the row is not in the database)
		{
			let mut pipe = redis::pipe();

			pipe.unlink(util_mm::key::player_config(player_id));
			if let Some(namespace_id) = namespace_id {
				pipe.zrem(
					util_mm::key::ns_player_ids(namespace_id),
					player_id.to_string(),
				);
			}
			if let Some(lobby_id) = lobby_id {
				pipe.zrem(
					util_mm::key::lobby_player_ids(lobby_id),
					player_id.to_string(),
				);
				pipe.zrem(
					util_mm::key::lobby_registered_player_ids(lobby_id),
					player_id.to_string(),
				);
			}
			pipe.zrem(util_mm::key::player_unregistered(), player_id.to_string());

			// TODO: Remove the remote address
			// if let Some(remote_address) = remote_address {
			// pipe.zrem(
			// 	util_mm::key::ns_remote_address_player_ids(namespace_id, &remote_address),
			// 	player_id.to_string(),
			// );
			// }

			// TODO: Update available lobby spots & idle lobbies here

			pipe.query_async(&mut redis_mm).await?;
		}
	}

	Ok(())
}

/// Checks for configs in Redis that are not present in Cockroach.
///
/// This does not remove the configs for now because of a potential race
/// condition with insertion.
#[tracing::instrument(skip_all)]
async fn check_for_zombie_configs(mut redis_mm: RedisPool, crdb: CrdbPool) -> GlobalResult<()> {
	let lobby_config_keys = redis_mm
		.keys::<_, Vec<String>>("mm:lobby:????????-????-????-????-????????????:config".to_string())
		.await?;
	let player_config_keys = redis_mm
		.keys::<_, Vec<String>>("mm:player:????????-????-????-????-????????????:config".to_string())
		.await?;

	let all_redis_lobby_ids = lobby_config_keys
		.iter()
		.map(|x| x.split(':').nth(2).unwrap())
		.map(util::uuid::parse)
		.collect::<Result<Vec<_>, _>>()?;
	let all_redis_player_ids = player_config_keys
		.iter()
		.map(|x| x.split(':').nth(2).unwrap())
		.map(util::uuid::parse)
		.collect::<Result<Vec<_>, _>>()?;

	tracing::info!(lobby_ids_len = ?all_redis_lobby_ids.len(), player_ids_len = ?all_redis_player_ids.len(), "fetched config keys");

	// Not 100% accurate. This has a race condition with newly inserted lobbies.
	let all_redis_lobby_ids_chunks = all_redis_lobby_ids.chunks(64);
	for (i, redis_lobby_ids) in all_redis_lobby_ids_chunks.enumerate() {
		tracing::info!(?i, len = ?(all_redis_lobby_ids.len() / 64), "checking lobby");
		let crdb_lobby_ids = sqlx::query_as::<_, (Uuid,)>(indoc!(
			"
			SELECT lobby_id
			FROM db_mm_state.lobbies
			WHERE lobby_id = ANY($1)
			"
		))
		.bind(redis_lobby_ids)
		.fetch_all(&crdb)
		.await?
		.into_iter()
		.map(|x| x.0)
		.collect::<HashSet<Uuid>>();

		for lobby_id in redis_lobby_ids {
			if crdb_lobby_ids.contains(lobby_id) {
				continue;
			}

			tracing::warn!(?lobby_id, "lobby id not in cockroach");
		}
	}

	// Not 100% accurate. This has a race condition with newly inserted players.
	let all_redis_player_ids_chunks = all_redis_player_ids.chunks(64);
	for (i, redis_player_ids) in all_redis_player_ids_chunks.enumerate() {
		tracing::info!(?i, len = ?(all_redis_player_ids.len() / 64), "checking players");
		let crdb_player_ids = sqlx::query_as::<_, (Uuid,)>(indoc!(
			"
			SELECT player_id
			FROM db_mm_state.players
			WHERE player_id = ANY($1)
			"
		))
		.bind(redis_player_ids)
		.fetch_all(&crdb)
		.await?
		.into_iter()
		.map(|x| x.0)
		.collect::<HashSet<Uuid>>();

		for player_id in redis_player_ids {
			if crdb_player_ids.contains(player_id) {
				continue;
			}

			tracing::warn!(?player_id, "player id not in cockroach");
		}
	}

	Ok(())
}
