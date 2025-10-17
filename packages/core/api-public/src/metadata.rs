use axum::Json;
use axum::response::IntoResponse;
use serde_json::json;

/// Returns metadata about the API including runtime and version
#[tracing::instrument(skip_all)]
pub async fn get_metadata() -> impl IntoResponse {
	Json(json!({
		"runtime": "engine",
		"version": env!("CARGO_PKG_VERSION")
	}))
}
