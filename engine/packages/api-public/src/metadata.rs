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
		"version": env!("CARGO_PKG_VERSION")
	}))
}
