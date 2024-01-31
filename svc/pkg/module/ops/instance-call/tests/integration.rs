use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;

#[worker_test]
async fn fetch(ctx: TestCtx) {
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
	}).await.unwrap();

	// Insert test modules
	msg!([ctx] module::msg::instance_create(instance_id) -> module::msg::instance_create_complete {
		instance_id: Some(instance_id.into()),
		module_version_id: Some(version_id.into()),
		driver: Some(module::msg::instance_create::message::Driver::Fly(module::msg::instance_create::message::Fly {})),
	})
	.await
	.unwrap();

	// Call request
	let res = op!([ctx] module_instance_call {
		instance_id: Some(instance_id.into()),
		script_name: "foo".into(),
		request_json: serde_json::to_string(&json!({
			"x": 5
		})).unwrap(),
	})
	.await
	.unwrap();

	// Validate response
	let response_json = serde_json::from_str::<serde_json::Value>(&res.response_json).unwrap();
	assert_eq!(response_json["y"], 10);
}
