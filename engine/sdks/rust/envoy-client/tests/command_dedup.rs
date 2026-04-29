use std::collections::HashMap;
use std::sync::Arc;

use rivet_envoy_client::actor::ToActor;
use rivet_envoy_client::commands::handle_commands;
use rivet_envoy_client::config::{
	BoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse, WebSocketHandler,
	WebSocketSender,
};
use rivet_envoy_client::context::{SharedContext, WsTxMessage};
use rivet_envoy_client::envoy::EnvoyContext;
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_client::utils::BufferMap;
use rivet_envoy_protocol as protocol;
use rivet_util::async_counter::AsyncCounter;
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
		envoy_key: "test-envoy".to_string(),
		envoy_tx,
		actors: Arc::new(std::sync::Mutex::new(HashMap::new())),
		live_tunnel_requests: Arc::new(std::sync::Mutex::new(HashMap::new())),
		pending_hibernation_restores: Arc::new(std::sync::Mutex::new(HashMap::new())),
		ws_tx: Arc::new(tokio::sync::Mutex::new(
			None::<mpsc::UnboundedSender<WsTxMessage>>,
		)),
		protocol_metadata: Arc::new(tokio::sync::Mutex::new(None)),
		shutting_down: std::sync::atomic::AtomicBool::new(false),
		stopped_tx: tokio::sync::watch::channel(true).0,
	});
	EnvoyContext {
		shared,
		shutting_down: false,
		actors: HashMap::new(),
		buffered_actor_messages: HashMap::new(),
		kv_requests: HashMap::new(),
		next_kv_request_id: 0,
		sqlite_requests: HashMap::new(),
		next_sqlite_request_id: 0,
		request_to_actor: BufferMap::new(),
		buffered_messages: Vec::new(),
		processed_command_idx: HashMap::new(),
	}
}

fn stop_command(actor_id: &str, generation: u32, index: i64) -> protocol::CommandWrapper {
	protocol::CommandWrapper {
		checkpoint: protocol::ActorCheckpoint {
			actor_id: actor_id.to_string(),
			generation,
			index,
		},
		inner: protocol::Command::CommandStopActor(protocol::CommandStopActor {
			reason: protocol::StopActorReason::StopIntent,
		}),
	}
}

#[tokio::test]
async fn replayed_stop_command_is_dropped() {
	let mut ctx = new_envoy_context();
	let (actor_tx, mut actor_rx) = mpsc::unbounded_channel::<ToActor>();
	ctx.insert_actor(
		"actor-replay".to_string(),
		1,
		actor_tx,
		Arc::new(AsyncCounter::new()),
		"actor-replay".to_string(),
		-1,
	);

	handle_commands(&mut ctx, vec![stop_command("actor-replay", 1, 5)]).await;
	assert!(matches!(
		actor_rx.try_recv(),
		Ok(ToActor::Stop { command_idx: 5, .. })
	));

	// Same index replayed: should be skipped.
	handle_commands(&mut ctx, vec![stop_command("actor-replay", 1, 5)]).await;
	assert!(actor_rx.try_recv().is_err());

	// Lower index from a stale replay: should also be skipped.
	handle_commands(&mut ctx, vec![stop_command("actor-replay", 1, 3)]).await;
	assert!(actor_rx.try_recv().is_err());

	// Higher index is processed.
	handle_commands(&mut ctx, vec![stop_command("actor-replay", 1, 7)]).await;
	assert!(matches!(
		actor_rx.try_recv(),
		Ok(ToActor::Stop { command_idx: 7, .. })
	));
}

#[tokio::test]
async fn dedup_is_per_actor_and_generation() {
	let mut ctx = new_envoy_context();
	let (tx_a1, mut rx_a1) = mpsc::unbounded_channel::<ToActor>();
	let (tx_a2, mut rx_a2) = mpsc::unbounded_channel::<ToActor>();
	let (tx_b1, mut rx_b1) = mpsc::unbounded_channel::<ToActor>();
	ctx.insert_actor(
		"actor-a".to_string(),
		1,
		tx_a1,
		Arc::new(AsyncCounter::new()),
		"actor-a".to_string(),
		-1,
	);
	ctx.insert_actor(
		"actor-a".to_string(),
		2,
		tx_a2,
		Arc::new(AsyncCounter::new()),
		"actor-a".to_string(),
		-1,
	);
	ctx.insert_actor(
		"actor-b".to_string(),
		1,
		tx_b1,
		Arc::new(AsyncCounter::new()),
		"actor-b".to_string(),
		-1,
	);

	handle_commands(&mut ctx, vec![stop_command("actor-a", 1, 5)]).await;
	assert!(rx_a1.try_recv().is_ok());

	// Same actor_id, different generation: not deduped.
	handle_commands(&mut ctx, vec![stop_command("actor-a", 2, 5)]).await;
	assert!(rx_a2.try_recv().is_ok());

	// Different actor_id, same index: not deduped.
	handle_commands(&mut ctx, vec![stop_command("actor-b", 1, 5)]).await;
	assert!(rx_b1.try_recv().is_ok());
}
