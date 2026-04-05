use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use rivet_envoy_protocol::{self as protocol, PROTOCOL_VERSION};
use std::{
	collections::HashMap,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
	time::Duration,
};
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

use crate::{actor::*, utils};

type ActorFactory = Arc<dyn Fn(ActorConfig) -> Box<dyn TestActor> + Send + Sync>;
type WsStream =
	tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Lifecycle events for actors that tests can subscribe to
#[derive(Debug, Clone)]
pub enum ActorLifecycleEvent {
	Started { actor_id: String, generation: u32 },
	Stopped { actor_id: String, generation: u32 },
}

/// Configuration for the envoy client.
///
/// This matches the TypeScript EnvoyConfig interface.
#[derive(Clone)]
pub struct EnvoyConfig {
	/// The endpoint URL to connect to (e.g., "http://127.0.0.1:8080")
	pub endpoint: String,
	/// Authentication token
	pub token: String,
	/// Namespace to connect to
	pub namespace: String,
	/// Name of the pool this envoy belongs to
	pub pool_name: String,
	/// Version number
	pub version: u32,
	/// Optional metadata to attach to the envoy
	pub metadata: Option<serde_json::Value>,
}

impl EnvoyConfig {
	/// Create a new builder for EnvoyConfig
	pub fn builder() -> EnvoyConfigBuilder {
		EnvoyConfigBuilder::default()
	}
}

/// Builder for EnvoyConfig
#[derive(Default)]
pub struct EnvoyConfigBuilder {
	endpoint: Option<String>,
	token: Option<String>,
	namespace: Option<String>,
	pool_name: Option<String>,
	version: Option<u32>,
	metadata: Option<serde_json::Value>,
}

impl EnvoyConfigBuilder {
	pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
		self.endpoint = Some(endpoint.into());
		self
	}

	pub fn token(mut self, token: impl Into<String>) -> Self {
		self.token = Some(token.into());
		self
	}

	pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
		self.namespace = Some(namespace.into());
		self
	}

	pub fn pool_name(mut self, name: impl Into<String>) -> Self {
		self.pool_name = Some(name.into());
		self
	}

	pub fn version(mut self, version: u32) -> Self {
		self.version = Some(version);
		self
	}

	pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
		self.metadata = Some(metadata);
		self
	}

	pub fn build(self) -> Result<EnvoyConfig> {
		Ok(EnvoyConfig {
			endpoint: self.endpoint.context("endpoint is required")?,
			token: self.token.unwrap_or_else(|| "dev".to_string()),
			namespace: self.namespace.context("namespace is required")?,
			pool_name: self.pool_name.unwrap_or_else(|| "default".to_string()),
			version: self.version.unwrap_or(1),
			metadata: self.metadata,
		})
	}
}

/// Internal configuration with actor factories
#[derive(Clone)]
struct InternalConfig {
	namespace: String,
	pool_name: String,
	version: u32,
	endpoint: String,
	token: String,
	actor_factories: HashMap<String, ActorFactory>,
}

/// Envoy client for programmatic actor lifecycle control
pub struct Envoy {
	config: InternalConfig,

	// State
	pub envoy_key: String,
	is_ready: Arc<AtomicBool>,
	actors: Arc<Mutex<HashMap<String, ActorState>>>,
	/// Per-actor event indices for checkpoints
	actor_event_indices: Arc<Mutex<HashMap<String, i64>>>,
	event_history: Arc<Mutex<Vec<protocol::EventWrapper>>>,
	shutdown: Arc<AtomicBool>,
	is_child_task: bool,

	// Event channel for actors to push events
	event_tx: mpsc::UnboundedSender<ActorEvent>,
	event_rx: Arc<Mutex<mpsc::UnboundedReceiver<ActorEvent>>>,

