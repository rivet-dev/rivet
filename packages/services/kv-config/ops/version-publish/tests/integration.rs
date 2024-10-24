use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let _game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let version_id = Into::<common::Uuid>::into(Uuid::new_v4());

	op!([ctx] kv_config_version_publish {
		version_id: Some(version_id),
		config: Some(backend::kv::VersionConfig {}),
		config_ctx: Some(backend::kv::VersionConfigCtx {}),
	})
	.await
	.unwrap();

	let res = op!([ctx] kv_config_version_get {
		version_ids: vec![version_id]
	})
	.await
	.unwrap();

	res.versions.first().expect("version not found");
}
