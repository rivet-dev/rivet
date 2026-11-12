use anyhow::{Context, Result};
use axum::{
	Extension,
	extract::{Query, State},
	http::HeaderMap,
	response::Response,
};
use rivet_engine_context::EngineContext;
use rivet_env::capabilities::websocket::{CloseFrame, Message, WebSocketUpgrade};
use rivet_env::tokio::task::JoinError;
use rivet_guard_core::auth;
use serde::Deserialize;
use std::sync::Arc;

use crate::connection;

/// WebSocket close code for a policy violation, used for rejected connects.
const POLICY_VIOLATION: u16 = 1008;

/// WebSocket close code for "try again later", used for connects shed at admission.
pub(crate) const TRY_AGAIN_LATER: u16 = 1013;

/// WebSocket close code for a server error, used when a connect cannot be admitted at all.
pub(crate) const INTERNAL_ERROR: u16 = 1011;

#[derive(Deserialize)]
pub struct EnvoyQuery {
	protocol_version: u16,
	#[serde(rename = "envoy_key")]
	key: String,
	/// The deployment version the envoy runs. Allocation prefers the newest version live in the
	/// pool, so a rollout moves new actors to the new deployment as its envoys connect.
	version: u32,
	namespace: Option<String>,
	pool_name: Option<String>,
}

/// The namespace and runner pool an Envoy connection serves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EnvoyScope {
	pub namespace: String,
	pub pool_name: String,
	/// The volume group of the namespace, resolved at connect: every actor volume this
	/// connection opens is in it.
	pub group: Option<rivet_keys::keys::codec::GroupId>,
	/// Legacy: which actors' volumes may still be unprefixed: every actor while the namespace's
	/// backfill is not done, the actors it skipped once done; see `pegboard::legacy_volumes`.
	/// Read from the process's cached standing while it was not done at connect, so a connection
	/// stops falling back once the backfill finishes. Removed in 3.1 with the backfill.
	pub legacy_fallback: LegacyFallbackView,
}

/// Legacy: an envoy connection's view of which actors' volumes may still be unprefixed. The
/// legacy behavior fixed the fallback at connect for the connection's whole life, so a connection
/// opened while the namespace's backfill ran paid the fallback's extra read on every open and
/// delivery for as long as it stayed open. The new behavior starts from the fallback at connect
/// and, while that falls back for every actor, reads the process's cached standing (no command)
/// at each use; the first standing of the connection's group whose backfill is done settles the
/// fallback for good, since a finished backfill never runs again and its skipped list only
/// shrinks. Taken by every open, delivery and resolve of a connection whose fallback at connect
/// was `Every`. Removed in 3.1 with the backfill.
/// Shared by every clone of the connection's scope, so a clone costs one reference count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LegacyFallbackView(Arc<LegacyFallbackViewInner>);

#[derive(Debug, PartialEq, Eq)]
struct LegacyFallbackViewInner {
	at_connect: pegboard::catalog::LegacyFallback,
	/// The fallback once a cached standing showed the backfill done.
	settled: std::sync::OnceLock<pegboard::catalog::LegacyFallback>,
}

impl LegacyFallbackView {
	pub(crate) fn new(at_connect: pegboard::catalog::LegacyFallback) -> Self {
		Self(Arc::new(LegacyFallbackViewInner {
			at_connect,
			settled: std::sync::OnceLock::new(),
		}))
	}

