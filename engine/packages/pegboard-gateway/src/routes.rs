//! The gateway's HTTP edge: authentication, the request's routing parameters, header hygiene
//! and the hand-off of an acquired actor stream to the HTTP or WebSocket tunnel.
use anyhow::{Context, Result};
use axum::response::IntoResponse;
use axum::{
	body::Body,
	extract::{OriginalUri, Path, RawQuery, State, ws::rejection::WebSocketUpgradeRejection},
	http::{Method, StatusCode},
	response::Response,
};
use axum_extra::extract::{Query, QueryRejection};
use hyper::{HeaderMap, header};
use rivet_engine_context::EngineContext;
use rivet_env::capabilities::websocket::{CloseFrame, Message, WebSocketUpgrade};
use rivet_envoy_protocol as protocol;
use rivet_guard_core::{RequestContext, auth, errors};
use rivet_util::Id;
use rivetkit_shared_types::actor_key;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, hash_map::Entry};

use crate::acquire::{CreateActor, RoutingDeadline, acquire};
use crate::raw_rest_path::RawRestPath;
use crate::retry_budget::RetryBudget;
use crate::start_waits::StartWaits;
use crate::{GatewayError, QueryInvalidParams};
use std::sync::Arc;

/// Read chunk for each client WebSocket. Tungstenite allocates it per socket up front, and its
/// 128 KiB default dominated gateway memory with many held, mostly idle actor sockets. Larger
/// messages still arrive; the buffer grows while they are read.
const ACTOR_WEBSOCKET_READ_BUFFER_BYTES: usize = 4 * 1024;

/// Query parameters with this prefix are reserved for routing and never reach the actor.
const RESERVED_QUERY_PREFIX: &str = "rvt-";

/// The reserved parameters a request may carry. `rvt-region` and `rvt-skip-ready-wait` are
/// accepted because clients send them, although this engine does not act on them. Any other
/// `rvt-*` parameter is a client mistake that silently dropping would hide.
const RESERVED_QUERY_PARAMS: &[&str] = &[
	"rvt-namespace",
	"rvt-runner",
	"rvt-pool",
	"rvt-crash-policy",
	"rvt-method",
	"rvt-key",
	"rvt-input",
	"rvt-token",
	"rvt-region",
	"rvt-skip-ready-wait",
];

/// A reserved query parameter this gateway does not define.

const X_RIVET_TARGET: &str = "x-rivet-target";

/// HTTP header naming the actor of a request whose target is an actor.
const X_RIVET_ACTOR: &str = "x-rivet-actor";

/// WebSocket subprotocol prefix carrying the routing target, as `rivet_target.<target>`.
const WS_PROTOCOL_TARGET: &str = "rivet_target.";

/// WebSocket subprotocol prefix carrying the actor id, as `rivet_actor.<id>`.
const WS_PROTOCOL_ACTOR: &str = "rivet_actor.";

/// The routing target that names an actor.
const ACTOR_TARGET: &str = "actor";

/// A request named an actor as its target without saying which one.
#[derive(Debug, Serialize, rivet_error::RivetError)]
#[error(
	"guard",
	"missing_header",
	"Missing header required for routing.",
	"Missing {header} header."
)]
pub(crate) struct MissingHeader {
	pub(crate) header: String,
}

#[derive(Default, Deserialize)]
pub struct GatewayQuery {
	#[serde(rename = "rvt-namespace", default)]
	namespace: Option<String>,
	#[serde(rename = "rvt-runner", default)]
	runner: Option<String>,
	#[serde(rename = "rvt-pool", default)]
	pool: Option<String>,
	#[serde(rename = "rvt-crash-policy", default)]
	crash_policy: Option<String>,
	#[serde(rename = "rvt-method", default)]
	method: Option<GatewayMethod>,
	#[serde(rename = "rvt-key", default)]
	key: Option<String>,
	#[serde(rename = "rvt-input", default)]
	input: Option<String>,
	#[serde(rename = "rvt-token", default)]
	token: Option<String>,
}