	// KV request channel for actors to send KV requests
	kv_request_tx: mpsc::UnboundedSender<KvRequest>,
	kv_request_rx: Arc<Mutex<mpsc::UnboundedReceiver<KvRequest>>>,
	next_kv_request_id: Arc<Mutex<u32>>,
	kv_pending_requests: Arc<Mutex<HashMap<u32, oneshot::Sender<protocol::KvResponseData>>>>,

	// Lifecycle event broadcast channel
	lifecycle_tx: broadcast::Sender<ActorLifecycleEvent>,

	// Shutdown channel
	shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

struct ActorState {
	#[allow(dead_code)]
	actor_id: String,
	#[allow(dead_code)]
	generation: u32,
	actor: Box<dyn TestActor>,
}

/// Builder for creating a Envoy instance
pub struct EnvoyBuilder {
	config: EnvoyConfig,
	actor_factories: HashMap<String, ActorFactory>,
}

impl EnvoyBuilder {
	/// Create a new EnvoyBuilder with the given configuration
	pub fn new(config: EnvoyConfig) -> Self {
		Self {
			config,
			actor_factories: HashMap::new(),
		}
	}

	/// Register an actor factory for a specific actor name
	pub fn with_actor_behavior<F>(mut self, actor_name: &str, factory: F) -> Self
	where
		F: Fn(ActorConfig) -> Box<dyn TestActor> + Send + Sync + 'static,
	{
		self.actor_factories
			.insert(actor_name.to_string(), Arc::new(factory));
		self
	}

	/// Build the Envoy instance
	pub fn build(self) -> Result<Envoy> {
		let config = InternalConfig {
			namespace: self.config.namespace,
			pool_name: self.config.pool_name,
			version: self.config.version,
			endpoint: self.config.endpoint,
			token: self.config.token,
			actor_factories: self.actor_factories,
		};

		// Create event channel for actors to push events
		let (event_tx, event_rx) = mpsc::unbounded_channel();

		// Create KV request channel for actors to send KV requests
		let (kv_request_tx, kv_request_rx) = mpsc::unbounded_channel();

		// Create lifecycle event broadcast channel (capacity of 100 for buffering)
		let (lifecycle_tx, _) = broadcast::channel(100);

		Ok(Envoy {
			config,
			envoy_key: Uuid::new_v4().to_string(),
			is_ready: Arc::new(AtomicBool::new(false)),
			actors: Arc::new(Mutex::new(HashMap::new())),
			actor_event_indices: Arc::new(Mutex::new(HashMap::new())),
			event_history: Arc::new(Mutex::new(Vec::new())),
			shutdown: Arc::new(AtomicBool::new(false)),
			is_child_task: false,
			event_tx,
			event_rx: Arc::new(Mutex::new(event_rx)),
			kv_request_tx,
			kv_request_rx: Arc::new(Mutex::new(kv_request_rx)),
			next_kv_request_id: Arc::new(Mutex::new(0)),
			kv_pending_requests: Arc::new(Mutex::new(HashMap::new())),
			lifecycle_tx,
			shutdown_tx: Arc::new(Mutex::new(None)),
		})
	}
}

impl Envoy {
	/// Subscribe to actor lifecycle events
	pub fn subscribe_lifecycle_events(&self) -> broadcast::Receiver<ActorLifecycleEvent> {
		self.lifecycle_tx.subscribe()
	}

