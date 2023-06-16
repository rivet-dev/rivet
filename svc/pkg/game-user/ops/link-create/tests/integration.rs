use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	let game_user_res = op!([ctx] game_user_create {
		namespace_id: Some(namespace_id.into()),
		user_id: Some(user_id.into()),
	})
	.await
	.unwrap();

	let res = op!([ctx] game_user_link_create {
		game_user_id: game_user_res.game_user_id,
	})
	.await
	.unwrap();

	tracing::info!(link=?util::route::identity_game_link(&res.user_link_token));

	let res = op!([ctx] game_user_link_get {
		link_ids: vec![res.link_id.unwrap()]
	})
	.await
	.unwrap();

	assert_eq!(res.game_user_links.len(), 1, "game user link not found");
}
