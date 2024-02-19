use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct GameUser {
	game_user_id: Uuid,
	user_id: Uuid,
}

#[operation(name = "game-user-list-for-user")]
async fn handle(
	ctx: OperationContext<game_user::list_for_user::Request>,
) -> GlobalResult<game_user::list_for_user::Response> {
	let user_ids = ctx
		.user_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let game_user_rows = sql_fetch_all!(
		[ctx, GameUser]
		"
		SELECT game_user_id, user_id
		FROM db_game_user.game_users
		WHERE user_id = ANY($1)
		",
		&user_ids,
	)
	.await?;

	let users = user_ids
		.into_iter()
		.map(|user_id| {
			let game_user_ids = game_user_rows
				.iter()
				.filter(|x| x.user_id == user_id)
				.map(|row| Into::<common::Uuid>::into(row.game_user_id))
				.collect();

			game_user::list_for_user::response::User {
				user_id: Some(user_id.into()),
				game_user_ids,
			}
		})
		.collect::<Vec<_>>();

	Ok(game_user::list_for_user::Response { users })
}
