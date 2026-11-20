use axum::{
	Json, Router,
	response::{IntoResponse, Response},
};
use hyper::StatusCode;
use rivet_engine_context::EngineContext;

mod admission;
mod connection;
mod drain;
mod envoy_record;
pub mod keys;
mod kv;
mod lifecycle;
mod metrics;
mod missing_pools;
mod pre_connect;
mod queue;
mod rejection;
mod routes;
mod sqlite;

/// Process-wide state shared by every Envoy connection. Building it starts nothing.
#[derive(Clone)]
pub struct Service {
	pub(crate) connections: rivet_engine_context::ingress::IngressDrain,
	pub(crate) stopping: tokio::sync::watch::Sender<bool>,
	/// Remote SQL from every connection runs on one worker pool.
	pub(crate) sqlite_pool: sqlite_embedded::worker::SqlitePool,
	/// The connect budget every Envoy connect to this process is admitted against.
	pub(crate) connect_admission: std::sync::Arc<admission::ConnectAdmission>,
	/// Pool names this process refused node connects for, for the sustained refusal warning.
	pub(crate) missing_pools: std::sync::Arc<missing_pools::MissingPools>,
	/// Writes every connection's heartbeat to its pool's ring index, one write per pool.
	pub(crate) index_writer: std::sync::Arc<connection::IndexWriter>,
	/// The node connections this process holds, sent in every pre-connect.
	pub(crate) envoy_connections: std::sync::Arc<pre_connect::ConnectionCount>,
}

/// Guard owns this signal; cancellation must also ask detached upgrades to drain.
pub struct ConnectionShutdown(tokio::sync::watch::Sender<bool>);

impl Drop for ConnectionShutdown {
	fn drop(&mut self) {
		self.0.send_replace(true);
	}
}

impl Service {
	pub fn shutdown_owner(&self) -> ConnectionShutdown {
		ConnectionShutdown(self.stopping.clone())
	}

	/// Called after the local HTTP listener and accepted response bodies drain.
	pub async fn drain_connections(&self) {
		self.stopping.send_replace(true);
		self.connections.drain().await;
		self.index_writer.shutdown().await;
	}
}

pub fn service(ctx: &EngineContext) -> Service {
	Service {
		connections: Default::default(),
		stopping: tokio::sync::watch::channel(false).0,
		sqlite_pool: sqlite_embedded::worker::SqlitePool::new(
			ctx.config().pegboard().sqlite_remote_worker_threads(),
			None,
		),
		index_writer: std::sync::Arc::new(connection::IndexWriter::new(ctx)),
		envoy_connections: Default::default(),
		missing_pools: std::sync::Arc::new(missing_pools::MissingPools::new(
			ctx.config().pegboard().envoy_missing_pools_capacity(),
		)),
		connect_admission: std::sync::Arc::new(admission::ConnectAdmission::new(
			admission::ConnectBudget {
				admissions_per_second: ctx
					.config()
					.pegboard()
					.envoy_connect_admissions_per_second(),
				burst: ctx.config().pegboard().envoy_connect_admission_burst(),
				retry_after_ms: ctx.config().pegboard().envoy_connect_retry_after_ms(),
			},
		)),
	}
}

pub fn router(ctx: &EngineContext, service: &Service) -> Router {
	Router::new()
		.route("/envoys/connect", axum::routing::any(routes::connect))
		.layer(axum::Extension(service.clone()))
		.with_state(ctx.clone())
}

/// An Envoy connect's failure as an HTTP response: a 500 with a `{ "error": "..." }` JSON body.
/// Rejections the Envoy can act on are sent as WebSocket close frames instead.
pub(crate) struct EnvoyError(anyhow::Error);

impl<E: Into<anyhow::Error>> From<E> for EnvoyError {
	fn from(error: E) -> Self {
		Self(error.into())
	}
}

impl IntoResponse for EnvoyError {
	fn into_response(self) -> Response {
		let body = serde_json::json!({ "error": self.0.to_string() });
		(StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
	}
}
