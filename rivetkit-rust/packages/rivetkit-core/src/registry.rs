use std::collections::HashMap;
use std::env;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use http::StatusCode;
#[cfg(unix)]
use nix::sys::signal::{self, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
use reqwest::Url;
use rivet_envoy_client::config::{
	ActorStopHandle, BoxFuture as EnvoyBoxFuture, EnvoyCallbacks, HttpRequest,
	HttpResponse, WebSocketHandler, WebSocketMessage, WebSocketSender,
};
use rivet_envoy_client::envoy::start_envoy;
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_client::protocol;
use rivet_error::RivetError;
use scc::HashMap as SccHashMap;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use serde_json::{Value as JsonValue, json};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex as TokioMutex, Notify, broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use uuid::Uuid;

use crate::actor::action::ActionDispatchError;
use crate::actor::callbacks::{
	ActorEvent, Reply, Request, Response, StateDelta,
};
use crate::actor::connection::{ConnHandle, HibernatableConnectionMetadata};
use crate::actor::config::CanHibernateWebSocket;
use crate::actor::context::ActorContext;
use crate::actor::factory::ActorFactory;
use crate::actor::state::{PERSIST_DATA_KEY, PersistedActor, decode_persisted_actor};
use crate::actor::task::{
	ActorTask, DispatchCommand, LifecycleCommand,
	try_send_dispatch_command, try_send_lifecycle_command,
};
use crate::actor::task_types::StopReason;
use crate::inspector::protocol::{self as inspector_protocol, ServerMessage as InspectorServerMessage};
use crate::inspector::{Inspector, InspectorAuth, InspectorSignal, InspectorSubscription};
use crate::kv::Kv;
use crate::sqlite::SqliteDb;
use crate::types::{ActorKey, ActorKeySegment, WsMessage};
use crate::websocket::WebSocket;

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

#[derive(Clone)]
struct PendingStop {
	reason: protocol::StopActorReason,
	stop_handle: ActorStopHandle,
}

struct RegistryDispatcher {
	factories: HashMap<String, Arc<ActorFactory>>,
	active_instances: SccHashMap<String, Arc<ActorTaskHandle>>,
	stopping_instances: SccHashMap<String, Arc<ActorTaskHandle>>,
	starting_instances: SccHashMap<String, Arc<Notify>>,
	pending_stops: SccHashMap<String, PendingStop>,
	region: String,
	inspector_token: Option<String>,
	handle_inspector_http_in_runtime: bool,
}

struct RegistryCallbacks {
	dispatcher: Arc<RegistryDispatcher>,
}

#[derive(Clone, Debug)]
struct StartActorRequest {
	actor_id: String,
	generation: u32,
	actor_name: String,
	input: Option<Vec<u8>>,
	preload_persisted_actor: Option<PersistedActor>,
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
}

#[derive(Debug, Deserialize)]
struct EngineHealthResponse {
	status: Option<String>,
	runtime: Option<String>,
	version: Option<String>,
}

#[derive(Debug)]
struct EngineProcessManager {
	child: Child,
	stdout_task: Option<JoinHandle<()>>,
	stderr_task: Option<JoinHandle<()>>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectorConnectionJson {
	#[serde(rename = "type")]
	connection_type: Option<String>,
	id: String,
	params: JsonValue,
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

const ACTOR_CONNECT_CURRENT_VERSION: u16 = 3;
const ACTOR_CONNECT_SUPPORTED_VERSIONS: &[u16] = &[1, 2, 3];
const WS_PROTOCOL_ENCODING: &str = "rivet_encoding.";
const WS_PROTOCOL_CONN_PARAMS: &str = "rivet_conn_params.";

#[derive(Debug, Serialize, Deserialize)]
struct ActorConnectInit {
	#[serde(rename = "actorId")]
	actor_id: String,
	#[serde(rename = "connectionId")]
	connection_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ActorConnectError {
	group: String,
	code: String,
	message: String,
	metadata: Option<ByteBuf>,
	#[serde(rename = "actionId")]
	action_id: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ActorConnectActionResponse {
	id: u64,
	output: ByteBuf,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
struct ActorConnectActionRequestJson {
	id: u64,
	name: String,
	args: JsonValue,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "tag", content = "val")]
enum ActorConnectToServerJsonBody {
	ActionRequest(ActorConnectActionRequestJson),
	SubscriptionRequest(ActorConnectSubscriptionRequest),
}

#[derive(Debug, Serialize, Deserialize)]
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

	pub async fn serve(self) -> Result<()> {
		self.serve_with_config(ServeConfig::from_env()).await
	}

	pub async fn serve_with_config(self, config: ServeConfig) -> Result<()> {
		let dispatcher = self.into_dispatcher(&config);
		let mut engine_process = match config.engine_binary_path.as_ref() {
			Some(binary_path) => {
				Some(EngineProcessManager::start(binary_path, &config.endpoint).await?)
			}
			None => None,
		};
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

		let shutdown_signal = tokio::signal::ctrl_c()
			.await
			.context("wait for registry shutdown signal");
		handle.shutdown(false);

		if let Some(engine_process) = engine_process.take() {
			engine_process.shutdown().await?;
		}

		shutdown_signal?;

		Ok(())
	}

	fn into_dispatcher(self, config: &ServeConfig) -> Arc<RegistryDispatcher> {
		Arc::new(RegistryDispatcher {
			factories: self.factories,
			active_instances: SccHashMap::new(),
			stopping_instances: SccHashMap::new(),
			starting_instances: SccHashMap::new(),
			pending_stops: SccHashMap::new(),
			region: env::var("RIVET_REGION").unwrap_or_default(),
			inspector_token: env::var("RIVET_INSPECTOR_TOKEN")
				.ok()
				.filter(|token| !token.is_empty()),
			handle_inspector_http_in_runtime: config.handle_inspector_http_in_runtime,
		})
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
			.ok_or_else(|| anyhow!("actor factory `{}` is not registered", request.actor_name))?;
		let config = factory.config().clone();
		let (lifecycle_tx, lifecycle_rx) =
			mpsc::channel(config.lifecycle_command_inbox_capacity);
		let (dispatch_tx, dispatch_rx) =
			mpsc::channel(config.dispatch_command_inbox_capacity);
		let (lifecycle_events_tx, lifecycle_events_rx) =
			mpsc::channel(config.lifecycle_event_inbox_capacity);
		request
			.ctx
			.configure_lifecycle_events(Some(lifecycle_events_tx));
		request.ctx.cancel_sleep_timer();
		request
			.ctx
			.schedule()
			.set_local_alarm_callback(Some(Arc::new({
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
			request.preload_persisted_actor,
		);
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
					if !matches!(pending_stop.reason, protocol::StopActorReason::SleepIntent) {
						instance.ctx.mark_destroy_requested();
					}
					let _ = self
						.stopping_instances
						.insert_async(actor_id.clone(), instance.clone())
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
								instance,
								pending_stop.reason,
								pending_stop.stop_handle,
							)
							.await
						{
							tracing::error!(actor_id, ?error, "failed to stop actor queued during startup");
						}
						let _ = dispatcher.stopping_instances.remove_async(&actor_id).await;
					});
					startup_notify.notify_waiters();

					Ok(())
				} else {
					let _ = self
						.active_instances
						.insert_async(request.actor_id.clone(), instance)
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

	async fn active_actor(&self, actor_id: &str) -> Result<Arc<ActorTaskHandle>> {
		if let Some(instance) = self.active_instances.get_async(&actor_id.to_owned()).await {
			return Ok(instance.get().clone());
		}

		if let Some(instance) = self.stopping_instances.get_async(&actor_id.to_owned()).await {
			let instance = instance.get().clone();
			instance.ctx.warn_work_sent_to_stopping_instance("active_actor");
			return Ok(instance);
		}

		tracing::warn!(actor_id, "actor instance not found");
		Err(anyhow!("actor instance `{actor_id}` was not found"))
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

		let instance = match self.active_actor(actor_id).await {
			Ok(instance) => instance,
			Err(_) => {
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
		let _ = self.active_instances.remove_async(&actor_id.to_owned()).await;
		let _ = self
			.stopping_instances
			.insert_async(actor_id.to_owned(), instance.clone())
			.await;
		let result = self
			.shutdown_started_instance(actor_id, instance, reason, stop_handle)
			.await;
		let _ = self.stopping_instances.remove_async(&actor_id.to_owned()).await;
		result
	}

	async fn shutdown_started_instance(
		&self,
		actor_id: &str,
		instance: Arc<ActorTaskHandle>,
		reason: protocol::StopActorReason,
		stop_handle: ActorStopHandle,
	) -> Result<()> {
		if !matches!(reason, protocol::StopActorReason::SleepIntent) {
			instance.ctx.mark_destroy_requested();
		}

		tracing::debug!(
			actor_id,
			handle_actor_id = %instance.actor_id,
			actor_name = %instance.actor_name,
			generation = instance.generation,
			?reason,
			"stopping actor instance"
		);

		let task_stop_reason = match reason {
			protocol::StopActorReason::SleepIntent => StopReason::Sleep,
			_ => StopReason::Destroy,
		};
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

		if !matches!(reason, protocol::StopActorReason::SleepIntent) {
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
				tracing::warn!(actor_id, "destroy shutdown timed out waiting for in-flight actions");
			}
			if !instance
				.ctx
				.wait_for_http_requests_drained(shutdown_deadline.into())
				.await
			{
				instance.ctx.record_direct_subsystem_shutdown_warning(
					"http_requests",
					"destroy_drain",
				);
				tracing::warn!(actor_id, "destroy shutdown timed out waiting for in-flight http requests");
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
				let _ = stop_handle.fail(anyhow!("{error:#}"));
				Err(error).with_context(|| format!("stop actor `{actor_id}`"))
			}
		}
	}

	async fn handle_fetch(
		&self,
		actor_id: &str,
		request: HttpRequest,
	) -> Result<HttpResponse> {
		let instance = self.active_actor(actor_id).await?;
		if request.path == "/metrics" {
			return self.handle_metrics_fetch(&instance, &request);
		}
		let request = build_http_request(request).await?;
		if let Some(response) = self.handle_inspector_fetch(&instance, &request).await? {
			return Ok(response);
		}

		instance.ctx.cancel_sleep_timer();

		let rearm_sleep_after_request = |ctx: ActorContext| {
			let sleep_ctx = ctx.clone();
			ctx.wait_until(async move {
				while sleep_ctx.can_sleep().await == crate::actor::sleep::CanSleep::ActiveHttpRequests {
					sleep(Duration::from_millis(10)).await;
				}
				sleep_ctx.reset_sleep_timer();
			});
		};

		let (reply_tx, reply_rx) = oneshot::channel();
		try_send_dispatch_command(
			&instance.dispatch,
			instance.factory.config().dispatch_command_inbox_capacity,
			"dispatch_http",
			DispatchCommand::Http {
				request,
				reply: reply_tx,
			},
			Some(instance.ctx.metrics()),
		)
		.context("send actor task HTTP dispatch command")?;

		match reply_rx
			.await
			.context("receive actor task HTTP dispatch reply")?
		{
			Ok(response) => {
				rearm_sleep_after_request(instance.ctx.clone());
				build_envoy_response(response)
			}
			Err(error) => {
				tracing::error!(actor_id, ?error, "actor request callback failed");
				rearm_sleep_after_request(instance.ctx.clone());
				Ok(inspector_anyhow_response(error))
			}
		}
	}

	async fn handle_inspector_fetch(
		&self,
		instance: &ActorTaskHandle,
		request: &Request,
	) -> Result<Option<HttpResponse>> {
		let url = inspector_request_url(request)?;
		if !url.path().starts_with("/inspector/") {
			return Ok(None);
		}
		if self.handle_inspector_http_in_runtime {
			return Ok(None);
		}
		if InspectorAuth::new()
			.verify(&instance.ctx, authorization_bearer_token(request.headers()))
			.await
			.is_err()
		{
			return Ok(Some(inspector_unauthorized_response()));
		}

		let method = request.method().clone();
		let path = url.path();
		let response = match (method, path) {
			(http::Method::GET, "/inspector/state") => json_http_response(
				StatusCode::OK,
				&json!({
					"state": decode_cbor_json_or_null(&instance.ctx.state()),
					"isStateEnabled": true,
				}),
			),
			(http::Method::PATCH, "/inspector/state") => {
				let body: InspectorPatchStateBody = match parse_json_body(request) {
					Ok(body) => body,
					Err(response) => return Ok(Some(response)),
				};
				instance.ctx.set_state(encode_json_as_cbor(&body.state)?)?;
				match instance
					.ctx
					.save_state(vec![StateDelta::ActorState(instance.ctx.state())])
					.await
				{
					Ok(_) => json_http_response(StatusCode::OK, &json!({ "ok": true })),
					Err(error) => Err(error).context("save inspector state patch"),
				}
			}
			(http::Method::GET, "/inspector/connections") => json_http_response(
				StatusCode::OK,
				&json!({
					"connections": inspector_connections(&instance.ctx),
				}),
			),
			(http::Method::GET, "/inspector/rpcs") => json_http_response(
				StatusCode::OK,
				&json!({
					"rpcs": inspector_rpcs(instance),
				}),
			),
			(http::Method::POST, action_path) if action_path.starts_with("/inspector/action/") => {
				let action_name = action_path
					.trim_start_matches("/inspector/action/")
					.to_owned();
				let body: InspectorActionBody = match parse_json_body(request) {
					Ok(body) => body,
					Err(response) => return Ok(Some(response)),
				};
				match self
					.execute_inspector_action(instance, &action_name, body.args)
					.await
				{
					Ok(output) => json_http_response(
						StatusCode::OK,
						&json!({
							"output": output,
						}),
					),
					Err(error) => Ok(action_error_response(error)),
				}
			}
			(http::Method::GET, "/inspector/queue") => {
				let limit = match parse_u32_query_param(&url, "limit", 100) {
					Ok(limit) => limit,
					Err(response) => return Ok(Some(response)),
				};
				let messages = match instance
					.ctx
					.queue()
					.inspect_messages()
					.await
				{
					Ok(messages) => messages,
					Err(error) => {
						return Ok(Some(inspector_anyhow_response(
							error.context("list inspector queue messages"),
						)));
					}
				};
				let queue_size = messages.len().try_into().unwrap_or(u32::MAX);
				let truncated = messages.len() > limit as usize;
				let messages = messages
					.into_iter()
					.take(limit as usize)
					.map(|message| InspectorQueueMessageJson {
						id: message.id,
						name: message.name,
						created_at_ms: message.created_at,
					})
					.collect();
				let payload = InspectorQueueResponseJson {
					size: queue_size,
					max_size: instance.ctx.queue().max_size(),
					truncated,
					messages,
				};
				json_http_response(StatusCode::OK, &payload)
			}
			(http::Method::GET, "/inspector/workflow-history") => self
				.inspector_workflow_history(instance)
				.await
				.and_then(|(workflow_supported, history)| {
					json_http_response(
						StatusCode::OK,
						&json!({
							"history": history,
							"isWorkflowEnabled": workflow_supported,
						}),
					)
				}),
			(http::Method::POST, "/inspector/workflow/replay") => {
				let body: InspectorWorkflowReplayBody = match parse_json_body(request) {
					Ok(body) => body,
					Err(response) => return Ok(Some(response)),
				};
				self
					.inspector_workflow_replay(instance, body.entry_id)
					.await
					.and_then(|(workflow_supported, history)| {
						json_http_response(
							StatusCode::OK,
							&json!({
								"history": history,
								"isWorkflowEnabled": workflow_supported,
							}),
						)
					})
			}
			(http::Method::GET, "/inspector/traces") => json_http_response(
				StatusCode::OK,
				&json!({
					"otlp": Vec::<u8>::new(),
					"clamped": false,
				}),
			),
			(http::Method::GET, "/inspector/database/schema") => {
				self
					.inspector_database_schema(&instance.ctx)
					.await
					.context("load inspector database schema")
					.and_then(|payload| {
						json_http_response(StatusCode::OK, &json!({ "schema": payload }))
					})
			}
			(http::Method::GET, "/inspector/database/rows") => {
				let table = match required_query_param(&url, "table") {
					Ok(table) => table,
					Err(response) => return Ok(Some(response)),
				};
				let limit = match parse_u32_query_param(&url, "limit", 100) {
					Ok(limit) => limit,
					Err(response) => return Ok(Some(response)),
				};
				let offset = match parse_u32_query_param(&url, "offset", 0) {
					Ok(offset) => offset,
					Err(response) => return Ok(Some(response)),
				};
				self
					.inspector_database_rows(&instance.ctx, &table, limit, offset)
					.await
					.context("load inspector database rows")
					.and_then(|rows| {
						json_http_response(StatusCode::OK, &json!({ "rows": rows }))
					})
			}
			(http::Method::POST, "/inspector/database/execute") => {
				let body: InspectorDatabaseExecuteBody = match parse_json_body(request) {
					Ok(body) => body,
					Err(response) => return Ok(Some(response)),
				};
				self
					.inspector_database_execute(&instance.ctx, body)
					.await
					.context("execute inspector database query")
					.and_then(|rows| {
						json_http_response(StatusCode::OK, &json!({ "rows": rows }))
					})
			}
			(http::Method::GET, "/inspector/summary") => {
				self
					.inspector_summary(instance)
					.await
					.and_then(|summary| json_http_response(StatusCode::OK, &summary))
			}
			_ => Ok(inspector_error_response(
				StatusCode::NOT_FOUND,
				"actor",
				"not_found",
				"Inspector route was not found",
			)),
		};

		Ok(Some(match response {
			Ok(response) => response,
			Err(error) => inspector_anyhow_response(error),
		}))
	}

	async fn execute_inspector_action(
		&self,
		instance: &ActorTaskHandle,
		action_name: &str,
		args: Vec<JsonValue>,
	) -> std::result::Result<JsonValue, ActionDispatchError> {
		self
			.execute_inspector_action_bytes(
				instance,
				action_name,
				encode_json_as_cbor(&args).map_err(ActionDispatchError::from_anyhow)?,
			)
			.await
			.map(|payload| decode_cbor_json_or_null(&payload))
	}

	async fn execute_inspector_action_bytes(
		&self,
		instance: &ActorTaskHandle,
		action_name: &str,
		args: Vec<u8>,
	) -> std::result::Result<Vec<u8>, ActionDispatchError> {
		let conn = match instance
			.ctx
			.connect_conn(Vec::new(), false, None, None, async { Ok(Vec::new()) })
			.await
		{
			Ok(conn) => conn,
			Err(error) => return Err(ActionDispatchError::from_anyhow(error)),
		};
		let output = dispatch_action_through_task(
			&instance.dispatch,
			instance.factory.config().dispatch_command_inbox_capacity,
			conn.clone(),
			action_name.to_owned(),
			args,
		)
		.await;
		if let Err(error) = conn.disconnect(None).await {
			tracing::warn!(?error, action_name, "failed to disconnect inspector action connection");
		}
		output
	}

	async fn inspector_summary(
		&self,
		instance: &ActorTaskHandle,
	) -> Result<InspectorSummaryJson> {
		let queue_messages = instance
			.ctx
			.queue()
			.inspect_messages()
			.await
			.context("list queue messages for inspector summary")?;
		let (workflow_supported, workflow_history) = self
			.inspector_workflow_history(instance)
			.await
			.context("load inspector workflow summary")?;
		Ok(InspectorSummaryJson {
			state: decode_cbor_json_or_null(&instance.ctx.state()),
			is_state_enabled: true,
			connections: inspector_connections(&instance.ctx),
			rpcs: inspector_rpcs(instance),
			queue_size: queue_messages.len().try_into().unwrap_or(u32::MAX),
			is_database_enabled: instance.ctx.sql().runtime_config().is_ok(),
			workflow_supported,
			workflow_history,
		})
	}

	async fn inspector_workflow_history(
		&self,
		instance: &ActorTaskHandle,
	) -> Result<(bool, Option<JsonValue>)> {
		self
			.inspector_workflow_history_bytes(instance)
			.await
			.map(|(workflow_supported, history)| {
				(
					workflow_supported,
					history
						.map(|payload| decode_cbor_json_or_null(&payload))
						.filter(|value| !value.is_null()),
				)
			})
	}

	async fn inspector_workflow_replay(
		&self,
		instance: &ActorTaskHandle,
		entry_id: Option<String>,
	) -> Result<(bool, Option<JsonValue>)> {
		self
			.inspector_workflow_replay_bytes(instance, entry_id)
			.await
			.map(|(workflow_supported, history)| {
				(
					workflow_supported,
					history
						.map(|payload| decode_cbor_json_or_null(&payload))
						.filter(|value| !value.is_null()),
				)
			})
	}

	async fn inspector_workflow_history_bytes(
		&self,
		instance: &ActorTaskHandle,
	) -> Result<(bool, Option<Vec<u8>>)> {
		let result = instance
			.ctx
			.internal_keep_awake(dispatch_workflow_history_through_task(
				&instance.dispatch,
				instance.factory.config().dispatch_command_inbox_capacity,
			))
			.await
			.context("load inspector workflow history");

		workflow_dispatch_result(result)
	}

	async fn inspector_workflow_replay_bytes(
		&self,
		instance: &ActorTaskHandle,
		entry_id: Option<String>,
	) -> Result<(bool, Option<Vec<u8>>)> {
		let result = instance
			.ctx
			.internal_keep_awake(dispatch_workflow_replay_request_through_task(
				&instance.dispatch,
				instance.factory.config().dispatch_command_inbox_capacity,
				entry_id,
			))
			.await
			.context("replay inspector workflow history");
		let (workflow_supported, history) = workflow_dispatch_result(result)?;
		if workflow_supported {
			instance.inspector.record_workflow_history_updated();
		}

		Ok((workflow_supported, history))
	}

	async fn inspector_database_schema(&self, ctx: &ActorContext) -> Result<JsonValue> {
		self
			.inspector_database_schema_bytes(ctx)
			.await
			.map(|payload| decode_cbor_json_or_null(&payload))
	}

	async fn inspector_database_schema_bytes(&self, ctx: &ActorContext) -> Result<Vec<u8>> {
		let tables = decode_cbor_json_or_null(
			&ctx
				.db_query(
					"SELECT name, type FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '__drizzle_%' ORDER BY name",
					None,
				)
				.await
				.context("query sqlite master tables")?,
		);
		let JsonValue::Array(tables) = tables else {
			return encode_json_as_cbor(&json!({ "tables": [] }));
		};

		let mut inspector_tables = Vec::with_capacity(tables.len());
		for table in tables {
			let name = table
				.get("name")
				.and_then(JsonValue::as_str)
				.ok_or_else(|| anyhow!("sqlite schema row missing table name"))?;
			let table_type = table
				.get("type")
				.and_then(JsonValue::as_str)
				.unwrap_or("table");
			let quoted = quote_sql_identifier(name);

			let columns = decode_cbor_json_or_null(
				&ctx
					.db_query(&format!("PRAGMA table_info({quoted})"), None)
					.await
					.with_context(|| format!("query pragma table_info for `{name}`"))?,
			);
			let foreign_keys = decode_cbor_json_or_null(
				&ctx
					.db_query(&format!("PRAGMA foreign_key_list({quoted})"), None)
					.await
					.with_context(|| format!("query pragma foreign_key_list for `{name}`"))?,
			);
			let count_rows = decode_cbor_json_or_null(
				&ctx
					.db_query(
						&format!("SELECT COUNT(*) as count FROM {quoted}"),
						None,
					)
					.await
					.with_context(|| format!("count rows for `{name}`"))?,
			);
			let records = count_rows
				.as_array()
				.and_then(|rows| rows.first())
				.and_then(|row| row.get("count"))
				.and_then(JsonValue::as_u64)
				.unwrap_or(0);

			inspector_tables.push(json!({
				"table": {
					"schema": "main",
					"name": name,
					"type": table_type,
				},
				"columns": columns,
				"foreignKeys": foreign_keys,
				"records": records,
			}));
		}

		encode_json_as_cbor(&json!({ "tables": inspector_tables }))
	}

	async fn inspector_database_rows(
		&self,
		ctx: &ActorContext,
		table: &str,
		limit: u32,
		offset: u32,
	) -> Result<JsonValue> {
		self
			.inspector_database_rows_bytes(ctx, table, limit, offset)
			.await
			.map(|payload| decode_cbor_json_or_null(&payload))
	}

	async fn inspector_database_rows_bytes(
		&self,
		ctx: &ActorContext,
		table: &str,
		limit: u32,
		offset: u32,
	) -> Result<Vec<u8>> {
		let params = encode_json_as_cbor(&vec![json!(limit.min(500)), json!(offset)])?;
		ctx
			.db_query(
				&format!(
					"SELECT * FROM {} LIMIT ? OFFSET ?",
					quote_sql_identifier(table)
				),
				Some(&params),
			)
			.await
			.with_context(|| format!("query rows for `{table}`"))
	}

	async fn inspector_database_execute(
		&self,
		ctx: &ActorContext,
		body: InspectorDatabaseExecuteBody,
	) -> Result<JsonValue> {
		if body.sql.trim().is_empty() {
			anyhow::bail!("inspector database execute requires non-empty sql");
		}

		let params = if let Some(properties) = body.properties {
			Some(encode_json_as_cbor(&properties)?)
		} else if body.args.is_empty() {
			None
		} else {
			Some(encode_json_as_cbor(&body.args)?)
		};

		if is_read_only_sql(&body.sql) {
			let rows = ctx
				.db_query(&body.sql, params.as_deref())
				.await
				.context("run inspector read-only database query")?;
			return Ok(decode_cbor_json_or_null(&rows));
		}

		ctx.db_run(&body.sql, params.as_deref())
			.await
			.context("run inspector database mutation")?;
		Ok(JsonValue::Array(Vec::new()))
	}

	fn handle_metrics_fetch(
		&self,
		instance: &ActorTaskHandle,
		request: &HttpRequest,
	) -> Result<HttpResponse> {
		if !request_has_bearer_token(request, self.inspector_token.as_deref()) {
			return Ok(unauthorized_response());
		}

		let mut headers = HashMap::new();
		headers.insert(
			http::header::CONTENT_TYPE.to_string(),
			instance.ctx.metrics_content_type().to_owned(),
		);

		Ok(HttpResponse {
			status: http::StatusCode::OK.as_u16(),
			headers,
			body: Some(
				instance
					.ctx
					.render_metrics()
					.context("render actor prometheus metrics")?
					.into_bytes(),
			),
			body_stream: None,
		})
	}

	#[allow(clippy::too_many_arguments)]
	async fn handle_websocket(
		self: &Arc<Self>,
		actor_id: &str,
		request: &HttpRequest,
		path: &str,
		headers: &HashMap<String, String>,
		gateway_id: &protocol::GatewayId,
		request_id: &protocol::RequestId,
		is_hibernatable: bool,
		is_restoring_hibernatable: bool,
		sender: WebSocketSender,
	) -> Result<WebSocketHandler> {
		let instance = self.active_actor(actor_id).await?;
		if is_inspector_connect_path(path)? {
			return self
				.handle_inspector_websocket(actor_id, instance, request, headers)
				.await;
		}
		if is_actor_connect_path(path)? {
			return self
				.handle_actor_connect_websocket(
					actor_id,
					instance,
					request,
					path,
					headers,
					gateway_id,
					request_id,
					is_hibernatable,
					is_restoring_hibernatable,
					sender,
				)
				.await;
		}
		match self
			.handle_raw_websocket(actor_id, instance, request, path, headers, sender)
			.await
		{
			Ok(handler) => Ok(handler),
			Err(error) => {
				let rivet_error = RivetError::extract(&error);
				tracing::warn!(
					actor_id,
					group = rivet_error.group(),
					code = rivet_error.code(),
					?error,
					"failed to establish raw websocket connection"
				);
				Ok(closing_websocket_handler(
					1011,
					&format!("{}.{}", rivet_error.group(), rivet_error.code()),
				))
			}
		}
	}

	#[allow(clippy::too_many_arguments)]
	async fn handle_actor_connect_websocket(
		self: &Arc<Self>,
		actor_id: &str,
		instance: Arc<ActorTaskHandle>,
		_request: &HttpRequest,
		path: &str,
		headers: &HashMap<String, String>,
		gateway_id: &protocol::GatewayId,
		request_id: &protocol::RequestId,
		is_hibernatable: bool,
		is_restoring_hibernatable: bool,
		sender: WebSocketSender,
	) -> Result<WebSocketHandler> {
		let encoding = match websocket_encoding(headers) {
			Ok(encoding) => encoding,
			Err(error) => {
				tracing::warn!(actor_id, ?error, "rejecting unsupported actor connect encoding");
				return Ok(closing_websocket_handler(
					1003,
					"actor.unsupported_websocket_encoding",
				));
			}
		};

		let conn_params = websocket_conn_params(headers)?;
		let connect_request =
			Request::from_parts("GET", path, headers.clone(), Vec::new())
				.context("build actor connect request")?;
		let conn = if is_restoring_hibernatable {
			match instance
				.ctx
				.reconnect_hibernatable_conn(gateway_id, request_id)
			{
				Ok(conn) => conn,
				Err(error) => {
					let rivet_error = RivetError::extract(&error);
					tracing::warn!(
						actor_id,
						group = rivet_error.group(),
						code = rivet_error.code(),
						?error,
						"failed to restore actor websocket connection"
					);
					return Ok(closing_websocket_handler(
						1011,
						&format!("{}.{}", rivet_error.group(), rivet_error.code()),
					));
				}
			}
		} else {
			let hibernation = is_hibernatable.then(|| HibernatableConnectionMetadata {
				gateway_id: gateway_id.to_vec(),
				request_id: request_id.to_vec(),
				server_message_index: 0,
				client_message_index: 0,
				request_path: path.to_owned(),
				request_headers: headers
					.iter()
					.map(|(name, value)| (name.to_ascii_lowercase(), value.clone()))
					.collect(),
			});

			match instance
				.ctx
				.connect_conn(
					conn_params,
					is_hibernatable,
					hibernation,
					Some(connect_request),
					async { Ok(Vec::new()) },
				)
				.await
			{
				Ok(conn) => conn,
				Err(error) => {
					let rivet_error = RivetError::extract(&error);
					tracing::warn!(
						actor_id,
						group = rivet_error.group(),
						code = rivet_error.code(),
						?error,
						"failed to establish actor websocket connection"
					);
					return Ok(closing_websocket_handler(
						1011,
						&format!("{}.{}", rivet_error.group(), rivet_error.code()),
					));
				}
			}
		};

		let managed_disconnect = conn
			.managed_disconnect_handler()
			.context("get actor websocket disconnect handler")?;
		let transport_closed = Arc::new(AtomicBool::new(false));
		let transport_disconnect_sender = sender.clone();
		conn.configure_transport_disconnect_handler(Some(Arc::new(move |reason| {
			let transport_closed = transport_closed.clone();
			let transport_disconnect_sender = transport_disconnect_sender.clone();
			Box::pin(async move {
				if !transport_closed.swap(true, Ordering::SeqCst) {
					transport_disconnect_sender.close(Some(1000), reason);
				}
				Ok(())
			})
		})));
		conn.configure_disconnect_handler(Some(managed_disconnect));

		let max_incoming_message_size = instance.factory.config().max_incoming_message_size as usize;
		let max_outgoing_message_size = instance.factory.config().max_outgoing_message_size as usize;

		let event_sender = sender.clone();
		conn.configure_event_sender(Some(Arc::new(move |event| {
			match send_actor_connect_message(
				&event_sender,
				encoding,
				&ActorConnectToClient::Event(ActorConnectEvent {
					name: event.name,
					args: ByteBuf::from(event.args),
				}),
				max_outgoing_message_size,
			) {
				Ok(()) => Ok(()),
				Err(ActorConnectSendError::OutgoingTooLong) => {
					event_sender.close(
						Some(1011),
						Some("message.outgoing_too_long".to_owned()),
					);
					Ok(())
				}
				Err(ActorConnectSendError::Encode(error)) => Err(error),
			}
		})));

		let init_actor_id = instance.ctx.actor_id().to_owned();
		let init_conn_id = conn.id().to_owned();
		let on_message_conn = conn.clone();
		let on_message_ctx = instance.ctx.clone();
		let on_message_dispatch = instance.dispatch.clone();
		let on_message_dispatch_capacity =
			instance.factory.config().dispatch_command_inbox_capacity;

		let on_open: Option<Box<dyn FnOnce(WebSocketSender) -> futures::future::BoxFuture<'static, ()> + Send>> =
			if is_restoring_hibernatable {
			None
		} else {
			Some(Box::new(move |sender| {
				let actor_id = init_actor_id.clone();
				let conn_id = init_conn_id.clone();
				Box::pin(async move {
					if let Err(error) = send_actor_connect_message(
						&sender,
						encoding,
						&ActorConnectToClient::Init(ActorConnectInit {
							actor_id,
							connection_id: conn_id,
						}),
						max_outgoing_message_size,
					) {
						match error {
							ActorConnectSendError::OutgoingTooLong => {
								sender.close(
									Some(1011),
									Some("message.outgoing_too_long".to_owned()),
								);
							}
							ActorConnectSendError::Encode(error) => {
								tracing::error!(?error, "failed to send actor websocket init message");
								sender.close(Some(1011), Some("actor.init_error".to_owned()));
							}
						}
					}
				})
			}))
		};

		Ok(WebSocketHandler {
			on_message: Box::new(move |message: WebSocketMessage| {
				let conn = on_message_conn.clone();
				let ctx = on_message_ctx.clone();
				let dispatch = on_message_dispatch.clone();
				Box::pin(async move {
					if message.data.len() > max_incoming_message_size {
						message.sender.close(
							Some(1011),
							Some("message.incoming_too_long".to_owned()),
						);
						return;
					}

					let parsed = match decode_actor_connect_message(&message.data, encoding) {
						Ok(parsed) => parsed,
						Err(error) => {
							tracing::warn!(
								?error,
								"failed to decode actor websocket message"
							);
							message
								.sender
								.close(Some(1011), Some("actor.invalid_request".to_owned()));
							return;
						}
					};

					if conn.is_hibernatable()
						&& let Err(error) = persist_and_ack_hibernatable_actor_message(
							&ctx,
							&conn,
							message.message_index,
						)
						.await
					{
						tracing::warn!(
							?error,
							conn_id = conn.id(),
							"failed to persist and ack hibernatable actor websocket message"
						);
						message.sender.close(
							Some(1011),
							Some("actor.hibernation_persist_failed".to_owned()),
						);
						return;
					}

					match parsed {
						ActorConnectToServer::SubscriptionRequest(request) => {
							if request.subscribe {
								if let Err(error) = dispatch_subscribe_request(
									&ctx,
									conn.clone(),
									request.event_name.clone(),
								)
								.await
								{
									let error = RivetError::extract(&error);
									message.sender.close(
										Some(1011),
										Some(format!("{}.{}", error.group(), error.code())),
									);
									return;
								}
								conn.subscribe(request.event_name);
							} else {
								conn.unsubscribe(&request.event_name);
							}
						}
						ActorConnectToServer::ActionRequest(request) => {
							let sender = message.sender.clone();
							let conn = conn.clone();
							tokio::spawn(async move {
								let response = match dispatch_action_through_task(
									&dispatch,
									on_message_dispatch_capacity,
									conn,
									request.name,
									request.args.into_vec(),
								)
								.await
								{
									Ok(output) => ActorConnectToClient::ActionResponse(
										ActorConnectActionResponse {
											id: request.id,
											output: ByteBuf::from(output),
										},
									),
									Err(error) => ActorConnectToClient::Error(
										action_dispatch_error_response(error, request.id),
									),
								};

								match send_actor_connect_message(
									&sender,
									encoding,
									&response,
									max_outgoing_message_size,
								) {
									Ok(()) => {}
									Err(ActorConnectSendError::OutgoingTooLong) => {
										sender.close(
											Some(1011),
											Some("message.outgoing_too_long".to_owned()),
										);
									}
									Err(ActorConnectSendError::Encode(error)) => {
										tracing::error!(?error, "failed to send actor websocket response");
										sender.close(
											Some(1011),
											Some("actor.send_failed".to_owned()),
										);
									}
								}
							});
						}
					}
				})
			}),
			on_close: Box::new(move |_code, reason| {
				let conn = conn.clone();
				Box::pin(async move {
					if let Err(error) = conn.disconnect(Some(reason.as_str())).await {
						tracing::warn!(?error, conn_id = conn.id(), "failed to disconnect actor websocket connection");
					}
				})
			}),
			on_open,
		})
	}

	async fn handle_raw_websocket(
		self: &Arc<Self>,
		actor_id: &str,
		instance: Arc<ActorTaskHandle>,
		request: &HttpRequest,
		path: &str,
		headers: &HashMap<String, String>,
		_sender: WebSocketSender,
	) -> Result<WebSocketHandler> {
		let conn_params = websocket_conn_params(headers)?;
		let websocket_request = Request::from_parts(
			&request.method,
			path,
			headers.clone(),
			request.body.clone().unwrap_or_default(),
		)
		.context("build actor websocket request")?;
		let conn = instance
			.ctx
			.connect_conn_with_request(
				conn_params,
				Some(websocket_request.clone()),
				async { Ok(Vec::new()) },
			)
			.await?;
		let ctx = instance.ctx.clone();
		let dispatch = instance.dispatch.clone();
		let dispatch_capacity = instance.factory.config().dispatch_command_inbox_capacity;
		let conn_for_close = conn.clone();
		let ctx_for_message = ctx.clone();
		let ctx_for_close = ctx.clone();
		let ws = WebSocket::new();
		let ws_for_open = ws.clone();
		let ws_for_message = ws.clone();
		let ws_for_close = ws.clone();
		let request_for_open = websocket_request.clone();
		let actor_id = actor_id.to_owned();
		let actor_id_for_close = actor_id.clone();
		let actor_id_for_open = actor_id.clone();
		let (closed_tx, _closed_rx) = oneshot::channel();
		let closed_tx = Arc::new(std::sync::Mutex::new(Some(closed_tx)));

		Ok(WebSocketHandler {
			on_message: Box::new(move |message: WebSocketMessage| {
				let ctx = ctx_for_message.clone();
				let ws = ws_for_message.clone();
				Box::pin(async move {
					ctx.with_websocket_callback(|| async move {
						let payload = if message.binary {
							WsMessage::Binary(message.data)
						} else {
							match String::from_utf8(message.data) {
								Ok(text) => WsMessage::Text(text),
								Err(error) => {
									tracing::warn!(?error, "raw websocket message was not valid utf-8");
									ws.close(Some(1007), Some("message.invalid_utf8".to_owned()));
									return;
								}
							}
						};
						ws.dispatch_message_event(payload, Some(message.message_index));
					})
					.await;
				})
			}),
			on_close: Box::new(move |code, reason| {
				let conn = conn_for_close.clone();
				let ws = ws_for_close.clone();
				let actor_id = actor_id_for_close.clone();
				let ctx = ctx_for_close.clone();
				let closed_tx = closed_tx.clone();
				Box::pin(async move {
					ws.close(Some(1000), Some("hack_force_close".to_owned()));
					ctx.with_websocket_callback(|| async move {
						ws.dispatch_close_event(code, reason.clone(), code == 1000);
						if let Err(error) = conn.disconnect(Some(reason.as_str())).await {
							tracing::warn!(actor_id, ?error, conn_id = conn.id(), "failed to disconnect raw websocket connection");
						}
					})
					.await;
					if let Some(closed_tx) = closed_tx
						.lock()
						.expect("websocket close sender lock poisoned")
						.take()
					{
						let _ = closed_tx.send(());
					}
				})
			}),
			on_open: Some(Box::new(move |sender| {
				let request = request_for_open.clone();
				let ws = ws_for_open.clone();
				let actor_id = actor_id_for_open.clone();
				let dispatch = dispatch.clone();
				Box::pin(async move {
					let close_sender = sender.clone();
					ws.configure_sender(sender);
					let result = dispatch_websocket_open_through_task(
						&dispatch,
						dispatch_capacity,
						ws.clone(),
						Some(request),
					)
					.await;
					if let Err(error) = result {
						let error = RivetError::extract(&error);
						tracing::error!(actor_id, ?error, "actor raw websocket callback failed");
						close_sender.close(
							Some(1011),
							Some(format!("{}.{}", error.group(), error.code())),
						);
					}
				})
			})),
		})
	}

	async fn handle_inspector_websocket(
		self: &Arc<Self>,
		actor_id: &str,
		instance: Arc<ActorTaskHandle>,
		_request: &HttpRequest,
		headers: &HashMap<String, String>,
	) -> Result<WebSocketHandler> {
		if InspectorAuth::new()
			.verify(
				&instance.ctx,
				websocket_inspector_token(headers)
					.or_else(|| authorization_bearer_token_map(headers)),
			)
			.await
			.is_err()
		{
			tracing::warn!(actor_id, "rejecting inspector websocket without a valid token");
			return Ok(closing_websocket_handler(1008, "inspector.unauthorized"));
		}

		let dispatcher = self.clone();
		let subscription_slot =
			Arc::new(std::sync::Mutex::new(None::<InspectorSubscription>));
		let overlay_task_slot =
			Arc::new(std::sync::Mutex::new(None::<JoinHandle<()>>));
		let on_open_instance = instance.clone();
		let on_open_dispatcher = dispatcher.clone();
		let on_open_slot = subscription_slot.clone();
		let on_open_overlay_slot = overlay_task_slot.clone();
		let on_message_instance = instance.clone();
		let on_message_dispatcher = dispatcher.clone();
		let on_close_instance = instance.clone();

		Ok(WebSocketHandler {
			on_message: Box::new(move |message: WebSocketMessage| {
				let dispatcher = on_message_dispatcher.clone();
				let instance = on_message_instance.clone();
				Box::pin(async move {
					dispatcher
						.handle_inspector_websocket_message(&instance, &message.sender, &message.data)
						.await;
				})
			}),
			on_close: Box::new(move |_code, _reason| {
				let slot = subscription_slot.clone();
				let overlay_slot = overlay_task_slot.clone();
				let instance = on_close_instance.clone();
				Box::pin(async move {
					let mut guard = match slot.lock() {
						Ok(guard) => guard,
						Err(poisoned) => poisoned.into_inner(),
					};
					guard.take();
					let mut overlay_guard = match overlay_slot.lock() {
						Ok(guard) => guard,
						Err(poisoned) => poisoned.into_inner(),
					};
					if let Some(task) = overlay_guard.take() {
						task.abort();
					}
					instance.ctx.inspector_detach();
				})
			}),
			on_open: Some(Box::new(move |open_sender| {
				Box::pin(async move {
					match on_open_dispatcher.inspector_init_message(&on_open_instance).await {
						Ok(message) => {
							if let Err(error) = send_inspector_message(&open_sender, &message) {
								tracing::error!(?error, "failed to send inspector init message");
								open_sender.close(Some(1011), Some("inspector.init_error".to_owned()));
								return;
							}
						}
						Err(error) => {
							tracing::error!(?error, "failed to build inspector init message");
							open_sender.close(Some(1011), Some("inspector.init_error".to_owned()));
							return;
						}
					}

					on_open_instance.ctx.inspector_attach();
					let mut overlay_rx = on_open_instance.ctx.subscribe_inspector();
					let overlay_sender = open_sender.clone();
					let overlay_task = tokio::spawn(async move {
						loop {
							match overlay_rx.recv().await {
								Ok(payload) => match decode_inspector_overlay_state(&payload) {
									Ok(Some(state)) => {
										if let Err(error) = send_inspector_message(
											&overlay_sender,
											&InspectorServerMessage::StateUpdated(
												inspector_protocol::StateUpdated { state },
											),
										) {
											tracing::error!(
												?error,
												"failed to push inspector overlay update"
											);
											break;
										}
									}
									Ok(None) => {}
									Err(error) => {
										tracing::error!(
											?error,
											"failed to decode inspector overlay update"
										);
									}
								},
								Err(broadcast::error::RecvError::Lagged(skipped)) => {
									tracing::warn!(
										skipped,
										"inspector overlay subscriber lagged; waiting for next sync"
									);
								}
								Err(broadcast::error::RecvError::Closed) => break,
							}
						}
					});
					let mut overlay_guard = match on_open_overlay_slot.lock() {
						Ok(guard) => guard,
						Err(poisoned) => poisoned.into_inner(),
					};
					*overlay_guard = Some(overlay_task);

					let listener_dispatcher = on_open_dispatcher.clone();
					let listener_instance = on_open_instance.clone();
					let listener_sender = open_sender.clone();
					let subscription = on_open_instance.inspector.subscribe(Arc::new(
						move |signal| {
							if signal == InspectorSignal::StateUpdated {
								return;
							}
							let dispatcher = listener_dispatcher.clone();
							let instance = listener_instance.clone();
							let sender = listener_sender.clone();
							tokio::spawn(async move {
								match dispatcher
									.inspector_push_message_for_signal(&instance, signal)
									.await
								{
									Ok(Some(message)) => {
										if let Err(error) =
											send_inspector_message(&sender, &message)
										{
											tracing::error!(
												?error,
												?signal,
												"failed to push inspector websocket update"
											);
										}
									}
									Ok(None) => {}
									Err(error) => {
										tracing::error!(
											?error,
											?signal,
											"failed to build inspector websocket update"
										);
									}
								}
							});
						},
					));
					let mut guard = match on_open_slot.lock() {
						Ok(guard) => guard,
						Err(poisoned) => poisoned.into_inner(),
					};
					*guard = Some(subscription);
				})
			})),
		})
	}

	async fn handle_inspector_websocket_message(
		&self,
		instance: &ActorTaskHandle,
		sender: &WebSocketSender,
		payload: &[u8],
	) {
		let response = match inspector_protocol::decode_client_message(payload) {
			Ok(message) => match self.process_inspector_websocket_message(instance, message).await {
				Ok(response) => response,
				Err(error) => Some(InspectorServerMessage::Error(
					inspector_protocol::ErrorMessage {
						message: error.to_string(),
					},
				)),
			},
			Err(error) => Some(InspectorServerMessage::Error(
				inspector_protocol::ErrorMessage {
					message: error.to_string(),
				},
			)),
		};

		if let Some(response) = response
			&& let Err(error) = send_inspector_message(sender, &response)
		{
			tracing::error!(?error, "failed to send inspector websocket response");
		}
	}

	async fn process_inspector_websocket_message(
		&self,
		instance: &ActorTaskHandle,
		message: inspector_protocol::ClientMessage,
	) -> Result<Option<InspectorServerMessage>> {
		match message {
			inspector_protocol::ClientMessage::PatchState(request) => {
				instance.ctx.set_state(request.state)?;
				instance
					.ctx
					.save_state(vec![StateDelta::ActorState(instance.ctx.state())])
					.await
					.context("save inspector websocket state patch")?;
				Ok(None)
			}
			inspector_protocol::ClientMessage::StateRequest(request) => {
				Ok(Some(InspectorServerMessage::StateResponse(
					self.inspector_state_response(instance, request.id),
				)))
			}
			inspector_protocol::ClientMessage::ConnectionsRequest(request) => {
				Ok(Some(InspectorServerMessage::ConnectionsResponse(
					inspector_protocol::ConnectionsResponse {
						rid: request.id,
						connections: inspector_wire_connections(&instance.ctx),
					},
				)))
			}
			inspector_protocol::ClientMessage::ActionRequest(request) => {
				let output = self
					.execute_inspector_action_bytes(instance, &request.name, request.args)
					.await
					.map_err(|error| anyhow!(error.message))?;
				Ok(Some(InspectorServerMessage::ActionResponse(
					inspector_protocol::ActionResponse {
						rid: request.id,
						output,
					},
				)))
			}
			inspector_protocol::ClientMessage::RpcsListRequest(request) => {
				Ok(Some(InspectorServerMessage::RpcsListResponse(
					inspector_protocol::RpcsListResponse {
						rid: request.id,
						rpcs: inspector_rpcs(instance),
					},
				)))
			}
			inspector_protocol::ClientMessage::TraceQueryRequest(request) => {
				Ok(Some(InspectorServerMessage::TraceQueryResponse(
					inspector_protocol::TraceQueryResponse {
						rid: request.id,
						payload: Vec::new(),
					},
				)))
			}
			inspector_protocol::ClientMessage::QueueRequest(request) => {
				let status = self
					.inspector_queue_status(
						instance,
						inspector_protocol::clamp_queue_limit(request.limit),
					)
					.await?;
				Ok(Some(InspectorServerMessage::QueueResponse(
					inspector_protocol::QueueResponse {
						rid: request.id,
						status,
					},
				)))
			}
			inspector_protocol::ClientMessage::WorkflowHistoryRequest(request) => {
				let (workflow_supported, history) =
					self.inspector_workflow_history_bytes(instance).await?;
				Ok(Some(InspectorServerMessage::WorkflowHistoryResponse(
					inspector_protocol::WorkflowHistoryResponse {
						rid: request.id,
						history,
						workflow_supported,
					},
				)))
			}
			inspector_protocol::ClientMessage::WorkflowReplayRequest(request) => {
				let (workflow_supported, history) = self
					.inspector_workflow_replay_bytes(instance, request.entry_id)
					.await?;
				Ok(Some(InspectorServerMessage::WorkflowReplayResponse(
					inspector_protocol::WorkflowReplayResponse {
						rid: request.id,
						history,
						workflow_supported,
					},
				)))
			}
			inspector_protocol::ClientMessage::DatabaseSchemaRequest(request) => {
				let schema = self.inspector_database_schema_bytes(&instance.ctx).await?;
				Ok(Some(InspectorServerMessage::DatabaseSchemaResponse(
					inspector_protocol::DatabaseSchemaResponse {
						rid: request.id,
						schema,
					},
				)))
			}
			inspector_protocol::ClientMessage::DatabaseTableRowsRequest(request) => {
				let result = self
					.inspector_database_rows_bytes(
						&instance.ctx,
						&request.table,
						request.limit.min(u64::from(u32::MAX)) as u32,
						request.offset.min(u64::from(u32::MAX)) as u32,
					)
					.await?;
				Ok(Some(InspectorServerMessage::DatabaseTableRowsResponse(
					inspector_protocol::DatabaseTableRowsResponse {
						rid: request.id,
						result,
					},
				)))
			}
		}
	}

	async fn inspector_init_message(
		&self,
		instance: &ActorTaskHandle,
	) -> Result<InspectorServerMessage> {
		let (workflow_supported, workflow_history) =
			self.inspector_workflow_history_bytes(instance).await?;
		let queue_size = self.inspector_current_queue_size(instance).await?;
		Ok(InspectorServerMessage::Init(
			inspector_protocol::InitMessage {
				connections: inspector_wire_connections(&instance.ctx),
				state: Some(instance.ctx.state()),
				is_state_enabled: true,
				rpcs: inspector_rpcs(instance),
				is_database_enabled: instance.ctx.sql().runtime_config().is_ok(),
				queue_size,
				workflow_history,
				workflow_supported,
			},
		))
	}

	fn inspector_state_response(
		&self,
		instance: &ActorTaskHandle,
		rid: u64,
	) -> inspector_protocol::StateResponse {
		inspector_protocol::StateResponse {
			rid,
			state: Some(instance.ctx.state()),
			is_state_enabled: true,
		}
	}

	async fn inspector_queue_status(
		&self,
		instance: &ActorTaskHandle,
		limit: u32,
	) -> Result<inspector_protocol::QueueStatus> {
		let messages = instance
			.ctx
			.queue()
			.inspect_messages()
			.await
			.context("list inspector queue messages")?;
		let queue_size = messages.len().try_into().unwrap_or(u32::MAX);
		let truncated = messages.len() > limit as usize;
		let messages = messages
			.into_iter()
			.take(limit as usize)
			.map(|message| inspector_protocol::QueueMessageSummary {
				id: message.id,
				name: message.name,
				created_at_ms: u64::try_from(message.created_at).unwrap_or_default(),
			})
			.collect();

		Ok(inspector_protocol::QueueStatus {
			size: u64::from(queue_size),
			max_size: u64::from(instance.ctx.queue().max_size()),
			messages,
			truncated,
		})
	}

	async fn inspector_current_queue_size(&self, instance: &ActorTaskHandle) -> Result<u64> {
		Ok(
			instance
				.ctx
				.queue()
				.inspect_messages()
				.await
				.context("list inspector queue messages for queue size")?
				.len()
				.try_into()
				.unwrap_or(u64::MAX),
		)
	}

	async fn inspector_push_message_for_signal(
		&self,
		instance: &ActorTaskHandle,
		signal: InspectorSignal,
	) -> Result<Option<InspectorServerMessage>> {
		match signal {
			InspectorSignal::StateUpdated => Ok(Some(InspectorServerMessage::StateUpdated(
				inspector_protocol::StateUpdated {
					state: instance.ctx.state(),
				},
			))),
			InspectorSignal::ConnectionsUpdated => Ok(Some(
				InspectorServerMessage::ConnectionsUpdated(
					inspector_protocol::ConnectionsUpdated {
						connections: inspector_wire_connections(&instance.ctx),
					},
				),
			)),
			InspectorSignal::QueueUpdated => Ok(Some(InspectorServerMessage::QueueUpdated(
				inspector_protocol::QueueUpdated {
					queue_size: self.inspector_current_queue_size(instance).await?,
				},
			))),
			InspectorSignal::WorkflowHistoryUpdated => {
				let (_, history) = self.inspector_workflow_history_bytes(instance).await?;
				Ok(history.map(|history| {
					InspectorServerMessage::WorkflowHistoryUpdated(
						inspector_protocol::WorkflowHistoryUpdated { history },
					)
				}))
			}
		}
	}

	fn can_hibernate(&self, actor_id: &str, request: &HttpRequest) -> bool {
		if matches!(is_actor_connect_path(&request.path), Ok(true)) {
			return true;
		}

		let Some(instance) = self
			.active_instances
			.read_sync(actor_id, |_, instance| instance.clone())
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
		sqlite_schema_version: u32,
		sqlite_startup_data: Option<protocol::SqliteStartupData>,
		factory: &ActorFactory,
	) -> ActorContext {
		let ctx = ActorContext::new_runtime(
			actor_id.to_owned(),
			actor_name.to_owned(),
			key,
			self.region.clone(),
			factory.config().clone(),
			Kv::new(handle.clone(), actor_id.to_owned()),
			SqliteDb::new(
				handle.clone(),
				actor_id.to_owned(),
				sqlite_schema_version,
				sqlite_startup_data,
			),
		);
		ctx.configure_envoy(handle, Some(generation));
		ctx
	}

}

impl EnvoyCallbacks for RegistryCallbacks {
	fn on_actor_start(
		&self,
		handle: EnvoyHandle,
		actor_id: String,
		generation: u32,
		config: protocol::ActorConfig,
		preloaded_kv: Option<protocol::PreloadedKv>,
		sqlite_schema_version: u32,
		sqlite_startup_data: Option<protocol::SqliteStartupData>,
	) -> EnvoyBoxFuture<anyhow::Result<()>> {
		let dispatcher = self.dispatcher.clone();
		let actor_name = config.name.clone();
		let key = actor_key_from_protocol(config.key.clone());
		let preload_persisted_actor = decode_preloaded_persisted_actor(preloaded_kv.as_ref());
		let input = config.input.clone();
		let factory = dispatcher.factories.get(&actor_name).cloned();

		Box::pin(async move {
			let factory = factory
				.ok_or_else(|| anyhow!("actor factory `{actor_name}` is not registered"))?;
			let ctx = dispatcher.build_actor_context(
				handle,
				&actor_id,
				generation,
				&actor_name,
				key,
				sqlite_schema_version,
				sqlite_startup_data,
				factory.as_ref(),
			);

			dispatcher
				.start_actor(StartActorRequest {
					actor_id: actor_id.clone(),
					generation,
					actor_name,
					input,
					preload_persisted_actor: preload_persisted_actor?,
					ctx,
				})
				.await?;

			Ok(())
		})
	}

	fn on_actor_stop_with_completion(
		&self,
		_handle: EnvoyHandle,
		actor_id: String,
		_generation: u32,
		reason: protocol::StopActorReason,
		stop_handle: ActorStopHandle,
	) -> EnvoyBoxFuture<anyhow::Result<()>> {
		let dispatcher = self.dispatcher.clone();
		Box::pin(async move { dispatcher.stop_actor(&actor_id, reason, stop_handle).await })
	}

	fn on_shutdown(&self) {
	}

	fn fetch(
		&self,
		_handle: EnvoyHandle,
		actor_id: String,
		_gateway_id: protocol::GatewayId,
		_request_id: protocol::RequestId,
		request: HttpRequest,
	) -> EnvoyBoxFuture<anyhow::Result<HttpResponse>> {
		let dispatcher = self.dispatcher.clone();
		Box::pin(async move { dispatcher.handle_fetch(&actor_id, request).await })
	}

	fn websocket(
		&self,
		_handle: EnvoyHandle,
		actor_id: String,
		_gateway_id: protocol::GatewayId,
		_request_id: protocol::RequestId,
		_request: HttpRequest,
		_path: String,
		_headers: HashMap<String, String>,
		_is_hibernatable: bool,
		_is_restoring_hibernatable: bool,
		sender: WebSocketSender,
	) -> EnvoyBoxFuture<anyhow::Result<WebSocketHandler>> {
		let dispatcher = self.dispatcher.clone();
		Box::pin(async move {
			dispatcher
				.handle_websocket(
					&actor_id,
					&_request,
					&_path,
					&_headers,
					&_gateway_id,
					&_request_id,
					_is_hibernatable,
					_is_restoring_hibernatable,
					sender,
				)
				.await
		})
	}

	fn can_hibernate(
		&self,
		actor_id: &str,
		_gateway_id: &protocol::GatewayId,
		_request_id: &protocol::RequestId,
		request: &HttpRequest,
	) -> bool {
		self.dispatcher.can_hibernate(actor_id, request)
	}
}

impl ServeSettings {
	fn from_env() -> Self {
		Self {
			version: env::var("RIVET_ENVOY_VERSION")
				.ok()
				.and_then(|value| value.parse().ok())
				.unwrap_or(1),
			endpoint: env::var("RIVET_ENDPOINT")
				.unwrap_or_else(|_| "http://127.0.0.1:6420".to_owned()),
			token: Some(env::var("RIVET_TOKEN").unwrap_or_else(|_| "dev".to_owned())),
			namespace: env::var("RIVET_NAMESPACE").unwrap_or_else(|_| "default".to_owned()),
			pool_name: env::var("RIVET_POOL_NAME")
				.unwrap_or_else(|_| "rivetkit-rust".to_owned()),
			engine_binary_path: env::var_os("RIVET_ENGINE_BINARY_PATH").map(PathBuf::from),
			handle_inspector_http_in_runtime: false,
		}
	}
}

impl Default for ServeConfig {
	fn default() -> Self {
		Self::from_env()
	}
}

impl ServeConfig {
	pub fn from_env() -> Self {
		let settings = ServeSettings::from_env();
		Self {
			version: settings.version,
			endpoint: settings.endpoint,
			token: settings.token,
			namespace: settings.namespace,
			pool_name: settings.pool_name,
			engine_binary_path: settings.engine_binary_path,
			handle_inspector_http_in_runtime: settings.handle_inspector_http_in_runtime,
		}
	}
}

impl EngineProcessManager {
	async fn start(binary_path: &Path, endpoint: &str) -> Result<Self> {
		if !binary_path.exists() {
			anyhow::bail!(
				"engine binary not found at `{}`",
				binary_path.display()
			);
		}

		let endpoint_url = Url::parse(endpoint)
			.with_context(|| format!("parse engine endpoint `{endpoint}`"))?;
		let guard_host = endpoint_url
			.host_str()
			.ok_or_else(|| anyhow!("engine endpoint `{endpoint}` is missing a host"))?
			.to_owned();
		let guard_port = endpoint_url
			.port_or_known_default()
			.ok_or_else(|| anyhow!("engine endpoint `{endpoint}` is missing a port"))?;
		let api_peer_port = guard_port
			.checked_add(1)
			.ok_or_else(|| anyhow!("engine endpoint port `{guard_port}` is too large"))?;
		let metrics_port = guard_port
			.checked_add(10)
			.ok_or_else(|| anyhow!("engine endpoint port `{guard_port}` is too large"))?;
		let db_path = std::env::temp_dir()
			.join(format!("rivetkit-engine-{}", Uuid::new_v4()))
			.join("db");

		let mut command = Command::new(binary_path);
		command
			.arg("start")
			.env("RIVET__GUARD__HOST", &guard_host)
			.env("RIVET__GUARD__PORT", guard_port.to_string())
			.env("RIVET__API_PEER__HOST", &guard_host)
			.env("RIVET__API_PEER__PORT", api_peer_port.to_string())
			.env("RIVET__METRICS__HOST", &guard_host)
			.env("RIVET__METRICS__PORT", metrics_port.to_string())
			.env("RIVET__FILE_SYSTEM__PATH", &db_path)
			.stdout(Stdio::piped())
			.stderr(Stdio::piped());

		let mut child = command.spawn().with_context(|| {
			format!(
				"spawn engine binary `{}`",
				binary_path.display()
			)
		})?;
		let pid = child
			.id()
			.ok_or_else(|| anyhow!("engine process missing pid after spawn"))?;
		let stdout_task = spawn_engine_log_task(child.stdout.take(), "stdout");
		let stderr_task = spawn_engine_log_task(child.stderr.take(), "stderr");

		tracing::info!(
			pid,
			path = %binary_path.display(),
			endpoint = %endpoint,
			db_path = %db_path.display(),
			"spawned engine process"
		);

		let health_url = engine_health_url(endpoint);
		let health = match wait_for_engine_health(&health_url).await {
			Ok(health) => health,
			Err(error) => {
				let error = match child.try_wait() {
					Ok(Some(status)) => error.context(format!(
						"engine process exited before becoming healthy with status {status}"
					)),
					Ok(None) => error,
					Err(wait_error) => error.context(format!(
						"failed to inspect engine process status: {wait_error:#}"
					)),
				};
				let manager = Self {
					child,
					stdout_task,
					stderr_task,
				};
				if let Err(shutdown_error) = manager.shutdown().await {
					tracing::warn!(
						?shutdown_error,
						"failed to clean up unhealthy engine process"
					);
				}
				return Err(error);
			}
		};

		tracing::info!(
			pid,
			status = ?health.status,
			runtime = ?health.runtime,
			version = ?health.version,
			"engine process is healthy"
		);

		Ok(Self {
			child,
			stdout_task,
			stderr_task,
		})
	}

	async fn shutdown(mut self) -> Result<()> {
		terminate_engine_process(&mut self.child).await?;
		join_log_task(self.stdout_task.take()).await;
		join_log_task(self.stderr_task.take()).await;
		Ok(())
	}
}

fn engine_health_url(endpoint: &str) -> String {
	format!("{}/health", endpoint.trim_end_matches('/'))
}

fn spawn_engine_log_task<R>(
	reader: Option<R>,
	stream: &'static str,
) -> Option<JoinHandle<()>>
where
	R: AsyncRead + Unpin + Send + 'static,
{
	reader.map(|reader| {
		tokio::spawn(async move {
			let mut lines = BufReader::new(reader).lines();
			while let Ok(Some(line)) = lines.next_line().await {
				match stream {
					"stderr" => tracing::warn!(stream, line, "engine process output"),
					_ => tracing::info!(stream, line, "engine process output"),
				}
			}
		})
	})
}

async fn join_log_task(task: Option<JoinHandle<()>>) {
	let Some(task) = task else {
		return;
	};
	if let Err(error) = task.await {
		tracing::warn!(?error, "engine log task failed");
	}
}

async fn wait_for_engine_health(health_url: &str) -> Result<EngineHealthResponse> {
	const HEALTH_MAX_WAIT: Duration = Duration::from_secs(10);
	const HEALTH_REQUEST_TIMEOUT: Duration = Duration::from_secs(1);
	const HEALTH_INITIAL_BACKOFF: Duration = Duration::from_millis(100);
	const HEALTH_MAX_BACKOFF: Duration = Duration::from_secs(1);

	let client = rivet_pools::reqwest::client()
		.await
		.context("build reqwest client for engine health check")?;
	let deadline = Instant::now() + HEALTH_MAX_WAIT;
	let mut attempt = 0u32;
	let mut backoff = HEALTH_INITIAL_BACKOFF;

	loop {
		attempt += 1;

		let last_error = match client
			.get(health_url)
			.timeout(HEALTH_REQUEST_TIMEOUT)
			.send()
			.await
		{
			Ok(response) if response.status().is_success() => {
				let health = response
					.json::<EngineHealthResponse>()
					.await
					.context("decode engine health response")?;
				return Ok(health);
			}
			Ok(response) => format!("unexpected status {}", response.status()),
			Err(error) => error.to_string(),
		};

		if Instant::now() >= deadline {
			anyhow::bail!(
				"engine health check failed after {attempt} attempts: {last_error}"
			);
		}

		tokio::time::sleep(backoff).await;
		backoff = std::cmp::min(backoff * 2, HEALTH_MAX_BACKOFF);
	}
}

async fn terminate_engine_process(child: &mut Child) -> Result<()> {
	const ENGINE_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

	let Some(pid) = child.id() else {
		return Ok(());
	};

	if let Some(status) = child.try_wait().context("check engine process status")? {
		tracing::info!(pid, ?status, "engine process already exited");
		return Ok(());
	}

	send_sigterm(child)?;
	tracing::info!(pid, "sent SIGTERM to engine process");

	match tokio::time::timeout(ENGINE_SHUTDOWN_TIMEOUT, child.wait()).await {
		Ok(wait_result) => {
			let status = wait_result.context("wait for engine process to exit")?;
			tracing::info!(pid, ?status, "engine process exited");
			Ok(())
		}
		Err(_) => {
			tracing::warn!(
				pid,
				"engine process did not exit after SIGTERM, forcing kill"
			);
			child
				.start_kill()
				.context("force kill engine process after SIGTERM timeout")?;
			let status = child
				.wait()
				.await
				.context("wait for forced engine process shutdown")?;
			tracing::warn!(pid, ?status, "engine process killed");
			Ok(())
		}
	}
}

fn send_sigterm(child: &mut Child) -> Result<()> {
	let pid = child
		.id()
		.ok_or_else(|| anyhow!("engine process missing pid"))?;

	#[cfg(unix)]
	{
		signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
			.with_context(|| format!("send SIGTERM to engine process {pid}"))?;
	}

	#[cfg(not(unix))]
	{
		child
			.start_kill()
			.with_context(|| format!("terminate engine process {pid}"))?;
	}

	Ok(())
}

fn actor_key_from_protocol(key: Option<String>) -> ActorKey {
	key.as_deref()
		.map(deserialize_actor_key_from_protocol)
		.unwrap_or_default()
}

fn deserialize_actor_key_from_protocol(key: &str) -> ActorKey {
	const EMPTY_KEY: &str = "/";
	const KEY_SEPARATOR: char = '/';

	if key.is_empty() || key == EMPTY_KEY {
		return Vec::new();
	}

	let mut parts = Vec::new();
	let mut current_part = String::new();
	let mut escaping = false;
	let mut empty_string_marker = false;

	for ch in key.chars() {
		if escaping {
			if ch == '0' {
				empty_string_marker = true;
			} else {
				current_part.push(ch);
			}
			escaping = false;
		} else if ch == '\\' {
			escaping = true;
		} else if ch == KEY_SEPARATOR {
			if empty_string_marker {
				parts.push(String::new());
				empty_string_marker = false;
			} else {
				parts.push(std::mem::take(&mut current_part));
			}
		} else {
			current_part.push(ch);
		}
	}

	if escaping {
		current_part.push('\\');
		parts.push(current_part);
	} else if empty_string_marker {
		parts.push(String::new());
	} else if !current_part.is_empty() || !parts.is_empty() {
		parts.push(current_part);
	}

	parts.into_iter().map(ActorKeySegment::String).collect()
}

fn decode_preloaded_persisted_actor(
	preloaded_kv: Option<&protocol::PreloadedKv>,
) -> Result<Option<PersistedActor>> {
	let Some(preloaded_kv) = preloaded_kv else {
		return Ok(None);
	};
	let Some(entry) = preloaded_kv.entries.iter().find(|entry| entry.key == PERSIST_DATA_KEY)
	else {
		return Ok(None);
	};

	decode_persisted_actor(&entry.value)
		.map(Some)
		.context("decode preloaded persisted actor")
}

fn inspector_connections(ctx: &ActorContext) -> Vec<InspectorConnectionJson> {
	ctx
		.conns()
		.map(|conn| InspectorConnectionJson {
			connection_type: None,
			id: conn.id().to_owned(),
			params: decode_cbor_json_or_null(&conn.params()),
			state: decode_cbor_json_or_null(&conn.state()),
			subscriptions: conn.subscriptions().len(),
			is_hibernatable: conn.is_hibernatable(),
		})
		.collect()
}

fn decode_inspector_overlay_state(payload: &[u8]) -> Result<Option<Vec<u8>>> {
	let deltas: Vec<StateDelta> = ciborium::from_reader(Cursor::new(payload))
		.context("decode inspector overlay deltas")?;
	Ok(deltas.into_iter().find_map(|delta| match delta {
		StateDelta::ActorState(bytes) => Some(bytes),
		StateDelta::ConnHibernation { .. } | StateDelta::ConnHibernationRemoved(_) => None,
	}))
}

fn inspector_wire_connections(ctx: &ActorContext) -> Vec<inspector_protocol::ConnectionDetails> {
	ctx
		.conns()
		.map(|conn| {
			let details = json!({
				"type": JsonValue::Null,
				"params": decode_cbor_json_or_null(&conn.params()),
				"stateEnabled": true,
				"state": decode_cbor_json_or_null(&conn.state()),
				"subscriptions": conn.subscriptions().len(),
				"isHibernatable": conn.is_hibernatable(),
			});
			inspector_protocol::ConnectionDetails {
				id: conn.id().to_owned(),
				details: encode_json_as_cbor(&details)
					.expect("inspector connection details should encode to cbor"),
			}
		})
		.collect()
}

fn build_actor_inspector() -> Inspector {
	Inspector::new()
}

fn inspector_rpcs(instance: &ActorTaskHandle) -> Vec<String> {
	let _ = instance;
	Vec::new()
}

fn inspector_request_url(request: &Request) -> Result<Url> {
	Url::parse(&format!("http://inspector{}", request.uri()))
		.context("parse inspector request url")
}

fn decode_cbor_json_or_null(payload: &[u8]) -> JsonValue {
	decode_cbor_json(payload).unwrap_or(JsonValue::Null)
}

fn decode_cbor_json(payload: &[u8]) -> Result<JsonValue> {
	if payload.is_empty() {
		return Ok(JsonValue::Null);
	}

	ciborium::from_reader::<JsonValue, _>(Cursor::new(payload))
		.context("decode cbor payload as json")
}

fn encode_json_as_cbor(value: &impl Serialize) -> Result<Vec<u8>> {
	let mut encoded = Vec::new();
	ciborium::into_writer(value, &mut encoded).context("encode inspector payload as cbor")?;
	Ok(encoded)
}

fn quote_sql_identifier(identifier: &str) -> String {
	format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn is_read_only_sql(sql: &str) -> bool {
	let statement = sql.trim_start().to_ascii_uppercase();
	matches!(
		statement.split_whitespace().next(),
		Some("SELECT" | "PRAGMA" | "WITH" | "EXPLAIN")
	)
}

fn json_http_response(status: StatusCode, payload: &impl Serialize) -> Result<HttpResponse> {
	let mut headers = HashMap::new();
	headers.insert(
		http::header::CONTENT_TYPE.to_string(),
		"application/json".to_owned(),
	);
	Ok(HttpResponse {
		status: status.as_u16(),
		headers,
		body: Some(
			serde_json::to_vec(payload).context("serialize inspector json response")?,
		),
		body_stream: None,
	})
}

async fn persist_and_ack_hibernatable_actor_message(
	ctx: &ActorContext,
	conn: &ConnHandle,
	message_index: u16,
) -> Result<()> {
	let Some(hibernation) = conn.set_server_message_index(message_index) else {
		return Ok(());
	};
	ctx.request_hibernation_transport_save(conn.id());
	ctx.ack_hibernatable_websocket_message(
		&hibernation.gateway_id,
		&hibernation.request_id,
		message_index,
	)?;
	Ok(())
}

fn inspector_unauthorized_response() -> HttpResponse {
	inspector_error_response(
		StatusCode::UNAUTHORIZED,
		"inspector",
		"unauthorized",
		"Inspector request requires a valid bearer token",
	)
}

fn action_error_response(error: ActionDispatchError) -> HttpResponse {
	let status = if error.code == "action_not_found" {
		StatusCode::NOT_FOUND
	} else {
		StatusCode::INTERNAL_SERVER_ERROR
	};
	inspector_error_response(status, &error.group, &error.code, &error.message)
}

async fn dispatch_action_through_task(
	dispatch: &mpsc::Sender<DispatchCommand>,
	capacity: usize,
	conn: ConnHandle,
	name: String,
	args: Vec<u8>,
) -> std::result::Result<Vec<u8>, ActionDispatchError> {
	let (reply_tx, reply_rx) = oneshot::channel();
	try_send_dispatch_command(
		dispatch,
		capacity,
		"dispatch_action",
		DispatchCommand::Action {
			name,
			args,
			conn,
			reply: reply_tx,
		},
		None,
	)
	.map_err(ActionDispatchError::from_anyhow)?;

	reply_rx
		.await
		.map_err(|_| {
		ActionDispatchError::from_anyhow(anyhow!(
			"actor task stopped before action dispatch reply was sent"
		))
	})?
	.map_err(ActionDispatchError::from_anyhow)
}

async fn dispatch_websocket_open_through_task(
	dispatch: &mpsc::Sender<DispatchCommand>,
	capacity: usize,
	ws: WebSocket,
	request: Option<Request>,
) -> Result<()> {
	let (reply_tx, reply_rx) = oneshot::channel();
	try_send_dispatch_command(
		dispatch,
		capacity,
		"dispatch_websocket_open",
		DispatchCommand::OpenWebSocket {
			ws,
			request,
			reply: reply_tx,
		},
		None,
	)
	.context("actor task stopped before websocket dispatch command could be sent")?;

	reply_rx
		.await
		.context("actor task stopped before websocket dispatch reply was sent")?
}

async fn dispatch_workflow_history_through_task(
	dispatch: &mpsc::Sender<DispatchCommand>,
	capacity: usize,
) -> Result<Option<Vec<u8>>> {
	let (reply_tx, reply_rx) = oneshot::channel();
	try_send_dispatch_command(
		dispatch,
		capacity,
		"dispatch_workflow_history",
		DispatchCommand::WorkflowHistory { reply: reply_tx },
		None,
	)
	.context("actor task stopped before workflow history dispatch command could be sent")?;

	reply_rx
		.await
		.context("actor task stopped before workflow history dispatch reply was sent")?
}

async fn dispatch_workflow_replay_request_through_task(
	dispatch: &mpsc::Sender<DispatchCommand>,
	capacity: usize,
	entry_id: Option<String>,
) -> Result<Option<Vec<u8>>> {
	let (reply_tx, reply_rx) = oneshot::channel();
	try_send_dispatch_command(
		dispatch,
		capacity,
		"dispatch_workflow_replay",
		DispatchCommand::WorkflowReplay {
			entry_id,
			reply: reply_tx,
		},
		None,
	)
	.context("actor task stopped before workflow replay dispatch command could be sent")?;

	reply_rx
		.await
		.context("actor task stopped before workflow replay dispatch reply was sent")?
}

fn workflow_dispatch_result(
	result: Result<Option<Vec<u8>>>,
) -> Result<(bool, Option<Vec<u8>>)> {
	match result {
		Ok(history) => Ok((true, history)),
		Err(error) if is_dropped_reply_error(&error) => Ok((false, None)),
		Err(error) => Err(error),
	}
}

fn is_dropped_reply_error(error: &anyhow::Error) -> bool {
	let error = RivetError::extract(error);
	error.group() == "actor" && error.code() == "dropped_reply"
}

async fn dispatch_subscribe_request(
	ctx: &ActorContext,
	conn: ConnHandle,
	event_name: String,
) -> Result<()> {
	let (reply_tx, reply_rx) = oneshot::channel();
	ctx.try_send_actor_event(
		ActorEvent::SubscribeRequest {
			conn,
			event_name,
			reply: Reply::from(reply_tx),
		},
		"subscribe_request",
	)?;
	reply_rx
		.await
		.context("actor task stopped before subscribe dispatch reply was sent")?
}

fn inspector_anyhow_response(error: anyhow::Error) -> HttpResponse {
	let error = RivetError::extract(&error);
	let status = inspector_error_status(error.group(), error.code());
	inspector_error_response(status, error.group(), error.code(), error.message())
}

#[cfg(test)]
mod tests {
	use super::{HttpResponseEncoding, message_boundary_error_response, workflow_dispatch_result};
	use crate::error::ActorLifecycle as ActorLifecycleError;
	use http::StatusCode;
	use rivet_error::RivetError;
	use serde_json::{Value as JsonValue, json};

	#[derive(RivetError)]
	#[error("message", "incoming_too_long", "Incoming message too long")]
	struct IncomingMessageTooLong;

	#[derive(RivetError)]
	#[error("message", "outgoing_too_long", "Outgoing message too long")]
	struct OutgoingMessageTooLong;

	#[test]
	fn workflow_dispatch_result_marks_handled_workflow_as_enabled() {
		assert_eq!(
			workflow_dispatch_result(Ok(Some(vec![1, 2, 3]))).expect("workflow dispatch should succeed"),
			(true, Some(vec![1, 2, 3])),
		);
		assert_eq!(
			workflow_dispatch_result(Ok(None)).expect("workflow dispatch should succeed"),
			(true, None),
		);
	}

	#[test]
	fn workflow_dispatch_result_treats_dropped_reply_as_disabled() {
		assert_eq!(
			workflow_dispatch_result(Err(ActorLifecycleError::DroppedReply.build()))
				.expect("dropped reply should map to workflow disabled"),
			(false, None),
		);
	}

	#[test]
	fn workflow_dispatch_result_preserves_non_dropped_reply_errors() {
		let error = workflow_dispatch_result(Err(ActorLifecycleError::Destroying.build()))
			.expect_err("non-dropped reply errors should be preserved");
		let error = rivet_error::RivetError::extract(&error);
		assert_eq!(error.group(), "actor");
		assert_eq!(error.code(), "destroying");
	}

	#[test]
	fn inspector_error_status_maps_action_timeout_to_408() {
		assert_eq!(
			super::inspector_error_status("actor", "action_timed_out"),
			StatusCode::REQUEST_TIMEOUT,
		);
	}

	#[test]
	fn message_boundary_error_response_defaults_to_json() {
		let response = message_boundary_error_response(
			HttpResponseEncoding::Json,
			StatusCode::BAD_REQUEST,
			IncomingMessageTooLong.build(),
		)
		.expect("json response should serialize");

		assert_eq!(response.status, StatusCode::BAD_REQUEST.as_u16());
		assert_eq!(
			response.headers.get(http::header::CONTENT_TYPE.as_str()),
			Some(&"application/json".to_owned())
		);
		assert_eq!(
			response.body,
			Some(
				serde_json::to_vec(&json!({
					"group": "message",
					"code": "incoming_too_long",
					"message": "Incoming message too long",
					"metadata": JsonValue::Null,
				}))
				.expect("json body should encode")
			)
		);
	}

	#[test]
	fn message_boundary_error_response_serializes_bare_v3() {
		let response = message_boundary_error_response(
			HttpResponseEncoding::Bare,
			StatusCode::BAD_REQUEST,
			OutgoingMessageTooLong.build(),
		)
		.expect("bare response should serialize");

		assert_eq!(
			response.headers.get(http::header::CONTENT_TYPE.as_str()),
			Some(&"application/octet-stream".to_owned())
		);

		let body = response.body.expect("bare response should include body");
		assert_eq!(&body[..2], 3u16.to_le_bytes().as_slice());

		let mut cursor = &body[2..];
		assert_eq!(read_bare_string(&mut cursor), "message");
		assert_eq!(read_bare_string(&mut cursor), "outgoing_too_long");
		assert_eq!(read_bare_string(&mut cursor), "Outgoing message too long");
		assert_eq!(cursor.first().copied(), Some(0));
		assert_eq!(cursor.len(), 1);
	}

	fn read_bare_string(cursor: &mut &[u8]) -> String {
		let len = read_bare_uint(cursor) as usize;
		let (value, rest) = cursor.split_at(len);
		*cursor = rest;
		String::from_utf8(value.to_vec()).expect("bare string should decode")
	}

	fn read_bare_uint(cursor: &mut &[u8]) -> u64 {
		let mut shift = 0;
		let mut value = 0u64;

		loop {
			let byte = cursor
				.first()
				.copied()
				.expect("bare uint should have another byte");
			*cursor = &cursor[1..];
			value |= u64::from(byte & 0x7f) << shift;
			if byte & 0x80 == 0 {
				return value;
			}
			shift += 7;
		}
	}
}

fn inspector_error_response(
	status: StatusCode,
	group: &str,
	code: &str,
	message: &str,
) -> HttpResponse {
	json_http_response(
		status,
		&json!({
			"group": group,
			"code": code,
			"message": message,
			"metadata": JsonValue::Null,
		}),
	)
	.expect("inspector error payload should serialize")
}

fn inspector_error_status(group: &str, code: &str) -> StatusCode {
	match (group, code) {
		("auth", "unauthorized") | ("inspector", "unauthorized") => {
			StatusCode::UNAUTHORIZED
		}
		("actor", "action_timed_out") => StatusCode::REQUEST_TIMEOUT,
		(_, "action_not_found") => StatusCode::NOT_FOUND,
		(_, "invalid_request") | (_, "state_not_enabled") | ("database", "not_enabled") => {
			StatusCode::BAD_REQUEST
		}
		_ => StatusCode::INTERNAL_SERVER_ERROR,
	}
}

fn parse_json_body<T>(request: &Request) -> std::result::Result<T, HttpResponse>
where
	T: serde::de::DeserializeOwned,
{
	serde_json::from_slice(request.body()).map_err(|error| {
		inspector_error_response(
			StatusCode::BAD_REQUEST,
			"actor",
			"invalid_request",
			&format!("Invalid inspector JSON body: {error}"),
		)
	})
}

fn required_query_param(url: &Url, key: &str) -> std::result::Result<String, HttpResponse> {
	url
		.query_pairs()
		.find(|(name, _)| name == key)
		.map(|(_, value)| value.into_owned())
		.ok_or_else(|| {
			inspector_error_response(
				StatusCode::BAD_REQUEST,
				"actor",
				"invalid_request",
				&format!("Missing required query parameter `{key}`"),
			)
		})
}

fn parse_u32_query_param(
	url: &Url,
	key: &str,
	default: u32,
) -> std::result::Result<u32, HttpResponse> {
	let Some(value) = url.query_pairs().find(|(name, _)| name == key).map(|(_, value)| value)
	else {
		return Ok(default);
	};
	value.parse::<u32>().map_err(|error| {
		inspector_error_response(
			StatusCode::BAD_REQUEST,
			"actor",
			"invalid_request",
			&format!("Invalid query parameter `{key}`: {error}"),
		)
	})
}

fn authorization_bearer_token(headers: &http::HeaderMap) -> Option<&str> {
	headers
		.get(http::header::AUTHORIZATION)
		.and_then(|value| value.to_str().ok())
		.and_then(|value| value.strip_prefix("Bearer "))
}

fn authorization_bearer_token_map(headers: &HashMap<String, String>) -> Option<&str> {
	headers
		.iter()
		.find(|(name, _)| name.eq_ignore_ascii_case(http::header::AUTHORIZATION.as_str()))
		.and_then(|(_, value)| value.strip_prefix("Bearer "))
}

fn websocket_inspector_token(headers: &HashMap<String, String>) -> Option<&str> {
	headers
		.iter()
		.find(|(name, _)| name.eq_ignore_ascii_case("sec-websocket-protocol"))
		.and_then(|(_, value)| {
			value
				.split(',')
				.map(str::trim)
				.find_map(|protocol| protocol.strip_prefix("rivet_inspector_token."))
		})
}

async fn build_http_request(request: HttpRequest) -> Result<Request> {
	let mut body = request.body.unwrap_or_default();
	if let Some(mut body_stream) = request.body_stream {
		while let Some(chunk) = body_stream.recv().await {
			body.extend_from_slice(&chunk);
		}
	}

	let request_path = normalize_actor_request_path(&request.path);
	Request::from_parts(&request.method, &request_path, request.headers, body)
		.with_context(|| format!("build actor request for `{}`", request.path))
}

fn normalize_actor_request_path(path: &str) -> String {
	let Some(stripped) = path.strip_prefix("/request") else {
		return path.to_owned();
	};

	if stripped.is_empty() {
		return "/".to_owned();
	}

	match stripped.as_bytes().first() {
		Some(b'/') | Some(b'?') => stripped.to_owned(),
		_ => path.to_owned(),
	}
}

fn build_envoy_response(response: Response) -> Result<HttpResponse> {
	let (status, headers, body) = response.to_parts();

	Ok(HttpResponse {
		status,
		headers,
		body: Some(body),
		body_stream: None,
	})
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HttpResponseEncoding {
	Json,
	Cbor,
	Bare,
}

#[cfg(test)]
fn request_encoding(headers: &http::HeaderMap) -> HttpResponseEncoding {
	headers
		.get("x-rivet-encoding")
		.and_then(|value| value.to_str().ok())
		.map(|value| match value {
			"cbor" => HttpResponseEncoding::Cbor,
			"bare" => HttpResponseEncoding::Bare,
			_ => HttpResponseEncoding::Json,
		})
		.unwrap_or(HttpResponseEncoding::Json)
}

#[cfg(test)]
fn message_boundary_error_response(
	encoding: HttpResponseEncoding,
	status: StatusCode,
	error: anyhow::Error,
) -> Result<HttpResponse> {
	let error = RivetError::extract(&error);
	let body = serialize_http_response_error(
		encoding,
		error.group(),
		error.code(),
		error.message(),
		None,
	)?;

	Ok(HttpResponse {
		status: status.as_u16(),
		headers: HashMap::from([(
			http::header::CONTENT_TYPE.to_string(),
			content_type_for_encoding(encoding).to_owned(),
		)]),
		body: Some(body),
		body_stream: None,
	})
}

#[cfg(test)]
fn content_type_for_encoding(encoding: HttpResponseEncoding) -> &'static str {
	match encoding {
		HttpResponseEncoding::Json => "application/json",
		HttpResponseEncoding::Cbor | HttpResponseEncoding::Bare => "application/octet-stream",
	}
}

#[cfg(test)]
fn serialize_http_response_error(
	encoding: HttpResponseEncoding,
	group: &str,
	code: &str,
	message: &str,
	metadata: Option<&JsonValue>,
) -> Result<Vec<u8>> {
	match encoding {
		HttpResponseEncoding::Json => Ok(serde_json::to_vec(&json!({
			"group": group,
			"code": code,
			"message": message,
			"metadata": metadata.cloned().unwrap_or(JsonValue::Null),
		}))?),
		HttpResponseEncoding::Cbor => {
			let mut out = Vec::new();
			ciborium::into_writer(
				&json!({
					"group": group,
					"code": code,
					"message": message,
					"metadata": metadata.cloned().unwrap_or(JsonValue::Null),
				}),
				&mut out,
			)?;
			Ok(out)
		}
		HttpResponseEncoding::Bare => {
			const CLIENT_PROTOCOL_CURRENT_VERSION: u16 = 3;

			let mut out = Vec::new();
			out.extend_from_slice(&CLIENT_PROTOCOL_CURRENT_VERSION.to_le_bytes());
			write_bare_string(&mut out, group);
			write_bare_string(&mut out, code);
			write_bare_string(&mut out, message);
			let metadata = metadata
				.map(|value| {
					let mut out = Vec::new();
					ciborium::into_writer(value, &mut out)?;
					Ok::<Vec<u8>, anyhow::Error>(out)
				})
				.transpose()?;
			write_bare_optional_data(
				&mut out,
				metadata.as_deref(),
			);
			Ok(out)
		}
	}
}

#[cfg(test)]
fn write_bare_string(out: &mut Vec<u8>, value: &str) {
	write_bare_data(out, value.as_bytes());
}

#[cfg(test)]
fn write_bare_data(out: &mut Vec<u8>, value: &[u8]) {
	write_bare_uint(out, value.len() as u64);
	out.extend_from_slice(value);
}

#[cfg(test)]
fn write_bare_optional_data(out: &mut Vec<u8>, value: Option<&[u8]>) {
	out.push(u8::from(value.is_some()));
	if let Some(value) = value {
		write_bare_data(out, value);
	}
}

#[cfg(test)]
fn write_bare_uint(out: &mut Vec<u8>, mut value: u64) {
	while value >= 0x80 {
		out.push((value as u8 & 0x7f) | 0x80);
		value >>= 7;
	}
	out.push(value as u8);
}

fn unauthorized_response() -> HttpResponse {
	HttpResponse {
		status: http::StatusCode::UNAUTHORIZED.as_u16(),
		headers: HashMap::new(),
		body: Some(Vec::new()),
		body_stream: None,
	}
}

fn request_has_bearer_token(request: &HttpRequest, configured_token: Option<&str>) -> bool {
	let Some(configured_token) = configured_token else {
		return false;
	};

	request.headers.iter().any(|(name, value)| {
		name.eq_ignore_ascii_case(http::header::AUTHORIZATION.as_str())
			&& value == &format!("Bearer {configured_token}")
	})
}

fn send_inspector_message(
	sender: &WebSocketSender,
	message: &InspectorServerMessage,
) -> Result<()> {
	let payload = inspector_protocol::encode_server_message(message)?;
	sender.send(payload, true);
	Ok(())
}

fn send_actor_connect_message(
	sender: &WebSocketSender,
	encoding: ActorConnectEncoding,
	message: &ActorConnectToClient,
	max_outgoing_message_size: usize,
) -> std::result::Result<(), ActorConnectSendError> {
	match encoding {
		ActorConnectEncoding::Json => {
			let payload = encode_actor_connect_message_json(message)
				.map_err(ActorConnectSendError::Encode)?;
			if payload.len() > max_outgoing_message_size {
				return Err(ActorConnectSendError::OutgoingTooLong);
			}
			sender.send_text(&payload);
		}
		ActorConnectEncoding::Cbor => {
			let payload = encode_actor_connect_message_cbor(message)
				.map_err(ActorConnectSendError::Encode)?;
			if payload.len() > max_outgoing_message_size {
				return Err(ActorConnectSendError::OutgoingTooLong);
			}
			sender.send(payload, true);
		}
		ActorConnectEncoding::Bare => {
			let payload = encode_actor_connect_message(message)
				.map_err(ActorConnectSendError::Encode)?;
			if payload.len() > max_outgoing_message_size {
				return Err(ActorConnectSendError::OutgoingTooLong);
			}
			sender.send(payload, true);
		}
	}
	Ok(())
}

fn is_inspector_connect_path(path: &str) -> Result<bool> {
	Ok(
		Url::parse(&format!("http://inspector{path}"))
			.context("parse inspector websocket path")?
			.path()
			== "/inspector/connect",
	)
}

fn is_actor_connect_path(path: &str) -> Result<bool> {
	Ok(
		Url::parse(&format!("http://actor{path}"))
			.context("parse actor websocket path")?
			.path()
			== "/connect",
	)
}

fn websocket_protocols(headers: &HashMap<String, String>) -> impl Iterator<Item = &str> {
	headers
		.iter()
		.find(|(name, _)| name.eq_ignore_ascii_case("sec-websocket-protocol"))
		.map(|(_, value)| value.split(',').map(str::trim))
		.into_iter()
		.flatten()
}

fn websocket_encoding(headers: &HashMap<String, String>) -> Result<ActorConnectEncoding> {
	match websocket_protocols(headers)
		.find_map(|protocol| protocol.strip_prefix(WS_PROTOCOL_ENCODING))
		.unwrap_or("json")
	{
		"json" => Ok(ActorConnectEncoding::Json),
		"cbor" => Ok(ActorConnectEncoding::Cbor),
		"bare" => Ok(ActorConnectEncoding::Bare),
		encoding => Err(anyhow!("unsupported actor websocket encoding `{encoding}`")),
	}
}

fn websocket_conn_params(headers: &HashMap<String, String>) -> Result<Vec<u8>> {
	let Some(encoded_params) = websocket_protocols(headers)
		.find_map(|protocol| protocol.strip_prefix(WS_PROTOCOL_CONN_PARAMS))
	else {
		return Ok(Vec::new());
	};

	let decoded = Url::parse(&format!("http://actor/?value={encoded_params}"))
		.context("decode websocket connection parameters")?
		.query_pairs()
		.find_map(|(name, value)| (name == "value").then_some(value.into_owned()))
		.ok_or_else(|| anyhow!("missing decoded websocket connection parameters"))?;
	let parsed: JsonValue = serde_json::from_str(&decoded)
		.context("parse websocket connection parameters")?;
	encode_json_as_cbor(&parsed)
}

fn encode_actor_connect_message(message: &ActorConnectToClient) -> Result<Vec<u8>> {
	let mut encoded = Vec::new();
	encoded.extend_from_slice(&ACTOR_CONNECT_CURRENT_VERSION.to_le_bytes());
	match message {
		ActorConnectToClient::Init(payload) => {
			encoded.push(0);
			bare_write_string(&mut encoded, &payload.actor_id);
			bare_write_string(&mut encoded, &payload.connection_id);
		}
		ActorConnectToClient::Error(payload) => {
			encoded.push(1);
			bare_write_string(&mut encoded, &payload.group);
			bare_write_string(&mut encoded, &payload.code);
			bare_write_string(&mut encoded, &payload.message);
			bare_write_optional_bytes(
				&mut encoded,
				payload.metadata.as_ref().map(|metadata| metadata.as_ref()),
			);
			bare_write_optional_uint(&mut encoded, payload.action_id);
		}
		ActorConnectToClient::ActionResponse(payload) => {
			encoded.push(2);
			bare_write_uint(&mut encoded, payload.id);
			bare_write_bytes(&mut encoded, payload.output.as_ref());
		}
		ActorConnectToClient::Event(payload) => {
			encoded.push(3);
			bare_write_string(&mut encoded, &payload.name);
			bare_write_bytes(&mut encoded, payload.args.as_ref());
		}
	}
	Ok(encoded)
}

fn encode_actor_connect_message_json(message: &ActorConnectToClient) -> Result<String> {
	serde_json::to_string(&actor_connect_message_json_value(message)?)
		.context("encode actor websocket message as json")
}

fn encode_actor_connect_message_cbor(message: &ActorConnectToClient) -> Result<Vec<u8>> {
	encode_actor_connect_message_cbor_manual(message)
}

fn actor_connect_message_json_value(message: &ActorConnectToClient) -> Result<JsonValue> {
	let body = match message {
		ActorConnectToClient::Init(payload) => json!({
			"tag": "Init",
			"val": {
				"actorId": payload.actor_id.clone(),
				"connectionId": payload.connection_id.clone(),
			},
		}),
		ActorConnectToClient::Error(payload) => {
			let mut value = serde_json::Map::from_iter([
				("group".to_owned(), JsonValue::String(payload.group.clone())),
				("code".to_owned(), JsonValue::String(payload.code.clone())),
				("message".to_owned(), JsonValue::String(payload.message.clone())),
			]);
			if let Some(metadata) = payload.metadata.as_ref() {
				value.insert(
					"metadata".to_owned(),
					decode_cbor_json(metadata.as_ref())?,
				);
			}
			if let Some(action_id) = payload.action_id {
				value.insert("actionId".to_owned(), json_compat_bigint(action_id));
			}
			JsonValue::Object(serde_json::Map::from_iter([
				("tag".to_owned(), JsonValue::String("Error".to_owned())),
				("val".to_owned(), JsonValue::Object(value)),
			]))
		}
		ActorConnectToClient::ActionResponse(payload) => json!({
			"tag": "ActionResponse",
			"val": {
				"id": json_compat_bigint(payload.id),
				"output": decode_cbor_json(payload.output.as_ref())?,
			},
		}),
		ActorConnectToClient::Event(payload) => json!({
			"tag": "Event",
			"val": {
				"name": payload.name.clone(),
				"args": decode_cbor_json(payload.args.as_ref())?,
			},
		}),
	};
	Ok(json!({ "body": body }))
}

fn decode_actor_connect_message(
	payload: &[u8],
	encoding: ActorConnectEncoding,
) -> Result<ActorConnectToServer> {
	match encoding {
		ActorConnectEncoding::Json => {
			let envelope: JsonValue = serde_json::from_slice(payload)
				.context("decode actor websocket json request")?;
			actor_connect_request_from_json_value(&envelope)
		}
		ActorConnectEncoding::Cbor => {
			let envelope: ActorConnectToServerJsonEnvelope =
				ciborium::from_reader(Cursor::new(payload))
					.context("decode actor websocket cbor request")?;
			actor_connect_request_from_json(envelope)
		}
		ActorConnectEncoding::Bare => decode_actor_connect_message_bare(payload),
	}
}

fn actor_connect_request_from_json(
	envelope: ActorConnectToServerJsonEnvelope,
) -> Result<ActorConnectToServer> {
	match envelope.body {
		ActorConnectToServerJsonBody::ActionRequest(request) => {
			Ok(ActorConnectToServer::ActionRequest(ActorConnectActionRequest {
				id: request.id,
				name: request.name,
				args: ByteBuf::from(
					encode_json_as_cbor(&request.args)
						.context("encode actor websocket action request args")?,
				),
			}))
		}
		ActorConnectToServerJsonBody::SubscriptionRequest(request) => {
			Ok(ActorConnectToServer::SubscriptionRequest(request))
		}
	}
}

fn actor_connect_request_from_json_value(envelope: &JsonValue) -> Result<ActorConnectToServer> {
	let body = envelope
		.get("body")
		.and_then(JsonValue::as_object)
		.ok_or_else(|| anyhow!("actor websocket json request missing body"))?;
	let tag = body
		.get("tag")
		.and_then(JsonValue::as_str)
		.ok_or_else(|| anyhow!("actor websocket json request missing tag"))?;
	let value = body
		.get("val")
		.and_then(JsonValue::as_object)
		.ok_or_else(|| anyhow!("actor websocket json request missing val"))?;

	match tag {
		"ActionRequest" => Ok(ActorConnectToServer::ActionRequest(
			ActorConnectActionRequest {
				id: parse_json_compat_u64(
					value
						.get("id")
						.ok_or_else(|| anyhow!("actor websocket json request missing id"))?,
				)?,
				name: value
					.get("name")
					.and_then(JsonValue::as_str)
					.ok_or_else(|| anyhow!("actor websocket json request missing name"))?
					.to_owned(),
				args: ByteBuf::from(encode_json_as_cbor(
					value
						.get("args")
						.ok_or_else(|| anyhow!("actor websocket json request missing args"))?,
				)?),
			},
		)),
		"SubscriptionRequest" => Ok(ActorConnectToServer::SubscriptionRequest(
			ActorConnectSubscriptionRequest {
				event_name: value
					.get("eventName")
					.and_then(JsonValue::as_str)
					.ok_or_else(|| anyhow!("actor websocket json request missing eventName"))?
					.to_owned(),
				subscribe: value
					.get("subscribe")
					.and_then(JsonValue::as_bool)
					.ok_or_else(|| anyhow!("actor websocket json request missing subscribe"))?,
			},
		)),
		other => Err(anyhow!("unknown actor websocket json request tag `{other}`")),
	}
}

fn json_compat_bigint(value: u64) -> JsonValue {
	JsonValue::Array(vec![
		JsonValue::String("$BigInt".to_owned()),
		JsonValue::String(value.to_string()),
	])
}

fn parse_json_compat_u64(value: &JsonValue) -> Result<u64> {
	match value {
		JsonValue::Number(number) => number
			.as_u64()
			.ok_or_else(|| anyhow!("actor websocket json bigint is not an unsigned integer")),
		JsonValue::Array(values) if values.len() == 2 => {
			let tag = values[0]
				.as_str()
				.ok_or_else(|| anyhow!("actor websocket json bigint tag is not a string"))?;
			let raw = values[1]
				.as_str()
				.ok_or_else(|| anyhow!("actor websocket json bigint value is not a string"))?;
			if tag != "$BigInt" {
				return Err(anyhow!("unsupported actor websocket json compat tag `{tag}`"));
			}
			raw.parse::<u64>()
				.context("parse actor websocket json bigint")
		}
		_ => Err(anyhow!("invalid actor websocket json bigint value")),
	}
}

fn encode_actor_connect_message_cbor_manual(
	message: &ActorConnectToClient,
) -> Result<Vec<u8>> {
	let mut encoded = Vec::new();
	cbor_write_map_len(&mut encoded, 1);
	cbor_write_string(&mut encoded, "body");

	match message {
		ActorConnectToClient::Init(payload) => {
			cbor_write_map_len(&mut encoded, 2);
			cbor_write_string(&mut encoded, "tag");
			cbor_write_string(&mut encoded, "Init");
			cbor_write_string(&mut encoded, "val");
			cbor_write_map_len(&mut encoded, 2);
			cbor_write_string(&mut encoded, "actorId");
			cbor_write_string(&mut encoded, &payload.actor_id);
			cbor_write_string(&mut encoded, "connectionId");
			cbor_write_string(&mut encoded, &payload.connection_id);
		}
		ActorConnectToClient::Error(payload) => {
			cbor_write_map_len(&mut encoded, 2);
			cbor_write_string(&mut encoded, "tag");
			cbor_write_string(&mut encoded, "Error");
			cbor_write_string(&mut encoded, "val");
			let mut field_count = 3usize;
			if payload.metadata.is_some() {
				field_count += 1;
			}
			if payload.action_id.is_some() {
				field_count += 1;
			}
			cbor_write_map_len(&mut encoded, field_count);
			cbor_write_string(&mut encoded, "group");
			cbor_write_string(&mut encoded, &payload.group);
			cbor_write_string(&mut encoded, "code");
			cbor_write_string(&mut encoded, &payload.code);
			cbor_write_string(&mut encoded, "message");
			cbor_write_string(&mut encoded, &payload.message);
			if let Some(metadata) = payload.metadata.as_ref() {
				cbor_write_string(&mut encoded, "metadata");
				encoded.extend_from_slice(metadata.as_ref());
			}
			if let Some(action_id) = payload.action_id {
				cbor_write_string(&mut encoded, "actionId");
				cbor_write_u64_force_64(&mut encoded, action_id);
			}
		}
		ActorConnectToClient::ActionResponse(payload) => {
			cbor_write_map_len(&mut encoded, 2);
			cbor_write_string(&mut encoded, "tag");
			cbor_write_string(&mut encoded, "ActionResponse");
			cbor_write_string(&mut encoded, "val");
			cbor_write_map_len(&mut encoded, 2);
			cbor_write_string(&mut encoded, "id");
			cbor_write_u64_force_64(&mut encoded, payload.id);
			cbor_write_string(&mut encoded, "output");
			encoded.extend_from_slice(payload.output.as_ref());
		}
		ActorConnectToClient::Event(payload) => {
			cbor_write_map_len(&mut encoded, 2);
			cbor_write_string(&mut encoded, "tag");
			cbor_write_string(&mut encoded, "Event");
			cbor_write_string(&mut encoded, "val");
			cbor_write_map_len(&mut encoded, 2);
			cbor_write_string(&mut encoded, "name");
			cbor_write_string(&mut encoded, &payload.name);
			cbor_write_string(&mut encoded, "args");
			encoded.extend_from_slice(payload.args.as_ref());
		}
	}

	Ok(encoded)
}

fn decode_actor_connect_message_bare(payload: &[u8]) -> Result<ActorConnectToServer> {
	if payload.len() < 3 {
		return Err(anyhow!("actor websocket payload too short for embedded version"));
	}

	let version = u16::from_le_bytes([payload[0], payload[1]]);
	if !ACTOR_CONNECT_SUPPORTED_VERSIONS.contains(&version) {
		return Err(anyhow!(
			"unsupported actor websocket version {version}; expected one of {:?}",
			ACTOR_CONNECT_SUPPORTED_VERSIONS
		));
	}

	let tag = payload[2];
	let mut cursor = BareCursor::new(&payload[3..]);
	match tag {
		0 => {
			let request = ActorConnectActionRequest {
				id: cursor.read_uint().context("decode actor websocket action request id")?,
				name: cursor
					.read_string()
					.context("decode actor websocket action request name")?,
				args: ByteBuf::from(
					cursor
						.read_bytes()
						.context("decode actor websocket action request args")?,
				),
			};
			cursor.finish().context("decode actor websocket action request")?;
			Ok(ActorConnectToServer::ActionRequest(request))
		}
		1 => {
			let request = ActorConnectSubscriptionRequest {
				event_name: cursor
					.read_string()
					.context("decode actor websocket subscription request event name")?,
				subscribe: cursor
					.read_bool()
					.context("decode actor websocket subscription request subscribe")?,
			};
			cursor
				.finish()
				.context("decode actor websocket subscription request")?;
			Ok(ActorConnectToServer::SubscriptionRequest(request))
		}
		_ => Err(anyhow!("unknown actor websocket request tag {tag}")),
	}
}

struct BareCursor<'a> {
	payload: &'a [u8],
	offset: usize,
}

impl<'a> BareCursor<'a> {
	fn new(payload: &'a [u8]) -> Self {
		Self { payload, offset: 0 }
	}

	fn finish(&self) -> Result<()> {
		if self.offset == self.payload.len() {
			Ok(())
		} else {
			Err(anyhow!(
				"remaining bytes after actor websocket decode: {}",
				self.payload.len() - self.offset
			))
		}
	}

	fn read_byte(&mut self) -> Result<u8> {
		let Some(byte) = self.payload.get(self.offset).copied() else {
			return Err(anyhow!("unexpected end of input"));
		};
		self.offset += 1;
		Ok(byte)
	}

	fn read_bool(&mut self) -> Result<bool> {
		match self.read_byte()? {
			0 => Ok(false),
			1 => Ok(true),
			value => Err(anyhow!("invalid bool value {value}")),
		}
	}

	fn read_uint(&mut self) -> Result<u64> {
		let mut result = 0u64;
		let mut shift = 0u32;
		let mut byte_count = 0u8;

		loop {
			let byte = self.read_byte()?;
			byte_count += 1;

			let value = u64::from(byte & 0x7f);
			result = result
				.checked_add(value << shift)
				.ok_or_else(|| anyhow!("actor websocket uint overflow"))?;

			if byte & 0x80 == 0 {
				if byte_count > 1 && byte == 0 {
					return Err(anyhow!("non-canonical actor websocket uint"));
				}
				return Ok(result);
			}

			shift += 7;
			if shift >= 64 || byte_count >= 10 {
				return Err(anyhow!("actor websocket uint overflow"));
			}
		}
	}

	fn read_len(&mut self) -> Result<usize> {
		let len = self.read_uint()?;
		usize::try_from(len).context("actor websocket length does not fit in usize")
	}

	fn read_bytes(&mut self) -> Result<Vec<u8>> {
		let len = self.read_len()?;
		let end = self
			.offset
			.checked_add(len)
			.ok_or_else(|| anyhow!("actor websocket length overflow"))?;
		let Some(bytes) = self.payload.get(self.offset..end) else {
			return Err(anyhow!("unexpected end of input"));
		};
		self.offset = end;
		Ok(bytes.to_vec())
	}

	fn read_string(&mut self) -> Result<String> {
		String::from_utf8(self.read_bytes()?).context("actor websocket string is not valid utf-8")
	}
}

fn bare_write_uint(buffer: &mut Vec<u8>, mut value: u64) {
	loop {
		let mut byte = (value & 0x7f) as u8;
		value >>= 7;
		if value != 0 {
			byte |= 0x80;
		}
		buffer.push(byte);
		if value == 0 {
			break;
		}
	}
}

fn bare_write_bool(buffer: &mut Vec<u8>, value: bool) {
	buffer.push(u8::from(value));
}

fn bare_write_bytes(buffer: &mut Vec<u8>, value: &[u8]) {
	bare_write_uint(buffer, value.len() as u64);
	buffer.extend_from_slice(value);
}

fn bare_write_string(buffer: &mut Vec<u8>, value: &str) {
	bare_write_bytes(buffer, value.as_bytes());
}

fn bare_write_optional_bytes(buffer: &mut Vec<u8>, value: Option<&[u8]>) {
	bare_write_bool(buffer, value.is_some());
	if let Some(value) = value {
		bare_write_bytes(buffer, value);
	}
}

fn bare_write_optional_uint(buffer: &mut Vec<u8>, value: Option<u64>) {
	bare_write_bool(buffer, value.is_some());
	if let Some(value) = value {
		bare_write_uint(buffer, value);
	}
}

fn cbor_write_type_and_len(buffer: &mut Vec<u8>, major: u8, len: usize) {
	match len {
		0..=23 => buffer.push((major << 5) | (len as u8)),
		24..=0xff => {
			buffer.push((major << 5) | 24);
			buffer.push(len as u8);
		}
		0x100..=0xffff => {
			buffer.push((major << 5) | 25);
			buffer.extend_from_slice(&(len as u16).to_be_bytes());
		}
		0x1_0000..=0xffff_ffff => {
			buffer.push((major << 5) | 26);
			buffer.extend_from_slice(&(len as u32).to_be_bytes());
		}
		_ => {
			buffer.push((major << 5) | 27);
			buffer.extend_from_slice(&(len as u64).to_be_bytes());
		}
	}
}

fn cbor_write_map_len(buffer: &mut Vec<u8>, len: usize) {
	cbor_write_type_and_len(buffer, 5, len);
}

fn cbor_write_string(buffer: &mut Vec<u8>, value: &str) {
	cbor_write_type_and_len(buffer, 3, value.len());
	buffer.extend_from_slice(value.as_bytes());
}

fn cbor_write_u64_force_64(buffer: &mut Vec<u8>, value: u64) {
	buffer.push(0x1b);
	buffer.extend_from_slice(&value.to_be_bytes());
}

fn action_dispatch_error_response(
	error: ActionDispatchError,
	action_id: u64,
) -> ActorConnectError {
	let metadata = error
		.metadata
		.as_ref()
		.and_then(|metadata| encode_json_as_cbor(metadata).ok().map(ByteBuf::from));
	ActorConnectError {
		group: error.group,
		code: error.code,
		message: error.message,
		metadata,
		action_id: Some(action_id),
	}
}

fn closing_websocket_handler(code: u16, reason: &str) -> WebSocketHandler {
	let reason = reason.to_owned();
	WebSocketHandler {
		on_message: Box::new(|_message: WebSocketMessage| Box::pin(async {})),
		on_close: Box::new(|_code, _reason| Box::pin(async {})),
		on_open: Some(Box::new(move |sender| {
			let reason = reason.clone();
			Box::pin(async move {
				sender.close(Some(code), Some(reason));
			})
		})),
	}
}
