use axum::Json;
use axum::response::IntoResponse;
use rivet_api_builder::extract::Extension;
use rivet_util::build_meta;
use serde_json::json;

use crate::ctx::ApiCtx;

/// Returns metadata about the API including runtime and version
#[tracing::instrument(skip_all)]
pub async fn get_metadata(Extension(ctx): Extension<ApiCtx>) -> impl IntoResponse {
	ctx.skip_auth();

	Json(json!({
		"runtime": build_meta::RUNTIME,
		"version": build_meta::VERSION,
		"git_sha": build_meta::GIT_SHA,
		"build_timestamp": build_meta::BUILD_TIMESTAMP,
		"rustc_version": build_meta::RUSTC_VERSION,
		"rustc_host": build_meta::RUSTC_HOST,
		"cargo_target": build_meta::CARGO_TARGET,
		"cargo_profile": build_meta::cargo_profile()
	}))
}
