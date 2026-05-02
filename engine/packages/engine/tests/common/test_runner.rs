//! Legacy Pegboard Runner test client.
//!
//! This helper intentionally speaks `rivet-runner-protocol` to `/runners/connect`.
//! Envoy tests must use `test_envoy.rs`.

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use rivet_runner_protocol::{self as rp, PROTOCOL_MK2_VERSION, mk2, versioned};
use std::{
	collections::HashMap,
	future::Future,
	pin::Pin,
	sync::{
		Arc, Mutex,
		atomic::{AtomicBool, AtomicU32, Ordering},
	},
	time::Duration,
};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use vbare::OwnedVersionedData;

pub use rivet_runner_protocol as protocol_types;
pub use rivet_runner_protocol::PROTOCOL_MK2_VERSION as PROTOCOL_VERSION;

type WsStream =
	tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
type ActorFactory = Arc<dyn Fn(ActorConfig) -> Box<dyn TestActor> + Send + Sync>;

pub type TestRunner = Runner;
pub type RunnerBuilderLegacy = RunnerBuilder;

#[derive(Clone)]
pub struct ActorConfig {
	pub actor_id: String,
	pub generation: u32,
	pub name: String,
	pub key: Option<String>,
	pub create_ts: i64,
	pub input: Option<Vec<u8>>,
	pub(crate) event_tx: mpsc::UnboundedSender<ActorEvent>,
	pub(crate) kv_request_tx: mpsc::UnboundedSender<KvRequest>,
}

impl ActorConfig {
	fn new(
		config: &mk2::ActorConfig,
		actor_id: String,
		generation: u32,
		event_tx: mpsc::UnboundedSender<ActorEvent>,
		kv_request_tx: mpsc::UnboundedSender<KvRequest>,
	) -> Self {
		Self {
			actor_id,
			generation,
			name: config.name.clone(),
			key: config.key.clone(),
			create_ts: config.create_ts,
			input: config.input.clone(),
			event_tx,
			kv_request_tx,
		}
	}

	pub fn send_sleep_intent(&self) {
		self.send_event(mk2::Event::EventActorIntent(mk2::EventActorIntent {
			intent: mk2::ActorIntent::ActorIntentSleep,
		}));
	}

	pub fn send_stop_intent(&self) {
		self.send_event(mk2::Event::EventActorIntent(mk2::EventActorIntent {
			intent: mk2::ActorIntent::ActorIntentStop,
		}));
	}

	pub fn send_set_alarm(&self, alarm_ts: i64) {
		self.send_event(mk2::Event::EventActorSetAlarm(mk2::EventActorSetAlarm {
			alarm_ts: Some(alarm_ts),
		}));
	}

	pub fn send_clear_alarm(&self) {
		self.send_event(mk2::Event::EventActorSetAlarm(mk2::EventActorSetAlarm {
			alarm_ts: None,
		}));
	}

	fn send_event(&self, event: mk2::Event) {
		let _ = self.event_tx.send(ActorEvent {
			actor_id: self.actor_id.clone(),
			generation: self.generation,
			event,
		});
	}

	pub async fn send_kv_get(&self, keys: Vec<Vec<u8>>) -> Result<mk2::KvGetResponse> {
		match self
			.send_kv(mk2::KvRequestData::KvGetRequest(mk2::KvGetRequest { keys }))
			.await?
		{
			mk2::KvResponseData::KvGetResponse(res) => Ok(res),
			mk2::KvResponseData::KvErrorResponse(err) => bail!("KV get failed: {}", err.message),
			_ => bail!("unexpected response type for KV get"),
		}
	}

	pub async fn send_kv_list(
		&self,
		query: mk2::KvListQuery,
		reverse: Option<bool>,
		limit: Option<u64>,
	) -> Result<mk2::KvListResponse> {
		match self
			.send_kv(mk2::KvRequestData::KvListRequest(mk2::KvListRequest {
				query,
				reverse,
				limit,
			}))
			.await?
		{
			mk2::KvResponseData::KvListResponse(res) => Ok(res),
			mk2::KvResponseData::KvErrorResponse(err) => bail!("KV list failed: {}", err.message),
			_ => bail!("unexpected response type for KV list"),
		}
	}

	pub async fn send_kv_put(&self, keys: Vec<Vec<u8>>, values: Vec<Vec<u8>>) -> Result<()> {
		match self
			.send_kv(mk2::KvRequestData::KvPutRequest(mk2::KvPutRequest {
				keys,
				values,
			}))
			.await?
		{
			mk2::KvResponseData::KvPutResponse => Ok(()),
			mk2::KvResponseData::KvErrorResponse(err) => bail!("KV put failed: {}", err.message),
			_ => bail!("unexpected response type for KV put"),
		}
	}

