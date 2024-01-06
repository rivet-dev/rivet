use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let build_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: backend::faker::Image::MmLobbyAutoReady as i32,
	})
	.await
	.unwrap();
	let build_id = build_res.build_id.as_ref().unwrap().as_uuid();

	let res = op!([ctx] build_get {
		build_ids: vec![build_id.into()],
	})
	.await
	.unwrap();
}
