use rivet_error::*;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"invalid_request_body",
	"Unable to parse request body.",
	"Unable to parse request body: {0}."
)]
pub struct InvalidRequestBody(pub String);

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"invalid_response_body",
	"Unable to parse response body.",
	"Unable to parse response body: {0}."
)]
pub struct InvalidResponseBody(pub String);

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"rate_limit",
	"Too many requests. Try again later.",
	"Too many requests to '{method} {path}' from IP {ip}."
)]
pub struct RateLimit {
	pub method: String,
	pub path: String,
	pub ip: String,
}

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
#[error("guard", "upstream_error", "Upstream error.", "Upstream error: {0}.")]
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
	"All {attempts} retry attempts failed."
)]
pub struct RetryAttemptsExceeded {
	pub attempts: u32,
}

#[derive(RivetError, Serialize, Deserialize)]
#[error("guard", "connection_error", "Connection error: {error_message}.")]
pub struct ConnectionError {
	pub error_message: String,
	pub remote_addr: String,
}

#[derive(RivetError, Serialize, Deserialize)]
#[error("guard", "service_unavailable", "Service unavailable.")]
pub struct ServiceUnavailable;

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"websocket_service_unavailable",
	"WebSocket service unavailable."
)]
pub struct WebSocketServiceUnavailable;

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"websocket_service_hibernate",
	"Initiate WebSocket service hibernation."
)]
pub struct WebSocketServiceHibernate;

#[derive(RivetError, Serialize, Deserialize)]
#[error("guard", "websocket_service_timeout", "WebSocket service timed out.")]
pub struct WebSocketServiceTimeout;

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"target_changed",
	"WebSocket target changed, retry not possible."
)]
pub struct WebSocketTargetChanged;

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"request_body_too_large",
	"Request body too large.",
	"Request body size {size} bytes exceeds maximum allowed {max_size} bytes."
)]
pub struct RequestBodyTooLarge {
	pub size: usize,
	pub max_size: usize,
}
