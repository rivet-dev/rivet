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

	let (exists,): (bool,) =
		sqlx::query_as("SELECT EXISTS (SELECT 1 FROM db_module.modules WHERE module_id = $1)")
			.bind(module_id)
			.fetch_one(&ctx.crdb().await.unwrap())
			.await
			.unwrap();
	assert!(exists, "module not created");
}
