use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let game_version_res = op!([ctx] faker_game_version {
		game_id: game_res.game_id,
		..Default::default()
	})
	.await
	.unwrap();

	let res = op!([ctx] identity_config_version_get {
		version_ids: vec![game_version_res.version_id.unwrap()]
	})
	.await
	.unwrap();

	res.versions.first().expect("version not found");
}