	/// The fallback now, for a connection in `group`. `cached_standing` reads the namespace's
	/// standing from the process cache and runs only while the fallback is not settled and fell
	/// back for every actor at connect. A standing of another group, as a cache from before the
	/// namespace's move holds, or one whose backfill still runs, leaves the fallback at connect.
	pub(crate) async fn current<F, Fut>(
		&self,
		group: Option<rivet_keys::keys::codec::GroupId>,
		cached_standing: F,
	) -> &pegboard::catalog::LegacyFallback
	where
		F: FnOnce() -> Fut,
		Fut: std::future::Future<Output = Result<Option<Arc<pegboard::NamespaceStanding>>>>,
	{
		let view = &*self.0;
		if let Some(settled) = view.settled.get() {
			return settled;
		}
		if !view.at_connect.every() {
			return &view.at_connect;
		}
		let standing = match cached_standing().await {
			Ok(Some(standing)) => standing,
			Ok(None) => return &view.at_connect,
			Err(error) => {
				tracing::debug!(
					?error,
					"could not read the cached namespace standing; keeping the connection's fallback"
				);
				return &view.at_connect;
			}
		};
		if standing.group_id() != group || !standing.backfill_done() {
			return &view.at_connect;
		}
		view.settled
			.get_or_init(|| standing.legacy_fallback().clone())
	}
}

impl EnvoyScope {
	/// Legacy: which actors' volumes may still be unprefixed now; see [`LegacyFallbackView`].
	/// Removed in 3.1 with the backfill.
	pub(crate) async fn legacy_fallback_now(
		&self,
		ctx: &EngineContext,
	) -> &pegboard::catalog::LegacyFallback {
		self.legacy_fallback
			.current(self.group, || {
				pegboard::catalog::cached_namespace_standing(ctx, &self.namespace)
			})
			.await
	}

	/// The group the volumes of `actor_id` are keyed in on this connection: the namespace's,
	/// or `None` for an actor from before groups; see `pegboard::legacy_volumes`.
	pub(crate) async fn actor_group(
		&self,
		ctx: &EngineContext,
		actor_id: rivet_util_id::Id,
	) -> anyhow::Result<Option<rivet_keys::keys::codec::GroupId>> {
		let fallback = self.legacy_fallback_now(ctx).await;
		pegboard::legacy_volumes::legacy_resolve_actor_volume_in(
			&ctx.depot(),
			self.group,
			fallback.namespace_for(actor_id),
			actor_id,
		)
		.await
	}

	/// The group the volumes of `actor_id` are keyed in on this connection, with its state
	/// volume read there; the read the legacy fallback made is reused, so a resolve costs no
	/// extra read for a grouped actor and one for an actor not yet moved. `None` when the state
	/// volume is missing or deleted, or when it is unprefixed and its record is another
	/// namespace's (see [`Self::legacy_in_scope`]).
	pub(crate) async fn read_actor(
		&self,
		ctx: &EngineContext,
		actor_id: rivet_util_id::Id,
	) -> anyhow::Result<(
		Option<rivet_keys::keys::codec::GroupId>,
		Option<pegboard::wire::ActorReadResponse>,
	)> {
		let depot = ctx.depot();
		let fallback = self.legacy_fallback_now(ctx).await;
		let found = pegboard::legacy_volumes::legacy_resolve_actor_in(
			&depot,
			self.group,
			fallback.namespace_for(actor_id),
			actor_id,
		)
		.await?;
		let read = found.read.into_read(&depot, found.group, actor_id).await?;
		Ok((found.group, self.legacy_in_scope(found.group, read)))
	}

	/// Legacy: `read` of an actor's state volume in `group`, or `None` when the volume is
	/// unprefixed and the record's legacy namespace is not this connection's. An unprefixed key
	/// carries no namespace, so an untrusted envoy could otherwise name another namespace's legacy
	/// actor id and have its record acted on. The epoch and phase checks on the record already keep such
	/// an envoy from binding or starting that actor, so this closes the cross-namespace read
	/// rather than a write. Grouped keys carry their namespace, so this check only matters while
	/// unprefixed volumes exist. Removed in 3.1 with the backfill.
	fn legacy_in_scope(
		&self,
		group: Option<rivet_keys::keys::codec::GroupId>,
		read: Option<pegboard::wire::ActorReadResponse>,
	) -> Option<pegboard::wire::ActorReadResponse> {
		let read = read?;
		let in_namespace = match &read.data {
			Some(data) => pegboard::catalog::actor_in_namespace(group, &self.namespace, data),
			None => group.is_some(),
		};
		match in_namespace {
			true => Some(read),
			false => None,
		}
	}
}

