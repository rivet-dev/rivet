//! Liveness of the API-peer listener, for load balancer health checks and the public
//! health fanout, which probe `/health` on every datacenter's peer URL.

use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

/// Serves `/health` without authentication or dependencies, so it reports only that this
/// worker's peer listener accepts requests.
pub fn router() -> Router {
	Router::new().route("/health", get(health))
}

async fn health() -> Json<Value> {
	Json(json!({
		"status": "ok",
		"runtime": "engine",
		"version": env!("CARGO_PKG_VERSION"),
	}))
}
