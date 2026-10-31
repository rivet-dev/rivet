//! Test-only actor context fixtures backed by a real in-memory SQLite database.
//!
//! Enable the `test-support` feature in downstream crates. The fixture keeps
//! the production invariant that every [`ActorContext`] has a configured
//! SQLite backend; it does not reintroduce an unavailable or in-memory runtime
//! backend.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use rivet_envoy_client::config::{
	BoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse, WebSocketHandler,
	WebSocketSender,
};
use rivet_envoy_client::context::{SharedContext, WsTxMessage};
use rivet_envoy_client::envoy::ToEnvoyMessage;
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_client::protocol;
use rivet_envoy_client::sqlite::{
	RemoteSqliteRequest, RemoteSqliteResponse, RemoteSqliteResponseEnvelope,
};
use tokio::sync::mpsc;

use crate::ActorConfig;
use crate::actor::context::ActorContext;
use crate::actor::kv::LegacyActorKv;
use crate::sqlite::SqliteDb;
use crate::types::ActorKey;

#[path = "../tests/support/sqlite_wire.rs"]
mod sqlite_wire;
use sqlite_wire::{execute_sqlite, execute_sqlite_batch};

/// Reusable in-memory SQLite store for constructing fully configured contexts.
/// Contexts created from the same harness observe the same database.
#[derive(Clone)]
pub struct ActorContextHarness {
	handle: EnvoyHandle,
}

impl Default for ActorContextHarness {
	fn default() -> Self {
		Self::new()
	}
}

impl ActorContextHarness {
	pub fn new() -> Self {
		Self::with_client_endpoint("http://127.0.0.1:1")
	}

	/// Constructs a harness whose actor clients use the provided engine endpoint.
	pub fn with_client_endpoint(endpoint: impl Into<String>) -> Self {
		let (handle, receiver) = test_envoy_handle(endpoint.into());
		spawn_remote_sqlite(receiver, None);
		Self { handle }
	}

	/// Keeps actor intent messages observable while servicing SQLite requests.
	pub fn with_intents() -> (Self, mpsc::UnboundedReceiver<ToEnvoyMessage>) {
		let (handle, receiver) = test_envoy_handle("http://127.0.0.1:1".to_owned());
		let (intent_tx, intent_rx) = mpsc::unbounded_channel();
		spawn_remote_sqlite(receiver, Some(intent_tx));
		(Self { handle }, intent_rx)
	}

	pub fn context(
		&self,
		actor_id: impl Into<String>,
		name: impl Into<String>,
		key: ActorKey,
		region: impl Into<String>,
	) -> ActorContext {
		self.context_with_config(actor_id, name, key, region, ActorConfig::default())
	}

	pub fn context_with_config(
		&self,
		actor_id: impl Into<String>,
		name: impl Into<String>,
		key: ActorKey,
		region: impl Into<String>,
		config: ActorConfig,
	) -> ActorContext {
		let actor_id = actor_id.into();
		let generation = Some(1);
		let sql = SqliteDb::new_with_remote_sqlite(
			self.handle.clone(),
			actor_id.clone(),
			None,
			generation.map(u64::from),
			false,
			true,
		)
		.expect("test remote sqlite should be configured");
		let ctx = ActorContext::build(
			actor_id.clone(),
			name.into(),
			key,
			region.into(),
			generation,
			self.handle.get_envoy_key().to_owned(),
			config,
			LegacyActorKv::new(self.handle.clone(), actor_id),
			sql,
		);
		ctx.configure_envoy(self.handle.clone(), generation);
		ctx
	}
}

/// Constructs a context with its own in-memory SQLite store.
pub fn actor_context(
	actor_id: impl Into<String>,
	name: impl Into<String>,
	key: ActorKey,
	region: impl Into<String>,
) -> ActorContext {
	ActorContextHarness::new().context(actor_id, name, key, region)
}

struct IdleEnvoyCallbacks;

impl EnvoyCallbacks for IdleEnvoyCallbacks {
	fn on_actor_start(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_generation: u32,
		_config: protocol::ActorConfig,
		_preloaded_kv: Option<protocol::PreloadedKv>,
	) -> BoxFuture<Result<()>> {
		Box::pin(async { Ok(()) })
	}

	fn on_shutdown(&self) {}

	fn fetch(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_gateway_id: protocol::GatewayId,
		_request_id: protocol::RequestId,
		_request: HttpRequest,
	) -> BoxFuture<Result<HttpResponse>> {
		Box::pin(async { anyhow::bail!("fetch should not run in actor context fixture") })
	}

