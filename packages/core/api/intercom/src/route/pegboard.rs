use api_helper::ctx::Ctx;
use rivet_api::models;
use rivet_operation::prelude::*;
use serde_json::json;

use crate::auth::Auth;

// MARK: POST /pegboard/client/{}/registered
pub async fn client_registered(
	ctx: Ctx<Auth>,
	client_id: Uuid,
	body: models::CoreIntercomPegboardMarkClientRegisteredRequest,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().bypass()?;

	ctx.signal(cluster::workflows::server::PegboardRegistered { client_id })
		.tag("server_id", body.server_id)
		.send()
		.await?;

	Ok(json!({}))
}
