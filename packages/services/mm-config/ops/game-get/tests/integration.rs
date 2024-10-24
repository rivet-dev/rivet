use chirp_worker::prelude::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
	// TODO: Test this better
	op!([ctx] mm_config_game_get {
		game_ids: vec![]
	})
	.await
	.unwrap();
}
