use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
struct CallRequest<'a> {
	script_name: String,
	request: &'a serde_json::Value,
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
	assert_with!(instance.destroy_ts.is_none(), MODULE_INSTANCE_DESTROYED);

	// Get version
	let versions = op!([ctx] module_version_get {
		version_ids: vec![version_id.into()],
	})
	.await?;
	let version = internal_unwrap_owned!(versions.versions.first());

	// Validate script exists
	assert_with!(
		version.scripts.iter().any(|x| x.name == ctx.script_name),
		MODULE_SCRIPT_NOT_FOUND,
		script = ctx.script_name,
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
				panic_with!(MODULE_INSTANCE_STARTING)
			}
		}
	};

	let request_json = serde_json::from_str::<serde_json::Value>(&ctx.request_json)?;

	// Call module
	//
	// Use backoff for this call because Fly.io can be _special_ sometimes.
	let mut backoff = util::Backoff::new(3, Some(3), 500, 100);
	let response = loop {
		let response = reqwest::Client::new()
			.post(&url)
			.timeout(Duration::from_secs(5))
			.json(&CallRequest {
				script_name: ctx.script_name.clone(),
				request: &request_json,
			})
			.send()
			.await;
		match response {
			Ok(x) => break x,
			Err(err) => {
				tracing::error!(?err, "failed to call module");
				if backoff.tick().await {
					panic_with!(MODULE_REQUEST_FAILED)
				}
			}
		}
	};
	if !response.status().is_success() {
		tracing::warn!(status = ?response.status(), "module error status");
		panic_with!(MODULE_ERROR_STATUS, status = response.status().to_string());
	}
	let res_body = match response.json::<CallResponse>().await {
		Ok(x) => x,
		Err(err) => {
			tracing::warn!(?err, "malformed module response");
			panic_with!(MODULE_MALFORMED_RESPONSE)
		}
	};
	let response_json = serde_json::to_string(&res_body.response)?;

	// TODO: Validate JSON response schema

	Ok(module::instance_call::Response { response_json })
}