	/// Start the envoy
	pub async fn start(&self) -> Result<()> {
		tracing::info!(
			namespace = %self.config.namespace,
			pool_name = %self.config.pool_name,
			envoy_key = %self.envoy_key,
			"starting envoy client"
		);

		let ws_url = self.build_ws_url();

		tracing::debug!(ws_url = %ws_url, "connecting to pegboard");

		// Connect to WebSocket with protocols
		let token_protocol = format!("rivet_token.{}", self.config.token);

		// Build the request properly with all WebSocket headers
		use tokio_tungstenite::tungstenite::client::IntoClientRequest;
		let mut request = ws_url
			.into_client_request()
			.context("failed to build WebSocket request")?;

		// Add the Sec-WebSocket-Protocol header
		request.headers_mut().insert(
			"Sec-WebSocket-Protocol",
			format!("rivet, {}", token_protocol).parse().unwrap(),
		);

		let (ws_stream, _response) = connect_async(request)
			.await
			.context("failed to connect to WebSocket")?;

		tracing::info!("websocket connected");

		// Create shutdown channel
		let (shutdown_tx, shutdown_rx) = oneshot::channel();
		*self.shutdown_tx.lock().await = Some(shutdown_tx);

		// Clone self for the spawned task
		let envoy = self.clone_for_task();

		tokio::spawn(async move {
			if let Err(err) = envoy.run_message_loop(ws_stream, shutdown_rx).await {
				tracing::error!(?err, "envoy client message loop failed");
			}
		});

		Ok(())
	}

	/// Clone the envoy for passing to async tasks
	fn clone_for_task(&self) -> Self {
		Self {
			config: self.config.clone(),
			envoy_key: self.envoy_key.clone(),
			is_ready: self.is_ready.clone(),
			actors: self.actors.clone(),
			actor_event_indices: self.actor_event_indices.clone(),
			event_history: self.event_history.clone(),
			is_child_task: true,
			shutdown: self.shutdown.clone(),
			event_tx: self.event_tx.clone(),
			event_rx: self.event_rx.clone(),
			kv_request_tx: self.kv_request_tx.clone(),
			kv_request_rx: self.kv_request_rx.clone(),
			next_kv_request_id: self.next_kv_request_id.clone(),
			kv_pending_requests: self.kv_pending_requests.clone(),
			lifecycle_tx: self.lifecycle_tx.clone(),
			shutdown_tx: self.shutdown_tx.clone(),
		}
	}

	/// Wait for envoy to be ready
	pub async fn wait_ready(&self) {
		loop {
			if self.is_ready.load(Ordering::SeqCst) {
				break;
			}
			tokio::time::sleep(Duration::from_millis(100)).await;
		}
	}

	/// Check if envoy has an actor
	pub async fn has_actor(&self, actor_id: &str) -> bool {
		let actors = self.actors.lock().await;
		actors.contains_key(actor_id)
	}

	/// Get envoy's current actor IDs
	pub async fn get_actor_ids(&self) -> Vec<String> {
		let actors = self.actors.lock().await;
		actors.keys().cloned().collect()
	}

	pub fn pool_name(&self) -> &str {
		&self.config.pool_name
	}

	/// Shutdown the envoy gracefully (destroys actors first)
	pub async fn shutdown(&self) {
		tracing::info!("shutting down envoy client");
		self.shutdown.store(true, Ordering::SeqCst);

		// Send shutdown signal to close ws_stream
		if let Some(tx) = self.shutdown_tx.lock().await.take() {
			let _ = tx.send(());
		}
	}

	/// Crash the envoy without graceful shutdown.
	/// This simulates an ungraceful disconnect where the envoy stops responding
	/// without destroying its actors first. Use this to test EnvoyNoResponse errors.
	pub async fn crash(&self) {
		tracing::info!("crashing envoy client (ungraceful disconnect)");
		self.shutdown.store(true, Ordering::SeqCst);

		// Just drop the websocket without cleanup - don't send any signals
		// The server will detect the disconnect and actors will remain in
		// an unresponsive state until they timeout.
		if let Some(tx) = self.shutdown_tx.lock().await.take() {
			let _ = tx.send(());
		}

		// Clear local actor state without notifying server
		self.actors.lock().await.clear();
	}

	fn build_ws_url(&self) -> String {
		let ws_endpoint = self.config.endpoint.replace("http://", "ws://");
		format!(
			"{}/envoys/connect?protocol_version={}&namespace={}&pool_name={}&envoy_key={}",
			ws_endpoint.trim_end_matches('/'),
			PROTOCOL_VERSION,
			urlencoding::encode(&self.config.namespace),
			urlencoding::encode(&self.config.pool_name),
			urlencoding::encode(&self.envoy_key)
		)
	}

