use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let game_id = game_res.game_id.as_ref().unwrap().as_uuid();

	let _res = op!([ctx] kv_config_version_prepare {
		config: Some(backend::kv::VersionConfig {}),
		game_id: Some(game_id.into()),
	})
	.await
	.unwrap();
}