impl GatewayQuery {
	/// The public pool parameter takes precedence over its runner alias.
	fn pool_name(&self) -> &str {
		self.pool
			.as_deref()
			.or(self.runner.as_deref())
			.unwrap_or(pegboard::DEFAULT_POOL)
	}
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum GatewayMethod {
	Get,
	GetOrCreate,
}

/// Every failure, from a malformed query to a tunnel that stops, reaches the client in the
/// canonical edge error format: an HTTP request gets the canonical error response, and a
/// WebSocket is upgraded and then closed with the error, since browsers cannot read the status of
/// a refused upgrade.
#[tracing::instrument(skip_all)]
pub async fn gateway(
	State(ctx): State<EngineContext>,
	axum::Extension(waits): axum::Extension<Arc<StartWaits>>,
	axum::Extension(budget): axum::Extension<Arc<RetryBudget>>,
	request: Option<axum::Extension<RequestContext>>,
	Path(selector): Path<String>,
	RawRestPath(path_rest): RawRestPath,
	RawQuery(raw_query): RawQuery,
	query: Result<Query<GatewayQuery>, QueryRejection>,
	method: Method,
	headers: HeaderMap,
	ws: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
	body: Body,
) -> Response {
	let ray_id = request.map(|axum::Extension(request)| request.ray_id());
	let query = match query {
		Ok(Query(query)) => query,
		Err(rejection) => {
			let error = QueryInvalidParams {
				detail: rejection.to_string(),
			}
			.build();
			return reject(ws, error, ray_id);
		}
	};

	let preparing = prepare(
		&ctx,
		&waits,
		&budget,
		&selector,
		path_rest,
		raw_query,
		query,
		&headers,
		ws.is_ok(),
	);
	let prepared = if ws.is_ok() {
		let open_timeout = std::time::Duration::from_millis(
			ctx.config().pegboard().gateway_websocket_open_timeout_ms(),
		);
		match rivet_env::timeout(ctx.env().clock().as_ref(), open_timeout, preparing).await {
			Ok(result) => result,
			Err(_) => {
				tracing::warn!(
					timeout_ms = open_timeout.as_millis() as u64,
					"actor WebSocket open timed out waiting for its actor"
				);
				return match ws {
					Ok(ws) => WebSocketRejection::Retry {
						retry_after_ms: WEBSOCKET_OPEN_RETRY_AFTER_MS,
					}
					.close(ws),
					Err(upgrade_error) => reject(
						Err(upgrade_error),
						crate::QueryInvalidParams {
							detail: "WebSocket upgrade unavailable".to_owned(),
						}
						.build(),
						ray_id,
					),
				};
			}
		}
	} else {
		preparing.await
	};
	let prepared = match prepared {
		Ok(prepared) => prepared,
		Err(error) => return reject(ws, error, ray_id),
	};
	let Prepared {
		actor_id,
		epoch,
		stream,
		message_id,
		class,
		path,
		request_headers,
		credential,
	} = prepared;

	match ws {
		Ok(ws) => {
			let protocols = forwarded_protocols(&ws);
			let open = websocket_open(actor_id, epoch, &path, &request_headers);
			// A pipelined open can reach the envoy before the actor's start does, and is then
			// refused before admission. It is opened again once the actor runs, on a new
			// stream, so the client never sees the refusal.
			let reopen = move || async move {
				let deadline = RoutingDeadline::new(&ctx)?;
				let (epoch, stream) = acquire(
					&ctx,
					&waits,
					actor_id,
					None,
					class,
					false,
					&credential,
					deadline,
				)
				.await?;
				let message_id = new_message_id(&ctx)?;
				let open = websocket_open(actor_id, epoch, &path, &request_headers);
				anyhow::Ok((stream, message_id, open))
			};

			ws.protocols(protocols)
				.read_buffer_size(ACTOR_WEBSOCKET_READ_BUFFER_BYTES)
				.on_upgrade(move |mut socket| async move {
					if let Err(error) = crate::websocket_tunnel::proxy(
						&mut socket,
						stream,
						message_id,
						open,
						reopen,
					)
					.await
					{
						tracing::warn!(%actor_id, %error, "actor WebSocket tunnel stopped");

						// Close with the error so the client sees why instead of a reset. A
						// client that already went away cannot receive it.
						let (code, reason) = errors::close_frame(&error, ray_id);
						let close = Message::Close(Some(CloseFrame {
							code,
							reason: reason.into(),
						}));
						if let Err(error) = socket.send(close).await {
							tracing::debug!(?error, "failed to send actor WebSocket tunnel close");
						}
					}
				})
		}
		Err(_) => {
			let start = protocol::ToEnvoyRequestStart {
				actor_id: actor_id.to_string().into(),
				actor_generation: Some(epoch),
				method: method.to_string().into(),
				path: path.into(),
				headers: request_headers
					.into_iter()
					.map(|(k, v)| (k.into(), v.into()))
					.collect(),
				body: None,
				stream: true,
				response_stream: true,
			};
			match crate::http_tunnel::proxy(&ctx, stream, body, message_id, start).await {
				Ok(response) => response,
				Err(error) => GatewayError::from(error).into_response(),
			}
		}
	}
}

/// A request whose actor stream is open, with what the tunnel forwards to the actor.
struct Prepared {
	actor_id: Id,
	epoch: u32,
	stream: universalpubsub::DuplexStream,
	message_id: protocol::MessageId,
	class: universalpubsub::RequestClass,
	path: String,
	request_headers: HashMap<String, String>,
	credential: auth::Credential,
}

/// Authenticates the request, resolves its actor, opens a stream to it and builds what the tunnel
/// forwards. Nothing has reached the actor when this fails.
#[allow(clippy::too_many_arguments)]
async fn prepare(
	ctx: &EngineContext,
	waits: &Arc<StartWaits>,
	budget: &RetryBudget,
	selector: &str,
	path_rest: String,
	raw_query: Option<String>,
	query: GatewayQuery,
	headers: &HeaderMap,
	websocket: bool,
) -> Result<Prepared> {
	let deadline = RoutingDeadline::new(ctx)?;
	let namespace = query
		.namespace
		.clone()
		.unwrap_or_else(|| pegboard::DEFAULT_NAMESPACE.to_owned());
	let pool_name = query.pool_name().to_owned();
	pegboard::validation::pool_name(&pool_name)?;
	if let Some(policy) = query.crash_policy.as_deref()
		&& policy != "sleep"
	{
		return Err(QueryInvalidParams {
			detail: format!("unsupported crash policy: {policy}, expected sleep"),
		}
		.build());
	}

	// Authenticate before any routing work. A token may ride in the selector as
	// `{actor_id}@{token}` when the selector names an actor directly, in `rvt-token`, or in the
	// transport. A keyed selector is an actor name, so an `@` there is never read as a token.
	let selector_token = match query.method {
		Some(_) => None,
		None => selector.split_once('@').map(|(_, token)| token),
	};
	let token = selector_token
		.or(query.token.as_deref())
		.or_else(|| auth::transport_token(headers));
	let credential = auth::authenticate(ctx, token).await?;
	// Query resolution may create an actor, so it needs actor access before acquisition.
	// Gateway access alone permits traffic to an existing actor and cannot create one.
	if let Some(method) = query.method {
		credential.authorize(
			ctx.env().clock().observe()?.unix_ms,
			&namespace,
			auth::ResourceKind::Actor,
			None,
			match method {
				GatewayMethod::Get => auth::OperationKind::Read,
				GatewayMethod::GetOrCreate => auth::OperationKind::Create,
			},
		)?;
	}
	pegboard::catalog::ensure_namespace(ctx, &namespace).await?;

	check_reserved_params(raw_query.as_deref())?;
	let message_id = new_message_id(ctx)?;
	let target = Target::of(
		ctx.config().dc_label(),
		selector,
		&query,
		namespace,
		pool_name,
	)?;
	let class = tunnel_class(websocket);

	// The gateway owns retries of a refused request. Nothing reaches the actor before the
	// stream opens, so a refused attempt runs again from the start, within one request deadline.
	budget.deposit();
	let mut retries = budget.retries(deadline.boot_ms);
	let (epoch, stream) = loop {
		let attempt = acquire(
			ctx,
			waits,
			target.actor_id,
			target.create(),
			class,
			true,
			&credential,
			deadline,
		)
		.await;
		let error = match attempt {
			Ok(acquired) => break acquired,
			Err(error) => error,
		};
		if rivet_error::RivetError::extract(&error).group() == "auth" {
			return Err(error);
		}
		retries.retry(ctx.env().as_ref(), error).await?;
	};

	// Reserved routing parameters must not become actor application parameters. The rest of
	// the query is the actor's to interpret, so it is forwarded as the client sent it.
	let path = match application_query(raw_query.as_deref()) {
		Some(application_query) => format!("{path_rest}?{application_query}"),
		None => path_rest,
	};

	Ok(Prepared {
		actor_id: target.actor_id,
		epoch,
		stream,
		message_id,
		class,
		path,
		request_headers: forwarded_headers(ctx, headers),
		credential,
	})
}

/// Routes header and subprotocol actor selectors through the gateway's common edge.
#[tracing::instrument(skip_all)]
pub async fn direct(
	State(ctx): State<EngineContext>,
	axum::Extension(waits): axum::Extension<Arc<StartWaits>>,
	axum::Extension(budget): axum::Extension<Arc<RetryBudget>>,
	request: Option<axum::Extension<RequestContext>>,
	OriginalUri(uri): OriginalUri,
	method: Method,
	headers: HeaderMap,
	ws: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
	body: Body,
) -> Response {
	let path = uri
		.path_and_query()
		.map_or(uri.path(), |path| path.as_str());
	if !targets_actor(&headers, ws.is_ok(), path) {
		return not_found();
	}
	let ray_id = request
		.as_ref()
		.map(|axum::Extension(request)| request.ray_id());
	if let Err(error) = auth::authenticate(&ctx, auth::transport_token(&headers)).await {
		return reject(ws, error, ray_id);
	}
	let actor_id = match direct_actor_id(&headers, ws.is_ok()) {
		Ok(actor_id) => actor_id,
		Err(error) => return reject(ws, error, ray_id),
	};
	gateway(
		State(ctx),
		axum::Extension(waits),
		axum::Extension(budget),
		request,
		Path(actor_id.to_string()),
		RawRestPath(uri.path().to_owned()),
		RawQuery(uri.query().map(ToOwned::to_owned)),
		Ok(Query(GatewayQuery::default())),
		method,
		headers,
		ws,
		body,
	)
	.await
}

fn not_found() -> Response {
	(StatusCode::NOT_FOUND, "amogus\n").into_response()
}

/// Whether a request outside the gateway paths names an actor as its target: by the
/// `x-rivet-target` header, or for a WebSocket by its `rivet_target.` subprotocol. An HTTP
/// request also has to be on the `/request` path, the only path an actor's router serves this
/// way; a WebSocket may open on any path.
fn targets_actor(headers: &HeaderMap, websocket: bool, path: &str) -> bool {
	let target = if websocket {
		protocol_value(headers, WS_PROTOCOL_TARGET)
	} else {
		headers
			.get(X_RIVET_TARGET)
			.and_then(|value| value.to_str().ok())
	};
	if target != Some(ACTOR_TARGET) {
		return false;
	}
	websocket || is_actor_request_path(path)
}

/// Whether `path`, query included, is `/request`, `/request/...` or `/request?...`.
fn is_actor_request_path(path: &str) -> bool {
	let Some(rest) = path.strip_prefix("/request") else {
		return false;
	};
	rest.is_empty() || rest.starts_with('/') || rest.starts_with('?')
}

/// The actor a request that [`targets_actor`] names: the `x-rivet-actor` header, or for a
/// WebSocket the `rivet_actor.` subprotocol.
fn direct_actor_id(headers: &HeaderMap, websocket: bool) -> Result<Id> {
	let (header, actor_id) = if websocket {
		(
			format!("`{WS_PROTOCOL_ACTOR}*` protocol in sec-websocket-protocol"),
			protocol_value(headers, WS_PROTOCOL_ACTOR),
		)
	} else {
		let actor_id = headers
			.get(X_RIVET_ACTOR)
			.map(|value| value.to_str())
			.transpose()
			.context("invalid x-rivet-actor header")?;
		(X_RIVET_ACTOR.to_owned(), actor_id)
	};
	let Some(actor_id) = actor_id else {
		return Err(MissingHeader { header }.build());
	};
	Id::parse(actor_id).context("invalid actor id in request headers")
}

/// The value of the first `sec-websocket-protocol` entry with `prefix`, across every header line.
fn protocol_value<'a>(headers: &'a HeaderMap, prefix: &str) -> Option<&'a str> {
	headers
		.get_all(header::SEC_WEBSOCKET_PROTOCOL)
		.iter()
		.filter_map(|value| value.to_str().ok())
		.flat_map(|protocols| protocols.split(','))
		.find_map(|protocol| protocol.trim().strip_prefix(prefix))
}

