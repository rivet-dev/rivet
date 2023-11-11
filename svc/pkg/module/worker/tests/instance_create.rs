use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn instance_create_dummy(ctx: TestCtx) {
	if !util::feature::fly() {
		return;
	}

	let module_id = Uuid::new_v4();
	let version_id = Uuid::new_v4();
	let instance_id = Uuid::new_v4();

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
			image_tag: "ghcr.io/rivet-gg/rivet-module-hello-world:0.0.1".into(),
		})),
	})
	.await
	.unwrap();

	msg!([ctx] module::msg::instance_create(instance_id) -> module::msg::instance_create_complete {
		instance_id: Some(instance_id.into()),
		module_version_id: Some(version_id.into()),
		driver: Some(module::msg::instance_create::message::Driver::Dummy(module::msg::instance_create::message::Dummy {})),
	})
	.await
	.unwrap();
}

#[worker_test]
async fn instance_create_fly(ctx: TestCtx) {
	if !util::feature::fly() {
		return;
	}

	let module_id = Uuid::new_v4();
	let version_id = Uuid::new_v4();
	let instance_id = Uuid::new_v4();

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
			image_tag: "ghcr.io/rivet-gg/rivet-module-hello-world:0.0.1".into(),
		})),
	})
	.await
	.unwrap();

	msg!([ctx] module::msg::instance_create(instance_id) -> module::msg::instance_create_complete {
		instance_id: Some(instance_id.into()),
		module_version_id: Some(version_id.into()),
		driver: Some(module::msg::instance_create::message::Driver::Fly(module::msg::instance_create::message::Fly {})),
	})
	.await
	.unwrap();
}
