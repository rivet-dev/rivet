use std::collections::HashMap;
use std::env;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ::http::StatusCode;
use anyhow::{Context, Result};
use parking_lot::Mutex;
use reqwest::Url;
use rivet_envoy_client::config::{
	ActorStopHandle, BoxFuture as EnvoyBoxFuture, EnvoyCallbacks, HttpRequest, HttpResponse,
	WebSocketHandler, WebSocketMessage, WebSocketSender,
};
use rivet_envoy_client::envoy::start_envoy;
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_client::protocol;
use rivet_error::RivetError;
use rivetkit_client_protocol as client_protocol;
use scc::{HashMap as SccHashMap, hash_map::Entry as SccEntry};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use serde_json::{Value as JsonValue, json};
use tokio::sync::{Mutex as TokioMutex, Notify, broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use vbare::OwnedVersionedData;

use crate::actor::action::ActionDispatchError;
use crate::actor::config::CanHibernateWebSocket;
use crate::actor::connection::{ConnHandle, HibernatableConnectionMetadata};
use crate::actor::context::{ActorContext, InspectorAttachGuard};
use crate::actor::factory::ActorFactory;
use crate::actor::lifecycle_hooks::Reply;
use crate::actor::messages::{ActorEvent, QueueSendResult, Request, Response, StateDelta};
use crate::actor::preload::{PreloadedKv, PreloadedPersistedActor};
use crate::actor::state::{PERSIST_DATA_KEY, decode_persisted_actor};
use crate::actor::task::{
	ActorTask,
	DispatchCommand,
	LifecycleCommand,
	// These helpers reserve bounded-channel capacity before sending; see
	// `actor::task` for the backpressure and lifecycle reply rationale.
	try_send_dispatch_command,
	try_send_lifecycle_command,
};
use crate::actor::task_types::ShutdownKind;
use crate::engine_process::EngineProcessManager;
use crate::error::{ActorLifecycle as ActorLifecycleError, ActorRuntime};
use crate::inspector::protocol::{
	self as inspector_protocol, ServerMessage as InspectorServerMessage,
};
use crate::inspector::{Inspector, InspectorAuth, InspectorSignal, InspectorSubscription};
use crate::kv::Kv;
use crate::sqlite::SqliteDb;
use crate::types::{ActorKey, ActorKeySegment, WsMessage};
use crate::websocket::WebSocket;

mod actor_connect;
mod dispatch;
mod envoy_callbacks;
mod http;
mod inspector;
mod inspector_ws;
mod runner_config;
mod websocket;

use inspector::build_actor_inspector;
use websocket::is_actor_connect_path;

/// Bound on `handle.shutdown_and_wait` inside `serve_with_config` teardown.
/// Protects against indefinite hangs if the envoy reconnect loop is stuck;
/// the TS/outer-host grace period is the ultimate backstop.
const SHUTDOWN_DRAIN_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Default)]
pub struct CoreRegistry {
	factories: HashMap<String, Arc<ActorFactory>>,
}

#[derive(Clone)]
struct ActorTaskHandle {
	actor_id: String,
	actor_name: String,
	generation: u32,
	ctx: ActorContext,
	factory: Arc<ActorFactory>,
	inspector: Inspector,
	lifecycle: mpsc::Sender<LifecycleCommand>,
	dispatch: mpsc::Sender<DispatchCommand>,
	join: Arc<TokioMutex<Option<JoinHandle<Result<()>>>>>,
}

type ActiveActorInstance = Arc<ActorTaskHandle>;

enum ActorInstanceState {
	Active(ActiveActorInstance),
	Stopping(ActiveActorInstance),
}

impl ActorInstanceState {
	fn instance(&self) -> ActiveActorInstance {
		match self {
			Self::Active(instance) | Self::Stopping(instance) => instance.clone(),
		}
	}

	fn active_instance(&self) -> Option<ActiveActorInstance> {
		match self {
			Self::Active(instance) => Some(instance.clone()),
			Self::Stopping(_) => None,
		}
	}
}

#[derive(Clone)]
struct PendingStop {
	reason: protocol::StopActorReason,
	stop_handle: ActorStopHandle,
}

