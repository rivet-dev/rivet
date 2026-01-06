use std::{net::SocketAddr, time::Instant};

use anyhow::Result;
use axum::{
	body::{Body, HttpBody},
	extract::{ConnectInfo, State},
	http::{Request, StatusCode},
	middleware::Next,
	response::Response,
};
use hyper::header::HeaderName;
use opentelemetry::trace::TraceContextExt;
use tower_http::trace::TraceLayer;
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{ErrorExt, RequestIds, RouterName, metrics};

pub const X_RIVET_RAY_ID: HeaderName = HeaderName::from_static("x-rivet-ray-id");

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
	State(RouterName(router_name)): State<RouterName>,
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

	// Add request IDs to request extensions if not already added by guard so they can be accessed by handlers
	let request_ids = if let Some(request_ids) = req.extensions().get::<RequestIds>() {
		*request_ids
	} else {
		let request_ids = RequestIds::new(config.dc_label());
		req.extensions_mut().insert(request_ids);
		request_ids
	};

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
	metrics::API_REQUEST_PENDING
		.with_label_values(&[router_name, method.as_str(), path.as_str()])
		.inc();
	metrics::API_REQUEST_TOTAL
		.with_label_values(&[router_name, method.as_str(), path.as_str()])
		.inc();

	// Clone values for the async block
	let method_clone = method.clone();
	let path_clone = path.clone();

	// Process the request
	let response = async move {
		let mut response = next.run(req).await;

		// Add ray_id to response headers
		if let Ok(ray_id_value) = request_ids.ray_id.to_string().parse() {
			response.headers_mut().insert(X_RIVET_RAY_ID, ray_id_value);
		}

		let status = response.status();
		let status_code = status.as_u16();

		let error = response.extensions().get::<ErrorExt>();

		// Log based on status
		if status.is_server_error() {
			let group = error.as_ref().map_or("-", |x| &x.group);
			let code = error.as_ref().map_or("-", |x| &x.code);
			let meta = error.as_ref().and_then(|x| x.metadata.as_ref()).unwrap_or(&serde_json::Value::Null);
			let internal = error.as_ref().and_then(|x| x.internal.as_ref()).map_or("-", |x| x.as_ref());

			tracing::error!(
				status=?status_code,
				%group,
				%code,
				%meta,
				%internal,
				"http server error"
			);
		} else if status.is_client_error() {
			let group = error.as_ref().map_or("-", |x| &x.group);
			let code = error.as_ref().map_or("-", |x| &x.code);
			let meta = error.as_ref().and_then(|x| x.metadata.as_ref()).unwrap_or(&serde_json::Value::Null);

			tracing::info!(
				status=?status_code,
				%group,
				%code,
				%meta,
				"http client error"
			);
		} else if status.is_redirection() {
			tracing::debug!(status=?status_code, "http redirection");
		} else if status.is_informational() {
			tracing::debug!(status=?status_code, "http informational");
		}

		let duration = start.elapsed().as_secs_f64();

		tracing::debug!(
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
		metrics::API_REQUEST_PENDING.with_label_values(&[router_name, method_clone.as_str(), path_clone.as_str()]).dec();

		let error_str: String = if status.is_success() {
			String::new()
		} else if let Some(err) = &error {
			format!("{}.{}", err.group, err.code)
		} else {
			String::new()
		};
 			metrics::API_REQUEST_DURATION
			.with_label_values(&[router_name, method_clone.as_str(), path_clone.as_str(), status.as_str(), error_str.as_str()])
			.observe(duration);

		if !status.is_success() {
			metrics::API_REQUEST_ERRORS
			.with_label_values(&[router_name, method_clone.as_str(), path_clone.as_str(), status.as_str(), error_str.as_str()])
			.inc();
		}

		response
	}
	.instrument(req_span)
	.await;

	Ok(response)
}
