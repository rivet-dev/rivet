use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let module_id = Uuid::new_v4();
	let version_id = Uuid::new_v4();

	msg!([ctx] module::msg::create(module_id) -> module::msg::create_complete {
		module_id: Some(module_id.into()),
		name_id: "test".into(),
		team_id: Some(Uuid::new_v4().into()),
		creator_user_id: None,
	})
	.await
	.unwrap();

	msg!([ctx] module::msg::version_create(version_id) -> module::msg::version_create_complete {
		version_id: Some(version_id.into()),
		module_id: Some(module_id.into()),
		creator_user_id: None,

		major: 1,
		minor: 0,
		patch: 0,

		scripts: vec![
			backend::module::Script {
				name: "foo".into(),
				request_schema: "{}".into(),
				response_schema: "{}".into(),
				callable: Some(backend::module::script::Callable {}),
			},
		],

		image: Some(module::msg::version_create::message::Image::Docker(module::msg::version_create::message::Docker {
			image_tag: "ghcr.io/rivet-gg/rivet-module-hello-world:latest".into(),
		})),
	})
	.await
	.unwrap();

	let crdb = ctx.crdb().await.unwrap();

	let (exists,): (bool,) =
		sqlx::query_as("SELECT EXISTS (SELECT 1 FROM db_module.versions WHERE version_id = $1)")
			.bind(version_id)
			.fetch_one(&crdb)
			.await
			.unwrap();
	assert!(exists, "version not created");

	let (exists,): (bool,) = sqlx::query_as(
		"SELECT EXISTS (SELECT 1 FROM db_module.scripts WHERE version_id = $1 AND name = 'foo')",
	)
	.bind(version_id)
	.fetch_one(&crdb)
	.await
	.unwrap();
	assert!(exists, "script not created");

	let (exists,): (bool,) = sqlx::query_as(
		"SELECT EXISTS (SELECT 1 FROM db_module.scripts_callable WHERE version_id = $1 AND name = 'foo')",
	)
	.bind(version_id)
	.fetch_one(&crdb)
	.await
	.unwrap();
	assert!(exists, "script not callable");
}