	pub async fn send_kv_delete(&self, keys: Vec<Vec<u8>>) -> Result<()> {
		match self
			.send_kv(mk2::KvRequestData::KvDeleteRequest(mk2::KvDeleteRequest {
				keys,
			}))
			.await?
		{
			mk2::KvResponseData::KvDeleteResponse => Ok(()),
			mk2::KvResponseData::KvErrorResponse(err) => bail!("KV delete failed: {}", err.message),
			_ => bail!("unexpected response type for KV delete"),
		}
	}

	pub async fn send_kv_delete_range(&self, start: Vec<u8>, end: Vec<u8>) -> Result<()> {
		match self
			.send_kv(mk2::KvRequestData::KvDeleteRangeRequest(
				mk2::KvDeleteRangeRequest { start, end },
			))
			.await?
		{
			mk2::KvResponseData::KvDeleteResponse => Ok(()),
			mk2::KvResponseData::KvErrorResponse(err) => {
				bail!("KV delete range failed: {}", err.message)
			}
			_ => bail!("unexpected response type for KV delete range"),
		}
	}

	pub async fn send_kv_drop(&self) -> Result<()> {
		match self.send_kv(mk2::KvRequestData::KvDropRequest).await? {
			mk2::KvResponseData::KvDropResponse => Ok(()),
			mk2::KvResponseData::KvErrorResponse(err) => bail!("KV drop failed: {}", err.message),
			_ => bail!("unexpected response type for KV drop"),
		}
	}

	async fn send_kv(&self, data: mk2::KvRequestData) -> Result<mk2::KvResponseData> {
		let (response_tx, response_rx) = oneshot::channel();
		self.kv_request_tx
			.send(KvRequest {
				actor_id: self.actor_id.clone(),
				data,
				response_tx,
			})
			.context("failed to send KV request")?;
		response_rx.await.context("KV response channel closed")
	}
}

#[derive(Debug, Clone)]
pub enum ActorStartResult {
	Running,
	Delay(Duration),
	Timeout,
	Crash { code: i32, message: String },
}

#[derive(Debug, Clone)]
pub enum ActorStopResult {
	Success,
	Delay(Duration),
	Crash { code: i32, message: String },
}

#[async_trait]
pub trait Actor: Send + Sync {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult>;
	async fn on_stop(&mut self) -> Result<ActorStopResult>;

	fn name(&self) -> &str {
		"TestActor"
	}
}
pub use Actor as TestActor;

#[derive(Debug, Clone)]
pub struct ActorEvent {
	pub actor_id: String,
	pub generation: u32,
	pub event: mk2::Event,
}

pub struct KvRequest {
	pub actor_id: String,
	pub data: mk2::KvRequestData,
	pub response_tx: oneshot::Sender<mk2::KvResponseData>,
}

#[derive(Debug, Clone)]
pub enum ActorLifecycleEvent {
	Started { actor_id: String, generation: u32 },
	Stopped { actor_id: String, generation: u32 },
}

#[derive(Clone)]
pub struct RunnerConfig {
	endpoint: String,
	token: String,
	namespace: String,
	runner_name: String,
	runner_key: String,
	version: u32,
	total_slots: u32,
}

impl RunnerConfig {
	pub fn builder() -> RunnerConfigBuilder {
		RunnerConfigBuilder::default()
	}
}

#[derive(Default)]
pub struct RunnerConfigBuilder {
	endpoint: Option<String>,
	token: Option<String>,
	namespace: Option<String>,
	runner_name: Option<String>,
	runner_key: Option<String>,
	version: Option<u32>,
	total_slots: Option<u32>,
}

impl RunnerConfigBuilder {
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

	pub fn runner_name(mut self, runner_name: impl Into<String>) -> Self {
		self.runner_name = Some(runner_name.into());
		self
	}

	pub fn runner_key(mut self, runner_key: impl Into<String>) -> Self {
		self.runner_key = Some(runner_key.into());
		self
	}

	pub fn version(mut self, version: u32) -> Self {
		self.version = Some(version);
		self
	}

	pub fn total_slots(mut self, total_slots: u32) -> Self {
		self.total_slots = Some(total_slots);
		self
	}

	pub fn build(self) -> Result<RunnerConfig> {
		Ok(RunnerConfig {
			endpoint: self.endpoint.context("endpoint is required")?,
			token: self.token.unwrap_or_else(|| "dev".to_string()),
			namespace: self.namespace.context("namespace is required")?,
			runner_name: self
				.runner_name
				.unwrap_or_else(|| "test-runner".to_string()),
			runner_key: self
				.runner_key
				.unwrap_or_else(|| format!("key-{:012x}", rand::random::<u64>())),
			version: self.version.unwrap_or(1),
			total_slots: self.total_slots.unwrap_or(100),
		})
	}
}

