use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let module_id = Uuid::new_v4();
	msg!([ctx] module::msg::create(module_id) -> module::msg::create_complete {
		module_id: Some(module_id.into()),
		name_id: "test".into(),
		team_id: Some(Uuid::new_v4().into()),
		creator_user_id: None,
	})
	.await
	.unwrap();

	// TODO: Check create works
	// TODO: Check update works
	// TODO: Check delete works
}
