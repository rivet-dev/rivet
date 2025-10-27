use axum::Json;
use axum::response::IntoResponse;
use rivet_api_builder::extract::Extension;
use serde_json::json;

use crate::ctx::ApiCtx;

/// Returns metadata about the API including runtime and version
#[tracing::instrument(skip_all)]
pub async fn get_metadata(Extension(ctx): Extension<ApiCtx>) -> impl IntoResponse {
	ctx.skip_auth();

	Json(json!({
		"runtime": "engine",
		"version": env!("CARGO_PKG_VERSION"),
		"git_sha": env!("VERGEN_GIT_SHA"),
		"build_timestamp": env!("VERGEN_BUILD_TIMESTAMP"),
		"rustc_version": env!("VERGEN_RUSTC_SEMVER"),
		"rustc_host": env!("VERGEN_RUSTC_HOST_TRIPLE"),
		"cargo_target": env!("VERGEN_CARGO_TARGET_TRIPLE"),
		"cargo_profile": if env!("VERGEN_CARGO_DEBUG") == "true" { "debug" } else { "release" }
	}))
}
