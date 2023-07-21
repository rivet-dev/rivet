use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use chirp_client::TailAnchorResponse;
use proto::backend::pkg::*;
use rivet_api::models;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{auth::Auth, utils};

// MARK: POST /modules/{}/endpoints/{}/call
pub async fn endpoint_call(
	ctx: Ctx<Auth>,
	body: models::ModuleCallRequest,
) -> GlobalResult<serde_json::Value> {
	let namespace_id = ctx
		.auth()
		.namespace(ctx.op_ctx(), body.namespace_id, false)
		.await?;

	// TODO:

	Ok(json!({}))
}
