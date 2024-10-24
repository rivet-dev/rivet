use std::time::Duration;

use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let team_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();
	let banner_user_id = Uuid::new_v4();

	msg!([ctx] team::msg::member_create(team_id, user_id) -> team::msg::member_create_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		invitation: None,
	})
	.await
	.unwrap();

	msg!([ctx] team::msg::user_ban(team_id, user_id) -> team::msg::user_ban_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		banner_user_id: Some(banner_user_id.into()),
	})
	.await
	.unwrap();

	tokio::time::sleep(Duration::from_secs(2)).await;

	msg!([ctx] team::msg::user_unban(team_id, user_id) -> team::msg::user_unban_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		unbanner_user_id: Some(banner_user_id.into()),
	})
	.await
	.unwrap();

	let banned_users_res = op!([ctx] team_user_ban_get {
		members: vec![team::user_ban_get::request::Member {
			team_id: Some(team_id.into()),
			user_id: Some(user_id.into()),
		}],
	})
	.await
	.unwrap();

	assert!(
		banned_users_res.banned_users.is_empty(),
		"member not unbanned"
	);
}
