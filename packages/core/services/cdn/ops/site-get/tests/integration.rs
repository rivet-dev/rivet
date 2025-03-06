use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let site_res = op!([ctx] faker_cdn_site {
		game_id: game_res.game_id,
	})
	.await
	.unwrap();
	let site_id = site_res.site_id.as_ref().unwrap().as_uuid();

	op!([ctx] cdn_site_get {
		site_ids: vec![site_id.into()],
	})
	.await
	.unwrap();
}
