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

	let _res = op!([ctx] identity_config_version_prepare {
		config: Some(backend::identity::VersionConfig {
			custom_display_names: vec![backend::identity::CustomDisplayName {
				display_name: "chicken nugget".to_string(),
			}],
			custom_avatars: vec![backend::identity::CustomAvatar {
				upload_id: Some(Uuid::new_v4().into()),
			}],
		}),
		game_id: Some(game_id.into()),
	})
	.await
	.unwrap();
}
