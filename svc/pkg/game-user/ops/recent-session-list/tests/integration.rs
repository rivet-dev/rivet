use std::time::Duration;

use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_create_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let version_create_res = op!([ctx] game_version_create {
		game_id: game_create_res.game_id,
		display_name: util::faker::ident(),
	})
	.await
	.unwrap();

	let res = op!([ctx] game_namespace_create {
		game_id: game_create_res.game_id,
		display_name: util::faker::display_name(),
		version_id: version_create_res.version_id,
		name_id: util::faker::ident(),
	})
	.await
	.unwrap();

	let namespace_id = res.namespace_id.unwrap();
	let user_id = Uuid::new_v4();

	let game_user_res = op!([ctx] game_user_create {
		namespace_id: Some(namespace_id),
		user_id: Some(user_id.into())
	})
	.await
	.unwrap();
	let user_id = game_user_res.user_id.unwrap();
	let game_user_id = game_user_res.user_id.unwrap().as_uuid();

	// Create a game session row
	msg!([ctx] @wait game_user::msg::session_create(game_user_id) {
		game_user_id: game_user_res.game_user_id,
		refresh_jti: Some(Uuid::new_v4().into()),
	})
	.await
	.unwrap();

	tokio::time::sleep(Duration::from_secs(1)).await;

	let res = op!([ctx] game_user_recent_session_list {
		user_ids: vec![user_id],
	})
	.await
	.unwrap();

	let user = res.users.first().unwrap();
	assert_eq!(user.sessions.len(), 1, "recent game not found");
}
