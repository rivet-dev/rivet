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
use rivet_envoy_client::envoy::start_envoy_sync;
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
use vbare::Versioned;

use crate::actor::action::ActionDispatchError;
use crate::actor::config::CanHibernateWebSocket;
use crate::actor::connection::{ConnHandle, DisconnectOnDrop, HibernatableConnectionMetadata};
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

const ENVOY_START_TIMEOUT: Duration = Duration::from_secs(30);

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

/// An actor id whose start is in flight on this process.
///
/// A stop that arrives during the start parks in this entry, so checking for the
/// start and parking the stop are one atomic map operation. Only a stop for
/// `generation` parks here; a stop for any other generation is stale and is
/// completed immediately. The start takes the parked stop when it removes the
/// entry, after it has published the instance.
struct StartingInstance {
	generation: u32,
	pending_stop: Option<PendingStop>,
}

/// Outcome of attempting to transition the actor instance under an id to stopping
/// for a specific generation.
enum TransitionResult {
	/// The current instance matches the requested generation and was moved to stopping.
	Transitioned(ActiveActorInstance),
	/// An instance exists but for a different generation than the stop targets, so it
	/// was left untouched. The stop must not be applied to it.
	Stale,
	/// No instance is registered for the actor id.
	Vacant,
}

pub(crate) struct RegistryDispatcher {
	pub(crate) factories: HashMap<String, Arc<ActorFactory>>,
	actor_instances: SccHashMap<String, ActorInstanceState>,
	starting_instances: SccHashMap<String, StartingInstance>,
	pending_stops: SccHashMap<String, PendingStop>,
	region: String,
	handle_inspector_http_in_runtime: bool,
	/// One bounded pool of OS threads runs every native SQLite database of this runtime.
	/// It spawns threads only when databases need them.
	#[cfg(feature = "sqlite-local")]
	sqlite_pool: sqlite_embedded::worker::SqlitePool,
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

/// Only config resolution can opt into managing a local development engine.
/// A supplied endpoint is connect-only, including loopback addresses.
pub(crate) fn should_manage_engine(spawn_mode: EngineSpawnMode) -> bool {
	match spawn_mode {
		EngineSpawnMode::Always => true,
		EngineSpawnMode::Auto | EngineSpawnMode::Never => false,
	}
}

/// Resolves the environment's spawn preference against an explicit engine endpoint.
fn resolve_engine_spawn(
	has_explicit_endpoint: bool,
	spawn_mode: EngineSpawnMode,
) -> Result<EngineSpawnMode> {
	if has_explicit_endpoint {
		if spawn_mode == EngineSpawnMode::Always {
			return Err(EngineSpawnEndpointConflict.build());
		}
		return Ok(EngineSpawnMode::Never);
	}
	Ok(match spawn_mode {
		EngineSpawnMode::Auto | EngineSpawnMode::Always => EngineSpawnMode::Always,
		EngineSpawnMode::Never => EngineSpawnMode::Never,
	})
}

#[derive(RivetError)]
#[error(
	"engine",
	"spawn_endpoint_conflict",
	"Engine spawn conflicts with the configured endpoint",
	"RIVETKIT_ENGINE_SPAWN=always cannot be used with RIVET_ENDPOINT. Remove one setting."
)]
struct EngineSpawnEndpointConflict;



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
		#[cfg(feature = "sqlite-local")]
		sqlite_embedded::vfs::verify_batch_atomic_writes_at_process_startup().await?;

		crate::metrics::record_rivetkit_info(
			config.serverless_package_version.clone(),
			config.version,
			"serverful",
			config.pool_name.clone(),
		);
		#[cfg(not(target_arch = "wasm32"))]
		crate::tokio_runtime_metrics::ensure_sampler_started();

		let dispatcher = self.into_dispatcher(&config);
		let manage_engine = should_manage_engine(config.engine_spawn);
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
		{
			let pool_ready: Result<bool> = tokio::select! {
				biased;
				_ = shutdown.cancelled() => Ok(false),
				result = timeout(ENVOY_START_TIMEOUT, runner_config::ensure_local_normal_runner_config(&config)) => {
					result.context("local node pool configuration timed out").and_then(|result| result).map(|_| true)
				}
			};
			if !matches!(&pool_ready, Ok(true)) {
				if let Some(development_processes) = development_processes {
					development_processes.shutdown().await;
				}
				return pool_ready.map(|_| ());
			}
		}
		let callbacks = Arc::new(RegistryCallbacks {
			dispatcher: dispatcher.clone(),
		});

		let prepopulate_actor_names = dispatcher
			.build_actor_metadata_map()
			.into_iter()
			.map(|(name, metadata)| (name, rivet_envoy_client::config::ActorName { metadata }))
			.collect();
		let handle = start_envoy_sync(rivet_envoy_client::config::EnvoyConfig {
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
			max_queued_requests_per_actor:
				rivet_envoy_client::config::DEFAULT_MAX_QUEUED_REQUESTS_PER_ACTOR,
			stopped_generation_floor_ttl:
				rivet_envoy_client::config::DEFAULT_STOPPED_GENERATION_FLOOR_TTL,
			stopped_generation_floor_capacity:
				rivet_envoy_client::config::DEFAULT_STOPPED_GENERATION_FLOOR_CAPACITY,
			callbacks,
			connector: None,
		});
		let envoy_ready: Result<bool> = tokio::select! {
			biased;
			_ = shutdown.cancelled() => Ok(false),
			result = timeout(ENVOY_START_TIMEOUT, handle.started()) => {
				result.context("serverful envoy startup timed out").and_then(|result| result).map(|_| true)
			}
		};
		if !matches!(&envoy_ready, Ok(true)) {
			handle.shutdown_and_wait(true).await;
			#[cfg(feature = "native-runtime")]
			if let Some(development_processes) = development_processes {
				development_processes.shutdown().await;
			}
			return envoy_ready.map(|_| ());
		}
		on_handle(CoreEnvoyHandle::new(handle.clone()));

		// Do not install `tokio::signal::ctrl_c()` here. It calls
		// `sigaction(SIGINT, ...)` at the POSIX level, which overrides the
		// host's default SIGINT handling when rivetkit-core is embedded in
		// Node via NAPI and leaves the host process unable to exit. Callers
		// trip the `shutdown` token instead.
		let terminal_error = tokio::select! {
			biased;
			_ = shutdown.cancelled() => None,
			_ = handle.wait_stopped() => match handle.terminal_error() {
				Some(error) => Some(error),
				None => {
					shutdown.cancelled().await;
					None
				}
			},
		};

		let shutdown_envoy = async {
			if manage_engine && terminal_error.is_none() {
				// Remove the old host from the engine's active pool before sleeping
				// actors. Otherwise a request racing the stopped report can start the
				// next generation on this host just before it exits. Keep the socket
				// open until the stopped reports have been acknowledged.
				if !handle.mark_stopping().await {
					tracing::warn!("could not mark local development envoy as stopping");
				}
				// A local dev reload replaces this host while keeping its engine. Ask
				// each running actor to sleep so the engine can receive its stopped
				// report and wake it on the new host.
				let mut actors = Vec::new();
				dispatcher
					.actor_instances
					.iter_async(|actor_id, state| {
						if let Some(instance) = state.active_instance() {
							actors.push((actor_id.clone(), instance.generation));
						}
						true
					})
					.await;
				let actor_count = actors.len();
				tracing::info!(
					actors = actor_count,
					"sleeping local development actors for shutdown"
				);
				for (actor_id, generation) in actors {
					handle.sleep_actor(actor_id, Some(generation), None);
				}
				if actor_count > 0 {
					// A stopped report sent just before closing the socket may not
					// reach the engine. Wait for its acknowledgement before exiting.
					let acknowledged = timeout(Duration::from_secs(3), async {
						handle.wait_actors_drained().await;
						handle.wait_for_events_acked().await
					})
					.await;
					if !matches!(acknowledged, Ok(true)) {
						tracing::warn!(
							"local development actor stops were not acknowledged before shutdown"
						);
					}
				}
			}

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
		{
			shutdown_envoy.await;
			shutdown_development_processes.await;
		}
		#[cfg(not(feature = "native-runtime"))]
		shutdown_envoy.await;

		match terminal_error {
			Some(error) => Err(anyhow::anyhow!(error)),
			None => Ok(()),
		}
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
			handle_inspector_http_in_runtime,
			#[cfg(feature = "sqlite-local")]
			sqlite_pool: sqlite_embedded::worker::SqlitePool::from_env(Some(Arc::new(
				crate::actor::sqlite::pool_metrics::PoolMetrics,
			))),
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
		let _ = self
			.starting_instances
			.insert_async(
				request.actor_id.clone(),
				StartingInstance {
					generation: request.generation,
					pending_stop: None,
				},
			)
			.await;
		// Test-only seam: lets a test hold a generation in the "starting" window so it
		// can deterministically deliver a stop for a previous generation that parks
		// under the actor id and gets consumed by this startup.
		
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
		let join = RuntimeSpawner::spawn(task.run());

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
				// Publish the instance before leaving the starting window. A stop that
				// no longer finds the starting entry then finds this instance instead.
				self.set_actor_instance_state(
					request.actor_id.clone(),
					ActorInstanceState::Active(instance),
				)
				.await;
				let parked_during_start = self
					.starting_instances
					.remove_async(&request.actor_id)
					.await
					.and_then(|(_, starting)| starting.pending_stop);
				let parked_before_start = self
					.pending_stops
					.remove_async(&request.actor_id)
					.await
					.map(|(_, pending_stop)| pending_stop);

				// Only apply a parked stop if it targets the generation we just started.
				// A stop parked for a previous generation is stale: complete its handle
				// so teardown finalizes, but leave the new generation running.
				let mut current_generation_stop = None;
				for pending_stop in [parked_before_start, parked_during_start]
					.into_iter()
					.flatten()
				{
					if pending_stop.generation == request.generation
						&& current_generation_stop.is_none()
					{
						current_generation_stop = Some(pending_stop);
					} else {
						let _ = pending_stop.stop_handle.complete();
					}
				}

				if let Some(pending_stop) = current_generation_stop {
					self.apply_stop_queued_during_startup(&request.actor_id, pending_stop)
						.await;
				}
				Ok(())
			}
			Err(error) => {
				// A stop parked while this start was in flight would otherwise leak its
				// ActorStopHandle in the map (the map holds the sender alive, hanging the
				// caller). Drain and complete it since there is no instance to stop.
				if let Some((_, starting)) = self
					.starting_instances
					.remove_async(&request.actor_id)
					.await && let Some(pending_stop) = starting.pending_stop
				{
					let _ = pending_stop.stop_handle.complete();
				}
				if let Some((_, pending_stop)) =
					self.pending_stops.remove_async(&request.actor_id).await
				{
					let _ = pending_stop.stop_handle.complete();
				}
				Err(error)
			}
		}
	}

	/// Stops a just-started instance for a stop that targeted its generation
	/// before it was published. Shutdown runs on its own task so startup returns.
	async fn apply_stop_queued_during_startup(
		self: &Arc<Self>,
		actor_id: &str,
		pending_stop: PendingStop,
	) {
		let stop_reason = map_envoy_stop_reason(&pending_stop.reason);
		let instance = match self
			.transition_actor_to_stopping(actor_id, pending_stop.generation, stop_reason)
			.await
		{
			TransitionResult::Transitioned(instance) => instance,
			TransitionResult::Stale | TransitionResult::Vacant => {
				// The instance was replaced or removed after it was published, so
				// there is nothing left for this stop to tear down.
				let _ = pending_stop.stop_handle.complete();
				return;
			}
		};

		let dispatcher = self.clone();
		let actor_id = actor_id.to_owned();
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
				.remove_stopping_actor_instance(&actor_id, &instance)
				.await;
		});
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

	async fn transition_actor_to_stopping(
		&self,
		actor_id: &str,
		generation: u32,
		reason: ShutdownKind,
	) -> TransitionResult {
		match self.actor_instances.entry_async(actor_id.to_owned()).await {
			SccEntry::Occupied(mut entry) => {
				let instance = entry.get().instance();
				// A stop is scoped to the generation it was issued for. If the currently
				// registered instance is a different generation (e.g. a lost previous
				// generation whose replacement is already running), the stop is stale and
				// must not tear down the newer generation.
				if instance.generation != generation {
					return TransitionResult::Stale;
				}
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

	async fn remove_stopping_actor_instance(&self, actor_id: &str, expected: &ActiveActorInstance) {
		match self.actor_instances.entry_async(actor_id.to_owned()).await {
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
					// TODO: Share admission policy with ActorTask::dispatch_lifecycle_error.
					if instance.ctx.started() {
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
					} else if instance.ctx.sleep_requested() {
						ActorLifecycleError::Stopping.build()
					} else {
						ActorLifecycleError::Starting.build()
					});
				}
				ActorInstanceState::Stopping { instance, reason } => {
					let instance = instance.clone();
					match reason {
						ShutdownKind::Sleep if instance.ctx.started() => return Ok(instance),
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
		&self,
		actor_id: &str,
		generation: u32,
		reason: protocol::StopActorReason,
		stop_handle: ActorStopHandle,
	) -> Result<()> {
		let mut pending_stop = PendingStop {
			generation,
			reason,
			stop_handle,
		};
		loop {
			// While a start is in flight, park the stop in its entry so startup can
			// decide whether it belongs to the generation being started.
			match self
				.starting_instances
				.entry_async(actor_id.to_owned())
				.await
			{
				SccEntry::Occupied(mut entry) => {
					let starting = entry.get_mut();
					let rejected = if starting.generation != pending_stop.generation {
						// The stop targets a generation other than the one being
						// started, so it is stale. It must not displace a parked stop
						// for the starting generation.
						Some(pending_stop)
					} else if starting.pending_stop.is_some() {
						// A stop for this generation is already parked and will be
						// applied by startup. Keep the first one.
						Some(pending_stop)
					} else {
						starting.pending_stop = Some(pending_stop);
						None
					};
					drop(entry);

					if let Some(rejected) = rejected {
						let _ = rejected.stop_handle.complete();
					}
					return Ok(());
				}
				SccEntry::Vacant(entry) => {
					drop(entry);
				}
			}

			let task_stop_reason = map_envoy_stop_reason(&pending_stop.reason);
			match self
				.transition_actor_to_stopping(actor_id, pending_stop.generation, task_stop_reason)
				.await
			{
				TransitionResult::Transitioned(instance) => {
					let result = self
						.shutdown_started_instance(
							actor_id,
							instance.clone(),
							pending_stop.reason,
							pending_stop.stop_handle,
						)
						.await;
					self.remove_stopping_actor_instance(actor_id, &instance)
						.await;
					return result;
				}
				TransitionResult::Stale => {
					// The running instance is a different generation; this stop targets a
					// generation that is already gone. Complete the handle so envoy-client
					// finalizes teardown cleanly instead of warning about a dropped handle.
					let _ = pending_stop.stop_handle.complete();
					return Ok(());
				}
				TransitionResult::Vacant => {}
			}

			

			// No instance is registered, so park the stop for the next start of this
			// actor id. A start publishes its instance before it drains this map, so
			// if one drained the map before this insert, its instance is visible now
			// and the stop is reclaimed and retried against it.
			if self
				.pending_stops
				.insert_async(actor_id.to_owned(), pending_stop)
				.await
				.is_err()
			{
				return Ok(());
			}
			if !self
				.actor_instances
				.contains_async(&actor_id.to_owned())
				.await
			{
				return Ok(());
			}
			let Some((_, reclaimed)) = self
				.pending_stops
				.remove_if_async(&actor_id.to_owned(), |parked| {
					parked.generation == generation
				})
				.await
			else {
				// A start drained the stop concurrently and now owns it.
				return Ok(());
			};
			pending_stop = reclaimed;
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

impl RegistryDispatcher {
	fn can_hibernate(&self, actor_id: &str, request: &HttpRequest) -> bool {
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
	) -> Result<ActorContext> {
		let formatted_key = format_actor_key(&key);
		#[cfg_attr(not(feature = "sqlite-local"), allow(unused_mut))]
		let mut sql = SqliteDb::new_with_remote_sqlite(
			handle.clone(),
			actor_id.to_owned(),
			Some(formatted_key),
			Some(generation as u64),
			factory.config().has_database,
			factory.config().remote_sqlite,
		)?;
		#[cfg(feature = "sqlite-local")]
		sql.set_worker_pool(self.sqlite_pool.clone());
		let ctx = ActorContext::build(
			actor_id.to_owned(),
			actor_name.to_owned(),
			key,
			self.region.clone(),
			Some(generation),
			handle.get_envoy_key().to_owned(),
			factory.config().clone(),
			LegacyActorKv::new(handle.clone(), actor_id.to_owned()),
			sql,
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


// Test-only hooks used by the moved registry tests to deterministically drive the
// generation-stop race. Gated behind `cfg(test)` so there is no production impact.

