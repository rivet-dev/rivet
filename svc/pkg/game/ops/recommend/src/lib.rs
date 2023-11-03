use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-recommend")]
async fn handle(
	ctx: OperationContext<game::recommend::Request>,
) -> GlobalResult<game::recommend::Response> {
	let game_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"SELECT game_id FROM db_game.games",
	)
	.await?
	.into_iter()
	.map(|row| row.0.into())
	.collect::<Vec<common::Uuid>>();

	Ok(game::recommend::Response { game_ids })
}
