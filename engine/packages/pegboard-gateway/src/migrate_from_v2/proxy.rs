//! Forwarding to the V2 deployment a migration takes over: gateway requests and WebSockets
//! proxied unchanged apart from their credentials. V2 cannot verify this cluster's tokens, so the
//! gateway authorizes the client here, removes its token, and reaches V2 with the migration's
//! token instead. A token this cluster cannot authenticate, which may be one of V2's own, is
//! forwarded as the client sent it while the migration only passes through, and V2 authorizes it.
//! The V2 public API client lives in `pegboard::migrate_from_v2`.

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
use rivet_guard_core::auth::{WS_PROTOCOL_TOKEN, X_RIVET_TOKEN, bearer_token};
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

/// The prefix of a gateway path whose first segment selects the actor.
const GATEWAY_PATH_PREFIX: &str = "/gateway/";

/// The V2 gateway URI for a request this gateway received as `original`. The path and query
/// are kept byte for byte, except that `rvt-namespace` names the V2 namespace and, when
/// `credentials` replace the client's token, that token is removed from both: from `rvt-token`,
/// and from a direct selector's `{actor_id}@{token}`. A keyed request gets `rvt-namespace` even
/// when the client left it out, because V2 requires it.
pub fn gateway_uri(
	config: &MigrateFromV2,
	original: &Uri,
	keyed: bool,
	credentials: &V2Credentials<'_>,
) -> Result<Uri> {
	let strip_token = credentials.replaces_client_token();
	let query = rewrite_query(original.query(), &config.namespace, keyed, strip_token);
	let path = match strip_token {
		true => without_selector_token(original.path(), keyed),
		false => original.path().to_owned(),
	};
	let path = path.as_str();
	let path_and_query = match query {
		Some(query) => format!("{path}?{query}"),
		None => path.to_owned(),
	};
	let base = config.endpoint.trim_end_matches('/');
	format!("{base}{path_and_query}")
		.parse()
		.context("invalid V2 gateway URI")
}

/// `path` without the token a direct selector carries as `{actor_id}@{token}`. A keyed selector
/// is an actor name, whose `@` is not a token, so its path is kept.
fn without_selector_token(path: &str, keyed: bool) -> String {
	if keyed {
		return path.to_owned();
	}
	let Some(rest) = path.strip_prefix(GATEWAY_PATH_PREFIX) else {
		return path.to_owned();
	};
	let (selector, tail) = match rest.find('/') {
		Some(end) => rest.split_at(end),
		None => (rest, ""),
	};
	let actor = match selector.split_once('@') {
		Some((actor, _token)) => actor,
		None => selector,
	};
	format!("{GATEWAY_PATH_PREFIX}{actor}{tail}")
}

/// `query` with `rvt-namespace` set to `namespace`, and without `rvt-token` when `strip_token`.
/// Every other parameter is kept as it was.
pub fn rewrite_query(
	query: Option<&str>,
	namespace: &str,
	ensure: bool,
	strip_token: bool,
) -> Option<String> {
	let encoded = serde_html_form::to_string([("rvt-namespace", namespace)]).ok()?;
	let mut found = false;
	let mut parts = Vec::new();
	for part in query
		.unwrap_or("")
		.split('&')
		.filter(|part| !part.is_empty())
	{
		let name = part.split('=').next().unwrap_or("");
		if strip_token && name == "rvt-token" {
			continue;
		}
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

/// How a request proxied to V2 is authorized.
pub enum V2Credentials<'a> {
	/// This cluster authorized the client, whose token V2 cannot verify. The client's token is
	/// removed wherever it rode and V2 authorizes the request with the migration's token.
	Migration {
		/// The token this cluster authenticated the client with.
		client_token: Option<&'a str>,
		/// `migrate_from_v2.token`.
		v2_token: Option<&'a str>,
	},
	/// This cluster could not authenticate the client's token, which may be one of V2's own, so V2
	/// authorizes the request with the client's credentials exactly as it sent them. The
	/// migration's token is never attached, since nothing here vouched for the client.
	Client,
}

impl V2Credentials<'_> {
	/// Whether the client's token is removed before the request reaches V2.
	fn replaces_client_token(&self) -> bool {
		match self {
			Self::Migration { .. } => true,
			Self::Client => false,
		}
	}
}

/// The client's headers without connection-scoped ones. With [`V2Credentials::Migration`] the
/// client's token is removed too and the migration's token takes its place. V2's gateway reads its
/// token from `x-rivet-token` for a request and from a `rivet_token.` subprotocol for a WebSocket,
/// and strips both before actor code sees the request, so the token goes there. An
/// `Authorization` bearer is removed only when it is the client's token, so an actor-level bearer
/// still reaches the actor.
pub fn forward_headers(
	headers: &HeaderMap,
	credentials: &V2Credentials<'_>,
	websocket: bool,
) -> Result<HeaderMap> {
	let mut forwarded = headers.clone();
	for name in HOP_BY_HOP_HEADERS {
		forwarded.remove(name);
	}
	let (client_token, v2_token) = match credentials {
		V2Credentials::Migration {
			client_token,
			v2_token,
		} => (*client_token, *v2_token),
		V2Credentials::Client => return Ok(forwarded),
	};
	forwarded.remove(X_RIVET_TOKEN);

	let client_bearer = forwarded
		.get(header::AUTHORIZATION)
		.and_then(|value| value.to_str().ok())
		.and_then(bearer_token)
		.is_some_and(|token| Some(token) == client_token);
	if client_bearer {
		forwarded.remove(header::AUTHORIZATION);
	}

	// Every subprotocol but a token one is kept, in order, on one header line.
	let mut protocols = forwarded
		.get_all(header::SEC_WEBSOCKET_PROTOCOL)
		.iter()
		.filter_map(|value| value.to_str().ok())
		.flat_map(|value| value.split(','))
		.map(str::trim)
		.filter(|protocol| !protocol.is_empty() && !protocol.starts_with(WS_PROTOCOL_TOKEN))
		.map(ToOwned::to_owned)
		.collect::<Vec<_>>();
	forwarded.remove(header::SEC_WEBSOCKET_PROTOCOL);

	if let Some(v2_token) = v2_token {
		if websocket {
			protocols.push(format!("{WS_PROTOCOL_TOKEN}{v2_token}"));
		} else {
			forwarded.insert(
				X_RIVET_TOKEN,
				header::HeaderValue::from_str(v2_token)
					.context("migrate_from_v2.token is not a valid header value")?,
			);
		}
	}
	if !protocols.is_empty() {
		forwarded.insert(
			header::SEC_WEBSOCKET_PROTOCOL,
			header::HeaderValue::from_str(&protocols.join(", "))
				.context("invalid WebSocket subprotocols")?,
		);
	}
	Ok(forwarded)
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
	credentials: &V2Credentials<'_>,
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
	*request.headers_mut() = forward_headers(headers, credentials, false)?;
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
	credentials: &V2Credentials<'_>,
) -> Result<ClientWebSocket> {
	let mut request = Request::builder().uri(uri).body(())?;
	*request.headers_mut() = forward_headers(headers, credentials, true)?;

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

/// The subprotocol V2 accepted, for the client's upgrade. A token subprotocol is never offered
/// to the client, since it would carry the migration's token.
pub fn accepted_protocol(socket: &ClientWebSocket) -> Option<String> {
	socket
		.response
		.headers()
		.get(header::SEC_WEBSOCKET_PROTOCOL)
		.and_then(|value| value.to_str().ok())
		.filter(|protocol| !protocol.trim().starts_with(WS_PROTOCOL_TOKEN))
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
