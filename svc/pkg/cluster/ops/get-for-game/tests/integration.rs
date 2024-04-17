use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_id = Uuid::new_v4();
	let cluster_id = Uuid::new_v4();

	msg!([ctx] cluster::msg::game_link(game_id, cluster_id) -> cluster::msg::game_link_complete {
		game_id: Some(game_id.into()),
		cluster_id: Some(cluster_id.into()),
	})
	.await
	.unwrap();

	let games_res = op!([ctx] cluster_get_for_game {
		game_ids: vec![game_id.into()],
	})
	.await
	.unwrap();
	let game = games_res.games.first().unwrap();

	assert_eq!(cluster_id, game.cluster_id.unwrap().as_uuid());
}
