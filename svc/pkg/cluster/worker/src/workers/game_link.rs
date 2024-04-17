use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "cluster-game-link")]
async fn worker(ctx: &OperationContext<cluster::msg::game_link::Message>) -> GlobalResult<()> {
	let game_id = unwrap_ref!(ctx.game_id).as_uuid();
	let cluster_id = unwrap_ref!(ctx.cluster_id).as_uuid();

	sql_execute!(
		[ctx]
		"
		INSERT INTO db_cluster.games (
			game_id,
			cluster_id
		)
		VALUES ($1, $2)
		",
		game_id,
		cluster_id,
	)
	.await?;

	msg!([ctx] cluster::msg::game_link_complete(game_id, cluster_id) {
		game_id: ctx.game_id,
		cluster_id: ctx.cluster_id,
	})
	.await?;

	Ok(())
}