pub struct RunnerBuilder {
	config: RunnerConfig,
	actor_factories: HashMap<String, ActorFactory>,
}

impl RunnerBuilder {
	pub fn new(config: RunnerConfig) -> Self {
		Self {
			config,
			actor_factories: HashMap::new(),
		}
	}

	pub fn with_actor_behavior<F>(mut self, actor_name: &str, factory: F) -> Self
	where
		F: Fn(ActorConfig) -> Box<dyn TestActor> + Send + Sync + 'static,
	{
		self.actor_factories
			.insert(actor_name.to_string(), Arc::new(factory));
		self
	}

	pub fn build(self) -> Result<Runner> {
		let (event_tx, event_rx) = mpsc::unbounded_channel();
		let (kv_request_tx, kv_request_rx) = mpsc::unbounded_channel();
		let (lifecycle_tx, _) = broadcast::channel(100);
		let (control_tx, control_rx) = mpsc::unbounded_channel();

		Ok(Runner {
			config: self.config,
			actor_factories: self.actor_factories,
			runner_id: Arc::new(tokio::sync::Mutex::new(None)),
			ready: Arc::new(AtomicBool::new(false)),
			actors: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
			event_indices: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
			pending_kv: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
			next_kv_request_id: Arc::new(tokio::sync::Mutex::new(1)),
			event_tx,
			event_rx: Arc::new(tokio::sync::Mutex::new(Some(event_rx))),
			kv_request_tx,
			kv_request_rx: Arc::new(tokio::sync::Mutex::new(Some(kv_request_rx))),
			lifecycle_tx,
			control_tx,
			control_rx: Arc::new(tokio::sync::Mutex::new(Some(control_rx))),
		})
	}
}

struct ActorState {
	generation: u32,
	actor: Box<dyn TestActor>,
}

#[derive(Clone, Copy)]
enum Control {
	Shutdown,
	Crash,
}

pub struct Runner {
	config: RunnerConfig,
	actor_factories: HashMap<String, ActorFactory>,
	runner_id: Arc<tokio::sync::Mutex<Option<String>>>,
	ready: Arc<AtomicBool>,
	actors: Arc<tokio::sync::Mutex<HashMap<String, ActorState>>>,
	event_indices: Arc<tokio::sync::Mutex<HashMap<(String, u32), i64>>>,
	pending_kv: Arc<tokio::sync::Mutex<HashMap<u32, oneshot::Sender<mk2::KvResponseData>>>>,
	next_kv_request_id: Arc<tokio::sync::Mutex<u32>>,
	event_tx: mpsc::UnboundedSender<ActorEvent>,
	event_rx: Arc<tokio::sync::Mutex<Option<mpsc::UnboundedReceiver<ActorEvent>>>>,
	kv_request_tx: mpsc::UnboundedSender<KvRequest>,
	kv_request_rx: Arc<tokio::sync::Mutex<Option<mpsc::UnboundedReceiver<KvRequest>>>>,
	lifecycle_tx: broadcast::Sender<ActorLifecycleEvent>,
	control_tx: mpsc::UnboundedSender<Control>,
	control_rx: Arc<tokio::sync::Mutex<Option<mpsc::UnboundedReceiver<Control>>>>,
}

impl Runner {
	pub async fn start(&self) -> Result<()> {
		let mut event_rx = self
			.event_rx
			.lock()
			.await
			.take()
			.context("runner already started")?;
		let mut kv_request_rx = self
			.kv_request_rx
			.lock()
			.await
			.take()
			.context("runner already started")?;
		let mut control_rx = self
			.control_rx
			.lock()
			.await
			.take()
			.context("runner already started")?;

		let ws_url = self.build_ws_url();
		let token_protocol = format!("rivet_token.{}", self.config.token);

		use tokio_tungstenite::tungstenite::client::IntoClientRequest;
		let mut request = ws_url.into_client_request()?;
		request.headers_mut().insert(
			"Sec-WebSocket-Protocol",
			format!("rivet, {}", token_protocol).parse()?,
		);

		let (mut ws_stream, _) = connect_async(request)
			.await
			.context("failed to connect to runner WebSocket")?;

		ws_stream
			.send(Message::Binary(
				self.encode_to_server(self.build_init())?.into(),
			))
			.await
			.context("failed to send runner init")?;

		let runner = self.clone_for_task();
		tokio::spawn(async move {
			if let Err(err) = runner
				.run_message_loop(
					&mut ws_stream,
					&mut event_rx,
					&mut kv_request_rx,
					&mut control_rx,
				)
				.await
			{
				tracing::error!(?err, "runner message loop failed");
			}
		});

		Ok(())
	}

