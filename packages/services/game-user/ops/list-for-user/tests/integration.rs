use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();
	let namespace_id2 = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	op!([ctx] game_user_create {
		namespace_id: Some(namespace_id.into()),
		user_id: Some(user_id.into())
	})
	.await
	.unwrap();

	op!([ctx] game_user_create {
		namespace_id: Some(namespace_id2.into()),
		user_id: Some(user_id.into())
	})
	.await
	.unwrap();

	let res = op!([ctx] game_user_list_for_user {
		user_ids: vec![user_id.into()]
	})
	.await
	.unwrap();

	assert_eq!(
		res.users.first().unwrap().game_user_ids.len(),
		2,
		"game users not found"
	);
}
