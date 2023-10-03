use chirp_worker::prelude::*;
use proto::backend::pkg::*;

const MAX_TEAM_SIZE: usize = 256;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let team_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	msg!([ctx] team::msg::member_create(team_id, user_id) -> team::msg::member_create_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		invitation: None,
	})
	.await
	.unwrap();

	let (_,): (i64,) =
		sqlx::query_as("SELECT 1 FROM db_team.team_members WHERE team_id = $1 AND user_id = $2")
			.bind(team_id)
			.bind(user_id)
			.fetch_one(&ctx.crdb().await.unwrap())
			.await
			.unwrap();
}

#[worker_test]
async fn create_user_dev(ctx: TestCtx) {
	let user_id = Uuid::new_v4();

	let team_res = op!([ctx] faker_team {
		is_dev: true,
		..Default::default()
	})
	.await
	.unwrap();
	let team_id = team_res.team_id.as_ref().unwrap().as_uuid();

	msg!([ctx] team::msg::member_create(team_id, user_id) -> team::msg::member_create_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		invitation: None,
	})
	.await
	.unwrap();

	tokio::time::sleep(std::time::Duration::from_secs(1)).await; // HACK:
}

#[worker_test]
async fn idempotent(ctx: TestCtx) {
	let team_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	msg!([ctx] team::msg::member_create(team_id, user_id) -> team::msg::member_create_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		invitation: None,
	})
	.await
	.unwrap();

	msg!([ctx] team::msg::member_create(team_id, user_id) -> team::msg::member_create_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		invitation: None,
	})
	.await
	.unwrap();
}

#[worker_test]
async fn full(ctx: TestCtx) {
	let team_id = Uuid::new_v4();

	for _ in 0..MAX_TEAM_SIZE {
		let user_id = Uuid::new_v4();
		msg!([ctx] team::msg::member_create(team_id, user_id) -> Result<team::msg::member_create_complete, team::msg::member_create_fail> {
			team_id: Some(team_id.into()),
			user_id: Some(user_id.into()),
			invitation: None,
		})
		.await
		.unwrap().unwrap();
	}

	let user_id = Uuid::new_v4();
	let err = msg!([ctx] team::msg::member_create(team_id, user_id) -> Result<team::msg::member_create_complete, team::msg::member_create_fail> {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		invitation: None,
	})
	.await
	.unwrap().unwrap_err();
	assert!(err.error_code == team::msg::member_create_fail::ErrorCode::TeamFull as i32);
}
