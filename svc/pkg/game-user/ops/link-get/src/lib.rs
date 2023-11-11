use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct GameUserLink {
	link_id: Uuid,
	namespace_id: Uuid,
	token_session_id: Uuid,
	current_game_user_id: Uuid,
	new_game_user_id: Option<Uuid>,
	new_game_user_token: Option<String>,
	create_ts: i64,
	complete_ts: Option<i64>,
	cancelled_ts: Option<i64>,
}

impl From<GameUserLink> for game_user::link_get::response::GameUserLink {
	fn from(value: GameUserLink) -> game_user::link_get::response::GameUserLink {
		game_user::link_get::response::GameUserLink {
			link_id: Some(value.link_id.into()),
			namespace_id: Some(value.namespace_id.into()),
			token_session_id: Some(value.token_session_id.into()),
			current_game_user_id: Some(value.current_game_user_id.into()),
			new_game_user_id: value.new_game_user_id.map(Into::into),
			new_game_user_token: value.new_game_user_token,
			create_ts: value.create_ts,
			complete_ts: value.complete_ts,
			cancelled_ts: value.cancelled_ts,

			status: if value.complete_ts.is_some() {
				game_user::link_get::response::GameUserLinkStatus::Complete as i32
			} else if value.cancelled_ts.is_some() {
				game_user::link_get::response::GameUserLinkStatus::Cancelled as i32
			} else {
				game_user::link_get::response::GameUserLinkStatus::Incomplete as i32
			},
		}
	}
}

#[operation(name = "game-user-link-get")]
async fn handle(
	ctx: OperationContext<game_user::link_get::Request>,
) -> GlobalResult<game_user::link_get::Response> {
	let crdb = ctx.crdb().await?;

	let link_ids = ctx
		.link_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let game_user_links = sql_fetch_all!(
		[ctx, GameUserLink]
		"
		SELECT
			link_id,
			namespace_id,
			token_session_id,
			current_game_user_id,
			new_game_user_id,
			new_game_user_token,
			create_ts,
			complete_ts,
			cancelled_ts
		FROM db_game_user.links
		WHERE link_id = ANY($1)
		",
		link_ids,
	)
	.await?
	.into_iter()
	.map(Into::into)
	.collect();

	Ok(game_user::link_get::Response { game_user_links })
}
