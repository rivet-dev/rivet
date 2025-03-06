use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = game_res.namespace_ids.first().unwrap();

	op!([ctx] cloud_namespace_token_public_create {
		namespace_id: Some(*namespace_id),
	})
	.await
	.unwrap();
}
