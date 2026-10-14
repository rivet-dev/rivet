use anyhow::Result;
use axum::{
	http::header::{CACHE_CONTROL, CONTENT_TYPE},
	response::{IntoResponse, Response},
};
use rivet_api_builder::{ApiError, extract::Extension};

use crate::ctx::ApiCtx;

/// Publishes only the public halves of Rivet's currently valid signing keys.
#[utoipa::path(
	get,
	operation_id = "auth_jwks_get",
	path = "/.well-known/jwks.json",
	responses((status = 200, description = "Public JSON Web Key Set", body = serde_json::Value)),
	security(())
)]
#[tracing::instrument(skip_all)]
pub async fn get(Extension(ctx): Extension<ApiCtx>) -> Response {
	match get_inner(ctx).await {
		Ok((json, max_age)) => (
			[
				(CONTENT_TYPE, "application/jwk-set+json".to_owned()),
				(CACHE_CONTROL, format!("public, max-age={max_age}")),
			],
			json,
		)
			.into_response(),
		Err(error) => ApiError::from(error).into_response(),
	}
}

async fn get_inner(ctx: ApiCtx) -> Result<(String, u64)> {
	ctx.skip_auth();
	let snapshot = ctx
		.jwt_key_ring_cache()
		.map_err(|_| rivet_auth::errors::Auth::VerificationUnavailable.build())?
		.public_snapshot()
		.await
		.map_err(rivet_auth_jwt::key_ring_cache::VerificationFailure::into_error)?;
	let now_ms = rivet_util::timestamp::now();
	let jwks = rivet_auth_jwt::jwks::JwkSet::from_verification_keys(&snapshot.keys, now_ms);
	let jwt = &ctx.config().auth_required()?.jwt;
	// External verifiers learn the pending key before it becomes active. Keep public caches
	// shorter than that publication lead, and bounded after an emergency key revocation.
	let max_age = jwt
		.verifier_cache_ttl()
		.as_secs()
		.min(jwt.key_publish_lead().as_secs().saturating_sub(1))
		.min(60);
	Ok((serde_json::to_string(&jwks)?, max_age))
}
