use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let team_res = op!([ctx] faker_team {
		is_dev: true,
		..Default::default()
	})
	.await
	.unwrap();

	let res = op!([ctx] team_dev_get {
		team_ids: vec![team_res.team_id.unwrap()]
	})
	.await
	.unwrap();

	assert!(!res.teams.is_empty(), "dev team not found");
}
