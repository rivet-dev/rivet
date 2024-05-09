use std::collections::HashMap;

use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let team_a = Uuid::new_v4();
	let team_b = Uuid::new_v4();
	let team_c = Uuid::new_v4();
	let team_d = Uuid::new_v4();

	let user_a = Uuid::new_v4();
	let user_b = Uuid::new_v4();
	let user_c = Uuid::new_v4();

	let entries = &[
		(team_a, user_a),
		(team_a, user_b),
		(team_b, user_a),
		(team_c, user_a),
		(team_c, user_c),
	];
	for (team_id, user_id) in entries {
		msg!([ctx] team::msg::member_create(team_id, user_id) -> team::msg::member_create_complete {
			team_id: Some((*team_id).into()),
			user_id: Some((*user_id).into()),
			invitation: None,
		})
		.await
		.unwrap();
	}

	let res = op!([ctx] team_member_list {
		team_ids: vec![team_a.into(), team_b.into(), team_c.into(), team_d.into()],
		limit: None,
		anchor: None,
	})
	.await
	.unwrap();

	assert_eq!(4, res.teams.len());
	let teams_map = res
		.teams
		.iter()
		.map(|t| (t.team_id.unwrap().as_uuid(), t.members.len()))
		.collect::<HashMap<Uuid, usize>>();
	assert_eq!(2, *teams_map.get(&team_a).unwrap());
	assert_eq!(1, *teams_map.get(&team_b).unwrap());
	assert_eq!(2, *teams_map.get(&team_c).unwrap());
	assert_eq!(0, *teams_map.get(&team_d).unwrap());
}
