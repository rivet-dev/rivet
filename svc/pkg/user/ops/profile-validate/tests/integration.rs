use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_res = op!([ctx] faker_user {}).await.unwrap();
	let user_id = user_res.user_id.unwrap();

	let res = op!([ctx] user_profile_validate {
		user_id: Some(user_id),
		display_name: Some("  bad display name".to_owned()),
		account_number: Some(10000),
		bio: Some("bad\n\n\n\n\n\nbio".to_owned())
	})
	.await
	.unwrap();

	assert_eq!(res.errors.len(), 3, "validation failed");
}
