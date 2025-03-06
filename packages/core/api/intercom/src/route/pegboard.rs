use api_helper::ctx::Ctx;
use rivet_api::models;
use rivet_operation::prelude::*;
use serde_json::json;

use crate::auth::Auth;

// MARK: POST /pegboard/client/registered
pub async fn client_registered(
	ctx: Ctx<Auth>,
	client_id: Uuid,
	_body: serde_json::Value,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().server(&ctx)?;

	ctx.signal(pegboard::workflows::client::Registered {})
		.tag("client_id", client_id)
		.send()
		.await?;

	Ok(json!({}))
}