/// Validates the connect query's namespace, defaulting to the default namespace, and its pool.
/// The pool is required: an Envoy always names the pool it serves.
pub(crate) fn scope(namespace: Option<&str>, pool_name: Option<&str>) -> Result<EnvoyScope> {
	let namespace = namespace.unwrap_or(pegboard::DEFAULT_NAMESPACE);
	pegboard::validation::namespace_name(namespace)?;
	let pool_name = pool_name.context("missing pool_name")?;
	pegboard::validation::pool_name(pool_name)?;
	Ok(EnvoyScope {
		namespace: namespace.to_owned(),
		pool_name: pool_name.to_owned(),
		group: None,
		legacy_fallback: LegacyFallbackView::new(pegboard::catalog::LegacyFallback::Off),
	})
}

/// Legacy: the connection's scope for the namespace's cached `standing`. A standing without a
/// group is read again uncached through `reread` first (see
/// `pegboard::catalog::legacy_fresh_ungrouped_standing`), so a connect served by a process whose
/// cache predates the namespace's move into its group is never fixed to a group-less scope for
/// its whole life, where a grouped actor's replayed events would find no actor. The legacy
/// behavior fixed the scope from the cached standing; the new behavior fixes it from the re-read
/// whenever the cached one has no group. Removed in 3.1 with the backfill.
pub(crate) async fn legacy_connect_scope<F, Fut>(
	scope: EnvoyScope,
	standing: &Arc<pegboard::NamespaceStanding>,
	volumes_open: Result<bool>,
	reread: F,
) -> Result<EnvoyScope>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = Result<Option<pegboard::namespace_wire::NamespaceState>>>,
{
	let standing =
		pegboard::catalog::legacy_fresh_ungrouped_standing(standing, volumes_open, reread).await?;
	Ok(EnvoyScope {
		group: standing.group_id(),
		legacy_fallback: LegacyFallbackView::new(standing.legacy_fallback().clone()),
		..scope
	})
}

/// The close code for a connect refused while reading its namespace: a policy violation when the
/// namespace does not exist or its phase refuses connects, and a server error when the namespace
/// could not be read.
fn namespace_rejection_code(error: &anyhow::Error) -> u16 {
	let error = rivet_error::RivetError::extract(error);
	match (error.group(), error.code()) {
		("namespace", "not_found" | "pending" | "creating" | "archived" | "failed")
		| ("envoy", "invalid_namespace_name") => POLICY_VIOLATION,
		_ => INTERNAL_ERROR,
	}
}

/// The close reason for a rejected connect: the error's `<group>.<code>`.
pub(crate) fn close_reason(error: &anyhow::Error) -> String {
	let error = rivet_error::RivetError::extract(error);
	format!("{}.{}", error.group(), error.code())
}

/// The close reason for a connect shed at admission: `envoy.connect_overloaded` with the retry
/// hint as a `;retry_after_ms=<ms>` parameter, which the Envoy client honors.
pub(crate) fn overloaded_close_reason(retry_after_ms: u64) -> String {
	let error = crate::rejection::ConnectOverloaded { retry_after_ms }.build();
	retry_close_reason(&error, retry_after_ms)
}

/// The close reason for a connect the envoy should retry: the error's `<group>.<code>` with the
/// retry hint as a `;retry_after_ms=<ms>` parameter.
pub(crate) fn retry_close_reason(error: &anyhow::Error, retry_after_ms: u64) -> String {
	format!("{};retry_after_ms={retry_after_ms}", close_reason(error))
}

