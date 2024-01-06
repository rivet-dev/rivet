use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] faker_team {
		..Default::default()
	})
	.await
	.unwrap();

	let get_res = op!([ctx] team_get {
		team_ids: vec![res.team_id.unwrap()],
	})
	.await
	.unwrap();
	assert_eq!(1, get_res.teams.len());
}

#[worker_test]
async fn dev(ctx: TestCtx) {
	let _res = op!([ctx] faker_team {

		..Default::default()
	})
	.await
	.unwrap();
}
