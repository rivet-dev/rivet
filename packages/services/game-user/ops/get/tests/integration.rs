use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	let res = op!([ctx] game_user_create {
		namespace_id: Some(namespace_id.into()),
		user_id: Some(user_id.into())
	})
	.await
	.unwrap();

	let game_user_id = res.game_user_id.unwrap();

	let res = op!([ctx] game_user_get {
		game_user_ids: vec![game_user_id]
	})
	.await
	.unwrap();

	assert_eq!(res.game_users.len(), 1, "game game_user not found");
}
