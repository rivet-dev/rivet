//! The canonical error replies of the engine's edge, in the format RivetKit clients parse.
//!
//! An HTTP error is a JSON body `{ "group", "code", "message", "metadata"? }` with a matching
//! `x-rivet-error: <group>.<code>` header and a status chosen by the error's group and code. A
//! WebSocket error is a close frame whose reason is `<group>.<code>#<ray id>`. Clients read the
//! group and code to tell a missing actor, which they re-resolve, from a retryable refusal,
//! which they retry, so the statuses and close codes here match earlier engines exactly. An
//! error that is not a [`RivetError`] is reported as `core.internal_error`.

use http::{HeaderValue, StatusCode, header};
use rivet_error::RivetError;
use rivet_util::Id;

/// HTTP header that names a failed request's error as `<group>.<code>`.
pub const X_RIVET_ERROR: &str = "x-rivet-error";

/// WebSocket close code for a normal closure.
pub const NORMAL_CLOSE_CODE: u16 = 1000;

/// WebSocket close code for a rejected credential or permission.
pub const POLICY_VIOLATION_CLOSE_CODE: u16 = 1008;

/// WebSocket close code for any other failure: the server hit an unexpected condition.
pub const INTERNAL_ERROR_CLOSE_CODE: u16 = 1011;

/// The WebSocket protocol caps a close reason at 123 bytes.
const MAX_CLOSE_REASON_BYTES: usize = 123;

/// The HTTP status of an error. Errors not listed are the client's fault.
pub fn status(group: &str, code: &str) -> StatusCode {
	match (group, code) {
		("api", "not_found") => StatusCode::NOT_FOUND,
		("api", "unauthorized") => StatusCode::UNAUTHORIZED,
		("api", "forbidden") => StatusCode::FORBIDDEN,
		("auth", "invalid_token") => StatusCode::UNAUTHORIZED,
		("auth", "token_expired") => StatusCode::UNAUTHORIZED,
		("auth", "insufficient_permissions") => StatusCode::FORBIDDEN,
		("auth", "verification_unavailable") => StatusCode::SERVICE_UNAVAILABLE,
		("auth", "issuance_unavailable") => StatusCode::SERVICE_UNAVAILABLE,
		("auth", "issuance_disabled") => StatusCode::SERVICE_UNAVAILABLE,
		("guard", "rate_limit") => StatusCode::TOO_MANY_REQUESTS,
		("guard", "upstream_error") => StatusCode::BAD_GATEWAY,
		("guard", "routing_error") => StatusCode::BAD_GATEWAY,
		("guard", "request_timeout") => StatusCode::GATEWAY_TIMEOUT,
		("guard", "route_dispatch_timeout") => StatusCode::GATEWAY_TIMEOUT,
		("guard", "route_api_public_timeout") => StatusCode::GATEWAY_TIMEOUT,
		("guard", "route_compute_timeout") => StatusCode::GATEWAY_TIMEOUT,
		("guard", "route_auth_check_timeout") => StatusCode::GATEWAY_TIMEOUT,
		("guard", "retry_attempts_exceeded") => StatusCode::BAD_GATEWAY,
		("pegboard", "route_subscribe_timeout") => StatusCode::GATEWAY_TIMEOUT,
		("pegboard", "route_fetch_actor_timeout") => StatusCode::GATEWAY_TIMEOUT,
		("pegboard", "route_auth_check_timeout") => StatusCode::GATEWAY_TIMEOUT,
		("pegboard", "route_wake_signal_timeout") => StatusCode::GATEWAY_TIMEOUT,
		("pegboard", "route_resolve_query_timeout") => StatusCode::GATEWAY_TIMEOUT,
		("guard", "service_unavailable") => StatusCode::SERVICE_UNAVAILABLE,
		("guard", "actor_wake_retries_exceeded") => StatusCode::SERVICE_UNAVAILABLE,
		("guard", "actor_stopped_while_waiting") => StatusCode::SERVICE_UNAVAILABLE,
		("guard", "tunnel_request_aborted") => StatusCode::SERVICE_UNAVAILABLE,
		("guard", "tunnel_message_timeout") => StatusCode::GATEWAY_TIMEOUT,
		("guard", "request_delivery_unconfirmed") => StatusCode::SERVICE_UNAVAILABLE,
		("guard", "tunnel_response_closed") => StatusCode::SERVICE_UNAVAILABLE,
		("guard", "gateway_response_start_timeout") => StatusCode::GATEWAY_TIMEOUT,
		("guard", "actor_ready_timeout") => StatusCode::SERVICE_UNAVAILABLE,
		("guard", "no_route") => StatusCode::NOT_FOUND,
		("guard", "invalid_request_body") => StatusCode::PAYLOAD_TOO_LARGE,
		("guard", "invalid_response_body") => StatusCode::BAD_GATEWAY,
		("actor", "creation_rate_limit") => StatusCode::TOO_MANY_REQUESTS,
		("actor", "not_found") => StatusCode::NOT_FOUND,
		("actor", "migrating_from_v2") => StatusCode::SERVICE_UNAVAILABLE,
		("core", "internal_error") => StatusCode::INTERNAL_SERVER_ERROR,
		_ => StatusCode::BAD_REQUEST,
	}
}

/// The canonical HTTP reply for `error`.
pub fn http_response(error: &anyhow::Error) -> http::Response<Vec<u8>> {
	let error = RivetError::extract(error);
	let mut body = serde_json::json!({
		"group": error.group(),
		"code": error.code(),
		"message": error.message(),
	});
	if let Some(metadata) = error.metadata() {
		body["metadata"] = metadata;
	}

	let mut response = http::Response::new(serde_json::to_vec(&body).unwrap_or_default());
	*response.status_mut() = status(error.group(), error.code());
	let headers = response.headers_mut();
	headers.insert(
		header::CONTENT_TYPE,
		HeaderValue::from_static("application/json"),
	);
	let name = format!("{}.{}", error.group(), error.code());
	if let Ok(value) = HeaderValue::from_str(&name) {
		headers.insert(X_RIVET_ERROR, value);
	}
	response
}

/// The close code and reason of a WebSocket that ends on `error`. The reason carries the
/// request's ray id when there is one.
pub fn close_frame(error: &anyhow::Error, ray_id: Option<Id>) -> (u16, String) {
	let error = RivetError::extract(error);
	let code = match (error.group(), error.code()) {
		("ws", "connection_closed") | ("ws", "eviction") => NORMAL_CLOSE_CODE,
		("auth", _) => POLICY_VIOLATION_CLOSE_CODE,
		_ => INTERNAL_ERROR_CLOSE_CODE,
	};

	let mut reason = format!("{}.{}", error.group(), error.code());
	if let Some(ray_id) = ray_id {
		reason.push('#');
		reason.push_str(&ray_id.to_string());
	}
	(
		code,
		truncate_to_char_boundary(reason, MAX_CLOSE_REASON_BYTES),
	)
}

fn truncate_to_char_boundary(mut value: String, max_bytes: usize) -> String {
	if value.len() <= max_bytes {
		return value;
	}
	let mut end = max_bytes;
	while !value.is_char_boundary(end) {
		end -= 1;
	}
	value.truncate(end);
	value
}


