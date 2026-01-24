use super::{actor::*, protocol};
use anyhow::*;
use futures_util::{SinkExt, StreamExt};
use rivet_runner_protocol::mk2 as rp;
use rivet_util::Id;
use std::{
	collections::HashMap,
	str::FromStr,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
	time::Duration,
};
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};
use tokio_tungstenite::{
	connect_async,
	tungstenite::{self, Message},
};

use super::actor::KvRequest;

const RUNNER_PING_INTERVAL: Duration = Duration::from_secs(15);

type ActorFactory = Arc<dyn Fn(ActorConfig) -> Box<dyn TestActor> + Send + Sync>;
type WsStream =
	tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Lifecycle events for actors that tests can subscribe to
#[derive(Debug, Clone)]
pub enum ActorLifecycleEvent {
	Started { actor_id: String, generation: u32 },
	Stopped { actor_id: String, generation: u32 },
}

/// Configuration for test runner
#[derive(Clone)]
struct Config {
	namespace: String,
	runner_name: String,
	runner_key: String,
	version: u32,
	total_slots: u32,
	endpoint: String,
	token: String,
	actor_factories: HashMap<String, ActorFactory>,
}

/// Test runner for actor lifecycle testing
pub struct TestRunner {
	config: Config,

	// State
	pub runner_id: Arc<Mutex<Option<String>>>,
	actors: Arc<Mutex<HashMap<String, ActorState>>>,
	/// Per-actor event indices for MK2 checkpoints
	actor_event_indices: Arc<Mutex<HashMap<String, i64>>>,
	event_history: Arc<Mutex<Vec<rp::EventWrapper>>>,
	shutdown: Arc<AtomicBool>,
	is_child_task: bool,

	// Event channel for actors to push events
	event_tx: mpsc::UnboundedSender<ActorEvent>,
	event_rx: Arc<Mutex<mpsc::UnboundedReceiver<ActorEvent>>>,

	// KV request channel for actors to send KV requests
	kv_request_tx: mpsc::UnboundedSender<KvRequest>,
	kv_request_rx: Arc<Mutex<mpsc::UnboundedReceiver<KvRequest>>>,
	next_kv_request_id: Arc<Mutex<u32>>,
	kv_pending_requests: Arc<Mutex<HashMap<u32, oneshot::Sender<rp::KvResponseData>>>>,

	// Lifecycle event broadcast channel
	lifecycle_tx: broadcast::Sender<ActorLifecycleEvent>,

	// Shutdown channel
	shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

struct ActorState {
	actor_id: String,
	generation: u32,
	actor: Box<dyn TestActor>,
}

/// Builder for test runner
pub struct TestRunnerBuilder {
	namespace: String,
	runner_name: String,
	runner_key: String,
	version: u32,
	total_slots: u32,
	actor_factories: HashMap<String, ActorFactory>,
}

impl TestRunnerBuilder {
	pub fn new(namespace: &str) -> Self {
		Self {
			namespace: namespace.to_string(),
			runner_name: "test-runner".to_string(),
			runner_key: format!("key-{:012x}", rand::random::<u64>()),
			version: 1,
			total_slots: 100,
			actor_factories: HashMap::new(),
		}
	}

	pub fn with_runner_name(mut self, name: &str) -> Self {
		self.runner_name = name.to_string();
		self
	}

	pub fn with_runner_key(mut self, key: &str) -> Self {
		self.runner_key = key.to_string();
		self
	}

	pub fn with_version(mut self, version: u32) -> Self {
		self.version = version;
		self
	}

