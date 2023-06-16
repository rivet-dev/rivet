use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let mut game_ids = Vec::<(common::Uuid, String)>::new();
	for _ in 0..4usize {
		let create_res = op!([ctx] faker_game {
			..Default::default()
		})
		.await
		.unwrap();

		let get_res = op!([ctx] game_get {
			game_ids: vec![create_res.game_id.unwrap()],
		})
		.await
		.unwrap();
		let game_data = get_res.games.first().unwrap();

		game_ids.push((game_data.game_id.unwrap(), game_data.name_id.clone()));
	}

	let mut req_name_ids = game_ids
		.iter()
		.map(|(_, name_id)| name_id.clone())
		.collect::<Vec<_>>();
	req_name_ids.push(util::faker::ident()); // Non-existent name
	let res = op!([ctx] game_resolve_name_id {
		name_ids: req_name_ids,
	})
	.await
	.unwrap();
	assert_eq!(game_ids.len(), res.games.len());
}
