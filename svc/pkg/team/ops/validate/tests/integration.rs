use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] team_validate {
		display_name: "bad display   name".to_owned(),
	})
	.await
	.unwrap();

	assert_eq!(res.errors.len(), 1, "validation failed");
}
