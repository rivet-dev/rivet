use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, sqlx::FromRow)]
struct LobbyRow {
	namespace_id: Uuid,
	region_id: Uuid,
	lobby_group_id: Uuid,
	run_id: Option<Uuid>,
	create_ts: i64,
	stop_ts: Option<i64>,
}

#[worker(name = "mm-lobby-cleanup")]
async fn worker(ctx: &OperationContext<mm::msg::lobby_cleanup::Message>) -> GlobalResult<()> {
	// NOTE: Idempotent

	let lobby_id = unwrap_ref!(ctx.lobby_id).as_uuid();

	let crdb = ctx.crdb().await?;

	// TODO: Wrap this in a MULTI/WATCH
	// Remove from Redis before attempting to remove from database.
	//
	// We do this before the SQL command because:
	// * this needs to execute ASAP in order to prevent new players from
	//   attempting to join this lobby
	// * cleans up lobbies correctly even if mm-lobby-find failed to insert in
	//   to SQL
	// * if mm-lobby-create times out, we need to clean up Redis nonetheless
	let mut redis_mm = ctx.redis_mm().await?;
	let lobby_config = redis_mm
		.hgetall::<_, HashMap<String, String>>(util_mm::key::lobby_config(lobby_id))
		.await?;
	let did_remove_from_redis = if let (Some(namespace_id), Some(region_id), Some(lobby_group_id)) = (
		lobby_config.get(util_mm::key::lobby_config::NAMESPACE_ID),
		lobby_config.get(util_mm::key::lobby_config::REGION_ID),
		lobby_config.get(util_mm::key::lobby_config::LOBBY_GROUP_ID),
	) {
		let namespace_id = util::uuid::parse(namespace_id)?;
		let region_id = util::uuid::parse(region_id)?;
		let lobby_group_id = util::uuid::parse(lobby_group_id)?;

		remove_from_redis(
			&mut redis_mm,
			namespace_id,
			region_id,
			lobby_group_id,
			lobby_id,
		)
		.await?;

		true
	} else {
		// This is idempotent, don't raise error
		tracing::info!("lobby not present in redis");

		false
	};

	// Fetch the lobby.
	//
	// This also ensures that mm-lobby-find or mm-lobby-create
	// has already inserted the row.
	//
	// This also locks the lobby row in case there is a race condition with
	// mm-lobby-create.
	let lobby_row = sql_fetch_optional!(
		[ctx, LobbyRow]
		"
		WITH
			select_lobby AS (
				SELECT namespace_id, region_id, lobby_group_id, run_id, create_ts, stop_ts
				FROM db_mm_state.lobbies
				WHERE lobby_id = $1
			),
			_update AS (
				UPDATE db_mm_state.lobbies
				SET stop_ts = $2
				WHERE lobby_id = $1 AND stop_ts IS NULL
				RETURNING 1
			)
		SELECT * FROM select_lobby
		",
		lobby_id,
		ctx.ts(),
	)
	.await?;
	tracing::info!(?lobby_row, "lobby row");

	let Some(lobby_row) = lobby_row else {
		if ctx.req_dt() > util::duration::minutes(5) {
			tracing::error!("discarding stale message");
			return Ok(());
		} else {
			retry_bail!("lobby not found, may be race condition with insertion");
		}
	};

	// TODO: Handle race condition here where mm-lobby-find will insert players on a stopped lobby. This can be fixed by checking the lobby stop_ts in the mm-lobby-find trans.
	// Find players to remove. This is idempotent, so we do this regardless of
	// if the lobby was already stopped.
	let players_to_remove = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT player_id
		FROM db_mm_state.players
		WHERE lobby_id = $1 AND remove_ts IS NULL
		",
		lobby_id,
	)
	.await?
	.into_iter()
	.map(|x| x.0)
	.collect::<Vec<_>>();

	tracing::info!(player_len = ?players_to_remove.len(), "removing players");
	for player_id in &players_to_remove {
		msg!([ctx] @wait mm::msg::player_remove(player_id) {
			player_id: Some((*player_id).into()),
			lobby_id: Some(lobby_id.into()),
			from_lobby_destroy: true,
		})
		.await?;
	}

	// If we couldn't read the lobby configuration from Redis, then try to
	// remove it from Redis again using the information fetched from the
	// database.
	//
	// This helps mitigate edge cases where part of the root lobby configuration
	// was removed from Redis but lingering data. These problems should no
	// longer exist, but required from much older deployments.
	//
	// See `util_mm:key::invalid_lobby_ids`
	if !did_remove_from_redis {
		tracing::info!("removing lobby from redis using information from crdb");

		remove_from_redis(
			&mut redis_mm,
			lobby_row.namespace_id,
			lobby_row.region_id,
			lobby_row.lobby_group_id,
			lobby_id,
		)
		.await?;
	}

	msg!([ctx] mm::msg::lobby_cleanup_complete(lobby_id) {
		lobby_id: Some(lobby_id.into()),
	})
	.await?;

	// Publish analytics event if not already stopped
	if lobby_row.stop_ts.is_none() {
		// Fetch run data
		let run_json = if let Some(run_id) = lobby_row.run_id {
			let run_res = op!([ctx] job_run_get {
				run_ids: vec![run_id.into()],
			})
			.await?;

			if let Some(run) = run_res.runs.first() {
				#[allow(clippy::manual_map)]
				let run_meta_json = match run.run_meta.as_ref().and_then(|x| x.kind.as_ref()) {
					Some(backend::job::run_meta::Kind::Nomad(meta)) => Some(json!({
						"nomad": {
							"failed": meta.failed,
							"exit_code": meta.exit_code,
						}
					})),
					None => None,
				};

				Some(json!({
					"meta": run_meta_json,
				}))
			} else {
				None
			}
		} else {
			None
		};

		msg!([ctx] analytics::msg::event_create() {
			events: vec![
				analytics::msg::event_create::Event {
					event_id: Some(Uuid::new_v4().into()),
					name: "mm.lobby.destroy".into(),
					properties_json: Some(serde_json::to_string(&json!({
						"namespace_id": lobby_row.namespace_id,
						"lobby_id": lobby_id,
						"lobby_group_id": lobby_row.lobby_group_id,
						"region_id": lobby_row.region_id,
						"create_ts": lobby_row.create_ts,
						"removed_player_count": players_to_remove.len(),
						"run": run_json,
					}))?),
					..Default::default()
				}
			],
		})
		.await?;
	}

	Ok(())
}

