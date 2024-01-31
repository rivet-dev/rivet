use api_helper::ctx::Ctx;

use rivet_claims::ClaimsDecode;
use rivet_operation::prelude::*;
use serde_json::json;

use crate::auth::Auth;

// MARK: POST /uploads/{}/complete
pub async fn complete(
	ctx: Ctx<Auth>,
	upload_id: Uuid,
	_body: serde_json::Value,
) -> GlobalResult<serde_json::Value> {
	// TODO: use auth module instead
	let claims = ctx.auth().claims()?;
	if claims.as_user().is_err() {
		claims.as_game_cloud()?;
	}

	op!([ctx] @dont_log_body upload_complete {
		upload_id: Some(upload_id.into()),
		bucket: None,
	})
	.await?;

	Ok(json!({}))
}
