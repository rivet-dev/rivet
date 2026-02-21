use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::Full;
use hyper::Response;
use hyper::StatusCode;
use hyper::header::HeaderName;
use rivet_api_builder::{ErrorResponse, RawErrorResponse};
use rivet_error::{INTERNAL_ERROR, RivetError};
use rivet_util::Id;
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, frame::coding::CloseCode};
use url::Url;

use crate::proxy_service::{X_FORWARDED_FOR, X_RIVET_ERROR};
use crate::response_body::ResponseBody;
use crate::{request_context::RequestContext, route::RouteTarget};

const X_RIVET_TARGET: HeaderName = HeaderName::from_static("x-rivet-target");
const X_RIVET_ACTOR: HeaderName = HeaderName::from_static("x-rivet-actor");
const X_RIVET_TOKEN: HeaderName = HeaderName::from_static("x-rivet-token");

// In-flight requests counter
pub(crate) struct InFlightCounter {
	count: usize,
	max: usize,
}

impl InFlightCounter {
	pub(crate) fn new(max: usize) -> Self {
		Self { count: 0, max }
	}

	pub(crate) fn try_acquire(&mut self) -> bool {
		if self.count < self.max {
			self.count += 1;
			true
		} else {
			false
		}
	}

	pub(crate) fn release(&mut self) {
		self.count = self.count.saturating_sub(1);
	}
}

// Rate limiter
pub(crate) struct RateLimiter {
	requests_remaining: u64,
	reset_time: Instant,
	requests_limit: u64,
	period: Duration,
}

impl RateLimiter {
	pub(crate) fn new(requests: u64, period_seconds: u64) -> Self {
		Self {
			requests_remaining: requests,
			reset_time: Instant::now() + Duration::from_secs(period_seconds),
			requests_limit: requests,
			period: Duration::from_secs(period_seconds),
		}
	}

	pub(crate) fn try_acquire(&mut self) -> bool {
		let now = Instant::now();

		// Check if we need to reset the counter
		if now >= self.reset_time {
			self.requests_remaining = self.requests_limit;
			self.reset_time = now + self.period;
		}

		// Try to consume a request
		if self.requests_remaining > 0 {
			self.requests_remaining -= 1;
			true
		} else {
			false
		}
	}
}

// Calculate backoff duration for a given retry attempt
pub(crate) fn calculate_backoff(attempt: u32, initial_interval: u64) -> Duration {
	Duration::from_millis(initial_interval * 2u64.pow(attempt - 1))
}

/// Modifies the incoming request before it is proxied.
pub(crate) fn proxied_request_builder(
	req_parts: &hyper::http::request::Parts,
	req_ctx: &RequestContext,
	target: &RouteTarget,
) -> Result<hyper::http::request::Builder> {
	let scheme = if target.port == 443 { "https" } else { "http" };

	// Bracket raw IPv6 hosts
	let host = if target.host.contains(':') && !target.host.starts_with('[') {
		format!("[{}]", target.host)
	} else {
		target.host.clone()
	};

	// Ensure path starts with a leading slash
	let path = if target.path.starts_with('/') {
		target.path.clone()
	} else {
		format!("/{}", target.path)
	};

	let url = Url::parse(&format!("{scheme}://{host}:{}{}", target.port, path))
		.context("invalid scheme/host/port when building URL")?;

	// Build the proxied request
	let mut builder = hyper::Request::builder()
		.method(req_parts.method.clone())
		.uri(url.to_string());

	// Modify proxy headers
	let headers = builder
		.headers_mut()
		.expect("request builder unexpectedly in error state");

	headers.remove(X_RIVET_TARGET);
	headers.remove(X_RIVET_ACTOR);
	headers.remove(X_RIVET_TOKEN);

	add_proxy_headers_with_addr(headers, &req_ctx)?;

	Ok(builder)
}

pub(crate) fn add_proxy_headers_with_addr(
	headers: &mut hyper::HeaderMap,
	req_ctx: &RequestContext,
) -> Result<()> {
	// Copy headers except Host
	for (key, value) in &req_ctx.headers {
		if key != hyper::header::HOST {
			headers.insert(key.clone(), value.clone());
		}
	}

	// Add X-Forwarded-For header
	if let Some(existing) = req_ctx.headers.get(X_FORWARDED_FOR) {
		if let Ok(forwarded) = existing.to_str() {
			if !forwarded.contains(&req_ctx.remote_addr.ip().to_string()) {
				headers.insert(
					X_FORWARDED_FOR,
					hyper::header::HeaderValue::from_str(&format!(
						"{}, {}",
						forwarded,
						req_ctx.remote_addr.ip()
					))?,
				);
			}
		}
	} else {
		headers.insert(
			X_FORWARDED_FOR,
			hyper::header::HeaderValue::from_str(&req_ctx.remote_addr.ip().to_string())?,
		);
	}

	Ok(())
}

