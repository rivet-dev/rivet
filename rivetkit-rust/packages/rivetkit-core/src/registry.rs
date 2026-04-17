use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
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
use scc::HashMap as SccHashMap;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::actor::callbacks::{OnRequestRequest, OnWebSocketRequest, Request, Response};
use crate::actor::config::CanHibernateWebSocket;
use crate::actor::context::ActorContext;
use crate::actor::factory::ActorFactory;
use crate::actor::lifecycle::{ActorLifecycle, StartupOptions};
use crate::actor::state::{PERSIST_DATA_KEY, PersistedActor, decode_persisted_actor};
use crate::kv::Kv;
use crate::sqlite::SqliteDb;
use crate::types::{ActorKey, ActorKeySegment};
use crate::websocket::WebSocket;

#[derive(Debug, Default)]
pub struct CoreRegistry {
	factories: HashMap<String, Arc<ActorFactory>>,
}

#[derive(Clone)]
struct ActiveActorInstance {
	actor_name: String,
	generation: u32,
	ctx: ActorContext,
	factory: Arc<ActorFactory>,
	callbacks: Arc<crate::actor::callbacks::ActorInstanceCallbacks>,
}

struct RegistryDispatcher {
	factories: HashMap<String, Arc<ActorFactory>>,
	active_instances: SccHashMap<String, ActiveActorInstance>,
	region: String,
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
}

#[derive(Clone, Debug)]
pub struct ServeConfig {
	pub version: u32,
	pub endpoint: String,
	pub token: Option<String>,
	pub namespace: String,
	pub pool_name: String,
	pub engine_binary_path: Option<PathBuf>,
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
		let dispatcher = self.into_dispatcher();
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

	fn into_dispatcher(self) -> Arc<RegistryDispatcher> {
		Arc::new(RegistryDispatcher {
			factories: self.factories,
			active_instances: SccHashMap::new(),
			region: env::var("RIVET_REGION").unwrap_or_default(),
		})
	}
}

impl RegistryDispatcher {
	async fn start_actor(&self, request: StartActorRequest) -> Result<()> {
		let factory = self
			.factories
			.get(&request.actor_name)
			.cloned()
			.ok_or_else(|| anyhow!("actor factory `{}` is not registered", request.actor_name))?;
		let lifecycle = ActorLifecycle;
		let outcome = lifecycle
			.startup(
				request.ctx.clone(),
				factory.as_ref(),
				StartupOptions {
					preload_persisted_actor: request.preload_persisted_actor,
					input: request.input,
					..StartupOptions::default()
				},
			)
			.await
			.map_err(|error| error.into_source())
			.with_context(|| format!("start actor `{}`", request.actor_id))?;

		let instance = ActiveActorInstance {
			actor_name: request.actor_name,
			generation: request.generation,
			ctx: request.ctx,
			factory,
			callbacks: outcome.callbacks,
		};
		let _ = self
			.active_instances
			.insert_async(request.actor_id.clone(), instance)
			.await;

		Ok(())
	}

	async fn active_actor(&self, actor_id: &str) -> Result<ActiveActorInstance> {
		let Some(instance) = self.active_instances.get_async(&actor_id.to_owned()).await else {
			tracing::warn!(actor_id, "actor instance not found");
			return Err(anyhow!("actor instance `{actor_id}` was not found"));
		};

		Ok(instance.get().clone())
	}

	async fn stop_actor(
		&self,
		actor_id: &str,
		reason: protocol::StopActorReason,
		stop_handle: ActorStopHandle,
	) -> Result<()> {
		let instance = match self.active_actor(actor_id).await {
			Ok(instance) => instance,
			Err(error) => {
				let _ = stop_handle.complete();
				return Err(error);
			}
		};
		let _ = self.active_instances.remove_async(&actor_id.to_owned()).await;
		tracing::debug!(
			actor_id,
			actor_name = %instance.actor_name,
			generation = instance.generation,
			?reason,
			"stopping actor instance"
		);

		let lifecycle = ActorLifecycle;
		let shutdown_result = match reason {
			protocol::StopActorReason::SleepIntent => {
				lifecycle
					.shutdown_for_sleep(
						instance.ctx.clone(),
						instance.factory.as_ref(),
						instance.callbacks.clone(),
					)
					.await
			}
			_ => {
				lifecycle
					.shutdown_for_destroy(
						instance.ctx.clone(),
						instance.factory.as_ref(),
						instance.callbacks.clone(),
					)
					.await
			}
		};

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
		let Some(callback) = instance.callbacks.on_request.as_ref() else {
			return Ok(HttpResponse {
				status: http::StatusCode::NOT_FOUND.as_u16(),
				headers: HashMap::new(),
				body: Some(Vec::new()),
				body_stream: None,
			});
		};

		let request = build_http_request(request).await?;
		match callback(OnRequestRequest {
			ctx: instance.ctx.clone(),
			request,
		})
		.await
		{
			Ok(response) => build_envoy_response(response),
			Err(error) => {
				tracing::error!(actor_id, ?error, "actor request callback failed");
				Ok(internal_server_error_response())
			}
		}
	}

