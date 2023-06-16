use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let team_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();
	let kicker_user_id = Uuid::new_v4();

	msg!([ctx] team::msg::member_create(team_id, user_id) -> team::msg::member_create_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		invitation: None,
	})
	.await
	.unwrap();

	// Verify that the member exists
	let (_,): (i64,) =
		sqlx::query_as("SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2")
			.bind(team_id)
			.bind(user_id)
			.fetch_one(&ctx.crdb("db-team").await.unwrap())
			.await
			.unwrap();

	msg!([ctx] team::msg::member_kick(team_id, user_id) -> team::msg::member_kick_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		kicker_user_id: Some(kicker_user_id.into()),
	})
	.await
	.unwrap();

	// Verify that the member no longer exists
	sqlx::query_as::<_, (i64,)>("SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2")
		.bind(team_id)
		.bind(user_id)
		.fetch_one(&ctx.crdb("db-team").await.unwrap())
		.await
		.unwrap_err();
}
