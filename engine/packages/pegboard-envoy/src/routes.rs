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
	namespace: Option<String>,
	pool_name: Option<String>,
}

/// The namespace and runner pool an Envoy connection serves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EnvoyScope {
	pub namespace: String,
	pub pool_name: String,
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
	})
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

	let Some(process_owner) = ctx.ingress().enter() else {
		return Ok(reject(
			ws,
			protocols,
			TRY_AGAIN_LATER,
			"engine.shutdown".into(),
		));
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
				service,
				connection_id,
			)
			.await
			{
				if ended_by_shutdown_or_supersede(&err) {
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
/// shutting down, or a newer connection of the same envoy superseded it. The envoy reconnects
/// in both cases, so neither is a fault.
fn ended_by_shutdown_or_supersede(error: &anyhow::Error) -> bool {
	let cancelled = error
		.downcast_ref::<JoinError>()
		.is_some_and(JoinError::is_cancelled);

	cancelled || error.is::<crate::envoy_record::Superseded>()
}