	fn build_init_message(&self) -> protocol::ToRivet {
		protocol::ToRivet::ToRivetInit(protocol::ToRivetInit {
			envoy_key: self.envoy_key.clone(),
			version: self.config.version,
			prepopulate_actor_names: None,
			metadata: None,
		})
	}

	async fn run_message_loop(
		self,
		mut ws_stream: WsStream,
		mut shutdown_rx: oneshot::Receiver<()>,
	) -> Result<()> {
		// Send init message
		let init_msg = self.build_init_message();
		let encoded = utils::encode_to_rivet(init_msg);
		ws_stream
			.send(Message::Binary(encoded.into()))
			.await
			.context("failed to send init message")?;

		tracing::debug!("sent init message");

		// We lock here as these rx's are only for run_message_loop
		let mut event_rx = self.event_rx.lock().await;
		let mut kv_request_rx = self.kv_request_rx.lock().await;

		loop {
			tokio::select! {
				biased;
				_ = &mut shutdown_rx => {
					tracing::info!("received shutdown signal, closing websocket");
					let _ = ws_stream.close(None).await;
					break;
				}

				// Listen for events pushed from actors
				Some(actor_event) = event_rx.recv() => {
					if self.shutdown.load(Ordering::SeqCst) {
						tracing::info!("shutting down");
						break;
					}

					tracing::debug!(
						actor_id = ?actor_event.actor_id,
						generation = actor_event.generation,
						"received event from actor"
					);

					self.send_actor_event(&mut ws_stream, actor_event).await?;
				}

				// Listen for KV requests from actors
				Some(kv_request) = kv_request_rx.recv() => {
					if self.shutdown.load(Ordering::SeqCst) {
						break;
					}

					tracing::debug!(
						actor_id = ?kv_request.actor_id,
						"received kv request from actor"
					);

					self.send_kv_request(&mut ws_stream, kv_request).await?;
				}

				msg = ws_stream.next() => {
					if self.shutdown.load(Ordering::SeqCst) {
						break;
					}

					match msg {
						Some(std::result::Result::Ok(Message::Binary(buf))) => {
							self.handle_message(&mut ws_stream, &buf).await?;
						}
						Some(std::result::Result::Ok(Message::Close(_))) => {
							tracing::info!("websocket closed by server");
							break;
						}
						Some(std::result::Result::Err(err)) => {
							tracing::error!(?err, "websocket error");
							return Err(err.into());
						}
						None => {
							tracing::info!("websocket stream ended");
							break;
						}
						_ => {}
					}
				}
			}
		}

		tracing::info!("envoy client message loop exiting");
		Ok(())
	}

	/// Send an event pushed from an actor
	async fn send_actor_event(
		&self,
		ws_stream: &mut WsStream,
		actor_event: ActorEvent,
	) -> Result<()> {
		// Get next event index for this actor
		let mut indices = self.actor_event_indices.lock().await;
		let idx = indices.entry(actor_event.actor_id.clone()).or_insert(-1);
		*idx += 1;
		let event_idx = *idx;
		drop(indices);

		let event_wrapper = utils::make_event_wrapper(
			&actor_event.actor_id,
			actor_event.generation,
			event_idx as u64,
			actor_event.event,
		);

		self.event_history.lock().await.push(event_wrapper.clone());

		tracing::debug!(
			actor_id = ?actor_event.actor_id,
			generation = actor_event.generation,
			event_idx = event_idx,
			"sending actor event"
		);

		let msg = protocol::ToRivet::ToRivetEvents(vec![event_wrapper]);
		let encoded = utils::encode_to_rivet(msg);
		ws_stream.send(Message::Binary(encoded.into())).await?;

		Ok(())
	}

