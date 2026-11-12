//! Forwarding to the V2 deployment a migration takes over: gateway requests and WebSockets
//! proxied unchanged. The V2 public API client lives in `pegboard::migrate_from_v2`.

use anyhow::{Context, Result};
use axum::body::Body;
use futures_util::{SinkExt, StreamExt};
use hyper::{HeaderMap, Method, StatusCode, Uri, header};
use rivet_config::config::MigrateFromV2;
use rivet_engine_context::EngineContext;
use rivet_env::capabilities::{
	http::{ClientWebSocket, Request},
	websocket::{CloseFrame, Message, WebSocket},
};
use rivet_env::{Clock, ClockError};
use std::{sync::Arc, time::Duration};

/// How long a proxied request waits for V2's response headers. V2's gateway may wake the
/// actor first, so this outlasts its own wake wait.
pub const V2_RESPONSE_TIMEOUT: Duration = Duration::from_secs(60);

/// V2's error header, `<group>.<code>`.
const X_RIVET_ERROR: &str = "x-rivet-error";

/// The error V2's gateway gives an actor it does not hold, in its error header and as the
/// prefix of a WebSocket close reason (`actor.not_found#<ray id>`).
const V2_ACTOR_NOT_FOUND: &str = "actor.not_found";

/// Connection-scoped headers a proxy must not forward, plus `host`, which names this gateway.
const HOP_BY_HOP_HEADERS: [&str; 9] = [
	"connection",
	"keep-alive",
	"proxy-authenticate",
	"proxy-authorization",
	"te",
	"trailer",
	"transfer-encoding",
	"upgrade",
	"host",
];

/// The V2 gateway URI for a request this gateway received as `original`. The path and query
/// are kept byte for byte, except that `rvt-namespace` names the V2 namespace. A keyed request
/// gets `rvt-namespace` even when the client left it out, because V2 requires it.
pub fn gateway_uri(config: &MigrateFromV2, original: &Uri, keyed: bool) -> Result<Uri> {
	let query = rewrite_namespace(original.query(), &config.namespace, keyed);
	let path = original.path();
	let path_and_query = match query {
		Some(query) => format!("{path}?{query}"),
		None => path.to_owned(),
	};
	let base = config.endpoint.trim_end_matches('/');
	format!("{base}{path_and_query}")
		.parse()
		.context("invalid V2 gateway URI")
}

/// `query` with `rvt-namespace` set to `namespace`. Every other parameter is kept as it was.
pub fn rewrite_namespace(query: Option<&str>, namespace: &str, ensure: bool) -> Option<String> {
	let encoded = serde_html_form::to_string([("rvt-namespace", namespace)]).ok()?;
	let mut found = false;
	let mut parts = Vec::new();
	for part in query
		.unwrap_or("")
		.split('&')
		.filter(|part| !part.is_empty())
	{
		let name = part.split('=').next().unwrap_or("");
		if name == "rvt-namespace" {
			found = true;
			parts.push(encoded.clone());
		} else {
			parts.push(part.to_owned());
		}
	}
	if !found && ensure {
		parts.push(encoded);
	}
	if parts.is_empty() {
		None
	} else {
		Some(parts.join("&"))
	}
}

/// The client's headers without connection-scoped ones. Credentials are kept: V2 authorizes
/// the request with the client's own token.
pub fn forward_headers(headers: &HeaderMap) -> HeaderMap {
	let mut forwarded = headers.clone();
	for name in HOP_BY_HOP_HEADERS {
		forwarded.remove(name);
	}
	forwarded
}

/// Whether V2's gateway answered that it does not hold the actor.
pub fn is_actor_not_found(status: StatusCode, headers: &HeaderMap) -> bool {
	status == StatusCode::NOT_FOUND
		&& headers
			.get(X_RIVET_ERROR)
			.and_then(|value| value.to_str().ok())
			.is_some_and(|value| value == V2_ACTOR_NOT_FOUND)
}

