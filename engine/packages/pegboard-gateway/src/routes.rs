//! The gateway's HTTP edge: authentication, the request's routing parameters, header hygiene
//! and the hand-off of an acquired actor stream to the HTTP or WebSocket tunnel.
use anyhow::{Context, Result, anyhow};
use axum::response::IntoResponse;
use axum::{
	body::Body,
	extract::{OriginalUri, Path, RawQuery, State, ws::rejection::WebSocketUpgradeRejection},
	http::{Method, StatusCode},
	response::Response,
};
use axum_extra::extract::{Query, QueryRejection};
use hyper::{HeaderMap, header};
use pegboard::errors::ActorMigratingFromV2;
use rivet_config::config::Root;
use rivet_engine_context::EngineContext;
use rivet_env::capabilities::websocket::{CloseFrame, Message, WebSocketUpgrade};
use rivet_envoy_protocol as protocol;
use rivet_guard_core::{RequestContext, auth, errors};
use rivet_util::Id;
use rivetkit_shared_types::actor_key;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, hash_map::Entry};

use crate::acquire::{
	AcquireTarget, CreateActor, DelegatedToV2, HeldByMigrateFromV2, RoutingDeadline, acquire,
};
use crate::migrate_from_v2::{self, V3Actor, decision, proxy};
use crate::namespace_fallback::{NamespaceFallbacks, RouteKind};
use crate::raw_rest_path::RawRestPath;
use crate::retry_budget::RetryBudget;
use crate::start_waits::StartWaits;
use crate::{ActorNotFound, GatewayError, QueryInvalidParams};
use pegboard::legacy_volumes::ResolvedRead;
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

/// HTTP header naming the namespace of a request whose target is an actor.
const X_RIVET_NAMESPACE: &str = "x-rivet-namespace";

