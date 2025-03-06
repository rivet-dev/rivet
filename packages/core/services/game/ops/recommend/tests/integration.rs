use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let mut game_ids = Vec::new();
	for _ in 0..10usize {
		let create_res = op!([ctx] faker_game {
			..Default::default()
		})
		.await
		.unwrap();
		let game_id = create_res.game_id.as_ref().unwrap().as_uuid();
		game_ids.push(game_id);
	}

	let res = op!([ctx] game_recommend {}).await.unwrap();

	assert!(res.game_ids.len() >= 10);
}