/// The request headers forwarded to the actor. Repeated headers are joined into one value since
/// the protocol carries one value per name, and the engine's credential stays at the edge.
fn forwarded_headers(ctx: &EngineContext, headers: &HeaderMap) -> HashMap<String, String> {
	let mut request_headers = HashMap::<String, String>::new();
	for (name, value) in headers.iter() {
		let Ok(value) = value.to_str() else {
			continue;
		};
		if name.as_str().eq_ignore_ascii_case(auth::X_RIVET_TOKEN) {
			continue;
		}
		if name == header::AUTHORIZATION && auth::is_engine_bearer(ctx.config(), value) {
			continue;
		}
		let stripped;
		let value = if name == header::SEC_WEBSOCKET_PROTOCOL {
			stripped = strip_token_protocol(value);
			stripped.as_str()
		} else {
			value
		};

		match request_headers.entry(name.as_str().to_owned()) {
			Entry::Occupied(mut entry) => {
				let separator = if name == header::COOKIE { "; " } else { ", " };
				let joined = entry.get_mut();
				joined.push_str(separator);
				joined.push_str(value);
			}
			Entry::Vacant(entry) => {
				entry.insert(value.to_owned());
			}
		}
	}
	request_headers
}

/// The WebSocket subprotocols the client asked for, without the engine's token protocol.
fn forwarded_protocols(ws: &WebSocketUpgrade) -> Vec<String> {
	ws.requested_protocols()
		.filter_map(|p| p.to_str().ok())
		.filter(|p| !auth::is_token_protocol(p))
		.map(ToOwned::to_owned)
		.collect()
}

