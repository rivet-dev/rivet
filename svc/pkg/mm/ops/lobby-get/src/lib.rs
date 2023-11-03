use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct LobbyRow {
	lobby_id: Uuid,
	lobby_group_id: Uuid,
	region_id: Uuid,
	token_session_id: Option<Uuid>,
	create_ts: i64,
	stop_ts: Option<i64>,
	ready_ts: Option<i64>,
	run_id: Option<Uuid>,
	is_closed: bool,
	namespace_id: Uuid,
	create_ray_id: Option<Uuid>,
	creator_user_id: Option<Uuid>,
	is_custom: bool,
	publicity: i64,

	max_players_normal: i64,
	max_players_direct: i64,
	max_players_party: i64,
}

impl From<LobbyRow> for backend::matchmaker::Lobby {
	fn from(value: LobbyRow) -> Self {
		backend::matchmaker::Lobby {
			lobby_id: Some(value.lobby_id.into()),
			lobby_group_id: Some(value.lobby_group_id.into()),
			region_id: Some(value.region_id.into()),
			token_session_id: value.token_session_id.map(|id| id.into()),
			create_ts: value.create_ts,
			stop_ts: value.stop_ts,
			ready_ts: value.ready_ts,
			run_id: value.run_id.map(|id| id.into()),
			is_closed: value.is_closed,
			namespace_id: Some(value.namespace_id.into()),
			create_ray_id: value.create_ray_id.map(Into::into),
			creator_user_id: value.creator_user_id.map(Into::into),
			is_custom: value.is_custom,
			publicity: value.publicity as i32,

			max_players_normal: value.max_players_normal as u32,
			max_players_direct: value.max_players_direct as u32,
			max_players_party: value.max_players_party as u32,
		}
	}
}

#[operation(name = "mm-lobby-get")]
async fn handle(
	ctx: OperationContext<mm::lobby_get::Request>,
) -> GlobalResult<mm::lobby_get::Response> {
	let crdb = ctx.crdb().await?;

	let lobby_ids = ctx
		.lobby_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let lobbies = sql_fetch_all!(
		[ctx, LobbyRow]
		"
		SELECT
			lobby_id,
			lobby_group_id,
			region_id,
			token_session_id,
			create_ts,
			stop_ts,
			ready_ts,
			run_id,
			is_closed,
			namespace_id,
			create_ray_id,
			creator_user_id,
			is_custom,
			publicity,

			max_players_normal,
			max_players_direct,
			max_players_party
		FROM db_mm_state.lobbies
		WHERE lobby_id = ANY($1)
		",
		lobby_ids,
	)
	.await?
	.into_iter()
	.filter(|x| x.stop_ts.is_none() || ctx.include_stopped)
	.map(Into::<backend::matchmaker::Lobby>::into)
	.collect();

	Ok(mm::lobby_get::Response { lobbies })
}
