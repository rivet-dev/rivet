use anyhow::Result;
use axum::response::{IntoResponse, Json, Response};
use rivet_api_builder::ApiError;
use rivet_api_builder::extract::Extension;
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
	epoxy_protocol_version: u16,
	runner_protocol_version: u16,
}

/// Returns metadata about the API including runtime and version
#[utoipa::path(
	get,
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

	let build_meta = ctx.config().build_meta();
	let protocols = ctx.config().protocols();

	Ok(GetResponse {
		runtime: build_meta.runtime.clone(),
		version: build_meta.version.clone(),
		git_sha: build_meta.git_sha.clone(),
		build_timestamp: build_meta.build_timestamp.clone(),
		rustc_version: build_meta.rustc_version.clone(),
		rustc_host: build_meta.rustc_host.clone(),
		cargo_target: build_meta.cargo_target.clone(),
		cargo_profile: build_meta.cargo_profile.clone(),
		// Advertise what the fleet has agreed to speak. A client that took this binary's compiled
		// version would be rejected by any pod still running older code.
		epoxy_protocol_version: protocols.epoxy.version(),
		runner_protocol_version: protocols.envoy.version(),
	})
}
