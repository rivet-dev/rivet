use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	op!([ctx] cloud_game_token_create {
		game_id: game_res.game_id,
	})
	.await
	.unwrap();
}
