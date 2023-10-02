use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;

#[derive(Debug, sqlx::FromRow)]
struct PlayerRow {
	lobby_id: Uuid,
	create_ts: i64,
	register_ts: Option<i64>,
	remove_ts: Option<i64>,
	namespace_id: Uuid,
}

#[worker(name = "mm-player-register")]
async fn worker(ctx: &OperationContext<mm::msg::player_register::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb().await?;

	let player_id = internal_unwrap!(ctx.player_id).as_uuid();
	let lobby_id = ctx.lobby_id.map(|x| x.as_uuid());

	// Get the player
	let expired_create_ts = ctx.ts() - util_mm::consts::PLAYER_READY_TIMEOUT;
	let player_row = sqlx::query_as::<_, PlayerRow>(indoc!(
		"
		WITH
			select_player AS (
				SELECT
					players.lobby_id,
					players.create_ts,
					players.register_ts,
					players.remove_ts,
					lobbies.namespace_id
				FROM db_mm_state.players
				INNER JOIN lobbies ON lobbies.lobby_id = players.lobby_id
				WHERE player_id = $1
			),
			_update AS (
				UPDATE db_mm_state.players
				SET register_ts = $3
				WHERE
					player_id = $1 AND
					register_ts IS NULL AND
					create_ts > $2 AND
					remove_ts IS NULL
				RETURNING 1
			)
		SELECT * FROM select_player
		"
	))
	.bind(player_id)
	.bind(expired_create_ts)
	.bind(ctx.ts())
	.fetch_optional(&crdb)
	.await?;
	tracing::info!(?player_row, "player row");

	let Some(player_row) = player_row else {
		if ctx.req_dt() > util::duration::minutes(5) {
			tracing::error!("discarding stale message");
			return Ok(());
		} else {
			retry_panic!("player not found, may be race condition with insertion");
		}
	};

	// Check player is in the correct lobby
	if let Some(lobby_id) = lobby_id {
		if player_row.lobby_id != lobby_id {
			tracing::info!("player in wrong lobby");

			// Remove the player from the lobby it's actually in if the
			// registration fails here.
			msg!([ctx] mm::msg::player_remove(player_id) {
				player_id: Some(player_id.into()),
				lobby_id: Some(player_row.lobby_id.into()),
				from_lobby_destroy: false,
			})
			.await?;

			return fail(
				ctx.chirp(),
				player_id,
				player_row,
				mm::msg::player_register_fail::ErrorCode::PlayerInDifferentLobby,
			)
			.await;
		}
	}

	if player_row.register_ts.is_some() {
		return fail(
			ctx.chirp(),
			player_id,
			player_row,
			mm::msg::player_register_fail::ErrorCode::PlayerAlreadyRegistered,
		)
		.await;
	} else if player_row.create_ts <= expired_create_ts {
		return fail(
			ctx.chirp(),
			player_id,
			player_row,
			mm::msg::player_register_fail::ErrorCode::RegistrationExpired,
		)
		.await;
	} else if player_row.remove_ts.is_some() {
		return fail(
			ctx.chirp(),
			player_id,
			player_row,
			mm::msg::player_register_fail::ErrorCode::PlayerRemoved,
		)
		.await;
	}

	// Write to Redis
	let update_redis_perf = ctx.perf().start("update-player-redis").await;
	redis::pipe()
		.atomic()
		.zadd(
			util_mm::key::lobby_registered_player_ids(player_row.lobby_id),
			player_id.to_string(),
			ctx.ts(),
		)
		.ignore()
		.zrem(util_mm::key::player_unregistered(), player_id.to_string())
		.ignore()
		.query_async::<_, redis::Value>(&mut ctx.redis_mm().await?)
		.await?;
	update_redis_perf.end();

	msg!([ctx] mm::msg::player_register_complete(player_id) {
		player_id: Some(player_id.into()),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "mm.player.ready".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"namespace_id": player_row.namespace_id,
					"player_id": player_id,
					"lobby_id": player_row.lobby_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}

#[tracing::instrument]
async fn fail(
	client: &chirp_client::Client,
	player_id: Uuid,
	player_row: PlayerRow,
	error_code: mm::msg::player_register_fail::ErrorCode,
) -> GlobalResult<()> {
	tracing::warn!(
		?player_id,
		?player_row,
		?error_code,
		"player register failed"
	);

	msg!([client] mm::msg::player_register_fail(player_id) {
		player_id: Some(player_id.into()),
		error_code: error_code as i32,
	})
	.await?;

	msg!([client] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "mm.player.ready_fail".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"namespace_id": player_row.namespace_id,
					"player_id": player_id,
					"lobby_id": player_row.lobby_id,
					"error": error_code as i32,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}
