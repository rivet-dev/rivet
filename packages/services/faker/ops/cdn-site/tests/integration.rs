use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		skip_namespaces_and_versions: true,
		..Default::default()
	})
	.await
	.unwrap();

	op!([ctx] faker_cdn_site {
		game_id: game_res.game_id,
	})
	.await
	.unwrap();
}
