use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] game_validate {
		name_id: " bad-name-id".to_owned(),
		display_name: util::faker::display_name(),
	})
	.await
	.unwrap();

	assert_eq!(res.errors.len(), 1, "validation failed");
}
