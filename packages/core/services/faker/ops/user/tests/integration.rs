use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] faker_user {}).await.unwrap();

	let get_res = op!([ctx] user_get {
		user_ids: vec![res.user_id.unwrap()],
	})
	.await
	.unwrap();

	assert_eq!(1, get_res.users.len(), "user not created");
}
