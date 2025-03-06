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

	let caches = ctx
		.cache()
		.immutable()
		.fetch_all_proto(
			"game_ids_from_namespace_ids",
			namespace_ids,
			|mut cache, namespace_ids| {
				let ctx = ctx.base();
				async move {
					let game_rows = sql_fetch_all!(
						[ctx, GameRow]
						"
						SELECT game_id, namespace_id
						FROM db_game.game_namespaces
						WHERE namespace_id = ANY($1)
						",
						&namespace_ids,
					)
					.await?;

					for row in game_rows {
						cache.resolve(
							&row.namespace_id,
							game::resolve_namespace_id::Cache {
								game_id: Some(row.game_id.into()),
								namespace_id: Some(row.namespace_id.into()),
							},
						);
					}

					Ok(cache)
				}
			},
		)
		.await?;

	let mut games = HashMap::<Uuid, Vec<Uuid>>::new();

	// Collect rows into hashmap
	for row in caches {
		let entry = games.entry(unwrap!(row.game_id).as_uuid()).or_default();
		entry.push(unwrap!(row.namespace_id).as_uuid());
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