	async fn handle_message(&self, ws_stream: &mut WsStream, buf: &[u8]) -> Result<()> {
		let msg = utils::decode_to_envoy(buf, PROTOCOL_VERSION)?;

		match msg {
			protocol::ToEnvoy::ToEnvoyInit(init) => {
				self.handle_init(init, ws_stream).await?;
			}
			protocol::ToEnvoy::ToEnvoyCommands(commands) => {
				self.handle_commands(commands, ws_stream).await?;
			}
			protocol::ToEnvoy::ToEnvoyAckEvents(ack) => {
				self.handle_ack_events(ack).await;
			}
			protocol::ToEnvoy::ToEnvoyKvResponse(response) => {
				self.handle_kv_response(response).await;
			}
			protocol::ToEnvoy::ToEnvoyPing(ping) => {
				self.handle_ping(ws_stream, ping).await?;
			}
			_ => {
				tracing::debug!(?msg, "ignoring message type");
			}
		}

		Ok(())
	}

	async fn handle_init(
		&self,
		_init: protocol::ToEnvoyInit,
		_ws_stream: &mut WsStream,
	) -> Result<()> {
		tracing::info!("received init from server");

		self.is_ready.store(true, Ordering::SeqCst);

		// For simplicity, we don't resend events on reconnect on the envoy

		Ok(())
	}

	async fn handle_commands(
		&self,
		commands: Vec<protocol::CommandWrapper>,
		ws_stream: &mut WsStream,
	) -> Result<()> {
		tracing::info!(count = commands.len(), "received commands");

		for cmd_wrapper in commands {
			let checkpoint = &cmd_wrapper.checkpoint;
			tracing::debug!(
				actor_id = %checkpoint.actor_id,
				generation = checkpoint.generation,
				index = checkpoint.index,
				command = ?cmd_wrapper.inner,
				"processing command"
			);

			match cmd_wrapper.inner {
				protocol::Command::CommandStartActor(start_cmd) => {
					self.handle_start_actor(
						checkpoint.actor_id.clone(),
						checkpoint.generation,
						start_cmd,
						ws_stream,
					)
					.await?;
				}
				protocol::Command::CommandStopActor(_stop_cmd) => {
					self.handle_stop_actor(
						checkpoint.actor_id.clone(),
						checkpoint.generation,
						ws_stream,
					)
					.await?;
				}
			}
		}

		Ok(())
	}

	async fn handle_start_actor(
		&self,
		actor_id: String,
		generation: u32,
		cmd: protocol::CommandStartActor,
		_ws_stream: &mut WsStream,
	) -> Result<()> {
		tracing::info!(?actor_id, generation, name = %cmd.config.name, "starting actor");

		// Create actor config
		let config = ActorConfig::new(
			&cmd.config,
			actor_id.clone(),
			generation,
			self.event_tx.clone(),
			self.kv_request_tx.clone(),
		);

		// Get factory for this actor name
		let factory = self
			.config
			.actor_factories
			.get(&cmd.config.name)
			.context(format!(
				"no factory registered for actor name: {}",
				cmd.config.name
			))?
			.clone();

		// Clone self for the spawned task
		let envoy = self.clone_for_task();
		let actor_id_clone = actor_id.clone();

		// Spawn actor execution in separate task to avoid blocking message loop
		tokio::spawn(async move {
			// Create actor
			let mut actor = factory(config.clone());

			tracing::debug!(
				?actor_id,
				generation,
				actor_type = actor.name(),
				"created actor instance"
			);

			// Call on_start
			let start_result = match actor.on_start(config).await {
				std::result::Result::Ok(result) => result,
				Err(err) => {
					tracing::error!(?actor_id_clone, generation, ?err, "actor on_start failed");
					return;
				}
			};

			tracing::debug!(
				?actor_id_clone,
				generation,
				?start_result,
				"actor on_start completed"
			);

			envoy
				.handle_actor_start_result(actor_id_clone, generation, actor, start_result)
				.await;
		});

		Ok(())
	}

