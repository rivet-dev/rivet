use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn request_accepted(ctx: TestCtx) {
	let team_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	msg!([ctx] team::msg::join_request_create(team_id, user_id) -> team::msg::join_request_create_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		..Default::default()
	})
	.await
	.unwrap();

	msg!([ctx] team::msg::join_request_resolve(team_id, user_id) -> team::msg::join_request_resolve_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		resolution: true,
	})
	.await
	.unwrap();

	// Verify that join request was removed
	sqlx::query_as::<_, (i64,)>(
		"SELECT 1 FROM db_team.join_requests WHERE team_id = $1 AND user_id = $2",
	)
	.bind(team_id)
	.bind(user_id)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap_err();

	// Verify that a new team member was added
	let (_,): (i64,) =
		sqlx::query_as("SELECT 1 FROM db_team.team_members WHERE team_id = $1 AND user_id = $2")
			.bind(team_id)
			.bind(user_id)
			.fetch_one(&ctx.crdb().await.unwrap())
			.await
			.unwrap();
}

#[worker_test]
async fn request_denied(ctx: TestCtx) {
	let team_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	msg!([ctx] team::msg::join_request_create(team_id, user_id) -> team::msg::join_request_create_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		..Default::default()
	})
	.await
	.unwrap();

	msg!([ctx] team::msg::join_request_resolve(team_id, user_id) -> team::msg::join_request_resolve_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		resolution: false,
	})
	.await
	.unwrap();

	// Verify that join request was removed
	sqlx::query_as::<_, (i64,)>(
		"SELECT 1 FROM db_team.join_requests WHERE team_id = $1 AND user_id = $2",
	)
	.bind(team_id)
	.bind(user_id)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap_err();
}