	async fn handle_websocket(
		&self,
		actor_id: &str,
		sender: WebSocketSender,
	) -> Result<WebSocketHandler> {
		let instance = self.active_actor(actor_id).await?;
		let Some(callback) = instance.callbacks.on_websocket.as_ref() else {
			return Ok(default_websocket_handler());
		};

		let ws = WebSocket::from_sender(sender);
		let result = instance
			.ctx
			.with_websocket_callback(|| async {
				callback(OnWebSocketRequest {
					ctx: instance.ctx.clone(),
					ws,
				})
				.await
			})
			.await;

		match result {
			Ok(()) => Ok(default_websocket_handler()),
			Err(error) => {
				tracing::error!(actor_id, ?error, "actor websocket callback failed");
				Err(error)
			}
		}
	}

	fn can_hibernate(&self, actor_id: &str, request: &HttpRequest) -> bool {
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

	#[cfg(test)]
	async fn start_actor_for_test(
		&self,
		actor_id: &str,
		generation: u32,
		actor_name: &str,
		input: Option<Vec<u8>>,
	) -> Result<()> {
		let factory = self
			.factories
			.get(actor_name)
			.cloned()
			.ok_or_else(|| anyhow!("actor factory `{actor_name}` is not registered"))?;
		let ctx = ActorContext::new_runtime(
			actor_id.to_owned(),
			actor_name.to_owned(),
			actor_key_from_protocol(None),
			self.region.clone(),
			factory.config().clone(),
			Kv::new_in_memory(),
			SqliteDb::default(),
		);
		self.start_actor(StartActorRequest {
			actor_id: actor_id.to_owned(),
			generation,
			actor_name: actor_name.to_owned(),
			input,
			preload_persisted_actor: None,
			ctx,
		})
		.await
	}

	#[cfg(test)]
	async fn handle_websocket_for_test(&self, actor_id: &str) -> Result<()> {
		let instance = self.active_actor(actor_id).await?;
		let Some(callback) = instance.callbacks.on_websocket.as_ref() else {
			return Ok(());
		};

		instance
			.ctx
			.with_websocket_callback(|| async {
				callback(OnWebSocketRequest {
					ctx: instance.ctx.clone(),
					ws: WebSocket::new(),
				})
				.await
			})
			.await
	}

	#[cfg(test)]
	async fn stop_actor_for_test(
		&self,
		actor_id: &str,
		reason: protocol::StopActorReason,
	) -> Result<()> {
		let instance = self.active_actor(actor_id).await?;
		let _ = self.active_instances.remove_async(actor_id).await;

		let lifecycle = ActorLifecycle;
		match reason {
			protocol::StopActorReason::SleepIntent => {
				lifecycle
					.shutdown_for_sleep(
						instance.ctx.clone(),
						instance.factory.as_ref(),
						instance.callbacks.clone(),
					)
					.await?;
			}
			_ => {
				lifecycle
					.shutdown_for_destroy(
						instance.ctx.clone(),
						instance.factory.as_ref(),
						instance.callbacks.clone(),
					)
					.await?;
			}
		}

		Ok(())
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
		Box::pin(async move { dispatcher.handle_websocket(&actor_id, sender).await })
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
	key.map(|value| vec![ActorKeySegment::String(value)])
		.unwrap_or_default()
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

async fn build_http_request(request: HttpRequest) -> Result<Request> {
	let mut body = request.body.unwrap_or_default();
	if let Some(mut body_stream) = request.body_stream {
		while let Some(chunk) = body_stream.recv().await {
			body.extend_from_slice(&chunk);
		}
	}

	let mut builder = http::Request::builder()
		.method(
			request
				.method
				.parse::<http::Method>()
				.with_context(|| format!("parse request method `{}`", request.method))?,
		)
		.uri(
			request
				.path
				.parse::<http::Uri>()
				.with_context(|| format!("parse request path `{}`", request.path))?,
		);
	for (name, value) in request.headers {
		builder = builder.header(name, value);
	}

	builder.body(body).context("build actor request")
}

fn build_envoy_response(response: Response) -> Result<HttpResponse> {
	let status = response.status().as_u16();
	let mut headers = HashMap::new();
	for (name, value) in response.headers() {
		headers.insert(
			name.to_string(),
			value
				.to_str()
				.context("convert response header to utf-8")?
				.to_owned(),
		);
	}

	Ok(HttpResponse {
		status,
		headers,
		body: Some(response.into_body()),
		body_stream: None,
	})
}

fn internal_server_error_response() -> HttpResponse {
	HttpResponse {
		status: http::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
		headers: HashMap::new(),
		body: Some(Vec::new()),
		body_stream: None,
	}
}

fn default_websocket_handler() -> WebSocketHandler {
	WebSocketHandler {
		on_message: Box::new(|_message: WebSocketMessage| Box::pin(async {})),
		on_close: Box::new(|_code, _reason| Box::pin(async {})),
		on_open: None,
	}
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;
	use std::process::Stdio;
	use std::sync::Arc;
	use std::sync::atomic::{AtomicBool, Ordering};

	use anyhow::Result;
	use futures::future::BoxFuture;
	use rivet_envoy_client::config::HttpRequest;
	use rivet_envoy_client::protocol;
	use tokio::io::AsyncWriteExt;
	use tokio::net::TcpListener;
	use tokio::process::Command;

	use super::{
		CoreRegistry, RegistryDispatcher, engine_health_url, terminate_engine_process,
		wait_for_engine_health,
	};
	use crate::actor::callbacks::{
		ActorInstanceCallbacks, LifecycleCallback, OnRequestRequest, OnWebSocketRequest,
		RequestCallback,
	};
	use crate::actor::factory::{ActorFactory, FactoryRequest};
	use crate::ActorConfig;

	fn request_callback<F>(callback: F) -> RequestCallback
	where
		F: Fn(OnRequestRequest) -> BoxFuture<'static, Result<super::Response>>
			+ Send
			+ Sync
			+ 'static,
	{
		Box::new(callback)
	}

	fn lifecycle_callback<F, T>(callback: F) -> LifecycleCallback<T>
	where
		F: Fn(T) -> BoxFuture<'static, Result<()>> + Send + Sync + 'static,
		T: Send + 'static,
	{
		Box::new(callback)
	}

	fn factory<F>(build: F) -> ActorFactory
	where
		F: Fn(FactoryRequest) -> BoxFuture<'static, Result<ActorInstanceCallbacks>>
			+ Send
			+ Sync
			+ 'static,
	{
		ActorFactory::new(ActorConfig::default(), build)
	}

	fn dispatcher_for(factory: ActorFactory) -> Arc<RegistryDispatcher> {
		let mut registry = CoreRegistry::new();
		registry.register("counter", factory);
		registry.into_dispatcher()
	}

	#[tokio::test]
	async fn dispatcher_routes_fetch_to_started_actor() {
		let dispatcher = dispatcher_for(factory(|_request| {
			Box::pin(async move {
				let mut callbacks = ActorInstanceCallbacks::default();
				callbacks.on_request = Some(request_callback(|request| {
					Box::pin(async move {
						let response = http::Response::builder()
							.status(http::StatusCode::CREATED)
							.body(request.request.into_body())
							.expect("build response");
						Ok(response)
					})
				}));
				Ok(callbacks)
			})
		}));

		dispatcher
			.start_actor_for_test("actor-1", 1, "counter", Some(b"seed".to_vec()))
			.await
			.expect("start actor");

		let response = dispatcher
			.handle_fetch(
				"actor-1",
				HttpRequest {
					method: "POST".to_owned(),
					path: "/".to_owned(),
					headers: HashMap::new(),
					body: Some(b"ping".to_vec()),
					body_stream: None,
				},
			)
			.await
			.expect("fetch should succeed");

		assert_eq!(response.status, http::StatusCode::CREATED.as_u16());
		assert_eq!(response.body, Some(b"ping".to_vec()));
	}

	#[tokio::test]
	async fn dispatcher_routes_websocket_to_started_actor() {
		let invoked = Arc::new(AtomicBool::new(false));
		let invoked_clone = invoked.clone();
		let dispatcher = dispatcher_for(factory(move |_request| {
			let invoked = invoked_clone.clone();
			Box::pin(async move {
				let mut callbacks = ActorInstanceCallbacks::default();
				callbacks.on_websocket = Some(lifecycle_callback(
					move |_request: OnWebSocketRequest| {
						let invoked = invoked.clone();
						Box::pin(async move {
							invoked.store(true, Ordering::SeqCst);
							Ok(())
						})
					},
				));
				Ok(callbacks)
			})
		}));

		dispatcher
			.start_actor_for_test("actor-1", 1, "counter", None)
			.await
			.expect("start actor");
		dispatcher
			.handle_websocket_for_test("actor-1")
			.await
			.expect("websocket should succeed");

		assert!(invoked.load(Ordering::SeqCst));
	}

	#[tokio::test]
	async fn dispatcher_stops_actor_and_removes_it_from_active_map() {
		let dispatcher = dispatcher_for(factory(|_request| {
			Box::pin(async move { Ok(ActorInstanceCallbacks::default()) })
		}));

		dispatcher
			.start_actor_for_test("actor-1", 1, "counter", None)
			.await
			.expect("start actor");
		dispatcher
			.stop_actor_for_test("actor-1", protocol::StopActorReason::Destroy)
			.await
			.expect("stop actor");

		assert!(dispatcher.active_instances.get_async(&"actor-1".to_owned()).await.is_none());
	}

	#[tokio::test]
	async fn dispatcher_returns_error_for_unknown_actor_fetch() {
		let dispatcher = dispatcher_for(factory(|_request| {
			Box::pin(async move { Ok(ActorInstanceCallbacks::default()) })
		}));

		let result = dispatcher
			.handle_fetch(
				"missing",
				HttpRequest {
					method: "GET".to_owned(),
					path: "/".to_owned(),
					headers: HashMap::new(),
					body: None,
					body_stream: None,
				},
			)
			.await;
		let error = match result {
			Ok(_) => panic!("missing actor should error"),
			Err(error) => error,
		};

		assert!(error.to_string().contains("missing"));
	}

	#[tokio::test]
	async fn engine_health_check_retries_until_success() {
		let listener = TcpListener::bind("127.0.0.1:0")
			.await
			.expect("bind health listener");
		let address = listener.local_addr().expect("health listener addr");
		let server = tokio::spawn(async move {
			for attempt in 0..3 {
				let (mut stream, _) = listener.accept().await.expect("accept health request");
				let mut request = [0u8; 1024];
				let _ = stream.readable().await;
				let _ = stream.try_read(&mut request);

				if attempt < 2 {
					stream
						.write_all(
							b"HTTP/1.1 503 Service Unavailable\r\ncontent-length: 0\r\n\r\n",
						)
						.await
						.expect("write unhealthy response");
				} else {
					stream
						.write_all(
							b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 51\r\n\r\n{\"status\":\"ok\",\"runtime\":\"engine\",\"version\":\"test\"}",
						)
						.await
						.expect("write healthy response");
				}
			}
		});

		let health = wait_for_engine_health(&engine_health_url(&format!("http://{address}")))
			.await
			.expect("wait for engine health");
		server.await.expect("join health server");

		assert_eq!(health.runtime.as_deref(), Some("engine"));
		assert_eq!(health.version.as_deref(), Some("test"));
	}

	#[tokio::test]
	#[cfg(unix)]
	async fn terminate_engine_process_prefers_sigterm() {
		let mut child = Command::new("sh")
			.arg("-c")
			.arg("trap 'exit 0' TERM; while true; do sleep 1; done")
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.spawn()
			.expect("spawn looping shell");

		terminate_engine_process(&mut child)
			.await
			.expect("terminate child process");

		assert!(child.try_wait().expect("inspect child").is_some());
	}
}
