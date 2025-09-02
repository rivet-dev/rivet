use std::{net::SocketAddr, time::Instant};

use anyhow::Result;
use axum::{
	body::{Body, HttpBody},
	extract::{ConnectInfo, State},
	http::{Request, StatusCode},
	middleware::Next,
	response::Response,
};
use opentelemetry::trace::TraceContextExt;
use rivet_metrics::KeyValue;
use tower_http::trace::TraceLayer;
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{ErrorExt, RequestIds, metrics};

// TODO: Remove this since this is duplicate logs & traces, but this is just to see what Axum adds
// natively vs our logging. We can add this once we're satisfied with our own logging.
pub fn create_trace_layer()
-> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>>
{
	TraceLayer::new_for_http()
}

/// HTTP request logging and metrics middleware
pub async fn http_logging_middleware(
	State(config): State<rivet_config::Config>,
	mut req: Request<Body>,
	next: Next,
) -> Result<Response, StatusCode> {
	let start = Instant::now();

	// Extract socket address from request extensions
	let remote_addr = req
		.extensions()
		.get::<ConnectInfo<SocketAddr>>()
		.map(|ci| ci.0)
		.unwrap_or(SocketAddr::from(([0, 0, 0, 0], 0)));

	// Get trace context
	let current_span_ctx = tracing::Span::current()
		.context()
		.span()
		.span_context()
		.clone();

	// Generate request IDs
	let request_ids = RequestIds::new(config.dc_label());

	// Add request IDs to request extensions so they can be accessed by handlers
	req.extensions_mut().insert(request_ids);

	// Create span for this request
	let req_span = tracing::info_span!(
		parent: None,
		"http_request",
		method = %req.method(),
		uri = %req.uri(),
		ray_id = %request_ids.ray_id,
		req_id = %request_ids.req_id,
	);
	req_span.add_link(current_span_ctx);

	// Extract headers for logging
	let headers = req.headers();
	let referrer = headers
		.get("referer")
		.map_or("-", |h| h.to_str().unwrap_or("-"))
		.to_string();
	let user_agent = headers
		.get("user-agent")
		.map_or("-", |h| h.to_str().unwrap_or("-"))
		.to_string();
	let x_forwarded_for = headers
		.get("x-forwarded-for")
		.map_or("-", |h| h.to_str().unwrap_or("-"))
		.to_string();

	let method = req.method().clone();
	let uri = req.uri().clone();
	let path = uri.path().to_string();
	let protocol = req.version();

	// Log request metadata
	tracing::debug!(
		%method,
		%uri,
		body_size_hint = ?req.body().size_hint(),
		%remote_addr,
		"http request"
	);

	// Metrics
	metrics::API_REQUEST_PENDING.add(
		1,
		&[
			KeyValue::new("method", method.to_string()),
			KeyValue::new("path", path.clone()),
		],
	);
	metrics::API_REQUEST_TOTAL.add(
		1,
		&[
			KeyValue::new("method", method.to_string()),
			KeyValue::new("path", path.clone()),
		],
	);

	// Clone values for the async block
	let method_clone = method.clone();
	let path_clone = path.clone();

	// Process the request
	let response = async move {
		let mut response = next.run(req).await;

		// Add ray_id to response headers
		if let Ok(ray_id_value) = request_ids.ray_id.to_string().parse() {
			response.headers_mut().insert("rvt-ray-id", ray_id_value);
		}

		let status = response.status();
		let status_code = status.as_u16();

		let error = response.extensions().get::<ErrorExt>();

		// Log based on status
		if status.is_server_error() {
			tracing::error!(
				status = ?status_code,
				group = %error.as_ref().map_or("-", |x| &x.group),
				code = ?error.as_ref().map_or("-", |x| &x.code),
				meta = %error.as_ref().and_then(|x| x.metadata.as_ref()).unwrap_or(&serde_json::Value::Null),
				internal = %error.as_ref().and_then(|x| x.internal.as_ref()).map_or("-", |x| x.as_ref()),
				"http server error"
			);
		} else if status.is_client_error() {
			tracing::info!(
				status = ?status_code,
				group = %error.as_ref().map_or("-", |x| &x.group),
				code = %error.as_ref().map_or("-", |x| &x.code),
				meta = %error.as_ref().and_then(|x| x.metadata.as_ref()).unwrap_or(&serde_json::Value::Null),
				"http client error"
			);
		} else if status.is_redirection() {
			tracing::debug!(status = ?status_code, "http redirection");
		} else if status.is_informational() {
			tracing::debug!(status = ?status_code, "http informational");
		}

		let duration = start.elapsed().as_secs_f64();

		tracing::debug!(
			ray_id = %request_ids.ray_id,
			req_id = %request_ids.req_id,
			%remote_addr,
			%method,
			%uri,
			?protocol,
			status = status_code,
			body_bytes_sent = response.body().size_hint().lower(),
			request_duration = %format!("{:.3}ms", duration * 1000.0),
			%referrer,
			%user_agent,
			%x_forwarded_for,
			error_group = %error.as_ref().map_or("-", |x| &x.group),
			error_code = %error.as_ref().map_or("-", |x| &x.code),
			error_meta = %error.as_ref().and_then(|x| x.metadata.as_ref()).unwrap_or(&serde_json::Value::Null),
			error_internal = %error.as_ref().and_then(|x| x.internal.as_ref()).map_or("-", |x| x.as_ref()),
			"http response"
		);

		// Update metrics
		metrics::API_REQUEST_PENDING.add(
			-1,
			&[
				KeyValue::new("method", method_clone.to_string()),
				KeyValue::new("path", path_clone.clone()),
				KeyValue::new("watch", "false"),
			],
		);

		let error_code: String = if status.is_success() {
			"".into()
		} else {
			status.to_string()
		};
		metrics::API_REQUEST_DURATION.record(
			duration,
			&[
				KeyValue::new("method", method_clone.to_string()),
				KeyValue::new("path", path_clone.clone()),
				KeyValue::new("watch", "false"),
				KeyValue::new("status", status.to_string()),
				KeyValue::new("error_code", error_code.clone()),
			],
		);

		if !status.is_success() {
			metrics::API_REQUEST_ERRORS.add(
				1,
				&[
					KeyValue::new("method", method_clone.to_string()),
					KeyValue::new("path", path_clone.clone()),
					KeyValue::new("watch", "false"),
					KeyValue::new("status", status.to_string()),
					KeyValue::new("error_code", error_code),
				],
			);
		}

		response
	}
	.instrument(req_span)
	.await;

	Ok(response)
}