/// Whether V2's gateway closed a WebSocket because it does not hold the actor.
pub fn close_is_actor_not_found(frame: &CloseFrame) -> bool {
	let reason = frame.reason.as_str();
	reason
		.strip_prefix(V2_ACTOR_NOT_FOUND)
		.is_some_and(|rest| rest.is_empty() || rest.starts_with('#'))
}

/// How V2 answered a proxied request.
pub enum HttpOutcome {
	/// V2's response, streamed to the client as it arrives.
	Response(axum::response::Response),
	/// V2 does not hold the actor. Its response was dropped.
	NotFound,
}

/// Proxies one HTTP request to V2's gateway. Both bodies stream.
pub async fn http(
	ctx: &EngineContext,
	namespace_label: &str,
	uri: Uri,
	method: Method,
	headers: &HeaderMap,
	body: Body,
) -> Result<HttpOutcome> {
	// A streamed body has no known length, so a request without one says so explicitly.
	// Otherwise it would be sent chunked, which a bodiless GET must not be.
	let empty = hyper::body::Body::size_hint(&body).exact() == Some(0);
	let mut request = Request::builder().method(method).uri(uri).body(
		body.into_data_stream()
			.map(|chunk| chunk.map_err(anyhow::Error::from))
			.boxed(),
	)?;
	*request.headers_mut() = forward_headers(headers);
	if empty {
		request
			.headers_mut()
			.insert(header::CONTENT_LENGTH, header::HeaderValue::from(0));
	}

	#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
	let started = std::time::Instant::now();
	let clock = ctx.env().clock();
	let deadline = rivet_env::deadline_after(clock.as_ref(), V2_RESPONSE_TIMEOUT)?;
	let response = rivet_env::timeout_at(
		clock.as_ref(),
		deadline,
		ctx.external_http().execute(request),
	)
	.await
	.context("V2 gateway did not answer before the deadline")??;
	super::metrics::FALLBACK_DURATION
		.with_label_values(&[namespace_label])
		.observe(started.elapsed().as_secs_f64());

	if is_actor_not_found(response.status(), response.headers()) {
		return Ok(HttpOutcome::NotFound);
	}

	let (mut parts, body) = response.into_parts();
	for name in HOP_BY_HOP_HEADERS {
		parts.headers.remove(name);
	}
	let response = axum::response::Response::from_parts(parts, Body::from_stream(body));
	Ok(HttpOutcome::Response(response))
}

/// Opens the WebSocket to V2's gateway before the client's upgrade completes, so the client
/// can be offered the subprotocol V2 chose.
pub async fn websocket_connect(
	ctx: &EngineContext,
	namespace_label: &str,
	uri: Uri,
	headers: &HeaderMap,
) -> Result<ClientWebSocket> {
	let mut request = Request::builder().uri(uri).body(())?;
	*request.headers_mut() = forward_headers(headers);

	#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
	let started = std::time::Instant::now();
	let clock = ctx.env().clock();
	let deadline = rivet_env::deadline_after(clock.as_ref(), V2_RESPONSE_TIMEOUT)?;
	let socket = rivet_env::timeout_at(
		clock.as_ref(),
		deadline,
		ctx.external_http().connect_websocket(request),
	)
	.await
	.context("V2 gateway did not accept the WebSocket before the deadline")??;
	super::metrics::FALLBACK_DURATION
		.with_label_values(&[namespace_label])
		.observe(started.elapsed().as_secs_f64());
	Ok(socket)
}

/// The subprotocol V2 accepted, for the client's upgrade.
pub fn accepted_protocol(socket: &ClientWebSocket) -> Option<String> {
	socket
		.response
		.headers()
		.get(header::SEC_WEBSOCKET_PROTOCOL)
		.and_then(|value| value.to_str().ok())
		.map(ToOwned::to_owned)
}

/// Whichever socket made progress, or the keepalive interval.
enum Input {
	Client(Option<Result<Message, axum::Error>>),
	Upstream(Option<Result<Message>>),
	Keepalive(Result<u64, ClockError>),
}

