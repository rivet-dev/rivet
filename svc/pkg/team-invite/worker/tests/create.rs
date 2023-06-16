use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let team_id = Uuid::new_v4();
	msg!([ctx] team_invite::msg::create(team_id) -> team_invite::msg::create_complete {
		team_id: Some(team_id.into()),
		ttl: Some(1000),
		max_use_count: Some(3),
	})
	.await
	.unwrap();
}
