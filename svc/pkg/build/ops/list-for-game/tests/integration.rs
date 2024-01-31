use chirp_worker::prelude::*;
use proto::backend::{self};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		skip_namespaces_and_versions: true,
		..Default::default()
	})
	.await
	.unwrap();

	let _build_a_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: backend::faker::Image::MmLobbyAutoReady as i32,
	})
	.await
	.unwrap();

	let _build_b_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: backend::faker::Image::MmLobbyAutoReady as i32,
	})
	.await
	.unwrap();

	let res = op!([ctx] build_list_for_game {
		game_id: game_res.game_id,
	})
	.await
	.unwrap();
	assert_eq!(2, res.build_ids.len());
}
