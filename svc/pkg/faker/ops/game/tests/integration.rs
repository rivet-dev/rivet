use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let get_res = op!([ctx] game_get {
		game_ids: vec![res.game_id.unwrap()],
	})
	.await
	.unwrap();
	assert_eq!(1, get_res.games.len(), "game not created");
}
