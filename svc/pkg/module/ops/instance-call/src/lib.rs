use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Module {
	module_id: Uuid,
	name_id: String,
	team_id: Uuid,
	create_ts: i64,
	publicity: i64,
}

#[operation(name = "module-instance-call")]
pub async fn handle(
	ctx: OperationContext<module::instance_call::Request>,
) -> GlobalResult<module::instance_call::Response> {
	let instance_id = internal_unwrap!(ctx.instance_id).as_uuid();

	todo!();

	// // Get instance
	// let instances = op!([ctx] module_instance_get {
	// 	instance_ids: vec![instance_id.into()],
	// }).await?;
	// let instance = internal_unwrap_owned!(instances.instances.first());

	// let 

	// // Call module
	// let url = format!(
	// 	"https://{}.fly.dev/call",
	// 	app_id
	// );
	// let response = reqwest::Client::new()
	// 	.post("https://rivet-module-test.fly.dev/call")
	// 	.json(&CallRequest {
	// 		parameters: body.parameters.unwrap_or_else(|| json!({})),
	// 	})
	// 	.send()
	// 	.await?;
	// let res_body = response.json::<CallResponse>().await?;

	Ok(module::instance_call::Response {
		response_json: todo!(),
	})
}