pub(crate) struct RegistryDispatcher {
	pub(crate) factories: HashMap<String, Arc<ActorFactory>>,
	actor_instances: SccHashMap<String, ActorInstanceState>,
	starting_instances: SccHashMap<String, Arc<Notify>>,
	pending_stops: SccHashMap<String, PendingStop>,
	region: String,
	/// Shared secret gating the Prometheus `/metrics` actor endpoint. When
	/// unset, the endpoint fails closed and is effectively disabled.
	metrics_token: Option<String>,
	handle_inspector_http_in_runtime: bool,
}

pub(crate) struct RegistryCallbacks {
	pub(crate) dispatcher: Arc<RegistryDispatcher>,
}

#[derive(Clone, Debug)]
struct StartActorRequest {
	actor_id: String,
	generation: u32,
	actor_name: String,
	input: Option<Vec<u8>>,
	preload_persisted_actor: PreloadedPersistedActor,
	preloaded_kv: Option<PreloadedKv>,
	ctx: ActorContext,
}

#[derive(Clone, Debug)]
struct ServeSettings {
	version: u32,
	endpoint: String,
	token: Option<String>,
	namespace: String,
	pool_name: String,
	engine_binary_path: Option<PathBuf>,
	handle_inspector_http_in_runtime: bool,
	serverless_base_path: Option<String>,
	serverless_package_version: String,
	serverless_client_endpoint: Option<String>,
	serverless_client_namespace: Option<String>,
	serverless_client_token: Option<String>,
	serverless_validate_endpoint: bool,
	serverless_max_start_payload_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct ServeConfig {
	pub version: u32,
	pub endpoint: String,
	pub token: Option<String>,
	pub namespace: String,
	pub pool_name: String,
	pub engine_binary_path: Option<PathBuf>,
	pub handle_inspector_http_in_runtime: bool,
	pub serverless_base_path: Option<String>,
	pub serverless_package_version: String,
	pub serverless_client_endpoint: Option<String>,
	pub serverless_client_namespace: Option<String>,
	pub serverless_client_token: Option<String>,
	pub serverless_validate_endpoint: bool,
	pub serverless_max_start_payload_bytes: usize,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct InspectorPatchStateBody {
	state: JsonValue,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct InspectorActionBody {
	args: Vec<JsonValue>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct InspectorDatabaseExecuteBody {
	sql: String,
	args: Vec<JsonValue>,
	properties: Option<JsonValue>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct InspectorWorkflowReplayBody {
	entry_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectorQueueMessageJson {
	id: u64,
	name: String,
	created_at_ms: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectorQueueResponseJson {
	size: u32,
	max_size: u32,
	truncated: bool,
	messages: Vec<InspectorQueueMessageJson>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct HttpActionRequestJson {
	args: JsonValue,
}

impl Default for HttpActionRequestJson {
	fn default() -> Self {
		Self {
			args: JsonValue::Array(Vec::new()),
		}
	}
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct HttpQueueSendRequestJson {
	body: JsonValue,
	wait: Option<bool>,
	timeout: Option<u64>,
}

impl Default for HttpQueueSendRequestJson {
	fn default() -> Self {
		Self {
			body: JsonValue::Null,
			wait: None,
			timeout: None,
		}
	}
}

#[derive(RivetError)]
#[error("message", "incoming_too_long", "Incoming message too long")]
struct IncomingMessageTooLong;

#[derive(RivetError)]
#[error("message", "outgoing_too_long", "Outgoing message too long")]
struct OutgoingMessageTooLong;

#[derive(RivetError)]
#[error("actor", "action_timed_out", "Action timed out")]
struct ActionTimedOut;

#[derive(RivetError, Serialize)]
#[error("actor", "method_not_allowed", "Method not allowed")]
struct MethodNotAllowed {
	method: String,
	path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectorConnectionJson {
	#[serde(rename = "type")]
	connection_type: Option<String>,
	id: String,
	details: InspectorConnectionDetailsJson,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectorConnectionDetailsJson {
	#[serde(rename = "type")]
	connection_type: Option<String>,
	params: JsonValue,
	state_enabled: bool,
	state: JsonValue,
	subscriptions: usize,
	is_hibernatable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectorSummaryJson {
	state: JsonValue,
	is_state_enabled: bool,
	connections: Vec<InspectorConnectionJson>,
	rpcs: Vec<String>,
	queue_size: u32,
	is_database_enabled: bool,
	#[serde(rename = "isWorkflowEnabled")]
	workflow_supported: bool,
	workflow_history: Option<JsonValue>,
}

const WS_PROTOCOL_ENCODING: &str = "rivet_encoding.";
const WS_PROTOCOL_CONN_PARAMS: &str = "rivet_conn_params.";

#[derive(Debug)]
struct ActorConnectInit {
	actor_id: String,
	connection_id: String,
}

#[derive(Debug)]
struct ActorConnectError {
	group: String,
	code: String,
	message: String,
	metadata: Option<ByteBuf>,
	action_id: Option<u64>,
}

#[derive(Debug)]
struct ActorConnectActionResponse {
	id: u64,
	output: ByteBuf,
}

#[derive(Debug)]
struct ActorConnectEvent {
	name: String,
	args: ByteBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActorConnectEncoding {
	Json,
	Cbor,
	Bare,
}

#[derive(Debug)]
enum ActorConnectToClient {
	Init(ActorConnectInit),
	Error(ActorConnectError),
	ActionResponse(ActorConnectActionResponse),
	Event(ActorConnectEvent),
}

#[derive(Debug)]
struct ActorConnectActionRequest {
	id: u64,
	name: String,
	args: ByteBuf,
}

#[derive(Debug)]
enum ActorConnectSendError {
	OutgoingTooLong,
	Encode(anyhow::Error),
}

#[derive(Debug, Deserialize)]
struct ActorConnectSubscriptionRequest {
	#[serde(rename = "eventName")]
	event_name: String,
	subscribe: bool,
}

#[derive(Debug)]
enum ActorConnectToServer {
	ActionRequest(ActorConnectActionRequest),
	SubscriptionRequest(ActorConnectSubscriptionRequest),
}

#[derive(Debug, Deserialize)]
struct ActorConnectActionRequestJson {
	id: u64,
	name: String,
	args: JsonValue,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "tag", content = "val")]
enum ActorConnectToServerJsonBody {
	ActionRequest(ActorConnectActionRequestJson),
	SubscriptionRequest(ActorConnectSubscriptionRequest),
}

#[derive(Debug, Deserialize)]
struct ActorConnectToServerJsonEnvelope {
	body: ActorConnectToServerJsonBody,
}

impl CoreRegistry {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn register(&mut self, name: &str, factory: ActorFactory) {
		self.factories.insert(name.to_owned(), Arc::new(factory));
	}

	pub fn register_shared(&mut self, name: &str, factory: Arc<ActorFactory>) {
		self.factories.insert(name.to_owned(), factory);
	}

	pub async fn serve(self, shutdown: CancellationToken) -> Result<()> {
		self.serve_with_config(ServeConfig::from_env(), shutdown)
			.await
	}

	pub async fn serve_with_config(
		self,
		config: ServeConfig,
		shutdown: CancellationToken,
	) -> Result<()> {
		let dispatcher = self.into_dispatcher(&config);
		let _engine_process = match config.engine_binary_path.as_ref() {
			Some(binary_path) => {
				Some(EngineProcessManager::start(binary_path, &config.endpoint).await?)
			}
			None => None,
		};
		runner_config::ensure_local_normal_runner_config(&config).await?;
		let callbacks = Arc::new(RegistryCallbacks {
			dispatcher: dispatcher.clone(),
		});

		let handle = start_envoy(rivet_envoy_client::config::EnvoyConfig {
			version: config.version,
			endpoint: config.endpoint,
			token: config.token,
			namespace: config.namespace,
			pool_name: config.pool_name,
			prepopulate_actor_names: HashMap::new(),
			metadata: None,
			not_global: false,
			debug_latency_ms: None,
			callbacks,
		})
		.await;

		// Do not install `tokio::signal::ctrl_c()` here. It calls
		// `sigaction(SIGINT, ...)` at the POSIX level, which overrides the
		// host's default SIGINT handling when rivetkit-core is embedded in
		// Node via NAPI and leaves the host process unable to exit. Callers
		// trip the `shutdown` token instead.
		shutdown.cancelled().await;

		// Bounded drain. If envoy cannot reach the engine (reconnect loop stuck),
		// we fall back to immediate `Stop` rather than hanging indefinitely.
		// The outer host (TS signal handler / Rust binary) is the backstop.
		match tokio::time::timeout(SHUTDOWN_DRAIN_TIMEOUT, handle.shutdown_and_wait(false)).await {
			Ok(()) => {}
			Err(_) => {
				tracing::warn!("envoy shutdown drain exceeded timeout; forcing immediate stop");
				handle.shutdown(true);
				handle.wait_stopped().await;
			}
		}

		Ok(())
	}

	fn into_dispatcher(self, config: &ServeConfig) -> Arc<RegistryDispatcher> {
		Arc::new(RegistryDispatcher::new(
			self.factories,
			config.handle_inspector_http_in_runtime,
		))
	}

	pub async fn into_serverless_runtime(
		self,
		config: ServeConfig,
	) -> Result<crate::serverless::CoreServerlessRuntime> {
		crate::serverless::CoreServerlessRuntime::new(self.factories, config).await
	}
}

impl RegistryDispatcher {
	pub(crate) fn new(
		factories: HashMap<String, Arc<ActorFactory>>,
		handle_inspector_http_in_runtime: bool,
	) -> Self {
		Self {
			factories,
			actor_instances: SccHashMap::new(),
			starting_instances: SccHashMap::new(),
			pending_stops: SccHashMap::new(),
			region: env::var("RIVET_REGION").unwrap_or_default(),
			metrics_token: env::var("_RIVET_METRICS_TOKEN")
				.ok()
				.filter(|token| !token.is_empty()),
			handle_inspector_http_in_runtime,
		}
	}
}

impl RegistryDispatcher {
	async fn start_actor(self: &Arc<Self>, request: StartActorRequest) -> Result<()> {
		let startup_notify = Arc::new(Notify::new());
		let _ = self
			.starting_instances
			.insert_async(request.actor_id.clone(), startup_notify.clone())
			.await;
		let factory = self
			.factories
			.get(&request.actor_name)
			.cloned()
			.ok_or_else(|| {
				ActorRuntime::NotRegistered {
					actor_name: request.actor_name.clone(),
				}
				.build()
			})?;
		let config = factory.config().clone();
		let (lifecycle_tx, lifecycle_rx) = mpsc::channel(config.lifecycle_command_inbox_capacity);
		let (dispatch_tx, dispatch_rx) = mpsc::channel(config.dispatch_command_inbox_capacity);
		let (lifecycle_events_tx, lifecycle_events_rx) =
			mpsc::channel(config.lifecycle_event_inbox_capacity);
		request
			.ctx
			.configure_lifecycle_events(Some(lifecycle_events_tx));
		request.ctx.cancel_sleep_timer();
		request.ctx.set_local_alarm_callback(Some(Arc::new({
			let lifecycle_tx = lifecycle_tx.clone();
			let metrics = request.ctx.metrics().clone();
			let capacity = config.lifecycle_command_inbox_capacity;
			move || {
				let lifecycle_tx = lifecycle_tx.clone();
				let metrics = metrics.clone();
				Box::pin(async move {
					let (reply_tx, reply_rx) = oneshot::channel();
					if let Err(error) = try_send_lifecycle_command(
						&lifecycle_tx,
						capacity,
						"fire_alarm",
						LifecycleCommand::FireAlarm { reply: reply_tx },
						Some(&metrics),
					) {
						tracing::warn!(?error, "failed to enqueue actor alarm");
						return;
					}
					let _ = reply_rx.await;
				})
			}
		})));
		let task = ActorTask::new(
			request.actor_id.clone(),
			request.generation,
			lifecycle_rx,
			dispatch_rx,
			lifecycle_events_rx,
			factory.clone(),
			request.ctx.clone(),
			request.input,
			None,
		)
		.with_preloaded_persisted_actor(request.preload_persisted_actor)
		.with_preloaded_kv(request.preloaded_kv);
		let join = tokio::spawn(task.run());

		let (start_tx, start_rx) = oneshot::channel();
		let result: Result<Arc<ActorTaskHandle>> = async {
			try_send_lifecycle_command(
				&lifecycle_tx,
				config.lifecycle_command_inbox_capacity,
				"start_actor",
				LifecycleCommand::Start { reply: start_tx },
				Some(request.ctx.metrics()),
			)
			.context("send actor task start command")?;
			start_rx
				.await
				.context("receive actor task start reply")?
				.context("actor task start")?;
			let inspector = build_actor_inspector();
			request.ctx.configure_inspector(Some(inspector.clone()));
			Ok::<Arc<ActorTaskHandle>, anyhow::Error>(Arc::new(ActorTaskHandle {
				actor_id: request.actor_id.clone(),
				actor_name: request.actor_name.clone(),
				generation: request.generation,
				ctx: request.ctx.clone(),
				factory,
				inspector,
				lifecycle: lifecycle_tx,
				dispatch: dispatch_tx,
				join: Arc::new(TokioMutex::new(Some(join))),
			}))
		}
		.await
		.with_context(|| format!("start actor `{}`", request.actor_id));

		match result {
			Ok(instance) => {
				let pending_stop = self
					.pending_stops
					.remove_async(&request.actor_id.clone())
					.await
					.map(|(_, pending_stop)| pending_stop);
				if let Some(pending_stop) = pending_stop {
					let actor_id = request.actor_id.clone();
					if matches!(
						map_envoy_stop_reason(&pending_stop.reason),
						ShutdownKind::Destroy
					) {
						instance.ctx.mark_destroy_requested();
					}
					self.set_actor_instance_state(
						actor_id.clone(),
						ActorInstanceState::Stopping(instance.clone()),
					)
					.await;
					let _ = self
						.starting_instances
						.remove_async(&request.actor_id.clone())
						.await;

					let dispatcher = self.clone();
					tokio::spawn(async move {
						if let Err(error) = dispatcher
							.shutdown_started_instance(
								&actor_id,
								instance.clone(),
								pending_stop.reason,
								pending_stop.stop_handle,
							)
							.await
						{
							tracing::error!(
								actor_id,
								?error,
								"failed to stop actor queued during startup"
							);
						}
						dispatcher
							.remove_stopping_actor_instance(&actor_id, &instance)
							.await;
					});
					startup_notify.notify_waiters();

					Ok(())
				} else {
					self.set_actor_instance_state(
						request.actor_id.clone(),
						ActorInstanceState::Active(instance),
					)
					.await;
					let _ = self
						.starting_instances
						.remove_async(&request.actor_id.clone())
						.await;
					startup_notify.notify_waiters();
					Ok(())
				}
			}
			Err(error) => {
				let _ = self
					.starting_instances
					.remove_async(&request.actor_id.clone())
					.await;
				startup_notify.notify_waiters();
				Err(error)
			}
		}
	}

	async fn set_actor_instance_state(&self, actor_id: String, state: ActorInstanceState) {
		match self.actor_instances.entry_async(actor_id).await {
			SccEntry::Occupied(mut entry) => {
				entry.insert(state);
			}
			SccEntry::Vacant(entry) => {
				entry.insert_entry(state);
			}
		}
	}

	async fn transition_actor_to_stopping(&self, actor_id: &str) -> Option<ActiveActorInstance> {
		match self.actor_instances.entry_async(actor_id.to_owned()).await {
			SccEntry::Occupied(mut entry) => {
				let instance = entry.get().instance();
				if matches!(entry.get(), ActorInstanceState::Active(_)) {
					entry.insert(ActorInstanceState::Stopping(instance.clone()));
				} else {
					instance
						.ctx
						.warn_work_sent_to_stopping_instance("stop_actor");
				}
				Some(instance)
			}
			SccEntry::Vacant(entry) => {
				drop(entry);
				None
			}
		}
	}

	async fn remove_stopping_actor_instance(&self, actor_id: &str, expected: &ActiveActorInstance) {
		match self.actor_instances.entry_async(actor_id.to_owned()).await {
			SccEntry::Occupied(entry) => {
				let should_remove = match entry.get() {
					ActorInstanceState::Stopping(instance) => Arc::ptr_eq(instance, expected),
					ActorInstanceState::Active(_) => false,
				};
				if should_remove {
					let _ = entry.remove_entry();
				}
			}
			SccEntry::Vacant(entry) => {
				drop(entry);
			}
		}
	}

	async fn active_actor(&self, actor_id: &str) -> Result<Arc<ActorTaskHandle>> {
		if let Some(instance) = self.actor_instances.get_async(&actor_id.to_owned()).await {
			match instance.get() {
				ActorInstanceState::Active(instance) => {
					let instance = instance.clone();
					if instance.ctx.started() {
						return Ok(instance);
					}

					instance
						.ctx
						.warn_work_sent_to_stopping_instance("active_actor");
					return Err(if instance.ctx.destroy_requested() {
						ActorLifecycleError::Destroying.build()
					} else {
						ActorLifecycleError::Starting.build()
					});
				}
				ActorInstanceState::Stopping(instance) => {
					let instance = instance.clone();
					instance
						.ctx
						.warn_work_sent_to_stopping_instance("active_actor");
					return Err(if instance.ctx.destroy_requested() {
						ActorLifecycleError::Destroying.build()
					} else {
						ActorLifecycleError::Stopping.build()
					});
				}
			}
		}

		tracing::warn!(actor_id, "actor instance not found");
		Err(ActorRuntime::NotFound {
			resource: "instance".to_owned(),
			id: actor_id.to_owned(),
		}
		.build())
	}

	async fn stop_actor(
		&self,
		actor_id: &str,
		reason: protocol::StopActorReason,
		stop_handle: ActorStopHandle,
	) -> Result<()> {
		if self
			.starting_instances
			.get_async(&actor_id.to_owned())
			.await
			.is_some()
		{
			let _ = self
				.pending_stops
				.insert_async(
					actor_id.to_owned(),
					PendingStop {
						reason,
						stop_handle,
					},
				)
				.await;
			return Ok(());
		}

		let instance = match self.transition_actor_to_stopping(actor_id).await {
			Some(instance) => instance,
			None => {
				let _ = self
					.pending_stops
					.insert_async(
						actor_id.to_owned(),
						PendingStop {
							reason,
							stop_handle,
						},
					)
					.await;
				return Ok(());
			}
		};
		let result = self
			.shutdown_started_instance(actor_id, instance.clone(), reason, stop_handle)
			.await;
		self.remove_stopping_actor_instance(actor_id, &instance)
			.await;
		result
	}

	async fn shutdown_started_instance(
		&self,
		actor_id: &str,
		instance: Arc<ActorTaskHandle>,
		reason: protocol::StopActorReason,
		stop_handle: ActorStopHandle,
	) -> Result<()> {
		let task_stop_reason = map_envoy_stop_reason(&reason);

		if matches!(task_stop_reason, ShutdownKind::Destroy) {
			instance.ctx.mark_destroy_requested();
		}

		tracing::debug!(
			actor_id,
			handle_actor_id = %instance.actor_id,
			actor_name = %instance.actor_name,
			generation = instance.generation,
			?reason,
			?task_stop_reason,
			"stopping actor instance"
		);

		let (reply_tx, reply_rx) = oneshot::channel();
		let shutdown_result = match try_send_lifecycle_command(
			&instance.lifecycle,
			instance.factory.config().lifecycle_command_inbox_capacity,
			"stop_actor",
			LifecycleCommand::Stop {
				reason: task_stop_reason,
				reply: reply_tx,
			},
			Some(instance.ctx.metrics()),
		) {
			Ok(()) => reply_rx
				.await
				.context("receive actor task stop reply")
				.and_then(|result| result),
			Err(error) => Err(error),
		};

		if matches!(task_stop_reason, ShutdownKind::Destroy) {
			let shutdown_deadline =
				Instant::now() + instance.factory.config().effective_sleep_grace_period();
			if !instance
				.ctx
				.wait_for_internal_keep_awake_idle(shutdown_deadline.into())
				.await
			{
				instance.ctx.record_direct_subsystem_shutdown_warning(
					"internal_keep_awake",
					"destroy_drain",
				);
				tracing::warn!(
					actor_id,
					"destroy shutdown timed out waiting for in-flight actions"
				);
			}
			if !instance
				.ctx
				.wait_for_http_requests_drained(shutdown_deadline.into())
				.await
			{
				instance
					.ctx
					.record_direct_subsystem_shutdown_warning("http_requests", "destroy_drain");
				tracing::warn!(
					actor_id,
					"destroy shutdown timed out waiting for in-flight http requests"
				);
			}
		}

		let mut join_guard = instance.join.lock().await;
		if let Some(join) = join_guard.take() {
			join.await
				.context("join actor task")?
				.context("actor task failed")?;
		}
		instance.ctx.configure_lifecycle_events(None);

		match shutdown_result {
			Ok(_) => {
				let _ = stop_handle.complete();
				Ok(())
			}
			Err(error) => {
				let _ = stop_handle.fail(anyhow::Error::new(RivetError::extract(&error)));
				Err(error).with_context(|| format!("stop actor `{actor_id}`"))
			}
		}
	}
}

impl RegistryDispatcher {
	fn can_hibernate(&self, actor_id: &str, request: &HttpRequest) -> bool {
		if matches!(is_actor_connect_path(&request.path), Ok(true)) {
			return true;
		}

		let Some(instance) = self
			.actor_instances
			.read_sync(actor_id, |_, state| state.active_instance())
			.flatten()
		else {
			return false;
		};

		match &instance.factory.config().can_hibernate_websocket {
			CanHibernateWebSocket::Bool(value) => *value,
			CanHibernateWebSocket::Callback(callback) => callback(request),
		}
	}

	#[allow(clippy::too_many_arguments)]
	fn build_actor_context(
		&self,
		handle: EnvoyHandle,
		actor_id: &str,
		generation: u32,
		actor_name: &str,
		key: ActorKey,
		factory: &ActorFactory,
	) -> ActorContext {
		let ctx = ActorContext::build(
			actor_id.to_owned(),
			actor_name.to_owned(),
			key,
			self.region.clone(),
			factory.config().clone(),
			Kv::new(handle.clone(), actor_id.to_owned()),
			SqliteDb::new(
				handle.clone(),
				actor_id.to_owned(),
				factory.config().has_database,
			),
		);
		ctx.configure_envoy(handle, Some(generation));
		ctx
	}
}

/// Maps an envoy-protocol stop reason to the lifecycle `ShutdownKind` used by
/// `ActorTask`. Reallocation paths (the actor will resurrect on a new envoy)
/// are routed through `Sleep` so user `onSleep` runs and durable state is
/// preserved without firing a permanent destroy.
fn map_envoy_stop_reason(reason: &protocol::StopActorReason) -> ShutdownKind {
	match reason {
		// Idle sleep requested by the actor itself.
		protocol::StopActorReason::SleepIntent => ShutdownKind::Sleep,
		// Runner is being drained; engine will reallocate the actor on a new
		// envoy. Treat as sleep so persistent state and onSleep semantics hold.
		protocol::StopActorReason::GoingAway => ShutdownKind::Sleep,
		// Runner connection lost; once reconnected (or another runner is
		// allocated) the actor resurrects with the same id.
		protocol::StopActorReason::Lost => ShutdownKind::Sleep,
		// User-initiated stop intent (`ctx.destroy()` and equivalents).
		protocol::StopActorReason::StopIntent => ShutdownKind::Destroy,
		// Engine-initiated permanent destroy.
		protocol::StopActorReason::Destroy => ShutdownKind::Destroy,
	}
}
