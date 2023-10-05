use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-resolve-name-id")]
async fn handle(
	ctx: OperationContext<game::resolve_name_id::Request>,
) -> GlobalResult<game::resolve_name_id::Response> {
	let crdb = ctx.crdb().await?;

	let games = ctx
		.cache()
		.fetch_all_proto("user", ctx.name_ids.clone(), move |mut cache, name_ids| {
			let crdb = crdb.clone();
			async move {
				let games = sqlx::query_as::<_, (String, Uuid)>(indoc!(
					"
					SELECT name_id, game_id
					FROM db_game.games
					WHERE name_id = ANY($1)
					"
				))
				.bind(&name_ids)
				.fetch_all(&crdb)
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
		})
		.await?;

	Ok(game::resolve_name_id::Response { games })
}
