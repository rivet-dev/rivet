use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] team_profile_validate {
		display_name: Some("  bad display name".to_owned()),
		bio: Some("bad\n\n\n\n\n\nbio".to_owned())
	})
	.await
	.unwrap();

	assert_eq!(res.errors.len(), 2, "validation failed");
}
