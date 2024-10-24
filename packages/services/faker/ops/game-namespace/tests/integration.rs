use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	op!([ctx] faker_game_namespace {
		game_id: game_res.game_id,
		version_id: game_res.version_ids.first().cloned(),
		..Default::default()
	})
	.await
	.unwrap();
}
