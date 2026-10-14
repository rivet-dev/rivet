use super::signing;
use crate::{ctx::ApiCtx, errors::Signing};
use anyhow::Result;
use axum::{
	http::{HeaderValue, header::CONTENT_TYPE},
	response::{IntoResponse, Response},
};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path},
};
use rivet_api_peer::auth::webhooks::Discovery;
use rivet_api_types::auth::webhooks::{Jwks, Metadata};
use rivet_util::Id;

#[utoipa::path(get, operation_id = "auth_webhooks_jwks", path = "/namespaces/{namespace}/auth/webhooks/jwks.json",
    params(("namespace" = String, Path, description = "Immutable namespace ID")),
    responses((status = 200, body = Jwks, content_type = "application/jwk-set+json")), security())]
#[tracing::instrument(skip_all)]
pub async fn jwks(
	Extension(ctx): Extension<ApiCtx>,
	Path((namespace,)): Path<(String,)>,
) -> Response {
	ctx.skip_auth();
	signing::no_store(match parse_and_get(&ctx, &namespace).await {
		Ok(discovery) => {
			let mut response = Json(discovery.jwks).into_response();
			response.headers_mut().insert(
				CONTENT_TYPE,
				HeaderValue::from_static("application/jwk-set+json"),
			);
			response
		}
		Err(error) => ApiError::from(error).into_response(),
	})
}

#[utoipa::path(get, operation_id = "auth_webhooks_metadata", path = "/namespaces/{namespace}/auth/webhooks",
    params(("namespace" = String, Path, description = "Immutable namespace ID")),
    responses((status = 200, body = Metadata)), security())]
#[tracing::instrument(skip_all)]
pub async fn metadata(
	Extension(ctx): Extension<ApiCtx>,
	Path((namespace,)): Path<(String,)>,
) -> Response {
	ctx.skip_auth();
	signing::no_store(
		match parse_and_get(&ctx, &namespace)
			.await
			.and_then(|d| d.metadata.ok_or_else(|| Signing::Unavailable.build()))
		{
			Ok(metadata) => Json(metadata).into_response(),
			Err(error) => ApiError::from(error).into_response(),
		},
	)
}

async fn parse_and_get(ctx: &ApiCtx, namespace: &str) -> Result<Discovery> {
	let id = namespace
		.parse::<Id>()
		.map_err(|_| Signing::NotFound.build())?;
	get(ctx, id).await
}

pub(crate) async fn get(ctx: &ApiCtx, id: Id) -> Result<Discovery> {
	let started = std::time::Instant::now();
	let response = if ctx.config().is_leader() {
		rivet_api_peer::auth::webhooks::discovery(ctx.clone().into(), (id,), ()).await?
	} else {
		rivet_api_util::request_remote_datacenter(
			ctx.config(),
			ctx.config().leader_dc()?.datacenter_label,
			&format!("/auth/webhooks/{id}/discovery"),
			rivet_api_util::Method::GET,
			Option::<&()>::None,
			Option::<&()>::None,
		)
		.await?
	};
	if started.elapsed() >= rivet_auth_jwt::SIGNING_CACHE_LEASE {
		return Err(Signing::Unavailable.build());
	}
	Ok(response)
}
