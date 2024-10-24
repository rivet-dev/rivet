use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let team_id = Uuid::new_v4();

	let invite_res =
		msg!([ctx] team_invite::msg::create(team_id) -> team_invite::msg::create_complete {
			team_id: Some(team_id.into()),
			ttl: None,
			max_use_count: None,
		})
		.await
		.unwrap();

	let invite_res = op!([ctx] team_invite_get {
		codes: vec![invite_res.code.clone()],
	})
	.await
	.unwrap();
	assert!(!invite_res.invites.is_empty(), "invite not found");
}
