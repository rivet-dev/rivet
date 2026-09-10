//! HTTP listener for `RIVETKIT_RUNTIME_MODE=serverless`.
//!
//! Mirrors the TypeScript `registry.listen()` serverless path: binds an HTTP
//! server and forwards every request to [`CoreServerlessRuntime::handle_request`],
//! which lazily starts and caches an envoy on the first request. The server
//! shuts down gracefully when the shutdown token is cancelled, then drains the
//! cached envoy via [`CoreServerlessRuntime::shutdown`].

use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};

use anyhow::{Context, Result};
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::response::Response;
use bytes::Bytes;
use futures::StreamExt;
use http::StatusCode;
use rivetkit_core::serverless::{CoreServerlessRuntime, ServerlessRequest, ServerlessResponse};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::sync::CancellationToken;

/// Default listen port, matching the TypeScript `RIVET_PORT` fallback.
const DEFAULT_PORT: u16 = 3000;

/// Runs the serverless HTTP listener until `shutdown` is cancelled, then drains
/// the runtime. Binds `0.0.0.0:$RIVET_PORT` (default 3000).
pub async fn serve(runtime: CoreServerlessRuntime, shutdown: CancellationToken) -> Result<()> {
	let port = listen_port();
	let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));

	let app = Router::new().fallback(handle).with_state(runtime.clone());

	let listener = tokio::net::TcpListener::bind(addr)
		.await
		.with_context(|| format!("bind serverless listener on {addr}"))?;
	tracing::info!(%addr, "rivetkit serverless listener started");

	let serve_result = axum::serve(listener, app)
		.with_graceful_shutdown(async move { shutdown.cancelled().await })
		.await
		.context("serverless listener failed");

	// Drain the cached envoy regardless of how the server loop ended so an
	// in-flight actor's `Stopped` reaches the engine before shutdown.
	runtime.shutdown().await;

	serve_result
}

/// Resolves the listen port from `RIVET_PORT`, falling back to [`DEFAULT_PORT`].
fn listen_port() -> u16 {
	std::env::var("RIVET_PORT")
		.ok()
		.and_then(|value| value.parse().ok())
		.unwrap_or(DEFAULT_PORT)
}

/// Forwards a single HTTP request to the serverless runtime and streams the
/// response back.
async fn handle(State(runtime): State<CoreServerlessRuntime>, request: Request) -> Response {
	let (parts, body) = request.into_parts();

	let headers: HashMap<String, String> = parts
		.headers
		.iter()
		.filter_map(|(name, value)| {
			value
				.to_str()
				.ok()
				.map(|value| (name.as_str().to_ascii_lowercase(), value.to_owned()))
		})
		.collect();

	// `route_path` parses an absolute URL, so reconstruct one from the request
	// target and the Host header (origin-form requests carry no authority).
	let host = headers
		.get("host")
		.map(String::as_str)
		.unwrap_or("localhost");
	let path_and_query = parts
		.uri
		.path_and_query()
		.map(|pq| pq.as_str())
		.unwrap_or("/");
	let url = format!("http://{host}{path_and_query}");

	let body = match to_bytes(body, runtime.max_request_body_bytes()).await {
		Ok(body) => body,
		Err(_) => return into_response(runtime.incoming_too_long_response()),
	};

	// Cancelled when the response body is dropped (e.g. client disconnect),
	// which tears down this in-flight request without touching the cached envoy.
	let cancel_token = CancellationToken::new();

	let response = runtime
		.handle_request(ServerlessRequest {
			method: parts.method.as_str().to_owned(),
			url,
			headers,
			body: body.to_vec(),
			cancel_token: cancel_token.clone(),
		})
		.await;

	into_response_with_guard(response, cancel_token.drop_guard())
}

fn into_response(response: ServerlessResponse) -> Response {
	build_response(response, None)
}

fn into_response_with_guard(
	response: ServerlessResponse,
	guard: tokio_util::sync::DropGuard,
) -> Response {
	build_response(response, Some(guard))
}

/// Builds an axum response that streams the runtime's response chunks. `guard`,
/// when present, is held for the lifetime of the stream so dropping the body
/// cancels the in-flight request.
fn build_response(
	response: ServerlessResponse,
	guard: Option<tokio_util::sync::DropGuard>,
) -> Response {
	let ServerlessResponse {
		status,
		headers,
		body,
	} = response;

	let stream = UnboundedReceiverStream::new(body).map(move |item| {
		// Touch the guard so it lives as long as the stream is polled.
		let _ = &guard;
		item.map(Bytes::from)
			.map_err(|error| io::Error::new(io::ErrorKind::Other, error.message))
	});

	let mut builder = Response::builder()
		.status(StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
	for (name, value) in headers {
		builder = builder.header(name, value);
	}

	builder
		.body(Body::from_stream(stream))
		.unwrap_or_else(|error| {
			tracing::error!(?error, "failed to build serverless response");
			Response::builder()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.body(Body::empty())
				.expect("static error response is valid")
		})
}
