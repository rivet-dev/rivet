use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let team_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();

	let invite_res =
		msg!([ctx] team_invite::msg::create(team_id) -> team_invite::msg::create_complete {
			team_id: Some(team_id.into()),
			ttl: None,
			max_use_count: None,
		})
		.await
		.unwrap();

	let member_res = msg!([ctx] team_invite::msg::consume(&invite_res.code, team_id) -> team::msg::member_create(team_id, user_id) {
		code: invite_res.code.clone(),
		user_id: Some(user_id.into()),
	})
	.await
	.unwrap();
	assert_eq!(
		team_id,
		member_res.team_id.as_ref().unwrap().as_uuid(),
		"joined wrong team"
	);
	assert_eq!(
		user_id,
		member_res.user_id.as_ref().unwrap().as_uuid(),
		"wrong user joined"
	);
}

// TODO: Test all error codes
