use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-namespace-resolve-name-id")]
async fn handle(
	ctx: OperationContext<game::namespace_resolve_name_id::Request>,
) -> GlobalResult<game::namespace_resolve_name_id::Response> {
	let game_id = unwrap_ref!(ctx.game_id).as_uuid();

	let namespaces = ctx
		.cache()
		.fetch_all_proto(
			"namespace_resolved",
			// Zipped so the cache resolves names per game id correctly
			ctx.name_ids
				.iter()
				.map(|name_id| (game_id, name_id.clone()))
				.collect::<Vec<_>>(),
			{
				let ctx = ctx.clone();
				move |mut cache, name_ids| {
					let ctx = ctx.clone();
					let game_id = game_id;
					async move {
						let namespaces = sql_fetch_all!(
							[ctx, (String, Uuid)]
							"
							SELECT name_id, namespace_id
							FROM db_game.game_namespaces
							WHERE game_id = $1 AND name_id = ANY($2)
							",
							game_id,
							name_ids.into_iter().map(|(_, name_id)| name_id).collect::<Vec<_>>(),
						)
						.await?;

						for (name_id, namespace_id) in namespaces {
							let proto = game::namespace_resolve_name_id::response::Namespace {
								name_id: name_id.clone(),
								namespace_id: Some(namespace_id.into()),
							};
							cache.resolve(&(game_id, name_id), proto);
						}

						Ok(cache)
					}
				}
			},
		)
		.await?;

	Ok(game::namespace_resolve_name_id::Response { namespaces })
}
