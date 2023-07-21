use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(Serialize)]
struct CallRequest {
	function_name: String,
	request: serde_json::Value,
}

#[derive(Deserialize)]
struct CallResponse {
	data: serde_json::Value,
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

	// Validate function exists
	internal_assert!(
		instance
			.functions
			.iter()
			.any(|x| x.name == ctx.function_name),
		"function does not exist"
	);

	// TODO: Validate JSON request schema

	// Call module
	let url = format!("https://{}.fly.dev/call", app_id);
	let request_json = serde_json::from_str::<serde_json::Value>(&ctx.request.request_json)?;
	let response = reqwest::Client::new()
		.post("https://rivet-module-test.fly.dev/call")
		.json(&CallRequest {
			function_name: ctx.function_name,
			request: request_json,
		})
		.send()
		.await?;
	let res_body = response.json::<CallResponse>().await?;

	// TODO: Validate JSON response schema

	Ok(module::instance_call::Response {
		response_json: todo!(),
	})
}
