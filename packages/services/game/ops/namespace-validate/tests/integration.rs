use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let res = op!([ctx] game_namespace_validate {
		game_id: Some(game_res.game_id.unwrap()),
		name_id: " bad-name-id".to_owned(),
		display_name: util::faker::display_name(),
	})
	.await
	.unwrap();

	assert_eq!(res.errors.len(), 1, "validation failed");
}