/// Removes the lobby from the Redis database.
async fn remove_from_redis(
	redis_mm: &mut RedisPool,
	namespace_id: Uuid,
	region_id: Uuid,
	lobby_group_id: Uuid,
	lobby_id: Uuid,
) -> GlobalResult<()> {
	let mut pipe = redis::pipe();
	pipe.atomic()
		.unlink(util_mm::key::lobby_config(lobby_id))
		.unlink(util_mm::key::lobby_tags(lobby_id))
		.zrem(
			util_mm::key::ns_lobby_ids(namespace_id),
			lobby_id.to_string(),
		)
		.zrem(
			util_mm::key::lobby_available_spots(
				namespace_id,
				region_id,
				lobby_group_id,
				util_mm::JoinKind::Normal,
			),
			lobby_id.to_string(),
		)
		.zrem(
			util_mm::key::lobby_available_spots(
				namespace_id,
				region_id,
				lobby_group_id,
				util_mm::JoinKind::Party,
			),
			lobby_id.to_string(),
		)
		.zrem(util_mm::key::lobby_unready(), lobby_id.to_string())
		.zrem(
			util_mm::key::idle_lobby_ids(namespace_id, region_id, lobby_group_id),
			lobby_id.to_string(),
		)
		.hdel(
			util_mm::key::idle_lobby_lobby_group_ids(namespace_id, region_id),
			lobby_id.to_string(),
		)
		.query_async(redis_mm)
		.await?;

	Ok(())
}
