use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let team_res = op!([ctx] faker_team {
		..Default::default()
	})
	.await
	.unwrap();

	let team_id = team_res.team_id.unwrap().as_uuid();
	msg!([ctx] team_dev::msg::create(team_id) -> team::msg::update {
		team_id: team_res.team_id,
	})
	.await
	.unwrap();

	let (sql_exists,) = sqlx::query_as::<_, (bool,)>(indoc!(
		"
		SELECT EXISTS (SELECT 1 FROM db_team_dev.dev_teams WHERE team_id = $1)
		"
	))
	.bind(team_res.team_id.unwrap().as_uuid())
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();

	assert!(sql_exists, "dev team not created");
}
