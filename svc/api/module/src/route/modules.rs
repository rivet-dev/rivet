use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use chirp_client::TailAnchorResponse;
use proto::backend::pkg::*;
use rivet_api::models;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{auth::Auth, utils};

// MARK: POST /modules/{}/endpoints/{}/call
pub async fn endpoint_call(
	ctx: Ctx<Auth>,
	module: String,
	endpoint: String,
	body: models::ModuleCallRequest,
) -> GlobalResult<models::ModuleCallResponse> {
	let namespace_id = ctx
		.auth()
		.namespace(ctx.op_ctx(), body.namespace_id, false)
		.await?;

	// Make POST request
	let response = reqwest::Client::new()
		.post("https://rivet-module-test.fly.dev/call")
		.json(&CallRequest {
			parameters: body.parameters.unwrap_or_else(|| json!({})),
		})
		.send()
		.await?;
	let res_body = response.json::<CallResponse>().await?;

	Ok(models::ModuleCallResponse {
		data: Some(res_body.data),
	})
}
