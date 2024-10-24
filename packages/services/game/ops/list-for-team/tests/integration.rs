use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let team_a_res = op!([ctx] faker_team {

		..Default::default()
	})
	.await
	.unwrap();
	let team_b_res = op!([ctx] faker_team {

		..Default::default()
	})
	.await
	.unwrap();

	for i in 0..8usize {
		let dev_team_id = if i < 6 {
			team_a_res.team_id
		} else {
			team_b_res.team_id
		};

		op!([ctx] faker_game {
			dev_team_id: dev_team_id,
			..Default::default()
		})
		.await
		.unwrap();
	}

	let teams = op!([ctx] game_list_for_team {
		team_ids: vec![
			team_a_res.team_id.unwrap(),
			team_b_res.team_id.unwrap(),
		],
	})
	.await
	.unwrap();
	assert_eq!(6, teams.teams[0].game_ids.len());
	assert_eq!(2, teams.teams[1].game_ids.len());
}
