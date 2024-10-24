use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] team_get {
		team_ids: Vec::new(),
	})
	.await
	.unwrap();
	assert!(res.teams.is_empty());
}

#[worker_test]
async fn fetch(ctx: TestCtx) {
	struct TestTeam {
		team_id: Uuid,
		display_name: String,
	}

	let mut teams = std::iter::repeat_with(|| TestTeam {
		team_id: Uuid::new_v4(),
		display_name: util::faker::display_name(),
	})
	.take(8)
	.collect::<Vec<_>>();

	let owner_user_id = Uuid::new_v4();

	for team in &mut teams {
		msg!([ctx] team::msg::create(team.team_id) -> team::msg::create_complete {
			team_id: Some(team.team_id.into()),
			display_name: team.display_name.clone(),
			owner_user_id: Some(owner_user_id.into())
		})
		.await
		.unwrap();
	}

	let res = op!([ctx] team_get {
		team_ids: teams.iter().map(|u| u.team_id).map(Into::<common::Uuid>::into).collect(),
	})
	.await
	.unwrap();

	assert_eq!(teams.len(), res.teams.len());
	for team in &teams {
		let team_res = res
			.teams
			.iter()
			.find(|u| u.team_id.as_ref().unwrap().as_uuid() == team.team_id)
			.expect("team not returned");
		assert_eq!(team.display_name, team_res.display_name);
	}
}
