use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let team_a_res = op!([ctx] faker_team {
		is_dev: true,
		..Default::default()
	})
	.await
	.unwrap();
	let team_b_res = op!([ctx] faker_team {
		is_dev: true,
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

	let team_dev_games = op!([ctx] team_dev_game_list {
		team_ids: vec![
			team_a_res.team_id.unwrap(),
			team_b_res.team_id.unwrap(),
		],
	})
	.await
	.unwrap();
	assert_eq!(6, team_dev_games.teams[0].game_ids.len());
	assert_eq!(2, team_dev_games.teams[1].game_ids.len());
}