	pub fn with_total_slots(mut self, total_slots: u32) -> Self {
		self.total_slots = total_slots;
		self
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

	pub async fn build(self, dc: &super::super::TestDatacenter) -> Result<TestRunner> {
		let endpoint = format!("http://localhost:{}", dc.guard_port());
		let token = "dev".to_string();

		let config = Config {
			namespace: self.namespace,
			runner_name: self.runner_name,
			runner_key: self.runner_key,
			version: self.version,
			total_slots: self.total_slots,
			endpoint,
			token,
			actor_factories: self.actor_factories,
		};

		// Create event channel for actors to push events
		let (event_tx, event_rx) = mpsc::unbounded_channel();

		// Create KV request channel for actors to send KV requests
		let (kv_request_tx, kv_request_rx) = mpsc::unbounded_channel();

		// Create lifecycle event broadcast channel (capacity of 100 for buffering)
		let (lifecycle_tx, _) = broadcast::channel(100);

		Ok(TestRunner {
			config,
			runner_id: Arc::new(Mutex::new(None)),
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

impl TestRunner {
	/// Subscribe to actor lifecycle events
	pub fn subscribe_lifecycle_events(&self) -> broadcast::Receiver<ActorLifecycleEvent> {
		self.lifecycle_tx.subscribe()
	}

	/// Start the test runner
	pub async fn start(&self) -> Result<()> {
		tracing::info!(
			namespace = %self.config.namespace,
			runner_name = %self.config.runner_name,
			runner_key = %self.config.runner_key,
			"starting test runner"
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
		let runner = self.clone_for_task();

		tokio::spawn(async move {
			if let Err(err) = runner.run_message_loop(ws_stream, shutdown_rx).await {
				tracing::error!(?err, "test runner message loop failed");
			}
		});

		Ok(())
	}

	/// Clone the runner for passing to async tasks
	fn clone_for_task(&self) -> Self {
		Self {
			config: self.config.clone(),
			runner_id: self.runner_id.clone(),
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

	/// Wait for runner to be ready and return runner ID
	pub async fn wait_ready(&self) -> String {
		// Poll until runner_id is set
		loop {
			let runner_id = self.runner_id.lock().await;
			if let Some(id) = runner_id.as_ref() {
				// In MK2, we need to wait for the workflow to process the Init signal
				// and mark the runner as eligible for actor allocation.
				// This can take some time due to workflow processing:
				// 1. Workflow receives Init signal
				// 2. Workflow executes MarkEligible activity
				// 3. Database is updated with runner allocation index
				tokio::time::sleep(Duration::from_millis(2000)).await;
				return id.clone();
			}
			drop(runner_id);
			tokio::time::sleep(Duration::from_millis(100)).await;
		}
	}

	/// Check if runner has an actor
	pub async fn has_actor(&self, actor_id: &str) -> bool {
		let actors = self.actors.lock().await;
		actors.contains_key(actor_id)
	}

	/// Get runner's current actors
	pub async fn get_actor_ids(&self) -> Vec<Id> {
		let actors = self.actors.lock().await;

		actors
			.keys()
			.map(|x| Id::from_str(x).expect("failed to parse actor_id"))
			.collect::<Vec<_>>()
	}

	pub fn name(&self) -> &str {
		&self.config.runner_name
	}

	/// Shutdown the runner gracefully (destroys actors first)
	pub async fn shutdown(&self) {
		tracing::info!("shutting down test runner");
		self.shutdown.store(true, Ordering::SeqCst);

		// Send shutdown signal to close ws_stream
		if let Some(tx) = self.shutdown_tx.lock().await.take() {
			let _ = tx.send(());
		}
	}

	/// Crash the runner without graceful shutdown.
	/// This simulates an ungraceful disconnect where the runner stops responding
	/// without destroying its actors first. Use this to test RunnerNoResponse errors.
	pub async fn crash(&self) {
		tracing::info!("crashing test runner (ungraceful disconnect)");
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
			"{}/runners/connect?protocol_version={}&namespace={}&runner_key={}",
			ws_endpoint.trim_end_matches('/'),
			super::protocol::PROTOCOL_VERSION,
			urlencoding::encode(&self.config.namespace),
			urlencoding::encode(&self.config.runner_key)
		)
	}

	fn build_init_message(&self) -> rp::ToServer {
		// MK2 init doesn't have lastCommandIdx - uses checkpoints instead
		rp::ToServer::ToServerInit(rp::ToServerInit {
			name: self.config.runner_name.clone(),
			version: self.config.version,
			total_slots: self.config.total_slots,
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
		let encoded = protocol::encode_to_server(init_msg);
		ws_stream
			.send(Message::Binary(encoded.into()))
			.await
			.context("failed to send init message")?;

		tracing::debug!("sent init message");

		let mut ping_interval = tokio::time::interval(RUNNER_PING_INTERVAL);
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

				_ = ping_interval.tick() => {
					if self.shutdown.load(Ordering::SeqCst) {
						break;
					}

					// Send pong (MK2 uses ToServerPong instead of ToServerPing)
					let pong = rp::ToServer::ToServerPong(rp::ToServerPong {
						ts: chrono::Utc::now().timestamp_millis(),
					});
					let encoded = protocol::encode_to_server(pong);
					ws_stream.send(Message::Binary(encoded.into())).await?;
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

		tracing::info!("test runner message loop exiting");
		Ok(())
	}

	/// Send an event pushed from an actor
	async fn send_actor_event(
		&self,
		ws_stream: &mut WsStream,
		actor_event: ActorEvent,
	) -> Result<()> {
		// Get next event index for this actor (MK2 uses per-actor checkpoints)
		let mut indices = self.actor_event_indices.lock().await;
		let idx = indices.entry(actor_event.actor_id.clone()).or_insert(-1);
		*idx += 1;
		let event_idx = *idx;
		drop(indices);

		let event_wrapper = protocol::make_event_wrapper(
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

		let msg = rp::ToServer::ToServerEvents(vec![event_wrapper]);
		let encoded = protocol::encode_to_server(msg);
		ws_stream.send(Message::Binary(encoded.into())).await?;

		Ok(())
	}

	async fn handle_message(&self, ws_stream: &mut WsStream, buf: &[u8]) -> Result<()> {
		let msg = protocol::decode_to_client(buf, super::protocol::PROTOCOL_VERSION)?;

		match msg {
			rp::ToClient::ToClientInit(init) => {
				self.handle_init(init, ws_stream).await?;
			}
			rp::ToClient::ToClientCommands(commands) => {
				self.handle_commands(commands, ws_stream).await?;
			}
			rp::ToClient::ToClientAckEvents(ack) => {
				self.handle_ack_events(ack).await;
			}
			rp::ToClient::ToClientKvResponse(response) => {
				self.handle_kv_response(response).await;
			}
			_ => {
				tracing::debug!(?msg, "ignoring message type");
			}
		}

		Ok(())
	}

	async fn handle_init(&self, init: rp::ToClientInit, _ws_stream: &mut WsStream) -> Result<()> {
		tracing::info!(
			runner_id = %init.runner_id,
			"received init from server"
		);

		*self.runner_id.lock().await = Some(init.runner_id.clone());

		// MK2 doesn't have lastEventIdx in init - events are acked via checkpoints
		// For simplicity, we don't resend events on reconnect in the test runner

		Ok(())
	}

	async fn handle_commands(
		&self,
		commands: Vec<rp::CommandWrapper>,
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
				rp::Command::CommandStartActor(start_cmd) => {
					self.handle_start_actor(
						checkpoint.actor_id.clone(),
						checkpoint.generation,
						start_cmd,
						ws_stream,
					)
					.await?;
				}
				rp::Command::CommandStopActor => {
					// MK2 CommandStopActor is void - actor info is in checkpoint
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
		cmd: rp::CommandStartActor,
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
		let runner = self.clone_for_task();
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
				Result::Ok(result) => result,
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

			runner
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
				let event = protocol::make_actor_state_update(rp::ActorState::ActorStateRunning);
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
						protocol::make_actor_state_update(rp::ActorState::ActorStateRunning);
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
				let event = protocol::make_actor_state_update(rp::ActorState::ActorStateStopped(
					rp::ActorStateStopped {
						code: if code == 0 {
							rp::StopCode::Ok
						} else {
							rp::StopCode::Error
						},
						message: Some(message),
					},
				));
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
					rp::ActorState::ActorStateStopped(rp::ActorStateStopped {
						code: rp::StopCode::Ok,
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
					rp::ActorState::ActorStateStopped(rp::ActorStateStopped {
						code: rp::StopCode::Ok,
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
					rp::ActorState::ActorStateStopped(rp::ActorStateStopped {
						code: if code == 0 {
							rp::StopCode::Ok
						} else {
							rp::StopCode::Error
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

	async fn handle_ack_events(&self, ack: rp::ToClientAckEvents) {
		// MK2 uses per-actor checkpoints for acknowledgments
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

	async fn send_actor_state_update(
		&self,
		actor_id: &str,
		generation: u32,
		state: rp::ActorState,
		ws_stream: &mut WsStream,
	) -> Result<()> {
		let event = protocol::make_actor_state_update(state);

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

		let msg = rp::ToServer::ToServerKvRequest(rp::ToServerKvRequest {
			actor_id: kv_request.actor_id,
			request_id: id,
			data: kv_request.data,
		});
		let encoded = protocol::encode_to_server(msg);
		ws_stream.send(Message::Binary(encoded.into())).await?;

		Ok(())
	}

	async fn handle_kv_response(&self, response: rp::ToClientKvResponse) {
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

impl Drop for TestRunner {
	fn drop(&mut self) {
		if self.is_child_task {
			return;
		}
		// Signal shutdown when runner is dropped
		self.shutdown.store(true, Ordering::SeqCst);
		tracing::debug!("test runner dropped, shutdown signaled");
	}
}
