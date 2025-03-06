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

	let site_res = op!([ctx] faker_cdn_site {
			game_id: Some(game_id),
	})
	.await
	.unwrap();
	let site_id = site_res.site_id.unwrap();

	let _res = op!([ctx] cdn_version_prepare {
		config: Some(backend::cdn::VersionConfig {
			site_id: Some(site_id),
			routes: Vec::new(),
		}),
		game_id: Some(game_id),
	})
	.await
	.unwrap();
}
