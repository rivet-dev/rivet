use chirp_worker::prelude::*;
use proto::backend::{self, module::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let version_id = Into::<common::Uuid>::into(Uuid::new_v4());

	op!([ctx] module_game_version_publish {
		version_id: Some(version_id),
		config: Some(backend::module::GameVersionConfig {
			module_dependencies: Vec::new()
		}),
		config_ctx: Some(backend::module::GameVersionConfigCtx {}),
	})
	.await
	.unwrap();

	let res = op!([ctx] module_game_version_get {
		version_ids: vec![version_id]
	})
	.await
	.unwrap();
	let version = res.versions.first().expect("version not found");
}
