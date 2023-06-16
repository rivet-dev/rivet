use api_helper::ctx::Ctx;
use rivet_claims::ClaimsDecode;
use rivet_cloud_server::models;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: POST /uploads/{}/complete
pub async fn complete(
	ctx: Ctx<Auth>,
	upload_id: Uuid,
	_body: models::CompleteUploadRequest,
) -> GlobalResult<models::CompleteUploadResponse> {
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

	Ok(models::CompleteUploadResponse {})
}
