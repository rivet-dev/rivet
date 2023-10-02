use std::collections::HashMap;

use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(Debug, sqlx::FromRow)]
struct GameRow {
	game_id: Uuid,
	namespace_id: Uuid,
}

#[operation(name = "game-resolve-namespace-id")]
async fn handle(
	ctx: OperationContext<game::resolve_namespace_id::Request>,
) -> GlobalResult<game::resolve_namespace_id::Response> {
	let namespace_ids = ctx
		.namespace_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let game_rows = sqlx::query_as::<_, GameRow>(indoc!(
		"
		SELECT game_id, namespace_id
		FROM db_game.game_namespaces
		WHERE namespace_id = ANY($1)
		"
	))
	.bind(&namespace_ids)
	.fetch_all(&ctx.crdb().await?)
	.await?;

	let mut games = HashMap::<Uuid, Vec<Uuid>>::new();

	// Collect rows into hashmap
	for row in &game_rows {
		let entry = games.entry(row.game_id).or_insert_with(Vec::new);
		entry.push(row.namespace_id);
	}

	Ok(game::resolve_namespace_id::Response {
		games: games
			.into_iter()
			.map(
				|(game_id, namespace_ids)| game::resolve_namespace_id::response::Game {
					game_id: Some(game_id.into()),
					namespace_ids: namespace_ids
						.into_iter()
						.map(Into::into)
						.collect::<Vec<_>>(),
				},
			)
			.collect::<Vec<_>>(),
	})
}