	async fn handle_actor_start_result(
		&self,
		actor_id: String,
		generation: u32,
		actor: Box<dyn TestActor>,
		start_result: ActorStartResult,
	) {
		// Broadcast lifecycle event
		tracing::info!("lifecycle_tx start");
		let _ = self.lifecycle_tx.send(ActorLifecycleEvent::Started {
			actor_id: actor_id.clone(),
			generation,
		});

		// Store actor
		let actor_state = ActorState {
			actor_id: actor_id.clone(),
			generation,
			actor,
		};
		self.actors
			.lock()
			.await
			.insert(actor_id.clone(), actor_state);

		// Handle start result and send state update via event
		match start_result {
			ActorStartResult::Running => {
				let event = utils::make_actor_state_update(protocol::ActorState::ActorStateRunning);
				self.event_tx
					.send(ActorEvent {
						actor_id: actor_id.clone(),
						generation,
						event,
					})
					.expect("failed to send state update");
			}
			ActorStartResult::Delay(duration) => {
				let actor_id_clone = actor_id.clone();
				let event_tx = self.event_tx.clone();
				tokio::spawn(async move {
					tracing::info!(
						?actor_id_clone,
						generation,
						delay_ms = duration.as_millis(),
						"delaying before sending running state"
					);
					tokio::time::sleep(duration).await;
					let event =
						utils::make_actor_state_update(protocol::ActorState::ActorStateRunning);
					event_tx
						.send(ActorEvent {
							actor_id: actor_id_clone,
							generation,
							event,
						})
						.expect("failed to send delayed state update");
				});
			}
			ActorStartResult::Timeout => {
				tracing::warn!(
					?actor_id,
					generation,
					"actor will timeout (not sending running)"
				);
				// Don't send running state
			}
			ActorStartResult::Crash { code, message } => {
				tracing::warn!(?actor_id, generation, code, %message, "actor crashed on start");
				let event = utils::make_actor_state_update(
					protocol::ActorState::ActorStateStopped(protocol::ActorStateStopped {
						code: if code == 0 {
							protocol::StopCode::Ok
						} else {
							protocol::StopCode::Error
						},
						message: Some(message),
					}),
				);
				let _ = self
					.event_tx
					.send(ActorEvent {
						actor_id: actor_id.clone(),
						generation,
						event,
					})
					.expect("failed to send crash state update");

				// Remove actor
				self.actors.lock().await.remove(&actor_id);
			}
		}
	}

	async fn handle_stop_actor(
		&self,
		actor_id: String,
		generation: u32,
		ws_stream: &mut WsStream,
	) -> Result<()> {
		tracing::info!(?actor_id, generation, "stopping actor");

		// Get actor
		let mut actors_guard = self.actors.lock().await;
		let actor_state = actors_guard.get_mut(&actor_id).context("actor not found")?;

		// Call on_stop
		let stop_result = actor_state
			.actor
			.on_stop()
			.await
			.context("actor on_stop failed")?;

		tracing::debug!(
			?actor_id,
			generation,
			?stop_result,
			"actor on_stop completed"
		);

		// Broadcast lifecycle event
		let _ = self.lifecycle_tx.send(ActorLifecycleEvent::Stopped {
			actor_id: actor_id.clone(),
			generation,
		});

		// Handle stop result
		match stop_result {
			ActorStopResult::Success => {
				self.send_actor_state_update(
					&actor_id,
					generation,
					protocol::ActorState::ActorStateStopped(protocol::ActorStateStopped {
						code: protocol::StopCode::Ok,
						message: None,
					}),
					ws_stream,
				)
				.await?;
			}
			ActorStopResult::Delay(duration) => {
				tracing::info!(?actor_id, generation, ?duration, "delaying stop");
				tokio::time::sleep(duration).await;
				self.send_actor_state_update(
					&actor_id,
					generation,
					protocol::ActorState::ActorStateStopped(protocol::ActorStateStopped {
						code: protocol::StopCode::Ok,
						message: None,
					}),
					ws_stream,
				)
				.await?;
			}
			ActorStopResult::Crash { code, message } => {
				tracing::warn!(?actor_id, generation, code, %message, "actor crashed on stop");
				self.send_actor_state_update(
					&actor_id,
					generation,
					protocol::ActorState::ActorStateStopped(protocol::ActorStateStopped {
						code: if code == 0 {
							protocol::StopCode::Ok
						} else {
							protocol::StopCode::Error
						},
						message: Some(message),
					}),
					ws_stream,
				)
				.await?;
			}
		}

		// Remove actor
		actors_guard.remove(&actor_id);

		Ok(())
	}