	fn clone_for_task(&self) -> Self {
		Self {
			config: self.config.clone(),
			actor_factories: self.actor_factories.clone(),
			runner_id: self.runner_id.clone(),
			ready: self.ready.clone(),
			actors: self.actors.clone(),
			event_indices: self.event_indices.clone(),
			pending_kv: self.pending_kv.clone(),
			next_kv_request_id: self.next_kv_request_id.clone(),
			event_tx: self.event_tx.clone(),
			event_rx: self.event_rx.clone(),
			kv_request_tx: self.kv_request_tx.clone(),
			kv_request_rx: self.kv_request_rx.clone(),
			lifecycle_tx: self.lifecycle_tx.clone(),
			control_tx: self.control_tx.clone(),
			control_rx: self.control_rx.clone(),
		}
	}

	async fn run_message_loop(
		self,
		ws_stream: &mut WsStream,
		event_rx: &mut mpsc::UnboundedReceiver<ActorEvent>,
		kv_request_rx: &mut mpsc::UnboundedReceiver<KvRequest>,
		control_rx: &mut mpsc::UnboundedReceiver<Control>,
	) -> Result<()> {
		loop {
			tokio::select! {
				Some(control) = control_rx.recv() => {
					match control {
						Control::Shutdown => {
							let _ = ws_stream.send(Message::Binary(self.encode_to_server(mk2::ToServer::ToServerStopping)?.into())).await;
							let _ = ws_stream.close(None).await;
						}
						Control::Crash => {
							let _ = ws_stream.close(None).await;
						}
					}
					break;
				}
				Some(event) = event_rx.recv() => {
					self.send_actor_event(ws_stream, event).await?;
				}
				Some(req) = kv_request_rx.recv() => {
					self.send_kv_request(ws_stream, req).await?;
				}
				msg = ws_stream.next() => {
					match msg {
						Some(Ok(Message::Binary(buf))) => self.handle_message(ws_stream, &buf).await?,
						Some(Ok(Message::Close(_))) | None => break,
						Some(Err(err)) => return Err(err.into()),
						_ => {}
					}
				}
			}
		}
		Ok(())
	}

	async fn handle_message(&self, ws_stream: &mut WsStream, buf: &[u8]) -> Result<()> {
		let msg = versioned::ToClientMk2::deserialize(buf, PROTOCOL_MK2_VERSION)?;
		match msg {
			mk2::ToClient::ToClientInit(init) => {
				*self.runner_id.lock().await = Some(init.runner_id);
				self.ready.store(true, Ordering::SeqCst);
			}
			mk2::ToClient::ToClientCommands(commands) => {
				self.handle_commands(ws_stream, commands).await?;
			}
			mk2::ToClient::ToClientAckEvents(_) => {}
			mk2::ToClient::ToClientKvResponse(response) => {
				if let Some(tx) = self.pending_kv.lock().await.remove(&response.request_id) {
					let _ = tx.send(response.data);
				}
			}
			mk2::ToClient::ToClientTunnelMessage(message) => {
				self.handle_tunnel_message(ws_stream, message).await?;
			}
			mk2::ToClient::ToClientPing(ping) => {
				ws_stream
					.send(Message::Binary(
						self.encode_to_server(mk2::ToServer::ToServerPong(mk2::ToServerPong {
							ts: ping.ts,
						}))?
						.into(),
					))
					.await?;
			}
		}
		Ok(())
	}

	async fn handle_commands(
		&self,
		ws_stream: &mut WsStream,
		commands: Vec<mk2::CommandWrapper>,
	) -> Result<()> {
		let mut checkpoints = Vec::new();
		for command in commands {
			let checkpoint = command.checkpoint.clone();
			match command.inner {
				mk2::Command::CommandStartActor(start) => {
					self.handle_start_actor(checkpoint.clone(), start).await?;
				}
				mk2::Command::CommandStopActor => {
					self.handle_stop_actor(checkpoint.clone()).await?;
				}
			}
			checkpoints.push(checkpoint);
		}

		ws_stream
			.send(Message::Binary(
				self.encode_to_server(mk2::ToServer::ToServerAckCommands(
					mk2::ToServerAckCommands {
						last_command_checkpoints: checkpoints,
					},
				))?
				.into(),
			))
			.await?;

		Ok(())
	}

