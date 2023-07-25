use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let game_id = game_res.game_id.unwrap();

	let _res = op!([ctx] module_game_version_prepare {
		config: Some(backend::module::GameVersionConfig {
			dependencies: Vec::new(),
		}),
		game_id: Some(game_id),
	})
	.await
	.unwrap();
}
