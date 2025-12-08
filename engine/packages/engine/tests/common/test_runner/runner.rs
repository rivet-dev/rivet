use super::{actor::*, protocol};
use anyhow::*;
use futures_util::{SinkExt, StreamExt};
use rivet_runner_protocol as rp;
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
	last_command_idx: Arc<Mutex<i64>>,
	next_event_idx: Arc<Mutex<u64>>,
	event_history: Arc<Mutex<Vec<rp::EventWrapper>>>,
	shutdown: Arc<AtomicBool>,

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
		let endpoint = format!("http://127.0.0.1:{}", dc.guard_port());
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
			last_command_idx: Arc::new(Mutex::new(-1)),
			next_event_idx: Arc::new(Mutex::new(0)),
			event_history: Arc::new(Mutex::new(Vec::new())),
			shutdown: Arc::new(AtomicBool::new(false)),
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
			last_command_idx: self.last_command_idx.clone(),
			next_event_idx: self.next_event_idx.clone(),
			event_history: self.event_history.clone(),
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

	/// Shutdown the runner
	pub async fn shutdown(&self) {
		tracing::info!("shutting down test runner");
		self.shutdown.store(true, Ordering::SeqCst);

		// Send shutdown signal to close ws_stream
		if let Some(tx) = self.shutdown_tx.lock().await.take() {
			let _ = tx.send(());
		}
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

	async fn build_init_message(&self) -> rp::ToServer {
		let last_command_idx = *self.last_command_idx.lock().await;

		rp::ToServer::ToServerInit(rp::ToServerInit {
			name: self.config.runner_name.clone(),
			version: self.config.version,
			total_slots: self.config.total_slots,
			last_command_idx: if last_command_idx >= 0 {
				Some(last_command_idx)
			} else {
				None
			},
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
		let init_msg = self.build_init_message().await;
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

					// Send ping
					let ping = rp::ToServer::ToServerPing(rp::ToServerPing {
						ts: chrono::Utc::now().timestamp_millis(),
					});
					let encoded = protocol::encode_to_server(ping);
					ws_stream.send(Message::Binary(encoded.into())).await?;
				}

				// Listen for events pushed from actors
				Some(actor_event) = event_rx.recv() => {
					if self.shutdown.load(Ordering::SeqCst) {
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
		let mut idx = self.next_event_idx.lock().await;
		let event_wrapper = protocol::make_event_wrapper(*idx, actor_event.event);
		*idx += 1;
		drop(idx);

		self.event_history.lock().await.push(event_wrapper.clone());

		tracing::debug!(
			actor_id = ?actor_event.actor_id,
			generation = actor_event.generation,
			event_idx = event_wrapper.index,
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

	async fn handle_init(&self, init: rp::ToClientInit, ws_stream: &mut WsStream) -> Result<()> {
		tracing::info!(
			runner_id = %init.runner_id,
			last_event_idx = ?init.last_event_idx,
			"received init from server"
		);

		*self.runner_id.lock().await = Some(init.runner_id.clone());

		// Resend unacknowledged events
		let events = self.event_history.lock().await;
		let to_resend: Vec<_> = events
			.iter()
			.filter(|e| e.index > init.last_event_idx)
			.cloned()
			.collect();
		drop(events);

		if !to_resend.is_empty() {
			tracing::info!(count = to_resend.len(), "resending unacknowledged events");
			let msg = rp::ToServer::ToServerEvents(to_resend);
			let encoded = protocol::encode_to_server(msg);
			ws_stream.send(Message::Binary(encoded.into())).await?;
		}

		Ok(())
	}

	async fn handle_commands(
		&self,
		commands: Vec<rp::CommandWrapper>,
		ws_stream: &mut WsStream,
	) -> Result<()> {
		tracing::info!(count = commands.len(), "received commands");

		for cmd_wrapper in commands {
			tracing::debug!(
				index = cmd_wrapper.index,
				command = ?cmd_wrapper.inner,
				"processing command"
			);

			match cmd_wrapper.inner {
				rp::Command::CommandStartActor(start_cmd) => {
					self.handle_start_actor(start_cmd, ws_stream).await?;
				}
				rp::Command::CommandStopActor(stop_cmd) => {
					self.handle_stop_actor(stop_cmd, ws_stream).await?;
				}
			}

			*self.last_command_idx.lock().await = cmd_wrapper.index as i64;
		}

		Ok(())
	}

	async fn handle_start_actor(
		&self,
		cmd: rp::CommandStartActor,
		ws_stream: &mut WsStream,
	) -> Result<()> {
		let actor_id = cmd.actor_id.clone();
		let generation = cmd.generation;

		tracing::info!(?actor_id, generation, name = %cmd.config.name, "starting actor");

		// Create actor config
		let mut config = ActorConfig::from(&cmd.config);
		config.actor_id = actor_id.clone();
		config.generation = generation;
		config.event_tx = self.event_tx.clone();
		config.kv_request_tx = self.kv_request_tx.clone();

		// Get factory for this actor name
		let factory = self
			.config
			.actor_factories
			.get(&cmd.config.name)
			.context(format!(
				"no factory registered for actor name: {}",
				cmd.config.name
			))?;

		// Create actor
		let mut actor = factory(config.clone());

		tracing::debug!(
			?actor_id,
			generation,
			actor_type = actor.name(),
			"created actor instance"
		);

		// Call on_start
		let start_result = actor
			.on_start(config)
			.await
			.context("actor on_start failed")?;

		tracing::debug!(
			?actor_id,
			generation,
			?start_result,
			"actor on_start completed"
		);

		// Broadcast lifecycle event
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

		// Handle start result
		match start_result {
			ActorStartResult::Running => {
				self.send_actor_state_update(
					&actor_id,
					generation,
					rp::ActorState::ActorStateRunning,
					ws_stream,
				)
				.await?;
			}
			ActorStartResult::Delay(duration) => {
				//TODO: For now, we just wait synchronously. In the future, we could
				// implement this with a channel-based event queue
				tracing::info!(
					?actor_id,
					generation,
					delay_ms = duration.as_millis(),
					"delaying before sending running state"
				);
				tokio::time::sleep(duration).await;
				self.send_actor_state_update(
					&actor_id,
					generation,
					rp::ActorState::ActorStateRunning,
					ws_stream,
				)
				.await?;
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

				// Remove actor
				self.actors.lock().await.remove(&actor_id);
			}
		}

		Ok(())
	}

	async fn handle_stop_actor(
		&self,
		cmd: rp::CommandStopActor,
		ws_stream: &mut WsStream,
	) -> Result<()> {
		let actor_id = cmd.actor_id.clone();
		let generation = cmd.generation;

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
		let last_acked_idx = ack.last_event_idx;

		let mut events = self.event_history.lock().await;
		let original_len = events.len();
		events.retain(|e| e.index > last_acked_idx);

		let pruned = original_len - events.len();
		if pruned > 0 {
			tracing::debug!(last_acked_idx, pruned, "pruned acknowledged events");
		}
	}

	async fn send_actor_state_update(
		&self,
		actor_id: &str,
		generation: u32,
		state: rp::ActorState,
		ws_stream: &mut WsStream,
	) -> Result<()> {
		let event = protocol::make_actor_state_update(actor_id, generation, state);

		let mut idx = self.next_event_idx.lock().await;
		let event_wrapper = protocol::make_event_wrapper(*idx, event);
		*idx += 1;
		drop(idx);

		self.event_history.lock().await.push(event_wrapper.clone());

		tracing::debug!(
			?actor_id,
			generation,
			event_idx = event_wrapper.index,
			"sending actor state update"
		);

		let msg = rp::ToServer::ToServerEvents(vec![event_wrapper]);
		let encoded = protocol::encode_to_server(msg);
		ws_stream.send(Message::Binary(encoded.into())).await?;

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
		// Signal shutdown when runner is dropped
		self.shutdown.store(true, Ordering::SeqCst);
		tracing::debug!("test runner dropped, shutdown signaled");
	}
}

// actor_graceful_stop_with_destroy_policy
// actor_sleep_intent
// actor_start_timeout
// crash_policy_destroy
// crash_policy_restart
// crash_policy_restart_resets_on_success
// crash_policy_sleep
// exponential_backoff_max_retries
