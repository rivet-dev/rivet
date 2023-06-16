use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_res = op!([ctx] faker_user {}).await.unwrap();

	let res = op!([ctx] user_token_create {
		user_id: user_res.user_id,
		client: Some(backend::net::ClientInfo {..Default::default()})
	})
	.await
	.unwrap();

	assert!(res.token.starts_with("usr"));
	assert!(res.refresh_token.starts_with("usr_rf"));
}
