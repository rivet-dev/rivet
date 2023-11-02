use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-namespace-resolve-name-id")]
async fn handle(
	ctx: OperationContext<game::namespace_resolve_name_id::Request>,
) -> GlobalResult<game::namespace_resolve_name_id::Response> {
	let game_id = unwrap_ref!(ctx.game_id).as_uuid();

	let namespaces = sql_fetch_all!(
		[ctx, (String, Uuid)]
		"
		SELECT name_id, namespace_id
		FROM db_game.game_namespaces
		WHERE game_id = $1 AND name_id = ANY($2)
		",
		game_id,
		&ctx.name_ids,
	)
	.await?
	.into_iter()
	.map(
		|(name_id, namespace_id)| game::namespace_resolve_name_id::response::Namespace {
			name_id,
			namespace_id: Some(namespace_id.into()),
		},
	)
	.collect::<Vec<_>>();

	Ok(game::namespace_resolve_name_id::Response { namespaces })
}
