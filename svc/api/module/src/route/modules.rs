use api_helper::ctx::Ctx;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: POST /modules/{}/scripts/{}/call
pub async fn script_call(
	ctx: Ctx<Auth>,
	module: String,
	func: String,
	body: models::ModuleCallRequest,
) -> GlobalResult<models::ModuleCallResponse> {
	let namespace_id = ctx
		.auth()
		.namespace(ctx.op_ctx(), body.namespace_id)
		.await?;

	// Get the associated instance ID
	let instance_res = op!([ctx] module_ns_instance_get {
		namespace_id: Some(namespace_id.into()),
		key: module.clone(),
	})
	.await?;
	let Some(instance) = instance_res.instance else {
		bail_with!(MODULE_KEY_NOT_FOUND, key = module);
	};

	// Get the module
	let res = op!([ctx] module_instance_call {
		instance_id: instance.instance_id,
		script_name: func,
		request_json: serde_json::to_string(&body.data)?,
	})
	.await?;
	let data = serde_json::from_str(&res.response_json)?;

	Ok(models::ModuleCallResponse { data })
}
