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
	let game_user_id = res.game_user_id.unwrap().as_uuid();

	let res = op!([ctx] game_user_link_create {
		game_user_id: Some(game_user_id.into()),
	})
	.await
	.unwrap();

	let link_id = res.link_id.unwrap().as_uuid();

	let res = op!([ctx] game_user_link_get {
		link_ids: vec![link_id.into()]
	})
	.await
	.unwrap();

	assert_eq!(
		res.game_user_links.len(),
		1,
		"game user link status not found"
	);
}
