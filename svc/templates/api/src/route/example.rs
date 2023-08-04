use api_helper::ctx::Ctx;
use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;

use crate::auth::Auth;

// MARK: POST /foo/{}/bar
pub async fn endpoint(
	ctx: Ctx<Auth>,
	parameter: String,
	_body: serde_json::Value,
) -> GlobalResult<serde_json::Value> {
	todo!();

	Ok(json!({}))
}