/// The reply to a request that failed before its actor stream opened. A WebSocket is upgraded
/// only to be closed with the error, so a browser can read why; HTTP gets the canonical error
/// response.
fn reject(
	ws: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
	error: anyhow::Error,
	ray_id: Option<Id>,
) -> Response {
	let ws = match ws {
		Ok(ws) => ws,
		Err(_) => return GatewayError::from(error).into_response(),
	};

	crate::log_failure(&error);
	if let Some(refusal) = crate::retry_budget::refusal(&error) {
		return WebSocketRejection::Retry {
			retry_after_ms: refusal.retry_after_ms,
		}
		.close(ws);
	}
	let error = match crate::retry_budget::refusal(&error) {
		Some(refusal) => crate::retry_budget::refusal_error(refusal),
		None => error,
	};
	let (code, reason) = errors::close_frame(&error, ray_id);
	let protocols = forwarded_protocols(&ws);
	ws.protocols(protocols)
		.on_upgrade(move |mut socket| async move {
			let close = Message::Close(Some(CloseFrame {
				code,
				reason: reason.into(),
			}));
			if let Err(error) = socket.send(close).await {
				tracing::debug!(?error, "failed to send gateway rejection close");
			}
		})
}

/// The actor a request goes to, and what a getOrCreate creates there.
struct Target {
	actor_id: Id,
	keyed: Option<KeyedTarget>,
}

