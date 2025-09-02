use rivet_error::*;
use serde::{Deserialize, Serialize};

#[derive(RivetError)]
#[error("guard", "rate_limit", "Too many requests. Try again later.")]
pub struct RateLimit;

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"http_request_build_failed",
	"Failed to build HTTP request.",
	"Failed to build HTTP request: {0}."
)]
pub struct HttpRequestBuildFailed(pub String);

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"uri_parse_error",
	"URI parse error.",
	"URI parse error: {0}."
)]
pub struct UriParseError(pub String);

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"request_build_error",
	"Request build error.",
	"Request build error: {0}."
)]
pub struct RequestBuildError(pub String);

#[derive(RivetError)]
#[error("guard", "upstream_error", "Upstream error.", "Upstream error: {0}")]
pub struct UpstreamError(pub String);

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"request_timeout",
	"Request timed out.",
	"Request timed out after {timeout_seconds} seconds."
)]
pub struct RequestTimeout {
	pub timeout_seconds: u64,
}

#[derive(RivetError, Serialize, Deserialize)]
#[error("guard", "no_route_targets", "No targets found.")]
pub struct NoRouteTargets;

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"retry_attempts_exceeded",
	"Retry attempts exceeded.",
	"All {attempts} retry attempts failed (max: {max_attempts})."
)]
pub struct RetryAttemptsExceeded {
	pub attempts: u32,
	pub max_attempts: u32,
}

#[derive(RivetError, Serialize, Deserialize)]
#[error("guard", "connection_error", "Connection error: {error_message}.")]
pub struct ConnectionError {
	pub error_message: String,
	pub remote_addr: String,
}

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"websocket_service_unavailable",
	"WebSocket service unavailable.",
	"WebSocket service unavailable."
)]
pub struct WebSocketServiceUnavailable;