/// Forwards frames both ways until either side closes. Each hop answers its own pings, so
/// ping and pong frames are not forwarded.
///
/// This gateway pings the client every `keepalive_interval_ms` itself, as the actor tunnel
/// does, because a proxy in front of it closes WebSockets that carry no frame for a while.
/// V2's own pings only keep the hop between this gateway and V2 alive.
///
/// When V2 closes because it does not hold the actor, `on_not_found` decides the client's
/// close; `None` forwards V2's close as it is.
pub async fn pump<F, Fut>(
	client: WebSocket,
	upstream: ClientWebSocket,
	on_not_found: F,
	clock: Arc<dyn Clock>,
	keepalive_interval_ms: u64,
) -> Result<()>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = Option<CloseFrame>>,
{
	let (mut client_sink, mut client_stream) = client.split();
	let ClientWebSocket {
		mut sink,
		mut stream,
		..
	} = upstream;
	let mut on_not_found = Some(on_not_found);

	// The first tick completes at once and is skipped; the socket was just opened. Missed ticks
	// are skipped too, so a connection that was busy for a while sends one ping, not a burst.
	let mut keepalive = rivet_env::interval(clock, keepalive_interval_ms);
	keepalive.tick().await.context("keepalive clock failed")?;

	loop {
		let input = rivet_env::select_biased!("V2 frames reach the client before more client input, and a keepalive ping never preempts either",
			message = stream.next() => Input::Upstream(message),
			message = client_stream.next() => Input::Client(message),
			tick = keepalive.tick() => Input::Keepalive(tick),
		);
		match input {
			Input::Upstream(Some(Ok(Message::Close(frame)))) => {
				let replacement = match (&frame, on_not_found.take()) {
					(Some(close), Some(decide)) if close_is_actor_not_found(close) => {
						decide().await
					}
					_ => None,
				};
				let close = replacement.or(frame);
				if let Err(error) = client_sink.send(Message::Close(close)).await {
					tracing::debug!(?error, "failed to forward the V2 close to the client");
				}
				return Ok(());
			}
			Input::Upstream(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => {}
			Input::Upstream(Some(Ok(message))) => {
				client_sink
					.send(message)
					.await
					.context("client WebSocket closed")?;
			}
			Input::Upstream(Some(Err(error))) => {
				let _ = client_sink.close().await;
				return Err(error.context("V2 WebSocket failed"));
			}
			Input::Upstream(None) => {
				let _ = client_sink.close().await;
				return Ok(());
			}
			Input::Client(Some(Ok(Message::Close(frame)))) => {
				if let Err(error) = sink.send(Message::Close(frame)).await {
					tracing::debug!(?error, "failed to forward the client close to V2");
				}
				return Ok(());
			}
			Input::Client(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => {}
			Input::Client(Some(Ok(message))) => {
				sink.send(message).await.context("V2 WebSocket closed")?;
			}
			Input::Client(Some(Err(error))) => {
				let _ = sink.close().await;
				return Err(anyhow::Error::from(error).context("client WebSocket failed"));
			}
			Input::Client(None) => {
				let _ = sink.close().await;
				return Ok(());
			}
			// A ping that cannot be sent means the client is gone, so V2 is closed too.
			Input::Keepalive(tick) => {
				tick.context("keepalive clock failed")?;
				if let Err(error) =
					crate::websocket_tunnel::send_keepalive_ping(&mut client_sink).await
				{
					let _ = sink.close().await;
					return Err(error);
				}
			}
		}
	}
}

/// A V2 public API response as the client should see it.
pub fn api_response(response: pegboard::migrate_from_v2::ApiResponse) -> axum::response::Response {
	let mut out = axum::response::Response::new(Body::from(response.body));
	*out.status_mut() = response.status;
	let headers = out.headers_mut();
	for name in [
		header::CONTENT_TYPE,
		header::HeaderName::from_static(X_RIVET_ERROR),
	] {
		if let Some(value) = response.headers.get(&name) {
			headers.insert(name, value.clone());
		}
	}
	out
}
