use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use std::collections::HashSet;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] module_get {
		module_ids: Vec::new(),
	})
	.await
	.unwrap();
	assert!(res.modules.is_empty());
}

#[worker_test]
async fn fetch(ctx: TestCtx) {
	let module_id = Uuid::new_v4();
	msg!([ctx] module::msg::create(module_id) -> module::msg::create_complete(module_id) {
		module_id: Some(module_id.into()),
		name_id: "test".into(),
		team_id: Some(Uuid::new_v4().into()),
		creator_user_id: None,
	})
	.await
	.unwrap();

	// Generate test versions
	let version_ids = std::iter::repeat_with(Uuid::new_v4)
		.take(8)
		.collect::<HashSet<_>>();

	// Insert test modules
	for version_id in &version_ids {
		msg!([ctx] module::msg::version_create(version_id) -> module::msg::version_create_complete {
			version_id: Some((*version_id).into()),
			module_id: Some(module_id.into()),
			creator_user_id: None,

			major: 1,
			minor: 0,
			patch: 0,

			functions: vec![
				backend::module::Function {
					name: "foo".into(),
					request_schema: "{}".into(),
					response_schema: "{}".into(),
					callable: Some(backend::module::function::Callable {}),
				},
			],

			image: Some(module::msg::version_create::message::Image::Docker(module::msg::version_create::message::Docker {
				image_tag: "test".into(),
			})),
		}).await.unwrap();
	}

	// Fetch the versions
	let res = op!([ctx] module_version_get {
		version_ids: version_ids.iter().cloned().map(|x| x.into()).collect(),
	})
	.await
	.unwrap();

	// Validate the modules
	assert_eq!(version_ids.len(), res.versions.len());
}
