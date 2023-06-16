use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] game_user_create {
		namespace_id: Some(Uuid::new_v4().into()),
		user_id: Some(Uuid::new_v4().into()),
	})
	.await
	.unwrap();

	let create_res = op!([ctx] game_user_link_create {
		game_user_id: res.game_user_id,
	})
	.await
	.unwrap();
	let create_token = rivet_claims::decode(&create_res.user_link_token)
		.unwrap()
		.unwrap();

	let new_user_id = Uuid::new_v4();

	msg!([ctx] game_user::msg::link_complete(create_res.link_id.unwrap()) -> game_user::msg::link_complete_complete {
		link_id: create_res.link_id,
		user_link_jti: Some(create_token.jti.unwrap()),
		user_id: Some(new_user_id.into()),
		resolution: game_user::msg::link_complete::GameUserLinkCompleteResolution::Complete as i32,
	})
	.await
	.unwrap();

	let res = op!([ctx] game_user_link_get {
		link_ids: vec![create_res.link_id.unwrap()]
	})
	.await
	.unwrap();

	let game_user_link = res.game_user_links.first().unwrap();

	assert_eq!(
		game_user::link_get::response::GameUserLinkStatus::Complete as i32,
		game_user_link.status,
		"status did not update"
	);

	let game_user_id = game_user_link.new_game_user_id.unwrap();
	let res = op!([ctx] game_user_get {
		game_user_ids: vec![game_user_id],
	})
	.await
	.unwrap();

	assert_eq!(
		*res.game_users.first().unwrap().user_id.unwrap(),
		new_user_id,
		"game user link did not update"
	);
}

#[worker_test]
async fn cancel(ctx: TestCtx) {
	let res = op!([ctx] game_user_create {
		namespace_id: Some(Uuid::new_v4().into()),
		user_id: Some(Uuid::new_v4().into()),
	})
	.await
	.unwrap();

	let create_res = op!([ctx] game_user_link_create {
		game_user_id: res.game_user_id,
	})
	.await
	.unwrap();
	let create_token = rivet_claims::decode(&create_res.user_link_token)
		.unwrap()
		.unwrap();

	let new_user_id = Uuid::new_v4();

	msg!([ctx] game_user::msg::link_complete(create_res.link_id.unwrap()) -> game_user::msg::link_complete_complete {
		link_id: create_res.link_id,
		user_link_jti: Some(create_token.jti.unwrap()),
		user_id: Some(new_user_id.into()),
		resolution: game_user::msg::link_complete::GameUserLinkCompleteResolution::Cancel as i32,
	})
	.await
	.unwrap();

	let res = op!([ctx] game_user_link_get {
		link_ids: vec![create_res.link_id.unwrap()]
	})
	.await
	.unwrap();

	let game_user_link = res.game_user_links.first().unwrap();

	assert_eq!(
		game_user::link_get::response::GameUserLinkStatus::Cancelled as i32,
		game_user_link.status,
		"status did not update"
	);
}
