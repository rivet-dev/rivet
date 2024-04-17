use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn create(ctx: TestCtx) {
	let game_id = Uuid::new_v4();
	let cluster_id = Uuid::new_v4();

	msg!([ctx] cluster::msg::game_link(game_id, cluster_id) -> cluster::msg::game_link_complete {
		game_id: Some(game_id.into()),
		cluster_id: Some(cluster_id.into()),
	})
	.await
	.unwrap();
}
