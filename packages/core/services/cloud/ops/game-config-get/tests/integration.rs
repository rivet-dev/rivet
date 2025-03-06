use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_ids = std::iter::repeat_with(Uuid::new_v4)
		.take(8)
		.collect::<Vec<_>>();
	let game_ids_proto = game_ids
		.iter()
		.cloned()
		.map(Into::<common::Uuid>::into)
		.collect::<Vec<_>>();

	for game_id in &game_ids_proto {
		op!([ctx] cloud_game_config_create {
			game_id: Some(*game_id),
		})
		.await
		.unwrap();
	}

	let res = op!([ctx] cloud_game_config_get {
		game_ids: game_ids_proto.clone(),
	})
	.await
	.unwrap();
	assert_eq!(8, res.game_configs.len());
}
