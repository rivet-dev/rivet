use chirp_worker::prelude::*;
use proto::backend::pkg::*;

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/lobby_closed_set.lua"));
}

#[derive(Debug, sqlx::FromRow)]
struct LobbyRow {
	namespace_id: Uuid,
	region_id: Uuid,
	lobby_group_id: Uuid,
	max_players_normal: i64,
	max_players_party: i64,
}

#[worker(name = "mm-lobby-closed-set")]
async fn worker(ctx: &OperationContext<mm::msg::lobby_closed_set::Message>) -> GlobalResult<()> {
	let lobby_id = unwrap_ref!(ctx.lobby_id).as_uuid();

	let lobby_row = sql_fetch_optional!(
		[ctx, LobbyRow]
		"
		UPDATE db_mm_state.lobbies
		SET is_closed = $2
		WHERE lobby_id = $1
		RETURNING namespace_id, region_id, lobby_group_id, max_players_normal, max_players_party
		",
		lobby_id,
		ctx.is_closed,
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

	// Update matchmaking index
	if ctx.is_closed {
		redis::pipe()
			.atomic()
			.hset(
				util_mm::key::lobby_config(lobby_id),
				util_mm::key::lobby_config::IS_CLOSED,
				true,
			)
			.zrem(
				util_mm::key::lobby_available_spots(
					lobby_row.namespace_id,
					lobby_row.region_id,
					lobby_row.lobby_group_id,
					util_mm::JoinKind::Normal,
				),
				lobby_id.to_string(),
			)
			.zrem(
				util_mm::key::lobby_available_spots(
					lobby_row.namespace_id,
					lobby_row.region_id,
					lobby_row.lobby_group_id,
					util_mm::JoinKind::Party,
				),
				lobby_id.to_string(),
			)
			.query_async(&mut ctx.redis_mm().await?)
			.await?;
	} else {
		REDIS_SCRIPT
			.key(util_mm::key::lobby_config(lobby_id))
			.key(util_mm::key::lobby_player_ids(lobby_id))
			.key(util_mm::key::lobby_available_spots(
				lobby_row.namespace_id,
				lobby_row.region_id,
				lobby_row.lobby_group_id,
				util_mm::JoinKind::Normal,
			))
			.key(util_mm::key::lobby_available_spots(
				lobby_row.namespace_id,
				lobby_row.region_id,
				lobby_row.lobby_group_id,
				util_mm::JoinKind::Party,
			))
			.arg(lobby_id.to_string())
			.arg(lobby_row.max_players_normal)
			.arg(lobby_row.max_players_party)
			.invoke_async(&mut ctx.redis_mm().await?)
			.await?;
	}

	msg!([ctx] mm::msg::lobby_closed_set_complete(lobby_id) {
		lobby_id: Some(lobby_id.into()),
	})
	.await?;

	Ok(())
}
