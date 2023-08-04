use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-recommend")]
async fn handle(
	ctx: OperationContext<game::recommend::Request>,
) -> GlobalResult<game::recommend::Response> {
	let game_ids = sqlx::query_as::<_, (Uuid,)>("SELECT game_id FROM games")
		.fetch_all(&ctx.crdb("db-game").await?)
		.await?
		.into_iter()
		.map(|row| row.0.into())
		.collect::<Vec<common::Uuid>>();

	Ok(game::recommend::Response { game_ids })
}