	async fn handle_ack_events(&self, ack: protocol::ToEnvoyAckEvents) {
		let checkpoints = &ack.last_event_checkpoints;

		let mut events = self.event_history.lock().await;
		let original_len = events.len();

		// Remove events that have been acknowledged based on checkpoints
		events.retain(|e| {
			// Check if this event's checkpoint is covered by any ack checkpoint
			!checkpoints.iter().any(|ck| {
				ck.actor_id == e.checkpoint.actor_id
					&& ck.generation == e.checkpoint.generation
					&& ck.index >= e.checkpoint.index
			})
		});

		let pruned = original_len - events.len();
		if pruned > 0 {
			tracing::debug!(
				checkpoint_count = checkpoints.len(),
				pruned,
				"pruned acknowledged events"
			);
		}
	}

	async fn handle_ping(
		&self,
		ws_stream: &mut WsStream,
		ping: protocol::ToEnvoyPing,
	) -> Result<()> {
		let pong = protocol::ToRivet::ToRivetPong(protocol::ToRivetPong { ts: ping.ts });
		let encoded = utils::encode_to_rivet(pong);
		ws_stream.send(Message::Binary(encoded.into())).await?;

		Ok(())
	}

	async fn send_actor_state_update(
		&self,
		actor_id: &str,
		generation: u32,
		state: protocol::ActorState,
		ws_stream: &mut WsStream,
	) -> Result<()> {
		let event = utils::make_actor_state_update(state);

		self.send_actor_event(
			ws_stream,
			ActorEvent {
				actor_id: actor_id.to_string(),
				generation,
				event,
			},
		)
		.await?;

		Ok(())
	}

	async fn send_kv_request(&self, ws_stream: &mut WsStream, kv_request: KvRequest) -> Result<()> {
		let mut request_id = self.next_kv_request_id.lock().await;
		let id = *request_id;
		*request_id += 1;
		drop(request_id);

		// Store the response channel
		self.kv_pending_requests
			.lock()
			.await
			.insert(id, kv_request.response_tx);

		tracing::debug!(
			actor_id = ?kv_request.actor_id,
			request_id = id,
			"sending kv request"
		);

		let msg = protocol::ToRivet::ToRivetKvRequest(protocol::ToRivetKvRequest {
			actor_id: kv_request.actor_id,
			request_id: id,
			data: kv_request.data,
		});
		let encoded = utils::encode_to_rivet(msg);
		ws_stream.send(Message::Binary(encoded.into())).await?;

		Ok(())
	}

	async fn handle_kv_response(&self, response: protocol::ToEnvoyKvResponse) {
		let request_id = response.request_id;

		tracing::debug!(request_id, "received kv response");

		let response_tx = self.kv_pending_requests.lock().await.remove(&request_id);

		if let Some(tx) = response_tx {
			let _ = tx.send(response.data);
		} else {
			tracing::warn!(request_id, "received kv response for unknown request id");
		}
	}
}

impl Drop for Envoy {
	fn drop(&mut self) {
		if self.is_child_task {
			return;
		}
		// Signal shutdown when envoy is dropped
		self.shutdown.store(true, Ordering::SeqCst);
		tracing::debug!("envoy client dropped, shutdown signaled");
	}
}
