use std::collections::HashMap;
use std::sync::Arc;

use rivet_envoy_client::actor::ToActor;
use rivet_envoy_client::async_counter::AsyncCounter;
use rivet_envoy_client::config::{
	BoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse, WebSocketHandler,
	WebSocketSender,
};
use rivet_envoy_client::context::{SharedContext, WsTxMessage};
use rivet_envoy_client::envoy::{EnvoyContext, PendingActorEvents, handle_lost_timeout};
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_client::utils::BufferMap;
use rivet_envoy_protocol as protocol;
use tokio::sync::mpsc;

struct IdleCallbacks;

impl EnvoyCallbacks for IdleCallbacks {
	fn on_actor_start(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_generation: u32,
		_config: protocol::ActorConfig,
		_preloaded_kv: Option<protocol::PreloadedKv>,
	) -> BoxFuture<anyhow::Result<()>> {
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
	) -> BoxFuture<anyhow::Result<HttpResponse>> {
		Box::pin(async { anyhow::bail!("fetch should not be called in command tests") })
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
	) -> BoxFuture<anyhow::Result<WebSocketHandler>> {
		Box::pin(async { anyhow::bail!("websocket should not be called in command tests") })
	}

	fn can_hibernate(
		&self,
		_actor_id: &str,
		_gateway_id: &protocol::GatewayId,
		_request_id: &protocol::RequestId,
		_request: &HttpRequest,
	) -> BoxFuture<anyhow::Result<bool>> {
		Box::pin(async { Ok(false) })
	}
}

fn new_envoy_context() -> EnvoyContext {
	let (envoy_tx, _envoy_rx) = mpsc::unbounded_channel();
	let shared = Arc::new(SharedContext {
		config: EnvoyConfig {
			version: 1,
			endpoint: "http://127.0.0.1:1".to_string(),
			token: None,
			namespace: "test".to_string(),
			pool_name: "test".to_string(),
			prepopulate_actor_names: HashMap::new(),
			metadata: None,
			not_global: true,
			debug_latency_ms: None,
			callbacks: Arc::new(IdleCallbacks),
		},
		envoy_key: parking_lot::RwLock::new("test-envoy".to_string()),
		envoy_tx,
		actors: Arc::new(std::sync::Mutex::new(HashMap::new())),
		actors_notify: Arc::new(tokio::sync::Notify::new()),
		live_tunnel_requests: Arc::new(std::sync::Mutex::new(HashMap::new())),
		pending_hibernation_restores: Arc::new(std::sync::Mutex::new(HashMap::new())),
		ws_tx: Arc::new(tokio::sync::Mutex::new(
			None::<mpsc::UnboundedSender<WsTxMessage>>,
		)),
		http_ws_tx: Arc::new(tokio::sync::Mutex::new(None)),
		connection_session: std::sync::atomic::AtomicU64::new(0),
		next_connection_session: std::sync::atomic::AtomicU64::new(0),
		connection_session_tx: tokio::sync::watch::channel(0).0,
		protocol_metadata: Arc::new(tokio::sync::Mutex::new(None)),
		shutting_down: std::sync::atomic::AtomicBool::new(false),
		last_ping_ts: std::sync::atomic::AtomicI64::new(0),
		stopped_tx: tokio::sync::watch::channel(true).0,
	});
	EnvoyContext {
		shared,
		shutting_down: false,
		actors: HashMap::new(),
		pending_events: HashMap::new(),
		buffered_actor_messages: HashMap::new(),
		kv_requests: HashMap::new(),
		next_kv_request_id: 0,
		sqlite_requests: HashMap::new(),
		next_sqlite_request_id: 0,
		remote_sqlite_requests: HashMap::new(),
		next_remote_sqlite_request_id: 0,
		request_to_actor: BufferMap::new(),
		http_request_routes: BufferMap::new(),
		http_message_indices: BufferMap::new(),
		http_request_cancellations: HashMap::new(),
		buffered_messages: Vec::new(),
		processed_command_idx: HashMap::new(),
	}
}

/// The lost threshold discards every actor locally. Under the old key the engine would keep
/// believing those actors run here, so the registration key must change and every piece of
/// state tied to the old registration must go with it.
#[tokio::test]
async fn lost_timeout_rotates_envoy_key_and_drops_registration_state() {
	let mut ctx = new_envoy_context();
	let old_key = ctx.shared.envoy_key();
	assert_eq!(old_key, "test-envoy");

	let (actor_tx, mut actor_rx) = mpsc::unbounded_channel::<ToActor>();
	ctx.insert_actor(
		"actor-a".to_string(),
		3,
		actor_tx,
		Arc::new(AsyncCounter::new()),
		"counter".to_string(),
		1,
	);
	ctx.pending_events.insert(
		("actor-a".to_string(), 3),
		PendingActorEvents {
			events: vec![protocol::EventWrapper {
				checkpoint: protocol::ActorCheckpoint {
					actor_id: "actor-a".to_string(),
					generation: 3,
					index: 0,
				},
				inner: protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
					state: protocol::ActorState::ActorStateRunning,
				}),
			}],
			retired_at: None,
		},
	);
	ctx.processed_command_idx
		.insert(("actor-a".to_string(), 3), 1);

	let (ws_tx, mut ws_rx) = mpsc::unbounded_channel::<WsTxMessage>();
	*ctx.shared.ws_tx.lock().await = Some(ws_tx);

	handle_lost_timeout(&mut ctx).await;

	assert!(
		matches!(actor_rx.try_recv(), Ok(ToActor::Lost)),
		"the actor task must be told it was lost"
	);
	assert!(ctx.actors.is_empty(), "local actor map must be cleared");
	assert!(
		ctx.shared
			.actors
			.lock()
			.expect("shared actor registry poisoned")
			.is_empty(),
		"shared actor registry must be cleared"
	);

	let new_key = ctx.shared.envoy_key();
	assert_ne!(new_key, old_key, "a fresh registration key must be minted");
	assert!(
		uuid::Uuid::parse_str(&new_key).is_ok(),
		"the new key must be a uuid like the initial one"
	);

	assert!(
		ctx.pending_events.is_empty(),
		"unacked events belong to the old registration"
	);
	assert!(
		ctx.processed_command_idx.is_empty(),
		"replayed command bookkeeping belongs to the old registration"
	);
	assert!(
		matches!(ws_rx.try_recv(), Ok(WsTxMessage::Close)),
		"a socket opened under the old key must be closed so the reconnect uses the new key"
	);
}

/// A lost timeout with nothing running still rotates: the engine's view of the old registration
/// is stale either way, and the next connection must register as a new envoy.
#[tokio::test]
async fn lost_timeout_rotates_even_with_no_actors() {
	let mut ctx = new_envoy_context();
	let old_key = ctx.shared.envoy_key();

	handle_lost_timeout(&mut ctx).await;

	assert_ne!(ctx.shared.envoy_key(), old_key);
	assert!(ctx.actors.is_empty());
}

/// The handle reads through to the current key, so callers that stringify it for the connect
/// URL or for logs see the rotated value.
#[tokio::test]
async fn handle_reports_rotated_key() {
	let mut ctx = new_envoy_context();
	let handle = EnvoyHandle::from_shared(ctx.shared.clone());
	let before = handle.get_envoy_key();

	handle_lost_timeout(&mut ctx).await;

	assert_ne!(handle.get_envoy_key(), before);
	assert_eq!(handle.get_envoy_key(), ctx.shared.envoy_key());
}