	async fn handle_start_actor(
		&self,
		checkpoint: mk2::ActorCheckpoint,
		start: mk2::CommandStartActor,
	) -> Result<()> {
		let factory = self
			.actor_factories
			.get(&start.config.name)
			.cloned()
			.unwrap_or_else(|| Arc::new(|_| Box::new(EchoActor::new())));
		let (actor_event_tx, actor_event_rx) = mpsc::unbounded_channel();
		let config = ActorConfig::new(
			&start.config,
			checkpoint.actor_id.clone(),
			checkpoint.generation,
			actor_event_tx,
			self.kv_request_tx.clone(),
		);
		let runner = self.clone_for_task();

		tokio::spawn(async move {
			let mut actor = factory(config.clone());
			let result = actor.on_start(config).await;
			match result {
				Ok(start_result) => {
					runner
						.handle_actor_start_result(
							checkpoint.actor_id,
							checkpoint.generation,
							actor,
							start_result,
							actor_event_rx,
						)
						.await;
				}
				Err(err) => {
					tracing::error!(?err, "actor on_start failed");
				}
			}
		});

		Ok(())
	}

	async fn handle_actor_start_result(
		&self,
		actor_id: String,
		generation: u32,
		actor: Box<dyn TestActor>,
		start_result: ActorStartResult,
		mut actor_event_rx: mpsc::UnboundedReceiver<ActorEvent>,
	) {
		let _ = self.lifecycle_tx.send(ActorLifecycleEvent::Started {
			actor_id: actor_id.clone(),
			generation,
		});
		self.actors
			.lock()
			.await
			.insert(actor_id.clone(), ActorState { generation, actor });

		match start_result {
			ActorStartResult::Running => {
				self.send_state_running(actor_id, generation);
				self.forward_actor_events(actor_event_rx);
			}
			ActorStartResult::Delay(duration) => {
				let event_tx = self.event_tx.clone();
				tokio::spawn(async move {
					tokio::time::sleep(duration).await;
					let _ = event_tx.send(ActorEvent {
						actor_id,
						generation,
						event: mk2::Event::EventActorStateUpdate(mk2::EventActorStateUpdate {
							state: mk2::ActorState::ActorStateRunning,
						}),
					});
					forward_actor_events_to(event_tx, actor_event_rx);
				});
			}
			ActorStartResult::Timeout => {}
			ActorStartResult::Crash { code, message } => {
				drain_actor_events_to(self.event_tx.clone(), &mut actor_event_rx);
				self.send_state_stopped(actor_id.clone(), generation, code, Some(message));
				self.actors.lock().await.remove(&actor_id);
			}
		}
	}

	fn forward_actor_events(&self, actor_event_rx: mpsc::UnboundedReceiver<ActorEvent>) {
		forward_actor_events_to(self.event_tx.clone(), actor_event_rx);
	}

	async fn handle_stop_actor(&self, checkpoint: mk2::ActorCheckpoint) -> Result<()> {
		let mut actors = self.actors.lock().await;
		let Some(mut actor_state) = actors.remove(&checkpoint.actor_id) else {
			return Ok(());
		};
		let stop_result = actor_state.actor.on_stop().await?;
		drop(actors);

		let _ = self.lifecycle_tx.send(ActorLifecycleEvent::Stopped {
			actor_id: checkpoint.actor_id.clone(),
			generation: checkpoint.generation,
		});

		match stop_result {
			ActorStopResult::Success => {
				self.send_state_stopped(checkpoint.actor_id, checkpoint.generation, 0, None)
			}
			ActorStopResult::Delay(duration) => {
				let actor_id = checkpoint.actor_id;
				let generation = checkpoint.generation;
				let event_tx = self.event_tx.clone();
				tokio::spawn(async move {
					tokio::time::sleep(duration).await;
					let _ = event_tx.send(ActorEvent {
						actor_id,
						generation,
						event: stopped_event(0, None),
					});
				});
			}
			ActorStopResult::Crash { code, message } => self.send_state_stopped(
				checkpoint.actor_id,
				checkpoint.generation,
				code,
				Some(message),
			),
		}

		Ok(())
	}

	fn send_state_running(&self, actor_id: String, generation: u32) {
		let _ = self.event_tx.send(ActorEvent {
			actor_id,
			generation,
			event: mk2::Event::EventActorStateUpdate(mk2::EventActorStateUpdate {
				state: mk2::ActorState::ActorStateRunning,
			}),
		});
	}

	fn send_state_stopped(
		&self,
		actor_id: String,
		generation: u32,
		code: i32,
		message: Option<String>,
	) {
		let _ = self.event_tx.send(ActorEvent {
			actor_id,
			generation,
			event: stopped_event(code, message),
		});
	}

	async fn send_actor_event(
		&self,
		ws_stream: &mut WsStream,
		actor_event: ActorEvent,
	) -> Result<()> {
		let mut indices = self.event_indices.lock().await;
		let index = indices
			.entry((actor_event.actor_id.clone(), actor_event.generation))
			.and_modify(|idx| *idx += 1)
			.or_insert(0);
		let event = mk2::EventWrapper {
			checkpoint: mk2::ActorCheckpoint {
				actor_id: actor_event.actor_id,
				generation: actor_event.generation,
				index: *index,
			},
			inner: actor_event.event,
		};
		drop(indices);

		ws_stream
			.send(Message::Binary(
				self.encode_to_server(mk2::ToServer::ToServerEvents(vec![event]))?
					.into(),
			))
			.await?;
		Ok(())
	}