/// WebSocket subprotocol prefix carrying the namespace, as `rivet_namespace.<name>`.
const WS_PROTOCOL_NAMESPACE: &str = "rivet_namespace.";

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
	/// The entry the request arrived on, set by its route and never read from the query.
	#[serde(skip)]
	route: RouteKind,
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
	axum::Extension(fallbacks): axum::Extension<Arc<NamespaceFallbacks>>,
	request: Option<axum::Extension<RequestContext>>,
	OriginalUri(original_uri): OriginalUri,
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

	// While a V2 namespace migrates into this one, actors this cluster does not hold are
	// served by V2. Whether the request takes part is decided from the namespace it resolves
	// to after authentication, which `prepare` reports even when it then fails.
	let dynamic = migrate_from_v2::snapshot(&ctx);
	let mut resolved_namespace = None;
	let keyed = query.method.is_some();

	let preparing = prepare(
		&ctx,
		&waits,
		&budget,
		&fallbacks,
		dynamic.as_deref(),
		&mut resolved_namespace,
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
	let migration = resolved_namespace.as_deref().and_then(|namespace| {
		migrate_from_v2::Active::for_namespace(dynamic.as_deref(), namespace)
	});
	let prepared = match prepared {
		Ok(Routed::V3(prepared)) => prepared,
		Ok(Routed::V2(v2)) => {
			let Some(migration) = &migration else {
				let error = anyhow!("routed to V2 without a migration from V2");
				return reject(ws, error, ray_id);
			};
			let request = V2Request {
				original_uri,
				method,
				headers,
				keyed,
			};
			return match proxy_to_v2(&ctx, migration, v2, request, ws, body, ray_id).await {
				Ok(response) => response,
				Err(error) => {
					// A V2 answer of not_found is counted by the route it resolves to; anything
					// else is a request V2 did not serve.
					if refused_route(&error).is_none() {
						migration.count_request("v2_error");
					}
					GatewayError::from(error).into_response()
				}
			};
		}
		Err(error) => {
			if let Some(migration) = &migration
				&& let Some(route) = refused_route(&error)
			{
				migration.count_request(route);
			}
			return reject(ws, error, ray_id);
		}
	};
	if let Some(migration) = &migration {
		migration.count_request("v3");
	}
	let Prepared {
		actor_id,
		namespace,
		group,
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
			let open = websocket_open(actor_id, group, epoch, &path, &request_headers);
			let clock = ctx.env().clock();
			let keepalive_interval_ms = ctx.config().guard().websocket_keepalive_interval_ms();
			// Each stream the tunnel opens uses one envoy protocol version: the one the
			// envoy's host worker reads.
			let version_of = {
				let ctx = ctx.clone();
				move |stream: &universalpubsub::DuplexStream| {
					pegboard::envoy_protocol::version_for_worker(
						&ctx,
						stream.peer_worker_id(),
						pegboard::envoy_protocol::InternalMessage::Tunnel,
					)
				}
			};
			// A pipelined open can reach the envoy before the actor's start does, and is then
			// refused before admission. It is opened again once the actor runs, on a new
			// stream, so the client never sees the refusal. A refused reopen is retried like
			// the first open, within the same budget and a fresh request deadline; one that is
			// not retried closes the socket with the same retryable close as a refused open.
			// Each reopen credits the budget like a received request before it may draw on it,
			// so reopens under sustained refusals pay for their own retries instead of draining
			// the budget first opens share.
			let reopen = move || async move {
				let deadline = RoutingDeadline::new(&ctx)?;
				budget.deposit();
				let mut retries = budget.retries(deadline.boot_ms);
				let mut target = AcquireTarget::new(
					actor_id,
					group,
					pegboard::legacy_volumes::ResolvedRead::Unread,
					None,
				);
				let acquired = loop {
					let attempt = acquire(
						&ctx,
						&waits,
						&mut target,
						&namespace,
						class,
						false,
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
					// Legacy: the reopen addresses the actor id the first open resolved, which
					// a move destroys, so a moving refusal is not retried here; the client's
					// retry resolves the actor again. Removed in 3.1 with the backfill.
					let moving = crate::retry_budget::refusal(&error).is_some_and(|refusal| {
						refusal.kind == crate::retry_budget::RefusalKind::LegacyActorMoving
					});
					if moving {
						return Err(error);
					}
					retries.retry(ctx.env().as_ref(), error).await?;
				};
				let epoch = acquired.epoch;
				let stream = acquired.stream;
				let message_id = new_message_id(&ctx)?;
				let open = websocket_open(actor_id, group, epoch, &path, &request_headers);
				anyhow::Ok((stream, message_id, open))
			};

			ws.protocols(protocols)
				.read_buffer_size(ACTOR_WEBSOCKET_READ_BUFFER_BYTES)
				.on_upgrade(move |mut socket| async move {
					if let Err(error) = crate::websocket_tunnel::proxy(
						&mut socket,
						stream,
						version_of,
						message_id,
						open,
						reopen,
						clock,
						keepalive_interval_ms,
					)
					.await
					{
						tracing::warn!(%actor_id, %error, "actor WebSocket tunnel stopped");

						// Close with the error so the client sees why instead of a reset. A
						// client that already went away cannot receive it.
						let (code, reason) = tunnel_close_frame(&error, ray_id);
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
				group: group.map(|group| group.to_vec().into()),
			};
			// One tunnel uses one version: the one the envoy's host worker reads.
			let version = pegboard::envoy_protocol::version_for_worker(
				&ctx,
				stream.peer_worker_id(),
				pegboard::envoy_protocol::InternalMessage::Tunnel,
			);
			match crate::http_tunnel::proxy(ctx.env(), version, stream, body, message_id, start)
				.await
			{
				Ok(response) => response,
				Err(error) => GatewayError::from(error).into_response(),
			}
		}
	}
}

/// A request whose actor stream is open, with what the tunnel forwards to the actor.
struct Prepared {
	actor_id: Id,
	/// The request's validated namespace, which tokens are checked against.
	namespace: String,
	/// The group the actor's volumes are in: its namespace's.
	group: Option<rivet_keys::keys::codec::GroupId>,
	epoch: u32,
	stream: universalpubsub::DuplexStream,
	message_id: protocol::MessageId,
	class: universalpubsub::RequestClass,
	path: String,
	request_headers: HashMap<String, String>,
	credential: auth::Credential,
}

/// The namespace a request is routed and authorized in: the one it names; else, for a direct
/// actor id, the namespace the request's token is scoped to; else `default`. Clients before
/// actor ids were read through their namespace's group named no namespace on direct-id URLs,
/// and the gateway found the actor in whichever namespace held it. A token scoped to one
/// namespace authorizes only actors in it, so resolving through that namespace keeps those
/// URLs working without a lookup. The admin token, a persistent ACL token and an
/// unauthenticated request name no one namespace, so their direct-id requests without one
/// resolve in `default`.
///
/// Resolving those requests in `default` is accepted and intended. RivetKit 3.0.x clients send
/// `getForId()` and `create()`-handle traffic this way, so after the upgrade's backfill has moved
/// an actor into its namespace's group, such a request for an actor outside `default` answers
/// `actor.not_found`. A namespace fork keeps its parent's actor ids, so in a fork of `default`
/// such a request reaches `default`'s actor, never the fork's. Current clients always name their
/// namespace in the `x-rivet-namespace` header or the `rvt-namespace` query parameter, and a
/// namespace-scoped JWT resolves in its own namespace, so neither is affected. The gateway adds
/// no lookup from actor id to namespace for these requests. Instead it counts each fallback to
/// `default` and logs a rate-limited warning; see [`NamespaceFallbacks`].
pub(crate) fn request_namespace(
	named: Option<&str>,
	direct: bool,
	credential: &auth::Credential,
) -> String {
	let namespace = named
		.or_else(|| credential_namespace(direct, credential))
		.unwrap_or(pegboard::DEFAULT_NAMESPACE);
	namespace.to_owned()
}

/// The namespace `credential` resolves a request in when the request names none: a
/// namespace-scoped JWT's own, for a direct actor id. The other credentials name no one
/// namespace, and a keyed request names its namespace or resolves in `default`.
fn credential_namespace(direct: bool, credential: &auth::Credential) -> Option<&str> {
	match credential {
		auth::Credential::Jwt(jwt) if direct => Some(jwt.token.namespace.as_str()),
		auth::Credential::Jwt(_) | auth::Credential::Admin | auth::Credential::Unauthenticated => {
			None
		}
	}
}

/// Authenticates the request, resolves its actor, opens a stream to it and builds what the tunnel
/// forwards, or names the V2 actor that serves it during a migration from V2. Nothing has reached
/// the actor when this fails. `resolved_namespace` is set to the request's namespace once it is
/// resolved, so the caller counts the request against that namespace's migration.
#[allow(clippy::too_many_arguments)]
async fn prepare(
	ctx: &EngineContext,
	waits: &Arc<StartWaits>,
	budget: &RetryBudget,
	fallbacks: &NamespaceFallbacks,
	dynamic: Option<&Root>,
	resolved_namespace: &mut Option<String>,
	selector: &str,
	path_rest: String,
	raw_query: Option<String>,
	query: GatewayQuery,
	headers: &HeaderMap,
	websocket: bool,
) -> Result<Routed> {
	let deadline = RoutingDeadline::new(ctx)?;
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
	let named = named_namespace(query.namespace.as_deref(), headers, websocket)?;
	let direct = query.method.is_none();
	let namespace = request_namespace(named.as_deref(), direct, &credential);
	if named.is_none() && credential_namespace(direct, &credential).is_none() {
		let now_boot_ms = ctx.env().clock().boot_ms()?;
		fallbacks.record(now_boot_ms, &credential, direct, query.route);
	}
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
	// Catalog admission can refuse concurrent cold-cache lookups before actor acquisition.
	// Spend the same request retry budget and original deadline on both routing stages.
	budget.deposit();
	let mut retries = budget.retries(deadline.boot_ms);
	let namespace = loop {
		let lookup = rivet_env::timeout_at(
			ctx.env().clock().as_ref(),
			deadline.boot_ms,
			pegboard::catalog::ensure_settled_namespace(ctx, &namespace, deadline.boot_ms),
		)
		.await;
		let result = match lookup {
			Ok(result) => result,
			Err(rivet_env::TimeoutError::Elapsed) => Err(crate::RequestTimeout.build()),
			Err(error) => Err(error.into()),
		};
		match result {
			Ok(standing) => break standing,
			// A catalog refusal of any kind looks the namespace up again.
			Err(error) => {
				retries.retry(ctx.env().as_ref(), error).await?;
			}
		}
	};
	let request_namespace = namespace.name.clone();
	// The migration from V2 is the one into the namespace the request resolved to here, after
	// authentication, never the one its query alone names: a direct-id request with a
	// namespace-scoped token and no named namespace runs in the token's namespace.
	*resolved_namespace = Some(request_namespace.clone());
	let migration = migrate_from_v2::Active::for_namespace(dynamic, &request_namespace);
	let migration = migration.as_ref();

	check_reserved_params(raw_query.as_deref())?;
	let message_id = new_message_id(ctx)?;
	let standing = namespace.clone();
	let mut target = Target::of(
		ctx.config().dc_label(),
		selector,
		&query,
		namespace,
		pool_name,
	)?;
	// Legacy: while the namespace's backfill is not done this reads the actor's state volume,
	// and for an actor not yet moved the unprefixed one too. The acquisition's first read reuses
	// what it read, so a grouped actor costs no extra read and one not yet moved costs one.
	// Removed in 3.1 with the backfill.
	let resolved = target.resolve(ctx, &standing).await?;
	let class = tunnel_class(websocket);

	let force_actor_v3 = match migration {
		Some(_) => migrate_from_v2::forces_actor_v3(headers)?,
		None => false,
	};
	let get_or_create = target.create(force_actor_v3);

	// The gateway owns retries of a refused request. Nothing reaches the actor before the
	// stream opens, so a refused attempt runs again, within one request deadline, from where the
	// last attempt left it: a create that moved the request to the create's own actor is not
	// undone, and a create already sent is not sent again before a read finds the actor gone.
	let mut attempt_target = AcquireTarget::new(
		target.actor_id,
		target.group,
		resolved,
		get_or_create.clone(),
	);
	let acquired = loop {
		let attempt = acquire(
			ctx,
			waits,
			&mut attempt_target,
			&request_namespace,
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

		// While a migration from V2 runs, V2 serves what this cluster does not: an actor it has
		// no record of, a key whose create placed it with V2, and a held actor until its
		// cutover. A held key's getOrCreate is refused instead, since after the cutover V2's
		// getOrCreate would create the key again instead of answering that it is gone.
		let not_here = error
			.downcast_ref::<rivet_error::RivetError>()
			.is_some_and(|error| error.group() == "actor" && error.code() == "not_found");
		let delegated = error.downcast_ref::<DelegatedToV2>().is_some();
		let held = error.downcast_ref::<HeldByMigrateFromV2>().is_some();
		if migration.is_some() && (not_here || delegated || (held && get_or_create.is_none())) {
			return Ok(Routed::V2(V2Target {
				v3_id: target.actor_id,
				group: target.group,
			}));
		}
		if held {
			return Err(ActorMigratingFromV2::error());
		}
		let refused = retries.retry(ctx.env().as_ref(), error).await?;
		match refused.kind {
			crate::retry_budget::RefusalKind::Overloaded
			| crate::retry_budget::RefusalKind::ReadInterrupted
			| crate::retry_budget::RefusalKind::RouteUnresolved => {}
			// Legacy: the actor was being moved into its namespace's group, so it is resolved
			// again, which finds it at its new id once the move made it. A re-resolve refused at
			// admission is tried once more after its hint, since a move's own writes can briefly
			// shed the read; any other failure goes to the client. Both stay within the request
			// deadline, past which the client gets the moving refusal. Removed in 3.1 with the
			// backfill.
			crate::retry_budget::RefusalKind::LegacyActorMoving => {
				let resolved =
					legacy_resolve_moved(ctx, &mut target, &standing, deadline, refused).await?;
				attempt_target = AcquireTarget::new(
					target.actor_id,
					target.group,
					resolved,
					get_or_create.clone(),
				);
			}
		}
	};

	// Reserved routing parameters must not become actor application parameters. The rest of
	// the query is the actor's to interpret, so it is forwarded as the client sent it.
	let path = match application_query(raw_query.as_deref()) {
		Some(application_query) => format!("{path_rest}?{application_query}"),
		None => path_rest,
	};

	Ok(Routed::V3(Prepared {
		actor_id: acquired.actor_id,
		namespace: request_namespace,
		group: acquired.group,
		epoch: acquired.epoch,
		stream: acquired.stream,
		message_id,
		class,
		path,
		request_headers: forwarded_headers(ctx, headers),
		credential,
	}))
}

/// Legacy: resolves `target` again after its actor answered the `moving` refusal, bounded by the
/// request's `deadline`; see [`legacy_reresolve_within`]. Removed in 3.1 with the backfill.
async fn legacy_resolve_moved(
	ctx: &EngineContext,
	target: &mut Target,
	standing: &pegboard::NamespaceStanding,
	deadline: RoutingDeadline,
	moving: crate::retry_budget::Refusal,
) -> Result<pegboard::legacy_volumes::ResolvedRead> {
	let clock = ctx.env().clock();
	// The resolve borrows everything it reads through the state, so each try's future lives
	// only as long as the try.
	let mut state = (target, ctx, standing);
	legacy_reresolve_within(
		clock.as_ref(),
		deadline.boot_ms,
		moving,
		&mut state,
		|(target, ctx, standing)| Box::pin(target.resolve(ctx, standing)),
	)
	.await
}

/// Legacy: runs `resolve` on `state` by `deadline_boot_ms`, after the actor answered the
/// `moving` refusal. A resolve refused at admission is tried once more after the refusal's hint
/// when the hint ends before the deadline; otherwise that refusal is the answer. Any other
/// failure is the request's error. A deadline that passes answers the `moving` refusal with its
/// hint, so the client gets the retryable 503 or WebSocket 1013 it gets for any refusal, never
/// an internal error. The legacy behavior answers wakes during a backfill move; the new behavior
/// has no move. Taken only for an actor of a namespace whose backfill is not done. Removed in
/// 3.1 with the backfill.
async fn legacy_reresolve_within<S, T, F>(
	clock: &dyn rivet_env::Clock,
	deadline_boot_ms: u64,
	moving: crate::retry_budget::Refusal,
	state: &mut S,
	mut resolve: F,
) -> Result<T>
where
	S: Send,
	F: for<'a> FnMut(
			&'a mut S,
		)
			-> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send + 'a>>
		+ Send,
{
	let resolve = async {
		let error = match resolve(state).await {
			Ok(resolved) => return Ok(resolved),
			Err(error) => error,
		};
		let Some(refused) = crate::retry_budget::refusal(&error) else {
			return Err(error);
		};
		let now_boot_ms = clock.observe()?.boot_ms;
		if now_boot_ms.saturating_add(refused.retry_after_ms) >= deadline_boot_ms {
			return Err(error);
		}
		tracing::debug!(
			?error,
			retry_after_ms = refused.retry_after_ms,
			"re-resolving a moving actor was refused; it is tried once more after the hint"
		);
		rivet_env::sleep(
			clock,
			std::time::Duration::from_millis(refused.retry_after_ms),
		)
		.await?;
		resolve(state).await
	};
	match rivet_env::timeout_at(clock, deadline_boot_ms, resolve).await {
		Ok(resolved) => resolved,
		Err(_elapsed) => {
			tracing::debug!(
				retry_after_ms = moving.retry_after_ms,
				"re-resolving a moving actor ran past the request deadline; the moving refusal goes to the client"
			);
			Err(pegboard::validation::LegacyActorMoving {
				retry_after_ms: moving.retry_after_ms,
			}
			.build())
		}
	}
}

/// Routes header and subprotocol actor selectors through the gateway's common edge.
#[tracing::instrument(skip_all)]
pub async fn direct(
	State(ctx): State<EngineContext>,
	axum::Extension(waits): axum::Extension<Arc<StartWaits>>,
	axum::Extension(budget): axum::Extension<Arc<RetryBudget>>,
	axum::Extension(fallbacks): axum::Extension<Arc<NamespaceFallbacks>>,
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
	let query = match direct_query(&headers, ws.is_ok()) {
		Ok(query) => query,
		Err(error) => return reject(ws, error, ray_id),
	};
	gateway(
		State(ctx),
		axum::Extension(waits),
		axum::Extension(budget),
		axum::Extension(fallbacks),
		request,
		OriginalUri(uri.clone()),
		Path(actor_id.to_string()),
		RawRestPath(uri.path().to_owned()),
		RawQuery(uri.query().map(ToOwned::to_owned)),
		Ok(Query(query)),
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

/// The gateway query of a request that [`targets_actor`] names. Its namespace comes from
/// [`header_namespace`], and like `rvt-namespace` it wins over the token's namespace; see
/// [`request_namespace`].
fn direct_query(headers: &HeaderMap, websocket: bool) -> Result<GatewayQuery> {
	let namespace = header_namespace(headers, websocket)?;

	Ok(GatewayQuery {
		namespace: namespace.map(ToOwned::to_owned),
		route: RouteKind::ActorHeaders,
		..GatewayQuery::default()
	})
}

/// The namespace a request names in its transport: the `x-rivet-namespace` header, or for a
/// WebSocket the `rivet_namespace.` subprotocol, since browsers cannot set headers on one. An
/// empty value names no namespace.
fn header_namespace(headers: &HeaderMap, websocket: bool) -> Result<Option<&str>> {
	let namespace = if websocket {
		protocol_value(headers, WS_PROTOCOL_NAMESPACE)
	} else {
		headers
			.get(X_RIVET_NAMESPACE)
			.map(|value| value.to_str())
			.transpose()
			.context("invalid x-rivet-namespace header")?
	};
	Ok(namespace.filter(|namespace| !namespace.is_empty()))
}

/// The namespace a gateway request names, which [`request_namespace`] resolves against the
/// token: `queried`, the `rvt-namespace` parameter, or its transport's [`header_namespace`].
/// Every route reads both, so a request that names its namespace in a header never resolves in
/// another one. A request naming two different namespaces is refused as invalid.
pub(crate) fn named_namespace(
	queried: Option<&str>,
	headers: &HeaderMap,
	websocket: bool,
) -> Result<Option<String>> {
	let header = header_namespace(headers, websocket)?;
	match (queried, header) {
		(Some(queried), Some(header)) if queried != header => Err(QueryInvalidParams {
			detail: format!(
				"the request names namespace {queried:?} in rvt-namespace and {header:?} in its transport"
			),
		}
		.build()),
		(Some(named), Some(_)) | (Some(named), None) | (None, Some(named)) => {
			Ok(Some(named.to_owned()))
		}
		(None, None) => Ok(None),
	}
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

/// Where a request's route led.
enum Routed {
	/// The actor runs here, with its stream open.
	V3(Prepared),
	/// V2 serves the actor.
	V2(V2Target),
}

/// An actor a request reaches in V2.
struct V2Target {
	/// The id this cluster would hold it under, read again when V2 does not have it.
	v3_id: Id,
	/// The group that id's volumes are keyed in.
	group: Option<rivet_keys::keys::codec::GroupId>,
}

/// The parts of a request proxied to V2 unchanged.
struct V2Request {
	original_uri: hyper::Uri,
	method: Method,
	headers: HeaderMap,
	keyed: bool,
}

/// The route label of a request that failed before reaching an actor, when it is a migration
/// outcome.
fn refused_route(error: &anyhow::Error) -> Option<&'static str> {
	let structured = error.downcast_ref::<rivet_error::RivetError>()?;
	match (structured.group(), structured.code()) {
		("actor", "migrating_from_v2") => Some("retry_later"),
		("actor", "not_found") => Some("not_found"),
		_ => None,
	}
}

/// Proxies a request to V2's gateway, path, query, headers and body unchanged except for the
/// namespace. When V2 does not hold the actor, this cluster's state is read again: an actor
/// still held, or one activated meanwhile, is refused as the retryable
/// `actor.migrating_from_v2`; anything else is `actor.not_found`.
async fn proxy_to_v2(
	ctx: &EngineContext,
	migration: &migrate_from_v2::Active<'_>,
	v2: V2Target,
	request: V2Request,
	ws: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
	body: Body,
	ray_id: Option<Id>,
) -> Result<Response> {
	let uri = proxy::gateway_uri(migration.config, &request.original_uri, request.keyed)?;
	let namespace = migration.namespace.clone();

	let ws = match ws {
		Ok(ws) => ws,
		Err(_) => {
			let outcome =
				proxy::http(ctx, &namespace, uri, request.method, &request.headers, body).await?;
			let response = match outcome {
				proxy::HttpOutcome::Response(response) => response,
				proxy::HttpOutcome::NotFound => {
					let state = migrate_from_v2::read_v3_actor(ctx, v2.group, v2.v3_id).await?;
					return Err(after_v2_not_found(migration, state));
				}
			};
			migration.count_request("v2_fallback");
			return Ok(response);
		}
	};

	let upstream = match proxy::websocket_connect(ctx, &namespace, uri, &request.headers).await {
		Ok(upstream) => upstream,
		Err(error) => {
			tracing::warn!(
				?error,
				"V2 gateway refused the WebSocket; check migrate_from_v2.endpoint"
			);
			migration.count_request("v2_error");
			return Ok(reject(Ok(ws), error, ray_id));
		}
	};
	migration.count_request("v2_fallback");
	let ws = match proxy::accepted_protocol(&upstream) {
		Some(protocol) => ws.protocols([protocol]),
		None => ws,
	};
	let clock = ctx.env().clock();
	let keepalive_interval_ms = ctx.config().guard().websocket_keepalive_interval_ms();
	let ctx = ctx.clone();
	let decide_namespace = namespace.clone();
	Ok(ws
		.read_buffer_size(ACTOR_WEBSOCKET_READ_BUFFER_BYTES)
		.on_upgrade(move |socket| async move {
			let decide = move || async move {
				let state = match migrate_from_v2::read_v3_actor(&ctx, v2.group, v2.v3_id).await {
					Ok(state) => state,
					Err(error) => {
						tracing::debug!(?error, "re-read after a V2 not_found failed");
						return None;
					}
				};
				let route = decision::after_v2_not_found(state);
				let error = match route {
					decision::Route::RetryLater => ActorMigratingFromV2::error(),
					decision::Route::NotFound | decision::Route::V3 | decision::Route::V2 => {
						ActorNotFound.build()
					}
				};
				if let Some(label) = refused_route(&error) {
					migrate_from_v2::metrics::REQUESTS
						.with_label_values(&[&decide_namespace, label])
						.inc();
				}
				let (code, reason) = errors::close_frame(&error, ray_id);
				Some(CloseFrame {
					code,
					reason: reason.into(),
				})
			};
			if let Err(error) =
				proxy::pump(socket, upstream, decide, clock, keepalive_interval_ms).await
			{
				tracing::debug!(?error, "V2 WebSocket proxy stopped");
			}
		}))
}

/// The error for a request V2 does not hold, from this cluster's fresh state, counted by its
/// route.
fn after_v2_not_found(migration: &migrate_from_v2::Active<'_>, state: V3Actor) -> anyhow::Error {
	match decision::after_v2_not_found(state) {
		decision::Route::RetryLater => {
			migration.count_request("retry_later");
			ActorMigratingFromV2::error()
		}
		decision::Route::NotFound | decision::Route::V3 | decision::Route::V2 => {
			migration.count_request("not_found");
			ActorNotFound.build()
		}
	}
}

/// The actor a request goes to, the group its volumes are in, and what a getOrCreate creates.
struct Target {
	actor_id: Id,
	group: Option<rivet_keys::keys::codec::GroupId>,
	keyed: Option<KeyedTarget>,
}

struct KeyedTarget {
	method: GatewayMethod,
	/// The actor a create of the key makes, in the namespace's own group; see
	/// [`pegboard::actor_key::keyed_create_actor`].
	created: pegboard::actor_key::KeyedActor,
	/// The request's namespace as last read, which a create acts on.
	standing: Arc<pegboard::NamespaceStanding>,
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
		namespace: Arc<pegboard::NamespaceStanding>,
		pool_name: String,
	) -> Result<Self> {
		let group = namespace.group_id();
		let Some(method) = query.method else {
			let actor_id = match selector.split_once('@') {
				Some((actor_id, _token)) => Id::parse(actor_id)?,
				None => Id::parse(selector).context(
					"invalid actor selector (parsing as ID). include rvt-method to use name-key selection instead",
				)?,
			};
			return Ok(Self {
				actor_id,
				group,
				keyed: None,
			});
		};

		// The key is hashed and stored in its canonical form, which the actor API translates
		// its slash form to, so `a,b` here and `a/b` through the API name one actor. Clients
		// omit `rvt-key` for the empty key.
		let key = actor_key::encode(&actor_key::from_gateway_query(query.key.as_deref()));
		pegboard::validation::actor_name(selector)?;
		pegboard::validation::actor_key(Some(&key))?;
		let created = pegboard::actor_key::keyed_create_actor(dc_label, &namespace, selector, &key);
		Ok(Self {
			actor_id: created.actor_id,
			group: created.group,
			keyed: Some(KeyedTarget {
				method,
				created,
				standing: namespace,
				pool_name,
				name: selector.to_owned(),
				key,
				input: query.input.clone(),
			}),
		})
	}

	/// Finds the group the target's volumes are in, and for a keyed target the actor its key
	/// names; see [`pegboard::actor_key::resolve_keyed_actor`]. Returns what the lookup learned
	/// about the actor's state volume, for the acquisition's first read. Legacy: an actor from
	/// before groups is routed through its unprefixed volumes, at its legacy id when keyed, until
	/// the namespace's backfill is done. Removed in 3.1 with the backfill.
	async fn resolve(
		&mut self,
		ctx: &EngineContext,
		namespace: &pegboard::NamespaceStanding,
	) -> Result<ResolvedRead> {
		match &self.keyed {
			Some(keyed) => {
				let found = pegboard::actor_key::resolve_keyed_actor(
					ctx,
					ctx.config().dc_label(),
					namespace,
					&keyed.name,
					&keyed.key,
				)
				.await?;
				self.actor_id = found.actor.actor_id;
				self.group = found.actor.group;
				Ok(found.read)
			}
			None => {
				let found =
					pegboard::legacy_volumes::legacy_resolve_actor(ctx, namespace, self.actor_id)
						.await?;
				self.group = found.group;
				Ok(found.read)
			}
		}
	}

	/// What a getOrCreate creates: the key's actor in the namespace's own group, even when the
	/// key resolved to a not yet moved actor at the legacy unprefixed keys, which a create never
	/// reuses. A get creates nothing and fails when the actor is missing. `force_actor_v3` asks
	/// for the actor here while its namespace migrates in from V2.
	fn create(&self, force_actor_v3: bool) -> Option<CreateActor> {
		let keyed = self.keyed.as_ref()?;
		match keyed.method {
			GatewayMethod::Get => None,
			GatewayMethod::GetOrCreate => Some(CreateActor {
				actor: keyed.created,
				standing: keyed.standing.clone(),
				pool_name: keyed.pool_name.clone(),
				name: keyed.name.clone(),
				key: keyed.key.clone(),
				input: keyed.input.clone(),
				force_actor_v3,
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

/// The open message of a WebSocket to `epoch` of the actor in `group`. The envoy protocol calls
/// the epoch the actor generation.
fn websocket_open(
	actor_id: Id,
	group: Option<rivet_keys::keys::codec::GroupId>,
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
		group: group.map(|group| group.to_vec().into()),
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

/// The close code and reason for a tunnel that stopped with `error`. A refusal nothing reached
/// the actor for, such as a reopen refused because storage shed it or the backfill is moving the
/// actor, closes with the same retryable `guard.service_unavailable` and hint as a refused open,
/// so the client never sees the internal `actor.moving` or `depot.overloaded`; anything else
/// closes with the error's own code.
fn tunnel_close_frame(error: &anyhow::Error, ray_id: Option<Id>) -> (u16, String) {
	match crate::retry_budget::refusal(error) {
		Some(refusal) => WebSocketRejection::Retry {
			retry_after_ms: refusal.retry_after_ms,
		}
		.close_frame(),
		None => errors::close_frame(error, ray_id),
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
