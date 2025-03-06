use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-list-all")]
async fn handle(
	ctx: OperationContext<game::list_all::Request>,
) -> GlobalResult<game::list_all::Response> {
	let game_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT game_id
		FROM db_game.games
		",
	)
	.await?
	.into_iter()
	.map(|(game_id,)| common::Uuid::from(game_id))
	.collect::<Vec<_>>();

	Ok(game::list_all::Response { game_ids })
}
