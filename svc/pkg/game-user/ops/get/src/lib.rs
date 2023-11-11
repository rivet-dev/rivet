use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct GameUser {
	game_user_id: Uuid,
	user_id: Uuid,
	token_session_id: Uuid,
	namespace_id: Uuid,
	create_ts: i64,
	link_id: Option<Uuid>,
	deleted_ts: Option<i64>,
}

impl From<GameUser> for game_user::get::response::GameUser {
	fn from(value: GameUser) -> game_user::get::response::GameUser {
		game_user::get::response::GameUser {
			game_user_id: Some(value.game_user_id.into()),
			user_id: Some(value.user_id.into()),
			token_session_id: Some(value.token_session_id.into()),
			namespace_id: Some(value.namespace_id.into()),
			create_ts: value.create_ts,
			link_id: value.link_id.map(Into::into),
			deleted_ts: value.deleted_ts,
		}
	}
}

#[operation(name = "game-user-get")]
async fn handle(
	ctx: OperationContext<game_user::get::Request>,
) -> GlobalResult<game_user::get::Response> {
	let crdb = ctx.crdb().await?;

	let game_user_ids = ctx
		.game_user_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let game_users = sql_fetch_all!(
		[ctx, GameUser]
		"
		SELECT gu.game_user_id, gu.user_id, gu.token_session_id, gu.namespace_id, gu.create_ts, l.link_id, gu.deleted_ts
		FROM db_game_user.game_users AS gu
		LEFT JOIN db_game_user.links AS l ON l.new_game_user_id = gu.game_user_id
		WHERE gu.game_user_id = ANY($1)
		",
		game_user_ids,
	)
	.await?
	.into_iter()
	.map(Into::into)
	.collect();

	Ok(game_user::get::Response { game_users })
}
