use std::collections::HashMap;
use std::env;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::time::{Instant, timeout};

use ::http::StatusCode;
use anyhow::{Context, Result};
use parking_lot::Mutex;
use rivet_envoy_client::config::{
	ActorStopHandle, BoxFuture as EnvoyBoxFuture, EnvoyCallbacks, HttpRequest, HttpResponse,
	WebSocketHandler, WebSocketMessage, WebSocketSender,
};
use rivet_envoy_client::envoy::start_envoy;
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_client::protocol;
use rivet_error::{ActorSpecifier, RivetError};
use rivetkit_client_protocol as client_protocol;
use rivetkit_shared_types::serverless_metadata::{
	ActorName, ServerlessMetadataEnvoy, ServerlessMetadataEnvoyKind, ServerlessMetadataPayload,
};
use scc::{HashMap as SccHashMap, hash_map::Entry as SccEntry};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use serde_json::{Value as JsonValue, json};
use tokio::sync::{Mutex as TokioMutex, broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use url::Url;
use vbare::OwnedVersionedData;

use crate::actor::action::ActionDispatchError;
use crate::actor::config::CanHibernateWebSocket;
use crate::actor::connection::{ConnHandle, HibernatableConnectionMetadata};
use crate::actor::context::{ActorContext, InspectorAttachGuard};
use crate::actor::factory::ActorFactory;
use crate::actor::kv::LegacyActorKv;
use crate::actor::lifecycle_hooks::Reply;
use crate::actor::messages::{ActorEvent, ActorHttpResponse, QueueSendResult, Request, StateDelta};
use crate::actor::task::{
	ActorTask, DispatchCommand, LifecycleCommand, try_send_dispatch_command,
	try_send_lifecycle_command,
};
use crate::actor::task_types::ShutdownKind;
#[cfg(feature = "native-runtime")]
use crate::development_process::DevelopmentProcessManager;
use crate::error::{ActorLifecycle as ActorLifecycleError, ActorRuntime};
use crate::inspector::protocol::{
	self as inspector_protocol, ServerMessage as InspectorServerMessage,
};
use crate::inspector::{Inspector, InspectorAuth, InspectorSignal, InspectorSubscription};
use crate::runtime::RuntimeSpawner;
use crate::sqlite::SqliteDb;
use crate::types::{ActorKey, ActorKeySegment, WsMessage, format_actor_key};
use crate::websocket::WebSocket;

mod actor_connect;
mod dispatch;
mod envoy_callbacks;
mod http;
mod inspector;
mod inspector_ws;
#[cfg(feature = "native-runtime")]
mod runner_config;
mod websocket;

use inspector::build_actor_inspector;
use websocket::is_actor_connect_path;

#[derive(Default)]
pub struct CoreRegistry {
	factories: HashMap<String, Arc<ActorFactory>>,
}

#[derive(Clone)]
pub struct CoreEnvoyHandle {
	handle: EnvoyHandle,
}

#[derive(Clone, Debug)]
pub struct CoreEnvoyStatus {
	pub active_actor_count: usize,
	pub ping_healthy: bool,
}

impl CoreEnvoyHandle {
	pub(crate) fn new(handle: EnvoyHandle) -> Self {
		Self { handle }
	}

	pub fn status(&self) -> CoreEnvoyStatus {
		CoreEnvoyStatus {
			active_actor_count: self.handle.active_actor_count(),
			ping_healthy: self.handle.is_ping_healthy(),
		}
	}

	/// Resolves after the Engine sends the envoy initialization message.
	pub async fn started(&self) -> anyhow::Result<()> {
		self.handle.started().await
	}

	/// Resolves once the envoy has no active actors (or has stopped).
	pub async fn wait_actors_drained(&self) {
		self.handle.wait_actors_drained().await
	}

	/// Engine-reported drain threshold in milliseconds. `None` until the
	/// envoy has completed its first protocol-metadata exchange with the
	/// engine.
	pub async fn actor_stop_threshold_ms(&self) -> Option<i64> {
		self.handle
			.get_protocol_metadata()
			.await
			.map(|metadata| metadata.actor_stop_threshold)
	}
}

#[derive(Clone)]
struct ActorTaskHandle {
	actor_id: String,
	actor_name: String,
	generation: u32,
	ctx: ActorContext,
	factory: Arc<ActorFactory>,
	inspector: Inspector,
	lifecycle: mpsc::UnboundedSender<LifecycleCommand>,
	dispatch: mpsc::UnboundedSender<DispatchCommand>,
	join: Arc<TokioMutex<Option<JoinHandle<Result<()>>>>>,
}

type ActiveActorInstance = Arc<ActorTaskHandle>;

enum ActorInstanceState {
	Active(ActiveActorInstance),
	Stopping {
		instance: ActiveActorInstance,
		reason: ShutdownKind,
	},
}

impl ActorInstanceState {
	fn instance(&self) -> ActiveActorInstance {
		match self {
			Self::Active(instance) | Self::Stopping { instance, .. } => instance.clone(),
		}
	}

	fn active_instance(&self) -> Option<ActiveActorInstance> {
		match self {
			Self::Active(instance) => Some(instance.clone()),
			Self::Stopping { .. } => None,
		}
	}
}

#[derive(Clone)]
struct PendingStop {
	generation: u32,
	reason: protocol::StopActorReason,
	stop_handle: ActorStopHandle,
}

/// Identifies one generation of an actor. Registry records are per generation so a lost
/// generation that is still shutting down stays reachable after the next generation starts.
type InstanceKey = (String, u32);

fn instance_key(actor_id: &str, generation: u32) -> InstanceKey {
	(actor_id.to_owned(), generation)
}

/// Outcome of attempting to transition one actor generation to stopping.
enum TransitionResult {
	/// The generation was registered and is now stopping.
	Transitioned(ActiveActorInstance),
	/// No instance is registered for the generation.
	Vacant,
}

pub(crate) struct RegistryDispatcher {
	pub(crate) factories: HashMap<String, Arc<ActorFactory>>,
	actor_instances: SccHashMap<InstanceKey, ActorInstanceState>,
	/// Newest registered generation per actor id. Dispatch routes only to this generation.
	current_generations: SccHashMap<String, u32>,
	starting_instances: SccHashMap<InstanceKey, ActorContext>,
	pending_stops: SccHashMap<InstanceKey, PendingStop>,
	region: String,
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
	start_services: bool,
	services_binary_path: Option<PathBuf>,
	engine_host: Option<String>,
	engine_port: Option<u16>,
	engine_spawn: EngineSpawnMode,
	engine_auto_download: bool,
	handle_inspector_http_in_runtime: bool,
	serverless_base_path: Option<String>,
	serverless_package_version: String,
	serverless_client_endpoint: Option<String>,
	serverless_client_namespace: Option<String>,
	serverless_client_token: Option<String>,
	serverless_validate_endpoint: bool,
	serverless_max_start_payload_bytes: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EngineSpawnMode {
	#[default]
	Auto,
	Always,
	Never,
}

impl EngineSpawnMode {
	pub(crate) fn from_env() -> Self {
		match env::var("RIVETKIT_ENGINE_SPAWN") {
			Ok(value) if value.eq_ignore_ascii_case("always") => Self::Always,
			Ok(value) if value.eq_ignore_ascii_case("never") => Self::Never,
			_ => Self::Auto,
		}
	}
}

/// Selects how `Registry::start` runs, mirroring the TypeScript
/// `RIVETKIT_RUNTIME_MODE` env var. `Envoy` holds one long-lived outbound
/// envoy for the process lifetime; `Serverless` runs an HTTP listener that
/// lazily starts and caches an envoy on the first request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RuntimeMode {
	#[default]
	Envoy,
	Serverless,
}

impl RuntimeMode {
	pub fn from_env() -> Self {
		match env::var("RIVETKIT_RUNTIME_MODE") {
			Ok(value) if value.eq_ignore_ascii_case("serverless") => Self::Serverless,
			_ => Self::Envoy,
		}
	}
}

#[derive(Clone, Debug, Default)]
pub struct ServeConfig {
	pub version: u32,
	pub endpoint: String,
	pub token: Option<String>,
	pub namespace: String,
	pub pool_name: String,
	pub engine_binary_path: Option<PathBuf>,
	pub start_services: bool,
	pub services_binary_path: Option<PathBuf>,
	pub engine_host: Option<String>,
	pub engine_port: Option<u16>,
	pub engine_spawn: EngineSpawnMode,
	pub engine_auto_download: bool,
	pub handle_inspector_http_in_runtime: bool,
	pub serverless_base_path: Option<String>,
	pub serverless_package_version: String,
	pub serverless_client_endpoint: Option<String>,
	pub serverless_client_namespace: Option<String>,
	pub serverless_client_token: Option<String>,
	pub serverless_validate_endpoint: bool,
	pub serverless_max_start_payload_bytes: usize,
	pub serverless_cache_envoy: bool,
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
	properties: Option<JsonValue>,
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

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct InspectorEnqueueBody {
	name: String,
	body: Option<JsonValue>,
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

pub(crate) fn should_manage_engine(endpoint: &str, spawn_mode: EngineSpawnMode) -> Result<bool> {
	match spawn_mode {
		EngineSpawnMode::Always => Ok(true),
		EngineSpawnMode::Never => Ok(false),
		EngineSpawnMode::Auto => is_loopback_endpoint(endpoint),
	}
}

fn is_loopback_endpoint(endpoint: &str) -> Result<bool> {
	let url =
		Url::parse(endpoint).with_context(|| format!("parse engine endpoint `{endpoint}`"))?;
	let Some(host) = url.host_str() else {
		anyhow::bail!("engine endpoint `{endpoint}` is invalid: missing host");
	};

	if host == "localhost" || host.ends_with(".localhost") {
		return Ok(true);
	}

	let ip_host = host
		.strip_prefix('[')
		.and_then(|value| value.strip_suffix(']'))
		.unwrap_or(host);

	Ok(ip_host
		.parse::<std::net::IpAddr>()
		.map(|ip| ip.is_loopback() || ip.is_unspecified())
		.unwrap_or(false))
}

#[cfg(test)]
mod engine_spawn_tests {
	use super::{EngineSpawnMode, should_manage_engine};

	#[test]
	fn auto_manages_loopback_endpoints() {
		assert!(should_manage_engine("http://127.0.0.1:6420", EngineSpawnMode::Auto).unwrap());
		assert!(should_manage_engine("http://localhost:6420", EngineSpawnMode::Auto).unwrap());
		assert!(should_manage_engine("http://dev.localhost:6420", EngineSpawnMode::Auto).unwrap());
		assert!(should_manage_engine("http://[::1]:6420", EngineSpawnMode::Auto).unwrap());
	}

	#[test]
	fn auto_leaves_remote_endpoints_connect_only() {
		assert!(!should_manage_engine("https://api.rivet.dev", EngineSpawnMode::Auto).unwrap());
		assert!(!should_manage_engine("http://192.0.2.10:6420", EngineSpawnMode::Auto).unwrap());
	}

	#[test]
	fn explicit_spawn_mode_overrides_endpoint_shape() {
		assert!(should_manage_engine("https://api.rivet.dev", EngineSpawnMode::Always).unwrap());
		assert!(!should_manage_engine("http://127.0.0.1:6420", EngineSpawnMode::Never).unwrap());
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

/// How long a new generation waits for older generations of the same actor on this runner to
/// finish. Lost generations abort within two seconds, so this only expires if an older task is
/// stuck, and the new generation then fails to start rather than overlap it.
const OLDER_GENERATION_STOP_TIMEOUT: Duration = Duration::from_secs(10);

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
	actor: Option<ActorSpecifier>,
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

	pub fn normal_metadata_payload(&self, config: &ServeConfig) -> ServerlessMetadataPayload {
		serverless_metadata_payload(
			build_actor_metadata_map_from_factories(&self.factories),
			config,
			ServerlessMetadataEnvoyKind::Normal {},
		)
	}

	pub fn serverless_metadata_payload(&self, config: &ServeConfig) -> ServerlessMetadataPayload {
		serverless_metadata_payload(
			build_actor_metadata_map_from_factories(&self.factories),
			config,
			ServerlessMetadataEnvoyKind::Serverless {},
		)
	}

	pub async fn serve(self, shutdown: CancellationToken) -> Result<()> {
		self.serve_with_config(ServeConfig::from_env()?, shutdown)
			.await
	}

	pub async fn serve_with_config(
		self,
		config: ServeConfig,
		shutdown: CancellationToken,
	) -> Result<()> {
		self.serve_with_config_and_handle_observer(config, shutdown, |_| {})
			.await
	}

	pub async fn serve_with_config_and_handle_observer(
		self,
		config: ServeConfig,
		shutdown: CancellationToken,
		on_handle: impl FnOnce(CoreEnvoyHandle) + Send + 'static,
	) -> Result<()> {
		crate::metrics::record_rivetkit_info(
			config.serverless_package_version.clone(),
			config.version,
			"serverful",
			config.pool_name.clone(),
		);
		#[cfg(not(target_arch = "wasm32"))]
		crate::tokio_runtime_metrics::ensure_sampler_started();

		let dispatcher = self.into_dispatcher(&config);
		let manage_engine = should_manage_engine(&config.endpoint, config.engine_spawn)?;
		#[cfg(feature = "native-runtime")]
		let development_processes = if manage_engine {
			Some(DevelopmentProcessManager::start(&config).await?)
		} else {
			None
		};
		#[cfg(not(feature = "native-runtime"))]
		if manage_engine {
			anyhow::bail!("engine process spawning requires the `native-runtime` feature");
		}

		#[cfg(feature = "native-runtime")]
		runner_config::ensure_local_normal_runner_config(&config).await?;
		let callbacks = Arc::new(RegistryCallbacks {
			dispatcher: dispatcher.clone(),
		});

		let prepopulate_actor_names = dispatcher
			.build_actor_metadata_map()
			.into_iter()
			.map(|(name, metadata)| (name, rivet_envoy_client::config::ActorName { metadata }))
			.collect();
		let handle = start_envoy(rivet_envoy_client::config::EnvoyConfig {
			version: config.version,
			endpoint: config.endpoint,
			token: config.token,
			namespace: config.namespace,
			pool_name: config.pool_name,
			prepopulate_actor_names,
			metadata: Some(json!({
				"rivetkit": { "version": config.serverless_package_version },
			})),
			not_global: false,
			debug_latency_ms: None,
			callbacks,
		})
		.await;
		on_handle(CoreEnvoyHandle::new(handle.clone()));

		// Do not install `tokio::signal::ctrl_c()` here. It calls
		// `sigaction(SIGINT, ...)` at the POSIX level, which overrides the
		// host's default SIGINT handling when rivetkit-core is embedded in
		// Node via NAPI and leaves the host process unable to exit. Callers
		// trip the `shutdown` token instead.
		shutdown.cancelled().await;

		let shutdown_envoy = async {
			// TODO: Move into envoy-client since timing out has to do with protocol compliance
			// Read threshold from protocol metadata, fall back to 30 min
			let stop_threshold = handle
				.get_protocol_metadata()
				.await
				.map(|x| x.actor_stop_threshold)
				.unwrap_or(30 * 60 * 1000);
			// Bounded drain. If envoy cannot reach the engine (reconnect loop stuck),
			// we fall back to immediate `Stop` rather than hanging indefinitely.
			// The outer host (TS signal handler / Rust binary) is the backstop.
			match timeout(
				Duration::from_millis(stop_threshold as u64),
				handle.shutdown_and_wait(false),
			)
			.await
			{
				Ok(()) => {}
				Err(_) => {
					tracing::warn!("envoy shutdown drain exceeded timeout; forcing immediate stop");
					handle.shutdown(true);
					handle.wait_stopped().await;
				}
			}
		};
		#[cfg(feature = "native-runtime")]
		let shutdown_development_processes = async move {
			if let Some(development_processes) = development_processes {
				development_processes.shutdown().await;
			}
		};
		#[cfg(feature = "native-runtime")]
		tokio::join!(shutdown_envoy, shutdown_development_processes);
		#[cfg(not(feature = "native-runtime"))]
		shutdown_envoy.await;

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
			current_generations: SccHashMap::new(),
			starting_instances: SccHashMap::new(),
			pending_stops: SccHashMap::new(),
			region: env::var("RIVET_REGION").unwrap_or_default(),
			handle_inspector_http_in_runtime,
		}
	}

	pub(crate) fn build_actor_metadata_map(&self) -> HashMap<String, JsonValue> {
		build_actor_metadata_map_from_factories(&self.factories)
	}
}

pub(crate) fn serverless_metadata_payload(
	actor_metadata: HashMap<String, JsonValue>,
	config: &ServeConfig,
	envoy_kind: ServerlessMetadataEnvoyKind,
) -> ServerlessMetadataPayload {
	let actor_names = actor_metadata
		.into_iter()
		.map(|(name, metadata)| {
			(
				name,
				ActorName {
					metadata: Some(metadata),
				},
			)
		})
		.collect::<HashMap<_, _>>();

	ServerlessMetadataPayload {
		runtime: "rivetkit".to_owned(),
		version: config.serverless_package_version.clone(),
		envoy_protocol_version: Some(protocol::PROTOCOL_VERSION),
		actor_names,
		envoy: Some(ServerlessMetadataEnvoy {
			kind: Some(envoy_kind),
			version: Some(config.version),
		}),
		runner: None,
		client_endpoint: config.serverless_client_endpoint.clone(),
		client_namespace: config.serverless_client_namespace.clone(),
		client_token: config.serverless_client_token.clone(),
	}
}

fn build_actor_metadata_map_from_factories(
	factories: &HashMap<String, Arc<ActorFactory>>,
) -> HashMap<String, JsonValue> {
	factories
		.iter()
		.map(|(actor_name, factory)| {
			let config = factory.config();
			let mut metadata = serde_json::Map::new();
			if let Some(icon) = &config.icon {
				metadata.insert("icon".to_owned(), json!(icon));
			}
			if let Some(name) = &config.name {
				metadata.insert("name".to_owned(), json!(name));
			}
			(actor_name.clone(), JsonValue::Object(metadata))
		})
		.collect()
}

impl RegistryDispatcher {
	async fn start_actor(self: &Arc<Self>, request: StartActorRequest) -> Result<()> {
		let key = instance_key(&request.actor_id, request.generation);
		// The actor task owns this hold. It is released when the task ends, or when startup is
		// abandoned before a task exists.
		let generation_hold = request.ctx.hold_generation();
		let _ = self
			.starting_instances
			.insert_async(key.clone(), request.ctx.clone())
			.await;
		let prepared = async {
			self.wait_for_older_generations(&request.actor_id, request.generation)
				.await?;
			// Test-only seam: lets a test hold a generation in the "starting" window so it
			// can deterministically deliver stops while the generation is starting.
			#[cfg(test)]
			test_hooks::wait_for_startup_gate(&request.actor_id).await;
			if request.ctx.is_lost() {
				return Err(ActorLifecycleError::Stopping.build())
					.context("actor generation was declared lost before it started");
			}
			self.factories
				.get(&request.actor_name)
				.cloned()
				.ok_or_else(|| {
					ActorRuntime::NotRegistered {
						actor_name: request.actor_name.clone(),
					}
					.build()
				})
		}
		.await;
		let factory = match prepared {
			Ok(factory) => factory,
			Err(error) => {
				self.abandon_startup(&key).await;
				return Err(error);
			}
		};
		let (lifecycle_tx, lifecycle_rx) = mpsc::unbounded_channel();
		let (dispatch_tx, dispatch_rx) = mpsc::unbounded_channel();
		let (lifecycle_events_tx, lifecycle_events_rx) = mpsc::unbounded_channel();
		request
			.ctx
			.configure_lifecycle_events(Some(lifecycle_events_tx));
		request.ctx.cancel_sleep_timer();
		request.ctx.set_local_alarm_callback(Some(Arc::new({
			let lifecycle_tx = lifecycle_tx.clone();
			move || {
				let lifecycle_tx = lifecycle_tx.clone();
				Box::pin(async move {
					let (reply_tx, reply_rx) = oneshot::channel();
					if let Err(error) = try_send_lifecycle_command(
						&lifecycle_tx,
						LifecycleCommand::FireAlarm { reply: reply_tx },
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
		);
		let join = RuntimeSpawner::spawn(async move {
			let _generation_hold = generation_hold;
			task.run().await
		});

		let (start_tx, start_rx) = oneshot::channel();
		let result: Result<Arc<ActorTaskHandle>> = async {
			try_send_lifecycle_command(&lifecycle_tx, LifecycleCommand::Start { reply: start_tx })
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
				// Hold the starting entry while consuming the parked stop and registering the
				// instance. `stop_actor` parks under the same entry, so a stop either parks
				// before this point and is consumed here, or finds the registered instance.
				let starting = self.starting_instances.entry_async(key.clone()).await;
				let pending_stop = self
					.pending_stops
					.remove_async(&key)
					.await
					.map(|(_, pending_stop)| pending_stop);
				match pending_stop {
					Some(pending_stop) => {
						let actor_id = request.actor_id.clone();
						let stop_reason = map_envoy_stop_reason(&pending_stop.reason);
						if matches!(stop_reason, ShutdownKind::Destroy) {
							instance.ctx.mark_destroy_requested();
						}
						self.register_actor_instance(
							key.clone(),
							ActorInstanceState::Stopping {
								instance: instance.clone(),
								reason: stop_reason,
							},
						)
						.await;
						remove_starting_entry(starting);

						let dispatcher = self.clone();
						RuntimeSpawner::spawn(async move {
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
								.remove_stopping_actor_instance_when_finished(&actor_id, &instance)
								.await;
						});
					}
					None => {
						self.register_actor_instance(
							key.clone(),
							ActorInstanceState::Active(instance),
						)
						.await;
						remove_starting_entry(starting);
					}
				}
				self.complete_stops_for_older_generations(&request.actor_id, request.generation)
					.await;
				Ok(())
			}
			Err(error) => {
				// A generation lost during startup drops its start reply while its task is still
				// aborting. Keep it registered as starting until the task finishes so envoy-client
				// does not report it stopped, and a newer generation does not start, too early.
				if request.ctx.is_lost()
					&& timeout(
						OLDER_GENERATION_STOP_TIMEOUT,
						request.ctx.wait_for_generation_finished(),
					)
					.await
					.is_err()
				{
					tracing::warn!(
						actor_id = %request.actor_id,
						generation = request.generation,
						"lost actor generation did not finish aborting its startup in time, likely stuck storage; keeping it registered until it finishes"
					);
					// Keep the starting record so newer generations still see this one and
					// refuse to start until it really finishes.
					let dispatcher = self.clone();
					let ctx = request.ctx.clone();
					RuntimeSpawner::spawn(async move {
						ctx.wait_for_generation_finished().await;
						dispatcher.abandon_startup(&key).await;
					});
					return Err(error);
				}
				self.abandon_startup(&key).await;
				Err(error)
			}
		}
	}

	/// Removes a generation whose startup failed. A stop parked while the start was in flight
	/// would otherwise leak its ActorStopHandle in the map and hang the caller, so complete it
	/// since there is no instance to stop.
	async fn abandon_startup(&self, key: &InstanceKey) {
		let starting = self.starting_instances.entry_async(key.clone()).await;
		if let Some((_, pending_stop)) = self.pending_stops.remove_async(key).await {
			let _ = pending_stop.stop_handle.complete();
		}
		remove_starting_entry(starting);
	}

	/// Registers a started generation and makes it the dispatch target unless a newer
	/// generation is already registered.
	async fn register_actor_instance(&self, key: InstanceKey, state: ActorInstanceState) {
		let (actor_id, generation) = key.clone();
		self.set_actor_instance_state(key, state).await;
		match self.current_generations.entry_async(actor_id).await {
			SccEntry::Occupied(mut entry) => {
				if *entry.get() < generation {
					entry.insert(generation);
				}
			}
			SccEntry::Vacant(entry) => {
				entry.insert_entry(generation);
			}
		}
	}

	/// Completes stops parked for generations older than `generation`. Generations only
	/// increase, so such a generation will never start and its stop has nothing left to do.
	async fn complete_stops_for_older_generations(&self, actor_id: &str, generation: u32) {
		let mut stale_stops = Vec::new();
		self.pending_stops
			.retain_async(|(pending_actor_id, pending_generation), pending_stop| {
				if pending_actor_id == actor_id && *pending_generation < generation {
					stale_stops.push(pending_stop.stop_handle.clone());
					false
				} else {
					true
				}
			})
			.await;
		for stop_handle in stale_stops {
			let _ = stop_handle.complete();
		}
	}

	async fn newer_generation_exists(&self, actor_id: &str, generation: u32) -> bool {
		let is_newer = |(other_actor_id, other_generation): &InstanceKey| {
			other_actor_id == actor_id && *other_generation > generation
		};
		// Each check releases its entry before the next one runs.
		let newer_starting = self
			.starting_instances
			.any_async(|key, _| is_newer(key))
			.await
			.is_some();
		if newer_starting {
			return true;
		}
		self.actor_instances
			.any_async(|key, _| is_newer(key))
			.await
			.is_some()
	}

	/// Contexts of older generations of `actor_id` that are starting or registered on this
	/// runner.
	async fn older_generation_contexts(
		&self,
		actor_id: &str,
		generation: u32,
	) -> Vec<ActorContext> {
		let mut older = Vec::new();
		self.starting_instances
			.iter_async(|(starting_actor_id, starting_generation), ctx| {
				if starting_actor_id == actor_id && *starting_generation < generation {
					older.push(ctx.clone());
				}
				true
			})
			.await;
		self.actor_instances
			.iter_async(|(instance_actor_id, instance_generation), state| {
				if instance_actor_id == actor_id && *instance_generation < generation {
					older.push(state.instance().ctx.clone());
				}
				true
			})
			.await;
		older
	}

	/// Keeps two generations of one actor from running on this runner at once. The engine only
	/// starts a generation after giving up on every older one, so older generations still here
	/// are lost: mark them lost so they abort, then wait for their tasks to finish. If one does
	/// not finish in time, fail this start rather than overlap it.
	async fn wait_for_older_generations(&self, actor_id: &str, generation: u32) -> Result<()> {
		// Start callbacks run in independent tasks, so a newer generation can register first.
		// Generations only increase, so this one has already been superseded and must not run.
		// Both starts insert their starting record before scanning, so at least one sees the
		// other.
		if self.newer_generation_exists(actor_id, generation).await {
			tracing::warn!(
				actor_id,
				generation,
				"refusing to start an actor generation that a newer generation already superseded"
			);
			return Err(ActorLifecycleError::Stopping.build())
				.context("a newer generation of this actor is already running on this runner");
		}
		let older = self.older_generation_contexts(actor_id, generation).await;
		if older.is_empty() {
			return Ok(());
		}
		for ctx in &older {
			if !ctx.is_lost() {
				tracing::warn!(
					actor_id,
					generation,
					"newer actor generation is starting while an older one still runs; marking the older generation lost"
				);
				ctx.mark_lost();
			}
		}
		let all_finished = async {
			for ctx in &older {
				ctx.wait_for_generation_finished().await;
			}
		};
		if timeout(OLDER_GENERATION_STOP_TIMEOUT, all_finished)
			.await
			.is_err()
		{
			tracing::error!(
				actor_id,
				generation,
				"an older actor generation is still running on this runner, likely a stuck task; refusing to start the new generation"
			);
			return Err(ActorLifecycleError::Stopping.build())
				.context("an older generation of this actor is still running on this runner");
		}
		Ok(())
	}

	async fn current_generation(&self, actor_id: &str) -> Option<u32> {
		self.current_generations
			.read_async(actor_id, |_, generation| *generation)
			.await
	}

	async fn set_actor_instance_state(&self, key: InstanceKey, state: ActorInstanceState) {
		match self.actor_instances.entry_async(key).await {
			SccEntry::Occupied(mut entry) => {
				entry.insert(state);
			}
			SccEntry::Vacant(entry) => {
				entry.insert_entry(state);
			}
		}
	}

	async fn transition_actor_to_stopping(
		&self,
		actor_id: &str,
		generation: u32,
		reason: ShutdownKind,
	) -> TransitionResult {
		match self
			.actor_instances
			.entry_async(instance_key(actor_id, generation))
			.await
		{
			SccEntry::Occupied(mut entry) => {
				let instance = entry.get().instance();
				if matches!(entry.get(), ActorInstanceState::Active(_)) {
					entry.insert(ActorInstanceState::Stopping {
						instance: instance.clone(),
						reason,
					});
				} else {
					instance
						.ctx
						.warn_work_sent_to_stopping_instance("stop_actor");
				}
				TransitionResult::Transitioned(instance)
			}
			SccEntry::Vacant(entry) => {
				drop(entry);
				TransitionResult::Vacant
			}
		}
	}

	/// Removes a stopped generation's record once the generation has finished. A lost
	/// generation's storage cleanup can outlive its task, and the record must stay visible to
	/// newer generations until it does.
	async fn remove_stopping_actor_instance_when_finished(
		self: &Arc<Self>,
		actor_id: &str,
		expected: &ActiveActorInstance,
	) {
		if expected.ctx.is_generation_finished() {
			self.remove_stopping_actor_instance(actor_id, expected)
				.await;
			return;
		}
		let dispatcher = self.clone();
		let actor_id = actor_id.to_owned();
		let expected = expected.clone();
		RuntimeSpawner::spawn(async move {
			expected.ctx.wait_for_generation_finished().await;
			dispatcher
				.remove_stopping_actor_instance(&actor_id, &expected)
				.await;
		});
	}

	async fn remove_stopping_actor_instance(&self, actor_id: &str, expected: &ActiveActorInstance) {
		let removed = match self
			.actor_instances
			.entry_async(instance_key(actor_id, expected.generation))
			.await
		{
			SccEntry::Occupied(entry) => {
				let should_remove = match entry.get() {
					ActorInstanceState::Stopping { instance, .. } => {
						Arc::ptr_eq(instance, expected)
					}
					ActorInstanceState::Active(_) => false,
				};
				if should_remove {
					let _ = entry.remove_entry();
				}
				should_remove
			}
			SccEntry::Vacant(entry) => {
				drop(entry);
				false
			}
		};
		if removed {
			// Only clear the dispatch target if no newer generation has replaced it.
			let _ = self
				.current_generations
				.remove_if_async(actor_id, |generation| *generation == expected.generation)
				.await;
		}
	}

	async fn active_actor(&self, actor_id: &str) -> Result<Arc<ActorTaskHandle>> {
		let instance = match self.current_generation(actor_id).await {
			Some(generation) => {
				self.actor_instances
					.get_async(&instance_key(actor_id, generation))
					.await
			}
			None => None,
		};
		if let Some(instance) = instance {
			match instance.get() {
				ActorInstanceState::Active(instance) => {
					let instance = instance.clone();
					// TODO: Share admission policy with ActorTask::dispatch_lifecycle_error.
					if instance.ctx.started() && !instance.ctx.is_lost() {
						if instance.ctx.destroy_requested() {
							instance
								.ctx
								.warn_work_sent_to_stopping_instance("active_actor");
							return Err(ActorLifecycleError::Destroying.build());
						}
						return Ok(instance);
					}

					instance
						.ctx
						.warn_work_sent_to_stopping_instance("active_actor");
					return Err(if instance.ctx.destroy_requested() {
						ActorLifecycleError::Destroying.build()
					} else if instance.ctx.sleep_requested() || instance.ctx.is_lost() {
						ActorLifecycleError::Stopping.build()
					} else {
						ActorLifecycleError::Starting.build()
					});
				}
				ActorInstanceState::Stopping { instance, reason } => {
					let instance = instance.clone();
					match reason {
						// A lost generation takes no new work; the engine routes it to the next
						// generation.
						ShutdownKind::Sleep
							if instance.ctx.started() && !instance.ctx.is_lost() =>
						{
							return Ok(instance);
						}
						ShutdownKind::Sleep => {
							instance
								.ctx
								.warn_work_sent_to_stopping_instance("active_actor");
							return Err(ActorLifecycleError::Stopping.build());
						}
						ShutdownKind::Destroy => {
							instance
								.ctx
								.warn_work_sent_to_stopping_instance("active_actor");
							return Err(ActorLifecycleError::Destroying.build());
						}
					}
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
		self: &Arc<Self>,
		actor_id: &str,
		generation: u32,
		reason: protocol::StopActorReason,
		stop_handle: ActorStopHandle,
	) -> Result<()> {
		let key = instance_key(actor_id, generation);
		let pending_stop = PendingStop {
			generation,
			reason,
			stop_handle,
		};
		let pending_stop = match self.starting_instances.entry_async(key.clone()).await {
			SccEntry::Occupied(starting) => {
				// The generation is still starting. Park the stop under the starting entry so
				// its startup consumes it.
				self.park_stop(key, pending_stop).await;
				drop(starting);
				return Ok(());
			}
			SccEntry::Vacant(starting) => {
				drop(starting);
				pending_stop
			}
		};
		let PendingStop {
			reason,
			stop_handle,
			..
		} = pending_stop;

		let task_stop_reason = map_envoy_stop_reason(&reason);
		match self
			.transition_actor_to_stopping(actor_id, generation, task_stop_reason)
			.await
		{
			TransitionResult::Transitioned(instance) => {
				let result = self
					.shutdown_started_instance(actor_id, instance.clone(), reason, stop_handle)
					.await;
				self.remove_stopping_actor_instance_when_finished(actor_id, &instance)
					.await;
				result
			}
			TransitionResult::Vacant => {
				match self.current_generation(actor_id).await {
					Some(current) if current > generation => {
						// A newer generation is registered, so this generation already finished
						// tearing down. Complete the handle so envoy-client finalizes the stop.
						let _ = stop_handle.complete();
					}
					Some(_) | None => {
						// The stop can arrive before its generation's start reaches the
						// registry. Park it for that startup.
						self.park_stop(
							key,
							PendingStop {
								generation,
								reason,
								stop_handle,
							},
						)
						.await;
					}
				}
				Ok(())
			}
		}
	}

	async fn park_stop(&self, key: InstanceKey, pending_stop: PendingStop) {
		if let Err((_, duplicate)) = self.pending_stops.insert_async(key, pending_stop).await {
			// envoy-client sends one stop per generation. A duplicate adds nothing, so complete
			// it rather than dropping its handle.
			tracing::warn!(
				generation = duplicate.generation,
				"duplicate stop parked for an actor generation"
			);
			let _ = duplicate.stop_handle.complete();
		}
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
		if matches!(reason, protocol::StopActorReason::Lost) {
			// envoy-client normally fires the lost signal before this stop arrives. Marking it
			// here as well covers stops that reach core by other paths.
			instance.ctx.mark_lost();
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
			LifecycleCommand::Stop {
				reason: task_stop_reason,
				reply: reply_tx,
			},
		) {
			Ok(()) => reply_rx
				.await
				.context("receive actor task stop reply")
				.and_then(|result| result),
			Err(error) => Err(error),
		};
		// A lost generation aborts on its lost signal without waiting for this stop, so the task
		// may already have exited and dropped the command. Its outcome is its join result below.
		let shutdown_result = match shutdown_result {
			Err(error) if instance.ctx.is_lost() => {
				tracing::debug!(
					actor_id,
					%error,
					"lost actor task finished before its stop command was handled"
				);
				Ok(())
			}
			result => result,
		};

		if matches!(task_stop_reason, ShutdownKind::Destroy) {
			let shutdown_deadline =
				Instant::now() + instance.factory.config().effective_sleep_grace_period();
			if !instance
				.ctx
				.wait_for_internal_keep_awake_idle(shutdown_deadline)
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
				.wait_for_http_requests_drained(shutdown_deadline)
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
		// Fold the join outcome into the result instead of propagating with `?`,
		// so the stop handle is always signaled and never dropped unsignaled.
		let join_result = if let Some(join) = join_guard.take() {
			join.await
				.context("join actor task")
				.and_then(|result| result.context("actor task failed"))
		} else {
			Ok(())
		};
		instance.ctx.configure_lifecycle_events(None);

		if let (Err(shutdown_error), Err(join_error)) = (&shutdown_result, &join_result) {
			tracing::warn!(
				actor_id,
				%shutdown_error,
				discarded_join_error = %join_error,
				"actor stop had both shutdown and join failures; only the shutdown error is returned"
			);
		}

		let final_result = shutdown_result.and(join_result);
		match &final_result {
			Ok(_) => {
				let _ = stop_handle.complete();
			}
			Err(error) => {
				let _ = stop_handle.fail(anyhow::Error::new(RivetError::extract(error)));
			}
		}

		final_result.with_context(|| format!("stop actor `{actor_id}`"))
	}
}

fn remove_starting_entry(starting: SccEntry<'_, InstanceKey, ActorContext>) {
	match starting {
		SccEntry::Occupied(entry) => {
			let _ = entry.remove_entry();
		}
		SccEntry::Vacant(entry) => {
			drop(entry);
		}
	}
}

impl RegistryDispatcher {
	fn can_hibernate(&self, actor_id: &str, request: &HttpRequest) -> bool {
		if matches!(is_actor_connect_path(&request.path), Ok(true)) {
			return true;
		}

		let Some(generation) = self
			.current_generations
			.read_sync(actor_id, |_, generation| *generation)
		else {
			return false;
		};
		let Some(instance) = self
			.actor_instances
			.read_sync(&instance_key(actor_id, generation), |_, state| {
				state.active_instance()
			})
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
	) -> Result<ActorContext> {
		let formatted_key = format_actor_key(&key);
		let ctx = ActorContext::build(
			actor_id.to_owned(),
			actor_name.to_owned(),
			key,
			self.region.clone(),
			Some(generation),
			handle.get_envoy_key().to_owned(),
			factory.config().clone(),
			LegacyActorKv::new(handle.clone(), actor_id.to_owned()),
			SqliteDb::new_with_remote_sqlite(
				handle.clone(),
				actor_id.to_owned(),
				Some(formatted_key),
				Some(generation as u64),
				factory.config().has_database,
				factory.config().remote_sqlite,
			)?,
		);
		ctx.configure_envoy(handle, Some(generation));
		Ok(ctx)
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

// Test shim keeps moved tests in crate-root tests/ with private-module access.
#[cfg(test)]
#[path = "../../tests/registry.rs"]
pub(crate) mod tests;

// Test-only hooks used by the moved registry tests to deterministically drive the
// generation-stop race. Gated behind `cfg(test)` so there is no production impact.
#[cfg(test)]
pub(crate) mod test_hooks {
	use std::sync::{Arc, OnceLock};

	use scc::HashMap as SccHashMap;
	use tokio::sync::Semaphore;

	static STARTUP_GATES: OnceLock<SccHashMap<String, Arc<Semaphore>>> = OnceLock::new();

	fn gates() -> &'static SccHashMap<String, Arc<Semaphore>> {
		STARTUP_GATES.get_or_init(SccHashMap::new)
	}

	/// Arms a gate so `start_actor` pauses for `actor_id` after registering as
	/// starting, until `release_startup_gate` is called.
	pub(crate) fn arm_startup_gate(actor_id: &str) {
		let _ = gates().insert_sync(actor_id.to_owned(), Arc::new(Semaphore::new(0)));
	}

	/// Releases a previously armed gate, letting the paused `start_actor` continue.
	pub(crate) fn release_startup_gate(actor_id: &str) {
		if let Some(sem) = gates().read_sync(actor_id, |_, sem| sem.clone()) {
			sem.add_permits(1);
		}
	}

	/// Called from inside `start_actor`. Blocks only if a gate is armed for the
	/// actor. Order-independent: a release before this runs still lets it through.
	pub(crate) async fn wait_for_startup_gate(actor_id: &str) {
		let sem = gates().read_sync(actor_id, |_, sem| sem.clone());
		if let Some(sem) = sem {
			let permit = sem.acquire().await.expect("startup gate semaphore closed");
			permit.forget();
			let _ = gates().remove_sync(actor_id);
		}
	}
}
