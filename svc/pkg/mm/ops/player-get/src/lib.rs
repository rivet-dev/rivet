use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct PlayerRow {
	player_id: Uuid,
	lobby_id: Uuid,
	create_ts: i64,
	register_ts: Option<i64>,
	remove_ts: Option<i64>,
	token_session_id: Uuid,
	create_ray_id: Uuid,
}

impl From<PlayerRow> for backend::matchmaker::Player {
	fn from(row: PlayerRow) -> backend::matchmaker::Player {
		backend::matchmaker::Player {
			player_id: Some(row.player_id.into()),
			lobby_id: Some(row.lobby_id.into()),
			create_ts: row.create_ts,
			register_ts: row.register_ts,
			remove_ts: row.remove_ts,
			token_session_id: Some(row.token_session_id.into()),
			create_ray_id: Some(row.create_ray_id.into()),
		}
	}
}

#[operation(name = "mm-player-get")]
async fn handle(
	ctx: OperationContext<mm::player_get::Request>,
) -> GlobalResult<mm::player_get::Response> {
	let _crdb = ctx.crdb().await?;

	let player_ids = ctx
		.player_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let players = sql_fetch_all!(
		[ctx, PlayerRow]
		"
		SELECT player_id, lobby_id, create_ts, register_ts, remove_ts, token_session_id, create_ray_id
		FROM db_mm_state.players
		WHERE player_id = ANY($1)
		",
		player_ids,
	)
	.await?
	.into_iter()
	.map(Into::<backend::matchmaker::Player>::into)
	.collect();

	Ok(mm::player_get::Response { players })
}
