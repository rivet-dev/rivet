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

	let res = op!([ctx] team_member_get {
		members: entries.iter().map(|(team_id, user_id)| team::member_get::request::TeamMember {
			team_id: Some((*team_id).into()),
			user_id: Some((*user_id).into()),
		}).collect::<Vec<_>>(),
	})
	.await
	.unwrap();

	assert_eq!(entries.len(), res.members.len());
}
