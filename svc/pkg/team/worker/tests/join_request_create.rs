use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let team_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	msg!([ctx] team::msg::join_request_create(team_id, user_id) -> team::msg::join_request_create_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		..Default::default()
	})
	.await
	.unwrap();

	let (sql_exists,): (bool,) = sqlx::query_as(
		"SELECT EXISTS (SELECT 1 FROM db_team.join_requests WHERE team_id = $1 AND user_id = $2)",
	)
	.bind(team_id)
	.bind(user_id)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();

	assert!(sql_exists, "join request does not exist");
}
