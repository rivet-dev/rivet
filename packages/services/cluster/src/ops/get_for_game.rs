use chirp_workflow::prelude::*;

#[derive(Debug)]
pub struct Input {
	pub game_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub games: Vec<Game>,
}

#[derive(Debug)]
pub struct Game {
	pub game_id: Uuid,
	pub cluster_id: Uuid,
}

#[operation]
pub async fn cluster_get_for_game(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let rows = sql_fetch_optional!(
		[ctx, (Uuid, Option<Uuid>)]
		"
		SELECT
			g.game_id, gc.cluster_id
		FROM unnest($1) AS g(game_id)
		LEFT JOIN db_cluster.games AS gc
		ON g.game_id = gc.game_id
		",
		&input.game_ids,
	)
	.await?;

	Ok(Output {
		games: rows
			.into_iter()
			.map(|(game_id, cluster_id)| Game {
				game_id,
				cluster_id: cluster_id.unwrap_or_else(crate::util::default_cluster_id),
			})
			.collect::<Vec<_>>(),
	})
}
