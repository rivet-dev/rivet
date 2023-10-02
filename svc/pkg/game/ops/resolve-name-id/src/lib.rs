use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-resolve-name-id")]
async fn handle(
	ctx: OperationContext<game::resolve_name_id::Request>,
) -> GlobalResult<game::resolve_name_id::Response> {
	let games = sqlx::query_as::<_, (String, Uuid)>(indoc!(
		"
		SELECT name_id, game_id
		FROM db_game.games
		WHERE name_id = ANY($1)
		"
	))
	.bind(&ctx.name_ids)
	.fetch_all(&ctx.crdb().await?)
	.await?
	.into_iter()
	.map(|(name_id, game_id)| game::resolve_name_id::response::Game {
		name_id,
		game_id: Some(game_id.into()),
	})
	.collect::<Vec<_>>();

	Ok(game::resolve_name_id::Response { games })
}
