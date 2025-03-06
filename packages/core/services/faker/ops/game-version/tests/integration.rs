use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		skip_namespaces_and_versions: true,
		..Default::default()
	})
	.await
	.unwrap();

	let _res = op!([ctx] faker_game_version {
		game_id: game_res.game_id,
		..Default::default()
	})
	.await
	.unwrap();
}
