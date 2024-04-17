use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "cluster-get-for-game")]
pub async fn handle(
	ctx: OperationContext<cluster::get_for_game::Request>,
) -> GlobalResult<cluster::get_for_game::Response> {
	let game_ids = ctx
		.game_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let rows = sql_fetch_optional!(
		[ctx, (Uuid, Option<Uuid>)]
		"
		SELECT
			g.game_id, gc.cluster_id
		FROM unnest($1) AS g(game_id)
		LEFT JOIN db_cluster.games AS gc
		ON g.game_id = gc.game_id
		",
		game_ids,
	)
	.await?;

	Ok(cluster::get_for_game::Response {
		games: rows
			.into_iter()
			.map(
				|(game_id, cluster_id)| cluster::get_for_game::response::Game {
					game_id: Some(game_id.into()),
					cluster_id: Some(
						cluster_id
							.unwrap_or_else(util::env::default_cluster_id)
							.into(),
					),
				},
			)
			.collect::<Vec<_>>(),
	})
}