	async fn send_kv_request(&self, ws_stream: &mut WsStream, req: KvRequest) -> Result<()> {
		let mut next_id = self.next_kv_request_id.lock().await;
		let request_id = *next_id;
		*next_id += 1;
		drop(next_id);

		self.pending_kv
			.lock()
			.await
			.insert(request_id, req.response_tx);
		ws_stream
			.send(Message::Binary(
				self.encode_to_server(mk2::ToServer::ToServerKvRequest(mk2::ToServerKvRequest {
					actor_id: req.actor_id,
					request_id,
					data: req.data,
				}))?
				.into(),
			))
			.await?;
		Ok(())
	}

	async fn handle_tunnel_message(
		&self,
		ws_stream: &mut WsStream,
		message: mk2::ToClientTunnelMessage,
	) -> Result<()> {
		let response = match message.message_kind {
			mk2::ToClientTunnelMessageKind::ToClientRequestStart(req) => {
				let (status, body) = if req.path == "/ping" && self.has_actor(&req.actor_id).await {
					(
						200,
						serde_json::to_vec(&serde_json::json!({
							"actorId": req.actor_id,
							"status": "ok",
							"timestamp": rivet_util::timestamp::now(),
						}))?,
					)
				} else {
					(404, b"not found".to_vec())
				};
				Some(mk2::ToServerTunnelMessageKind::ToServerResponseStart(
					mk2::ToServerResponseStart {
						status,
						headers: HashMap::new().into(),
						body: Some(body),
						stream: false,
					},
				))
			}
			_ => None,
		};

		if let Some(message_kind) = response {
			ws_stream
				.send(Message::Binary(
					self.encode_to_server(mk2::ToServer::ToServerTunnelMessage(
						mk2::ToServerTunnelMessage {
							message_id: message.message_id,
							message_kind,
						},
					))?
					.into(),
				))
				.await?;
		}
		Ok(())
	}

	fn build_ws_url(&self) -> String {
		let endpoint = self.config.endpoint.replace("http://", "ws://");
		format!(
			"{}/runners/connect?protocol_version={}&namespace={}&runner_key={}",
			endpoint.trim_end_matches('/'),
			PROTOCOL_MK2_VERSION,
			urlencoding::encode(&self.config.namespace),
			urlencoding::encode(&self.config.runner_key),
		)
	}

	fn build_init(&self) -> mk2::ToServer {
		mk2::ToServer::ToServerInit(mk2::ToServerInit {
			name: self.config.runner_name.clone(),
			version: self.config.version,
			total_slots: self.config.total_slots,
			prepopulate_actor_names: None,
			metadata: Some(
				serde_json::json!({
					"runner_key": self.config.runner_key,
					"total_slots": self.config.total_slots,
				})
				.to_string(),
			),
		})
	}

	fn encode_to_server(&self, msg: mk2::ToServer) -> Result<Vec<u8>> {
		versioned::ToServerMk2::wrap_latest(msg)
			.serialize(PROTOCOL_MK2_VERSION)
			.map_err(Into::into)
	}

	pub async fn wait_ready(&self) -> String {
		while !self.ready.load(Ordering::SeqCst) {
			tokio::time::sleep(Duration::from_millis(25)).await;
		}
		self.runner_id
			.lock()
			.await
			.clone()
			.expect("runner id should be set when ready")
	}

	pub async fn has_actor(&self, actor_id: &str) -> bool {
		self.actors.lock().await.contains_key(actor_id)
	}

	pub async fn get_actor_ids(&self) -> Vec<String> {
		self.actors.lock().await.keys().cloned().collect()
	}

	pub fn name(&self) -> &str {
		&self.config.runner_name
	}

	pub fn subscribe_lifecycle_events(&self) -> broadcast::Receiver<ActorLifecycleEvent> {
		self.lifecycle_tx.subscribe()
	}

	pub async fn shutdown(&self) {
		let _ = self.control_tx.send(Control::Shutdown);
		self.actors.lock().await.clear();
	}

	pub async fn crash(&self) {
		let _ = self.control_tx.send(Control::Crash);
		self.actors.lock().await.clear();
	}
}

fn stopped_event(code: i32, message: Option<String>) -> mk2::Event {
	mk2::Event::EventActorStateUpdate(mk2::EventActorStateUpdate {
		state: mk2::ActorState::ActorStateStopped(mk2::ActorStateStopped {
			code: if code == 0 {
				mk2::StopCode::Ok
			} else {
				mk2::StopCode::Error
			},
			message,
		}),
	})
}