pub(crate) fn err_into_response(err: anyhow::Error) -> Result<Response<ResponseBody>> {
	let (status, error_response) =
		if let Some(rivet_err) = err.chain().find_map(|x| x.downcast_ref::<RivetError>()) {
			let status = match (rivet_err.group(), rivet_err.code()) {
				("api", "not_found") => StatusCode::NOT_FOUND,
				("api", "unauthorized") => StatusCode::UNAUTHORIZED,
				("api", "forbidden") => StatusCode::FORBIDDEN,
				("guard", "rate_limit") => StatusCode::TOO_MANY_REQUESTS,
				("guard", "upstream_error") => StatusCode::BAD_GATEWAY,
				("guard", "routing_error") => StatusCode::BAD_GATEWAY,
				("guard", "request_timeout") => StatusCode::GATEWAY_TIMEOUT,
				("guard", "retry_attempts_exceeded") => StatusCode::BAD_GATEWAY,
				("actor", "not_found") => StatusCode::NOT_FOUND,
				("guard", "service_unavailable") => StatusCode::SERVICE_UNAVAILABLE,
				("guard", "actor_ready_timeout") => StatusCode::SERVICE_UNAVAILABLE,
				("guard", "no_route") => StatusCode::NOT_FOUND,
				("guard", "invalid_request_body") => StatusCode::PAYLOAD_TOO_LARGE,
				("guard", "invalid_response_body") => StatusCode::BAD_GATEWAY,
				_ => StatusCode::BAD_REQUEST,
			};

			(status, ErrorResponse::from(rivet_err))
		} else if let Some(raw_err) = err
			.chain()
			.find_map(|x| x.downcast_ref::<RawErrorResponse>())
		{
			(raw_err.0, raw_err.1.clone())
		} else {
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				ErrorResponse::from(&RivetError {
					schema: &rivet_error::INTERNAL_ERROR,
					meta: None,
					message: None,
				}),
			)
		};

	let body_json = serde_json::to_vec(&error_response)?;
	let bytes = Bytes::from(body_json);

	Response::builder()
		.status(status)
		.header(hyper::header::CONTENT_TYPE, "application/json")
		.body(ResponseBody::Full(Full::new(bytes)))
		.map_err(Into::into)
}

pub(crate) fn should_retry_request(res: &Result<Response<ResponseBody>>) -> bool {
	match res {
		Ok(resp) => should_retry_request_inner(resp.status(), resp.headers()),
		Err(err) => {
			if let Some(rivet_err) = err.chain().find_map(|x| x.downcast_ref::<RivetError>()) {
				rivet_err.group() == "guard" && rivet_err.code() == "service_unavailable"
			} else {
				false
			}
		}
	}
}

// Determine if a response should trigger a retry: 503 + x-rivet-error
pub(crate) fn should_retry_request_inner(status: StatusCode, headers: &hyper::HeaderMap) -> bool {
	status == StatusCode::SERVICE_UNAVAILABLE && headers.contains_key(X_RIVET_ERROR)
}

// Determine if a websocket error is retryable (e.g., transient UPS/tunnel issues)
pub(crate) fn is_retryable_ws_error(err: &anyhow::Error) -> bool {
	if let Some(rivet_err) = err.chain().find_map(|x| x.downcast_ref::<RivetError>()) {
		rivet_err.group() == "guard" && rivet_err.code() == "websocket_service_unavailable"
	} else {
		false
	}
}

pub fn is_ws_hibernate(err: &anyhow::Error) -> bool {
	if let Some(rivet_err) = err.chain().find_map(|x| x.downcast_ref::<RivetError>()) {
		rivet_err.group() == "guard" && rivet_err.code() == "websocket_service_hibernate"
	} else {
		false
	}
}

pub(crate) fn err_to_close_frame(err: anyhow::Error, ray_id: Id) -> CloseFrame {
	let rivet_err = err
		.chain()
		.find_map(|x| x.downcast_ref::<RivetError>())
		.cloned()
		.unwrap_or_else(|| RivetError::from(&INTERNAL_ERROR));

	let code = match (rivet_err.group(), rivet_err.code()) {
		("ws", "connection_closed") | ("ws", "eviction") => CloseCode::Normal,
		_ => CloseCode::Error,
	};

	match code {
		CloseCode::Normal => tracing::debug!("websocket closed"),
		_ => tracing::error!(?err, "websocket failed"),
	}

	let reason = format!("{}.{}#{}", rivet_err.group(), rivet_err.code(), ray_id);

	// NOTE: reason cannot be more than 123 bytes as per the WS protocol
	let reason = rivet_util::safe_slice(&reason, 0, 123).into();

	CloseFrame { code, reason }
}

pub(crate) fn to_hyper_close(frame: Option<CloseFrame>) -> hyper_tungstenite::tungstenite::Message {
	if let Some(frame) = frame {
		// Manual conversion to handle different tungstenite versions
		let code_num: u16 = frame.code.into();
		let reason = frame.reason.clone();

		tokio_tungstenite::tungstenite::Message::Close(Some(
			tokio_tungstenite::tungstenite::protocol::CloseFrame {
				code: code_num.into(),
				reason,
			},
		))
	} else {
		tokio_tungstenite::tungstenite::Message::Close(Some(
			tokio_tungstenite::tungstenite::protocol::CloseFrame {
				code: CloseCode::Normal,
				reason: "ws.closed".into(),
			},
		))
	}
}
