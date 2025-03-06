use chirp_worker::prelude::*;

// TODO: Fix tests by using faker-team and making it a dev team

#[worker_test]
async fn empty(ctx: TestCtx) {
	let team_res = op!([ctx] faker_team {

		..Default::default()
	})
	.await
	.unwrap();

	let res = op!([ctx] game_create {
		name_id: util::faker::ident(),
		display_name: util::faker::display_name(),
		developer_team_id: team_res.team_id,
	})
	.await
	.unwrap();
	let _game_id = res.game_id.unwrap();
}

#[worker_test]
async fn duplicate_name_id(ctx: TestCtx) {
	let name_id = util::faker::ident();

	let team_res = op!([ctx] faker_team {

		..Default::default()
	})
	.await
	.unwrap();

	op!([ctx] game_create {
		name_id: name_id.clone(),
		display_name: util::faker::display_name(),
		developer_team_id: team_res.team_id,
	})
	.await
	.unwrap();

	// Create game with duplicate name id
	op!([ctx] game_create {
		name_id: name_id.clone(),
		display_name: util::faker::display_name(),
		developer_team_id: team_res.team_id,
	})
	.await
	.unwrap_err();
}
