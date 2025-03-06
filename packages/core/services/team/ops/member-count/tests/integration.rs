use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let teams = vec![Uuid::new_v4(), Uuid::new_v4()];
	for team_id in &teams {
		for _ in 0..10 {
			let user_id = Uuid::new_v4();
			msg!([ctx] team::msg::member_create(team_id, user_id) -> team::msg::member_create_complete {
				team_id: Some((*team_id).into()),
				user_id: Some(user_id.into()),
				invitation: None,
			})
			.await
			.unwrap();
		}
	}

	let res = op!([ctx] team_member_count {
		team_ids: teams.iter().map(|id| (*id).into()).collect(),
	})
	.await
	.unwrap();
	assert_eq!(2, res.teams.len());
	assert!(res.teams.iter().all(|t| t.member_count == 10));
}
