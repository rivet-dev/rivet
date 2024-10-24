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

	op!([ctx] identity_config_version_publish {
		version_id: Some(version_id),
		config: Some(backend::identity::VersionConfig {
			custom_display_names: vec![backend::identity::CustomDisplayName {
				display_name: "chicken nugget".to_string(),
			}],
			custom_avatars: vec![backend::identity::CustomAvatar {
				upload_id: Some(Uuid::new_v4().into()),
			}],
		}),
		config_ctx: Some(backend::identity::VersionConfigCtx {}),
	})
	.await
	.unwrap();

	let res = op!([ctx] identity_config_version_get {
		version_ids: vec![version_id]
	})
	.await
	.unwrap();

	res.versions.first().expect("version not found");
}