	fn websocket(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_gateway_id: protocol::GatewayId,
		_request_id: protocol::RequestId,
		_request: HttpRequest,
		_path: String,
		_headers: HashMap<String, String>,
		_is_hibernatable: bool,
		_is_restoring_hibernatable: bool,
		_sender: WebSocketSender,
	) -> BoxFuture<Result<WebSocketHandler>> {
		Box::pin(async { anyhow::bail!("websocket should not run in actor context fixture") })
	}

	fn can_hibernate(
		&self,
		_actor_id: &str,
		_gateway_id: &protocol::GatewayId,
		_request_id: &protocol::RequestId,
		_request: &HttpRequest,
	) -> BoxFuture<Result<bool>> {
		Box::pin(async { Ok(false) })
	}
}

fn test_envoy_handle(endpoint: String) -> (EnvoyHandle, mpsc::UnboundedReceiver<ToEnvoyMessage>) {
	let (envoy_tx, envoy_rx) = mpsc::unbounded_channel();
	let shared = Arc::new(SharedContext {
		config: EnvoyConfig {
			version: 1,
			endpoint,
			token: None,
			namespace: "test".to_owned(),
			pool_name: "test".to_owned(),
			prepopulate_actor_names: HashMap::new(),
			metadata: None,
			not_global: true,
			debug_latency_ms: None,
			max_queued_requests_per_actor:
				rivet_envoy_client::config::DEFAULT_MAX_QUEUED_REQUESTS_PER_ACTOR,
			stopped_generation_floor_ttl:
				rivet_envoy_client::config::DEFAULT_STOPPED_GENERATION_FLOOR_TTL,
			stopped_generation_floor_capacity:
				rivet_envoy_client::config::DEFAULT_STOPPED_GENERATION_FLOOR_CAPACITY,
			callbacks: Arc::new(IdleEnvoyCallbacks),
			connector: None,
		},
		envoy_key: "test-envoy".to_owned(),
		envoy_tx,
		actors: Default::default(),
		actors_notify: Arc::new(tokio::sync::Notify::new()),
		live_tunnel_requests: Default::default(),
		pending_hibernation_restores: Default::default(),
		ws_tx: Arc::new(tokio::sync::Mutex::new(
			None::<mpsc::UnboundedSender<WsTxMessage>>,
		)),
		http_ws_tx: Arc::new(tokio::sync::Mutex::new(None)),
		connection_session: std::sync::atomic::AtomicU64::new(1),
		next_connection_session: std::sync::atomic::AtomicU64::new(1),
		connection_session_tx: tokio::sync::watch::channel(1).0,
		protocol_metadata: Arc::new(tokio::sync::Mutex::new(None)),
		shutting_down: std::sync::atomic::AtomicBool::new(false),
		last_ping_ts: std::sync::atomic::AtomicI64::new(i64::MAX),
		last_contact_ms: std::sync::atomic::AtomicI64::new(0),
		contact_before_install_ms: std::sync::atomic::AtomicI64::new(0),
		stopped_tx: tokio::sync::watch::channel(
			rivet_envoy_client::context::StopStatus::Interrupted,
		)
		.0,
	});
	(EnvoyHandle::from_shared(shared), envoy_rx)
}

fn spawn_remote_sqlite(
	mut receiver: mpsc::UnboundedReceiver<ToEnvoyMessage>,
	intent_tx: Option<mpsc::UnboundedSender<ToEnvoyMessage>>,
) {
	std::thread::spawn(move || {
		let conn =
			rusqlite::Connection::open_in_memory().expect("test sqlite connection should open");
		crate::actor::internal_storage::schema::initialize_test_schema(&conn)
			.expect("test sqlite schema should initialize");
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("test sqlite runtime should build");
		runtime.block_on(async move {
			while let Some(message) = receiver.recv().await {
				let ToEnvoyMessage::RemoteSqliteRequest {
					request,
					expected_session: _,
					response_tx,
				} = message
				else {
					if matches!(message, ToEnvoyMessage::ActorIntent { .. }) {
						if let Some(sender) = &intent_tx {
							let _ = sender.send(message);
						}
					}
					continue;
				};
				let response = match request {
					RemoteSqliteRequest::Execute(request) => {
						RemoteSqliteResponse::Execute(execute_sqlite(&conn, request))
					}
					RemoteSqliteRequest::ExecuteBatch(request) => {
						RemoteSqliteResponse::ExecuteBatch(execute_sqlite_batch(&conn, request))
					}
					RemoteSqliteRequest::Exec(_) => continue,
				};
				let _ = response_tx.send(Ok(RemoteSqliteResponseEnvelope {
					response,
					session: 1,
				}));
			}
		});
	});
}

#[path = "../tests/support/legacy_migration.rs"]
mod legacy_migration;
