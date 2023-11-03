use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct NamespaceRow {
	namespace_id: Uuid,
	game_id: Uuid,
}

#[operation(name = "game-namespace-list")]
async fn handle(
	ctx: OperationContext<game::namespace_list::Request>,
) -> GlobalResult<game::namespace_list::Response> {
	let game_ids = ctx
		.game_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let namespace_rows = sql_fetch_all!(
		[ctx, NamespaceRow]
		"
		SELECT namespace_id, game_id
		FROM db_game.game_namespaces
		WHERE game_id = ANY($1)
		",
		&game_ids,
	)
	.await?;

	let games = game_ids
		.iter()
		.map(|game_id| game::namespace_list::response::Game {
			game_id: Some((*game_id).into()),
			namespace_ids: namespace_rows
				.iter()
				.filter(|r| r.game_id == *game_id)
				.map(|row| common::Uuid::from(row.namespace_id))
				.collect::<Vec<_>>(),
		})
		.collect::<Vec<_>>();

	Ok(game::namespace_list::Response { games })
}
