use anyhow::Result;
use axum::response::{IntoResponse, Json, Response};
use rivet_api_builder::ApiError;
use rivet_api_builder::extract::Extension;
use rivet_util::build_meta;
use serde::Serialize;
use utoipa::ToSchema;

use crate::ctx::ApiCtx;

#[derive(Serialize, ToSchema)]
#[schema(as = MetadataGetResponse)]
pub struct GetResponse {
	runtime: String,
	version: String,
	git_sha: String,
	build_timestamp: String,
	rustc_version: String,
	rustc_host: String,
	cargo_target: String,
	cargo_profile: String,
}

/// Returns metadata about the API including runtime and version
#[utoipa::path(
	delete,
	operation_id = "metadata_get",
	path = "/metadata",
	responses(
		(status = 200, body = GetResponse),
	),
)]
#[tracing::instrument(skip_all)]
pub async fn get(Extension(ctx): Extension<ApiCtx>) -> Response {
	match get_inner(ctx).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

pub async fn get_inner(ctx: ApiCtx) -> Result<GetResponse> {
	ctx.skip_auth();

	Ok(GetResponse {
		runtime: build_meta::RUNTIME.to_string(),
		version: build_meta::VERSION.to_string(),
		git_sha: build_meta::GIT_SHA.to_string(),
		build_timestamp: build_meta::BUILD_TIMESTAMP.to_string(),
		rustc_version: build_meta::RUSTC_VERSION.to_string(),
		rustc_host: build_meta::RUSTC_HOST.to_string(),
		cargo_target: build_meta::CARGO_TARGET.to_string(),
		cargo_profile: build_meta::cargo_profile().to_string(),
	})
}
