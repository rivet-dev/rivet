use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
struct CallRequest {
	function_name: String,
	request: serde_json::Value,
}

#[derive(Deserialize)]
struct CallResponse {
	response: serde_json::Value,
}

#[operation(name = "module-instance-call")]
pub async fn handle(
	ctx: OperationContext<module::instance_call::Request>,
) -> GlobalResult<module::instance_call::Response> {
	let instance_id = internal_unwrap!(ctx.instance_id).as_uuid();

	// Get instance
	let instances = op!([ctx] module_instance_get {
		instance_ids: vec![instance_id.into()],
	})
	.await?;
	let instance = internal_unwrap_owned!(instances.instances.first());
	let version_id = internal_unwrap!(instance.module_version_id).as_uuid();

	// Get version
	let versions = op!([ctx] module_version_get {
		version_ids: vec![version_id.into()],
	})
	.await?;
	let version = internal_unwrap_owned!(versions.versions.first());

	// Validate function exists
	internal_assert!(
		version
			.functions
			.iter()
			.any(|x| x.name == ctx.function_name),
		"function does not exist"
	);

	// TODO: Validate JSON request schema

	// Handle driver
	let url = match internal_unwrap!(instance.driver) {
		backend::module::instance::Driver::Dummy(_) => {
			return Ok(module::instance_call::Response {
				response_json: "{}".into(),
			})
		}
		backend::module::instance::Driver::Fly(driver) => {
			if let Some(app_id) = &driver.fly_app_id {
				format!("https://{}.fly.dev/call", app_id)
			} else {
				internal_panic!("fly app not created yet");
			}
		}
	};

	// Call module
	let request_json = serde_json::from_str::<serde_json::Value>(&ctx.request_json)?;
	let response = reqwest::Client::new()
		.post(url)
		.timeout(Duration::from_secs(15))
		.json(&CallRequest {
			function_name: ctx.function_name.clone(),
			request: request_json,
		})
		.send()
		.await?;
	let res_body = response.json::<CallResponse>().await?;
	let response_json = serde_json::to_string(&res_body.response)?;

	// TODO: Validate JSON response schema

	Ok(module::instance_call::Response { response_json })
}
