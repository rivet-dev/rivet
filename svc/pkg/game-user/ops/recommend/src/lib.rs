use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-user-recommend")]
async fn handle(
	ctx: OperationContext<game_user::recommend::Request>,
) -> GlobalResult<game_user::recommend::Response> {
	let _count = ctx.count as i32;

	Ok(game_user::recommend::Response {
		game_user_ids: Vec::new(),
	})

	// 	// TODO: This is very slow, we should use a hash shard for this
	// 	// Selects X newest game users
	// 	let game_user_ids = sql_fetch_all!(
	// 		[ctx, (Uuid,)]
	// 		"
	// 		SELECT game_user_id
	// 		FROM game_users
	// 		ORDER BY create_ts DESC
	// 		LIMIT $1
	// 		",
	// 		count,
	// 	)
	// 	.await?;

	// 	Ok(game_user_recommend::Response {
	// 		game_user_ids: game_user_ids
	// 			.into_iter()
	// 			.map(|(game_user_id,)| game_user_id.into())
	// 			.collect::<Vec<_>>(),
	// 	})
}