/// Upgrades `ws` only to close it at once with `code` and `reason`, so the client can read why.
fn reject(ws: WebSocketUpgrade, protocols: Vec<String>, code: u16, reason: String) -> Response {
	ws.protocols(protocols)
		.on_upgrade(move |mut socket| async move {
			let close = Message::Close(Some(CloseFrame {
				code,
				reason: reason.into(),
			}));
			if let Err(error) = socket.send(close).await {
				tracing::debug!(?error, "failed to send envoy connect rejection");
			}
		})
}

pub async fn connect(
	State(ctx): State<EngineContext>,
	Extension(service): Extension<crate::Service>,
	Query(query): Query<EnvoyQuery>,
	headers: HeaderMap,
	ws: WebSocketUpgrade,
) -> Result<Response, crate::EnvoyError> {
	if !(rivet_envoy_protocol::versioned::MIN_PROTOCOL_VERSION
		..=rivet_envoy_protocol::PROTOCOL_VERSION)
		.contains(&query.protocol_version)
	{
		return Err(anyhow::anyhow!("unsupported envoy protocol").into());
	}

	let protocols = ws
		.requested_protocols()
		.filter_map(|p| p.to_str().ok())
		// A token protocol is never selected, so the upgrade reply does not echo the token.
		.filter(|p| !auth::is_token_protocol(p))
		.map(ToOwned::to_owned)
		.collect::<Vec<String>>();

	// A rejected connect is still upgraded, so the client can read the close reason.
	let scope = match scope(query.namespace.as_deref(), query.pool_name.as_deref()) {
		Ok(scope) => scope,
		Err(error) => {
			let reason = close_reason(&error);
			tracing::debug!(?error, %reason, "rejecting envoy connect");
			return Ok(reject(ws, protocols, POLICY_VIOLATION, reason));
		}
	};

	// Only an authenticated connect may spend the connect budget.
	if let Err(error) = auth::check_request(
		&ctx,
		auth::transport_token(&headers),
		&scope.namespace,
		auth::ResourceKind::Runner,
		None,
		auth::OperationKind::Create,
	)
	.await
	{
		let reason = auth::close_reason(&error);
		tracing::debug!(%reason, "rejecting unauthenticated envoy connect");
		return Ok(reject(
			ws,
			protocols,
			auth::POLICY_VIOLATION_CLOSE_CODE,
			reason,
		));
	}

	// Refuse a connect during shutdown before it pays for any namespace read.
	let Some(process_owner) = ctx.ingress().enter() else {
		return Ok(reject(
			ws,
			protocols,
			TRY_AGAIN_LATER,
			"engine.shutdown".into(),
		));
	};

	// The namespace's group and status, read after the token is checked so an unauthenticated
	// connect learns nothing about the namespace. A namespace that refuses new work closes
	// with its status as the reason.
	let standing = match pegboard::catalog::ensure_namespace(&ctx, &scope.namespace).await {
		Ok(standing) => standing,
		Err(error) => {
			// A retryable refusal such as `namespace.creating` carries its hint to the envoy.
			let retry_after_ms = rivet_error::RivetError::extract(&error)
				.metadata()
				.and_then(|metadata| metadata.get("retry_after_ms")?.as_u64());
			let reason = match retry_after_ms {
				Some(retry_after_ms) => retry_close_reason(&error, retry_after_ms),
				None => close_reason(&error),
			};
			tracing::debug!(?error, %reason, "rejecting envoy connect");
			let code = namespace_rejection_code(&error);
			return Ok(reject(ws, protocols, code, reason));
		}
	};

	// Shed before any connection state is built. The scope and auth checks above do no I/O, so
	// a malformed or unauthenticated connect never spends the budget.
	let now_ms = match ctx.env().clock().observe() {
		Ok(clocks) => clocks.boot_ms,
		Err(error) => {
			tracing::warn!(
				?error,
				"clock unavailable while admitting an envoy connect; the process is likely shutting down"
			);
			let reason = close_reason(&anyhow::Error::from(error));
			return Ok(reject(ws, protocols, INTERNAL_ERROR, reason));
		}
	};
	if let Err(retry_after_ms) = service.connect_admission.try_admit(now_ms) {
		let reason = overloaded_close_reason(retry_after_ms);
		return Ok(reject(ws, protocols, TRY_AGAIN_LATER, reason));
	}

	// Legacy: a cached standing without a group is read again uncached, after admission since it
	// is the one read a connect pays beyond the cache. Removed in 3.1 with the backfill.
	let namespace = scope.namespace.clone();
	let scope = match legacy_connect_scope(
		scope,
		&standing,
		pegboard::catalog::namespace_volumes_open(&ctx),
		|| pegboard::catalog::lookup_namespace(&ctx, &namespace),
	)
	.await
	{
		Ok(scope) => scope,
		Err(error) => {
			// A retryable refusal such as `namespace.creating` carries its hint to the envoy.
			let retry_after_ms = rivet_error::RivetError::extract(&error)
				.metadata()
				.and_then(|metadata| metadata.get("retry_after_ms")?.as_u64());
			let reason = match retry_after_ms {
				Some(retry_after_ms) => retry_close_reason(&error, retry_after_ms),
				None => close_reason(&error),
			};
			tracing::debug!(?error, %reason, "rejecting envoy connect");
			let code = namespace_rejection_code(&error);
			return Ok(reject(ws, protocols, code, reason));
		}
	};

	// Reserve ownership before on_upgrade can detach from the HTTP response.
	let Some(connection_owner) = service.connections.enter() else {
		return Ok(reject(
			ws,
			protocols,
			TRY_AGAIN_LATER,
			"engine.shutdown".into(),
		));
	};
	// Disabled captures leave identity allocation at its original point inside run.
	// Enabled captures publish that same identity early in the successful upgrade.
	let connection_id = trace_connection_id(|| ctx.env().rng("envoy.connection_id").uuid_v4());
	let trace_connection = connection_id.map(|id| {
		axum::http::HeaderValue::from_str(&id.to_string()).expect("UUID is a valid header")
	});
	let mut response = ws
		.protocols(protocols)
		.max_message_size(32 * 1024 * 1024)
		.on_upgrade(move |socket| async move {
			let _connection_owner = connection_owner;
			let _process_owner = process_owner;
			if let Err(err) = connection::run(
				&ctx,
				socket,
				query.key,
				scope,
				query.protocol_version,
				query.version,
				service,
				connection_id,
			)
			.await
			{
				if ended_by_shutdown_or_reconnect(&err) {
					tracing::info!(%err, "envoy connection closed");
				} else {
					tracing::warn!(%err, "Envoy connection closed");
				}
			}
		});
	if let Some(connection) = trace_connection {
		response
			.headers_mut()
			.insert("x-rivet-sqlite-trace-connection", connection);
	}
	Ok(response)
}

/// No diagnostic identity is allocated or exposed when the target is disabled.
fn trace_connection_id(allocate: impl FnOnce() -> uuid::Uuid) -> Option<uuid::Uuid> {
	tracing::enabled!(target: "sqlite_prefetch::trace", tracing::Level::TRACE).then(allocate)
}

/// Whether a connection ended for an expected reason: this process cancelled its tasks while
/// shutting down, a newer connection superseded it, its pool admission must be obtained again
/// through a fresh connection, or its namespace moved into its group after connect.
fn ended_by_shutdown_or_reconnect(error: &anyhow::Error) -> bool {
	let cancelled = error
		.downcast_ref::<JoinError>()
		.is_some_and(JoinError::is_cancelled);

	cancelled
		|| error.is::<crate::envoy_record::Superseded>()
		|| error.is::<crate::envoy_record::PoolAdmissionLost>()
		|| error.is::<crate::lifecycle::LegacyNamespaceMoved>()
}