struct KeyedTarget {
	method: GatewayMethod,
	namespace: String,
	pool_name: String,
	name: String,
	key: String,
	input: Option<String>,
}

impl Target {
	/// Resolves the selector: an actor id, or with `rvt-method` an actor name whose key names
	/// the actor. A keyed actor's id is the key's placement with this datacenter's label, so no
	/// lookup is needed; a key another datacenter owns is learned from the create.
	fn of(
		dc_label: u16,
		selector: &str,
		query: &GatewayQuery,
		namespace: String,
		pool_name: String,
	) -> Result<Self> {
		let Some(method) = query.method else {
			let actor_id = match selector.split_once('@') {
				Some((actor_id, _token)) => Id::parse(actor_id)?,
				None => Id::parse(selector).context(
					"invalid actor selector (parsing as ID). include rvt-method to use name-key selection instead",
				)?,
			};
			return Ok(Self {
				actor_id,
				keyed: None,
			});
		};

		// The key is hashed and stored in its canonical form, which the actor API translates
		// its slash form to, so `a,b` here and `a/b` through the API name one actor. Clients
		// omit `rvt-key` for the empty key.
		let key = actor_key::encode(&actor_key::from_gateway_query(query.key.as_deref()));
		pegboard::validation::actor_name(selector)?;
		pegboard::validation::actor_key(Some(&key))?;
		Ok(Self {
			actor_id: pegboard::actor_key::keyed_actor_id(dc_label, &namespace, selector, &key),
			keyed: Some(KeyedTarget {
				method,
				namespace,
				pool_name,
				name: selector.to_owned(),
				key,
				input: query.input.clone(),
			}),
		})
	}

