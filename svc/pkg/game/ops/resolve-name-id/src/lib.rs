use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-resolve-name-id")]
async fn handle(
	ctx: OperationContext<game::resolve_name_id::Request>,
) -> GlobalResult<game::resolve_name_id::Response> {
	let games = ctx
		.cache()
		.fetch_all_proto("game_resolved", ctx.name_ids.clone(), {
			let ctx = ctx.clone();
			move |mut cache, name_ids| {
				let ctx = ctx.clone();
				async move {
					let games = sql_fetch_all!(
						[ctx, (String, Uuid)]
						"
						SELECT name_id, game_id
						FROM db_game.games
						WHERE name_id = ANY($1)
						",
						&name_ids,
					)
					.await?;

					for (name_id, game_id) in games {
						let proto = game::resolve_name_id::response::Game {
							name_id: name_id.clone(),
							game_id: Some(game_id.into()),
						};
						cache.resolve(&name_id, proto);
					}

					Ok(cache)
				}
			}
		})
		.await?;

	Ok(game::resolve_name_id::Response { games })
}
