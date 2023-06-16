use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] faker_team {
		is_dev: false,
		..Default::default()
	})
	.await
	.unwrap();
	let team_id = res.team_id.as_ref().unwrap().as_uuid();

	let new_owner_user_id = Uuid::new_v4();

	msg!([ctx] team::msg::owner_transfer(team_id) -> team::msg::update {
		team_id: res.team_id,
		new_owner_user_id: Some(new_owner_user_id.into()),
	})
	.await
	.unwrap();

	let (owner_user_id,): (Uuid,) =
		sqlx::query_as("SELECT owner_user_id FROM teams WHERE team_id = $1")
			.bind(team_id)
			.fetch_one(&ctx.crdb("db-team").await.unwrap())
			.await
			.unwrap();
	assert_eq!(new_owner_user_id, owner_user_id);
}