	/// What a getOrCreate creates. A get creates nothing and fails when the actor is missing.
	fn create(&self) -> Option<CreateActor> {
		let keyed = self.keyed.as_ref()?;
		match keyed.method {
			GatewayMethod::Get => None,
			GatewayMethod::GetOrCreate => Some(CreateActor {
				namespace: keyed.namespace.clone(),
				pool_name: keyed.pool_name.clone(),
				name: keyed.name.clone(),
				key: keyed.key.clone(),
				input: keyed.input.clone(),
			}),
		}
	}
}

/// A fresh tunnel identity for one request's stream to its actor.
fn new_message_id(ctx: &EngineContext) -> Result<protocol::MessageId> {
	let random = ctx.env().rng("gateway.request").uuid_v4();
	Ok(protocol::MessageId {
		gateway_id: random.as_bytes()[..4].try_into()?,
		request_id: random.as_bytes()[4..8].try_into()?,
		message_index: 0,
	})
}

/// The open message of a WebSocket to `epoch` of the actor. The envoy protocol calls the epoch
/// the actor generation.
fn websocket_open(
	actor_id: Id,
	epoch: u32,
	path: &str,
	headers: &HashMap<String, String>,
) -> protocol::ToEnvoyWebSocketOpen {
	protocol::ToEnvoyWebSocketOpen {
		actor_id: actor_id.to_string().into(),
		actor_generation: Some(epoch),
		path: path.to_owned().into(),
		headers: headers
			.iter()
			.map(|(name, value)| (name.clone().into(), value.clone().into()))
			.collect(),
	}
}

/// WebSocket tunnels are held for the socket's life, so they are capped apart from HTTP tunnels.
fn tunnel_class(websocket: bool) -> universalpubsub::RequestClass {
	if websocket {
		universalpubsub::RequestClass::GatewayWebSocket
	} else {
		universalpubsub::RequestClass::GatewayHttp
	}
}

