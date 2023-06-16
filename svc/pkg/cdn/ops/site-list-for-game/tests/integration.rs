use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		skip_namespaces_and_versions: true,
		..Default::default()
	})
	.await
	.unwrap();

	let site_a_res = op!([ctx] faker_cdn_site {
		game_id: game_res.game_id,
	})
	.await
	.unwrap();

	let site_b_res = op!([ctx] faker_cdn_site {
		game_id: game_res.game_id,
	})
	.await
	.unwrap();

	let res = op!([ctx] cdn_site_list_for_game {
		game_id: game_res.game_id,
	})
	.await
	.unwrap();
	assert_eq!(2, res.site_ids.len());
}