fn forward_actor_events_to(
	event_tx: mpsc::UnboundedSender<ActorEvent>,
	mut actor_event_rx: mpsc::UnboundedReceiver<ActorEvent>,
) {
	drain_actor_events_to(event_tx.clone(), &mut actor_event_rx);

	tokio::spawn(async move {
		while let Some(event) = actor_event_rx.recv().await {
			if event_tx.send(event).is_err() {
				break;
			}
		}
	});
}

fn drain_actor_events_to(
	event_tx: mpsc::UnboundedSender<ActorEvent>,
	actor_event_rx: &mut mpsc::UnboundedReceiver<ActorEvent>,
) {
	while let Ok(event) = actor_event_rx.try_recv() {
		let _ = event_tx.send(event);
	}
}

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

	pub fn with_actor_behavior<F>(mut self, actor_name: &str, factory: F) -> Self
	where
		F: Fn(ActorConfig) -> Box<dyn TestActor> + Send + Sync + 'static,
	{
		self.actor_factories
			.insert(actor_name.to_string(), Arc::new(factory));
		self
	}

	pub async fn build(self, dc: &super::TestDatacenter) -> Result<Runner> {
		let config = RunnerConfig::builder()
			.endpoint(format!("http://127.0.0.1:{}", dc.guard_port()))
			.token("dev")
			.namespace(&self.namespace)
			.runner_name(&self.runner_name)
			.runner_key(&self.runner_key)
			.version(self.version)
			.total_slots(self.total_slots)
			.build()?;

		let mut builder = RunnerBuilder::new(config);
		for (name, factory) in self.actor_factories {
			builder = builder.with_actor_behavior(&name, move |config| factory(config));
		}
		builder.build()
	}
}

pub struct EchoActor;

impl EchoActor {
	pub fn new() -> Self {
		Self
	}
}

impl Default for EchoActor {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl TestActor for EchoActor {
	async fn on_start(&mut self, _config: ActorConfig) -> Result<ActorStartResult> {
		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}
}

pub struct TimeoutActor;

impl TimeoutActor {
	pub fn new() -> Self {
		Self
	}
}

#[async_trait]
impl TestActor for TimeoutActor {
	async fn on_start(&mut self, _config: ActorConfig) -> Result<ActorStartResult> {
		Ok(ActorStartResult::Timeout)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}
}

pub struct DelayedStartActor {
	pub delay: Duration,
}

impl DelayedStartActor {
	pub fn new(delay: Duration) -> Self {
		Self { delay }
	}
}

#[async_trait]
impl TestActor for DelayedStartActor {
	async fn on_start(&mut self, _config: ActorConfig) -> Result<ActorStartResult> {
		Ok(ActorStartResult::Delay(self.delay))
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}
}

pub struct CrashOnStartActor {
	exit_code: i32,
	notify_tx: Option<Arc<Mutex<Option<oneshot::Sender<()>>>>>,
}

impl CrashOnStartActor {
	pub fn new(exit_code: i32) -> Self {
		Self {
			exit_code,
			notify_tx: None,
		}
	}

