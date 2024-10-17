use proto::backend::pkg::*;
use redis::AsyncCommands;
use rivet_operation::prelude::*;

pub async fn start() -> GlobalResult<()> {
	// TODO: Handle ctrl-c

	let pools = rivet_pools::from_env().await?;

	let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
	loop {
		interval.tick().await;

		let ts = util::timestamp::now();
		run_from_env(ts, pools.clone()).await?;
	}
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(ts: i64, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("mm-gc");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"mm-gc".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools.clone(), cache.clone()),
		Uuid::new_v4(),
		Uuid::new_v4(),
		ts,
		ts,
		(),
	);
	let redis_mm = ctx.redis_mm().await?;

	let mut return_err: Option<GlobalError> = None;
	let (unready_res, unregistered_res, auto_remove_res) = tokio::join!(
		cull_unready_lobbies(ts, redis_mm.clone(), ctx.chirp().clone()),
		cull_unregistered_players(ts, redis_mm.clone(), ctx.chirp().clone()),
		cull_auto_remove_players(ts, redis_mm.clone(), ctx.chirp().clone()),
	);
	if let Err(err) = unready_res {
		tracing::error!(?err, "error in cull unready lobbies");
		return_err = Some(err)
	}
	if let Err(err) = unregistered_res {
		tracing::error!(?err, "error in cull unregistered players");
		return_err = Some(err)
	}
	if let Err(err) = auto_remove_res {
		tracing::error!(?err, "error in cull auto remove players");
		return_err = Some(err)
	}

	match return_err {
		Some(x) => Err(x),
		None => Ok(()),
	}
}

/// Find all lobbies that have not flagged as ready. It's OK for this to be
/// called multiple times on a lobby that's already flagged as `is_removing`
/// in case something happened to the service that was responsible for removing it
/// originally.
#[tracing::instrument(skip_all)]
async fn cull_unready_lobbies(
	ts: i64,
	mut redis_mm: RedisPool,
	client: chirp_client::Client,
) -> GlobalResult<()> {
	// We don't remove from the set here since this will be removed in the mm-lobby-cleanup
	// service.
	let unready_lobby_ids = redis_mm
		.zrangebyscore::<_, _, _, Vec<String>>(util_mm::key::lobby_unready(), 0, ts as isize)
		.await?
		.into_iter()
		.filter_map(|x| util::uuid::parse(&x).ok())
		.collect::<Vec<_>>();

	// Stop the lobbies
	tracing::info!(count = unready_lobby_ids.len(), "stopping lobbies");
	for lobby_id in unready_lobby_ids.into_iter() {
		msg!([client] @wait mm::msg::lobby_stop(lobby_id) {
			lobby_id: Some(lobby_id.into()),
		})
		.await?;
	}

	// We don't call idle update here in case the lobbies are failing to boot
	// immediately, so we don't want to automatically start new lobbies that
	// aren't running correctly.

	Ok(())
}

/// Removes all players that are not registered.
#[tracing::instrument(skip_all)]
async fn cull_unregistered_players(
	ts: i64,
	mut redis_mm: RedisPool,
	client: chirp_client::Client,
) -> GlobalResult<()> {
	// We don't remove from the set here since this will be removed in the mm-player-remove
	// service.
	let remove_player_ids = redis_mm
		.zrangebyscore::<_, _, _, Vec<String>>(util_mm::key::player_unregistered(), 0, ts as isize)
		.await?
		.into_iter()
		.filter_map(|x| util::uuid::parse(&x).ok())
		.collect::<Vec<_>>();
	tracing::info!(count = %remove_player_ids.len(), "removing unregistered players");

	for player_id in &remove_player_ids {
		msg!([client] @wait mm::msg::player_remove(player_id) {
			player_id: Some((*player_id).into()),
			lobby_id: None,
			..Default::default()
		})
		.await?;
	}

	Ok(())
}

/// Removes all players that are not registered.
#[tracing::instrument(skip_all)]
async fn cull_auto_remove_players(
	ts: i64,
	mut redis_mm: RedisPool,
	client: chirp_client::Client,
) -> GlobalResult<()> {
	// We don't remove from the set here since this will be removed in the mm-player-remove
	// service.
	let remove_player_ids = redis_mm
		.zrangebyscore::<_, _, _, Vec<String>>(util_mm::key::player_auto_remove(), 0, ts as isize)
		.await?
		.into_iter()
		.filter_map(|x| util::uuid::parse(&x).ok())
		.collect::<Vec<_>>();
	tracing::info!(count = %remove_player_ids.len(), "auto removing players");

	for player_id in &remove_player_ids {
		msg!([client] @wait mm::msg::player_remove(player_id) {
			player_id: Some((*player_id).into()),
			lobby_id: None,
			..Default::default()
		})
		.await?;
	}

	Ok(())
}
