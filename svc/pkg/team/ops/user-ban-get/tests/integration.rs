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

	// Verify that the member exists
	let (_,): (i64,) =
		sqlx::query_as("SELECT 1 FROM db_team.team_members WHERE team_id = $1 AND user_id = $2")
			.bind(team_id)
			.bind(user_id)
			.fetch_one(&ctx.crdb().await.unwrap())
			.await
			.unwrap();

	msg!([ctx] team::msg::user_ban(team_id, user_id) -> team::msg::user_ban_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		banner_user_id: Some(banner_user_id.into()),
	})
	.await
	.unwrap();

	// Verify that the member no longer exists
	sqlx::query_as::<_, (i64,)>(
		"SELECT 1 FROM db_team.team_members WHERE team_id = $1 AND user_id = $2",
	)
	.bind(team_id)
	.bind(user_id)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap_err();

	let banned_users_res = op!([ctx] team_user_ban_get {
		members: vec![team::user_ban_get::request::Member {
			team_id: Some(team_id.into()),
			user_id: Some(user_id.into()),
		}],
	})
	.await
	.unwrap();

	assert_eq!(1, banned_users_res.banned_users.len(), "member not banned");
}