	pub fn new_with_notify(
		exit_code: i32,
		notify_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
	) -> Self {
		Self {
			exit_code,
			notify_tx: Some(notify_tx),
		}
	}
}

#[async_trait]
impl TestActor for CrashOnStartActor {
	async fn on_start(&mut self, _config: ActorConfig) -> Result<ActorStartResult> {
		if let Some(notify_tx) = &self.notify_tx {
			if let Some(tx) = notify_tx.lock().expect("notify lock").take() {
				let _ = tx.send(());
			}
		}
		Ok(ActorStartResult::Crash {
			code: self.exit_code,
			message: format!("crash on start with code {}", self.exit_code),
		})
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}
}

pub struct CountingCrashActor {
	crash_count: Arc<AtomicU32>,
}

impl CountingCrashActor {
	pub fn new(crash_count: Arc<AtomicU32>) -> Self {
		Self { crash_count }
	}
}

#[async_trait]
impl TestActor for CountingCrashActor {
	async fn on_start(&mut self, _config: ActorConfig) -> Result<ActorStartResult> {
		let count = self.crash_count.fetch_add(1, Ordering::SeqCst) + 1;
		Ok(ActorStartResult::Crash {
			code: 1,
			message: format!("crash #{count}"),
		})
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}
}

pub struct CrashNTimesThenSucceedActor {
	crash_count: Arc<Mutex<usize>>,
	max_crashes: usize,
}

impl CrashNTimesThenSucceedActor {
	pub fn new(max_crashes: usize, crash_count: Arc<Mutex<usize>>) -> Self {
		Self {
			crash_count,
			max_crashes,
		}
	}
}

#[async_trait]
impl TestActor for CrashNTimesThenSucceedActor {
	async fn on_start(&mut self, _config: ActorConfig) -> Result<ActorStartResult> {
		let mut count = self.crash_count.lock().expect("crash count lock");
		if *count < self.max_crashes {
			*count += 1;
			Ok(ActorStartResult::Crash {
				code: 1,
				message: format!("crash {} of {}", *count, self.max_crashes),
			})
		} else {
			Ok(ActorStartResult::Running)
		}
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}
}

pub struct NotifyOnStartActor {
	notify_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl NotifyOnStartActor {
	pub fn new(notify_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for NotifyOnStartActor {
	async fn on_start(&mut self, _config: ActorConfig) -> Result<ActorStartResult> {
		if let Some(tx) = self.notify_tx.lock().expect("notify lock").take() {
			let _ = tx.send(());
		}
		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}
}

pub struct VerifyInputActor {
	expected_input: Vec<u8>,
}

impl VerifyInputActor {
	pub fn new(expected_input: Vec<u8>) -> Self {
		Self { expected_input }
	}
}

#[async_trait]
impl TestActor for VerifyInputActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		if config.input.as_ref() == Some(&self.expected_input) {
			Ok(ActorStartResult::Running)
		} else {
			Ok(ActorStartResult::Crash {
				code: 1,
				message: "input mismatch".to_string(),
			})
		}
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}
}

pub struct SleepImmediatelyActor {
	notify_tx: Option<Arc<Mutex<Option<oneshot::Sender<()>>>>>,
}

impl SleepImmediatelyActor {
	pub fn new() -> Self {
		Self { notify_tx: None }
	}

	pub fn new_with_notify(notify_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>) -> Self {
		Self {
			notify_tx: Some(notify_tx),
		}
	}
}

#[async_trait]
impl TestActor for SleepImmediatelyActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		config.send_sleep_intent();
		if let Some(notify_tx) = &self.notify_tx {
			if let Some(tx) = notify_tx.lock().expect("notify lock").take() {
				let _ = tx.send(());
			}
		}
		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}
}

pub struct StopImmediatelyActor;

impl StopImmediatelyActor {
	pub fn new() -> Self {
		Self
	}
}

#[async_trait]
impl TestActor for StopImmediatelyActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		config.send_stop_intent();
		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}
}

pub struct CustomActor {
	on_start_fn: Box<
		dyn Fn(ActorConfig) -> Pin<Box<dyn Future<Output = Result<ActorStartResult>> + Send>>
			+ Send
			+ Sync,
	>,
	on_stop_fn: Box<
		dyn Fn() -> Pin<Box<dyn Future<Output = Result<ActorStopResult>> + Send>> + Send + Sync,
	>,
}

pub struct CustomActorBuilder {
	on_start_fn: Option<
		Box<
			dyn Fn(ActorConfig) -> Pin<Box<dyn Future<Output = Result<ActorStartResult>> + Send>>
				+ Send
				+ Sync,
		>,
	>,
	on_stop_fn: Option<
		Box<
			dyn Fn() -> Pin<Box<dyn Future<Output = Result<ActorStopResult>> + Send>> + Send + Sync,
		>,
	>,
}

impl CustomActorBuilder {
	pub fn new() -> Self {
		Self {
			on_start_fn: None,
			on_stop_fn: None,
		}
	}

	pub fn on_start<F>(mut self, f: F) -> Self
	where
		F: Fn(ActorConfig) -> Pin<Box<dyn Future<Output = Result<ActorStartResult>> + Send>>
			+ Send
			+ Sync
			+ 'static,
	{
		self.on_start_fn = Some(Box::new(f));
		self
	}

	pub fn on_stop<F>(mut self, f: F) -> Self
	where
		F: Fn() -> Pin<Box<dyn Future<Output = Result<ActorStopResult>> + Send>>
			+ Send
			+ Sync
			+ 'static,
	{
		self.on_stop_fn = Some(Box::new(f));
		self
	}

	pub fn build(self) -> CustomActor {
		CustomActor {
			on_start_fn: self
				.on_start_fn
				.unwrap_or_else(|| Box::new(|_| Box::pin(async { Ok(ActorStartResult::Running) }))),
			on_stop_fn: self
				.on_stop_fn
				.unwrap_or_else(|| Box::new(|| Box::pin(async { Ok(ActorStopResult::Success) }))),
		}
	}
}

impl Default for CustomActorBuilder {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl TestActor for CustomActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		(self.on_start_fn)(config).await
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		(self.on_stop_fn)().await
	}
}