/// The query the actor receives: `raw_query` without its reserved `rvt-*` parameters, byte for
/// byte. Reparsing and re-encoding the query would reorder it and normalize its encoding, and
/// both belong to the actor; a query without reserved parameters is returned unchanged.
fn application_query(raw_query: Option<&str>) -> Option<String> {
	let raw_query = raw_query.filter(|query| !query.is_empty())?;
	if !raw_query.split('&').any(is_reserved_param) {
		return Some(raw_query.to_owned());
	}
	let application_parts: Vec<&str> = raw_query
		.split('&')
		.filter(|part| !part.is_empty() && !is_reserved_param(part))
		.collect();
	if application_parts.is_empty() {
		None
	} else {
		Some(application_parts.join("&"))
	}
}

/// Whether one `key=value` part of a raw query is a reserved routing parameter.
fn is_reserved_param(part: &str) -> bool {
	param_key(part).starts_with(RESERVED_QUERY_PREFIX)
}

/// The key of one `key=value` part of a raw query.
fn param_key(part: &str) -> &str {
	part.split('=').next().unwrap_or("")
}

/// Refuses a query with a reserved parameter this gateway does not define, so a mistyped
/// routing parameter is reported instead of being dropped.
fn check_reserved_params(raw_query: Option<&str>) -> Result<()> {
	let Some(raw_query) = raw_query else {
		return Ok(());
	};
	for part in raw_query.split('&') {
		let key = param_key(part);
		if key.starts_with(RESERVED_QUERY_PREFIX) && !RESERVED_QUERY_PARAMS.contains(&key) {
			return Err(QueryInvalidParams {
				detail: format!("unknown parameter `{key}`"),
			}
			.build());
		}
	}
	Ok(())
}



/// The close code for an open the client should retry later.
const TRY_AGAIN_LATER_CLOSE_CODE: u16 = 1013;

/// Retry hint for a WebSocket open that ran out of time waiting for its actor.
const WEBSOCKET_OPEN_RETRY_AFTER_MS: u64 = 1_000;

/// Why an actor WebSocket failed to open.
enum WebSocketRejection {
	/// The open may succeed later; the client retries after `retry_after_ms`.
	Retry { retry_after_ms: u64 },
	
}

impl WebSocketRejection {
	

	/// The close code and reason this rejection closes the client WebSocket with.
	fn close_frame(&self) -> (u16, String) {
		match self {
			WebSocketRejection::Retry { retry_after_ms } => (
				TRY_AGAIN_LATER_CLOSE_CODE,
				websocket_retry_close_reason(*retry_after_ms),
			),
			
		}
	}

	/// Accepts the upgrade only to close it with this rejection's code and reason.
	fn close(self, ws: WebSocketUpgrade) -> Response {
		let (code, reason) = self.close_frame();
		
		let protocols = ws
			.requested_protocols()
			.filter_map(|p| p.to_str().ok())
			.filter(|p| !auth::is_token_protocol(p))
			.map(ToOwned::to_owned)
			.collect::<Vec<String>>();
		ws.protocols(protocols)
			.on_upgrade(move |mut socket| async move {
				let close = Message::Close(Some(CloseFrame {
					code,
					reason: reason.into(),
				}));
				if let Err(error) = socket.send(close).await {
					tracing::debug!(?error, "failed to send actor WebSocket open rejection");
				}
			})
	}
}

/// The close reason for an actor WebSocket open the client should retry:
/// `guard.service_unavailable`, the code HTTP refusals use, with the hint after `#`. Released
/// clients read `<group>.<code>` up to the first `#` and treat the rest as an opaque ray id, so
/// they still classify the close as retryable; a `;` parameter would corrupt their code.
fn websocket_retry_close_reason(retry_after_ms: u64) -> String {
	format!("guard.service_unavailable#retry_after_ms={retry_after_ms}")
}

/// `protocols` without its `rivet_token.` entries, so the engine's credential is not forwarded
/// to the actor.
fn strip_token_protocol(protocols: &str) -> String {
	protocols
		.split(',')
		.map(str::trim)
		.filter(|protocol| !auth::is_token_protocol(protocol))
		.collect::<Vec<_>>()
		.join(", ")
}




