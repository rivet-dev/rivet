use std::collections::HashMap;
use std::io::Cursor;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Stdio;
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
use serde_json::{Value as JsonValue, json};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::actor::action::{ActionDispatchError, ActionInvoker};
use crate::actor::callbacks::{ActionRequest, OnRequestRequest, OnWebSocketRequest, Request, Response};
use crate::actor::callbacks::{GetWorkflowHistoryRequest, ReplayWorkflowRequest};
use crate::actor::config::CanHibernateWebSocket;
use crate::actor::context::ActorContext;
use crate::actor::factory::ActorFactory;
use crate::actor::lifecycle::{ActorLifecycle, StartupOptions};
use crate::actor::state::{PERSIST_DATA_KEY, PersistedActor, decode_persisted_actor};
use crate::inspector::Inspector;
use crate::kv::Kv;
use crate::sqlite::SqliteDb;
use crate::types::{ActorKey, ActorKeySegment, SaveStateOpts};
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
	inspector: Inspector,
}

struct RegistryDispatcher {
	factories: HashMap<String, Arc<ActorFactory>>,
	active_instances: SccHashMap<String, ActiveActorInstance>,
	region: String,
	inspector_token: Option<String>,
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
	is_workflow_enabled: bool,
	workflow_history: Option<JsonValue>,
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
			inspector_token: env::var("RIVET_INSPECTOR_TOKEN")
				.ok()
				.filter(|token| !token.is_empty()),
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
		let inspector =
			build_actor_inspector(request.ctx.clone(), outcome.callbacks.clone());
		request.ctx.configure_inspector(Some(inspector.clone()));

		let instance = ActiveActorInstance {
			actor_name: request.actor_name,
			generation: request.generation,
			ctx: request.ctx,
			factory,
			callbacks: outcome.callbacks,
			inspector,
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
		if request.path == "/metrics" {
			return self.handle_metrics_fetch(&instance, &request);
		}
		let request = build_http_request(request).await?;
		if let Some(response) = self.handle_inspector_fetch(&instance, &request).await? {
			return Ok(response);
		}
		let Some(callback) = instance.callbacks.on_request.as_ref() else {
			return Ok(not_found_response());
		};

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

	async fn handle_inspector_fetch(
		&self,
		instance: &ActiveActorInstance,
		request: &Request,
	) -> Result<Option<HttpResponse>> {
		let url = inspector_request_url(request)?;
		if !url.path().starts_with("/inspector/") {
			return Ok(None);
		}
		if !request_has_inspector_access(request, self.inspector_token.as_deref()) {
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
				instance.ctx.set_state(encode_json_as_cbor(&body.state)?);
				match instance
					.ctx
					.save_state(SaveStateOpts { immediate: true })
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
				.and_then(|(is_workflow_enabled, history)| {
					json_http_response(
						StatusCode::OK,
						&json!({
							"history": history,
							"isWorkflowEnabled": is_workflow_enabled,
						}),
					)
				}),
			(http::Method::POST, "/inspector/workflow/replay") => {
				let body: InspectorWorkflowReplayBody = match parse_json_body(request) {
					Ok(body) => body,
					Err(response) => return Ok(Some(response)),
				};
				self
					.inspector_replay_workflow(instance, body.entry_id)
					.await
					.and_then(|(is_workflow_enabled, history)| {
						json_http_response(
							StatusCode::OK,
							&json!({
								"history": history,
								"isWorkflowEnabled": is_workflow_enabled,
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
		instance: &ActiveActorInstance,
		action_name: &str,
		args: Vec<JsonValue>,
	) -> std::result::Result<JsonValue, ActionDispatchError> {
		let conn = match instance
			.ctx
			.connect_conn(Vec::new(), false, None, async { Ok(Vec::new()) })
			.await
		{
			Ok(conn) => conn,
			Err(error) => return Err(ActionDispatchError::from_anyhow(error)),
		};
		let invoker = ActionInvoker::with_shared_callbacks(
			instance.factory.config().clone(),
			instance.callbacks.clone(),
		);
		let output = invoker
			.dispatch(ActionRequest {
				ctx: instance.ctx.clone(),
				conn: conn.clone(),
				name: action_name.to_owned(),
				args: encode_json_as_cbor(&args).map_err(ActionDispatchError::from_anyhow)?,
			})
			.await;
		if let Err(error) = conn.disconnect(None).await {
			tracing::warn!(?error, action_name, "failed to disconnect inspector action connection");
		}
		output.map(|payload| decode_cbor_json_or_null(&payload))
	}

	async fn inspector_summary(
		&self,
		instance: &ActiveActorInstance,
	) -> Result<InspectorSummaryJson> {
		let queue_messages = instance
			.ctx
			.queue()
			.inspect_messages()
			.await
			.context("list queue messages for inspector summary")?;
		let (is_workflow_enabled, workflow_history) = self
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
			is_workflow_enabled,
			workflow_history,
		})
	}

	async fn inspector_workflow_history(
		&self,
		instance: &ActiveActorInstance,
	) -> Result<(bool, Option<JsonValue>)> {
		let is_workflow_enabled = instance.inspector.is_workflow_enabled();
		if !is_workflow_enabled {
			return Ok((false, None));
		}

		let history = instance
			.inspector
			.get_workflow_history()
			.await
			.context("load inspector workflow history")?
			.map(|payload| decode_cbor_json_or_null(&payload))
			.filter(|value| !value.is_null());

		Ok((true, history))
	}

	async fn inspector_replay_workflow(
		&self,
		instance: &ActiveActorInstance,
		entry_id: Option<String>,
	) -> Result<(bool, Option<JsonValue>)> {
		let is_workflow_enabled = instance.inspector.is_workflow_enabled();
		if !is_workflow_enabled {
			return Ok((false, None));
		}

		let history = instance
			.inspector
			.replay_workflow(entry_id)
			.await
			.context("replay inspector workflow history")?
			.map(|payload| decode_cbor_json_or_null(&payload))
			.filter(|value| !value.is_null());

		Ok((true, history))
	}

	async fn inspector_database_schema(&self, ctx: &ActorContext) -> Result<JsonValue> {
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
			return Ok(json!({ "tables": [] }));
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

		Ok(json!({ "tables": inspector_tables }))
	}

	async fn inspector_database_rows(
		&self,
		ctx: &ActorContext,
		table: &str,
		limit: u32,
		offset: u32,
	) -> Result<JsonValue> {
		let params = encode_json_as_cbor(&vec![json!(limit.min(500)), json!(offset)])?;
		let rows = ctx
			.db_query(
				&format!(
					"SELECT * FROM {} LIMIT ? OFFSET ?",
					quote_sql_identifier(table)
				),
				Some(&params),
			)
			.await
			.with_context(|| format!("query rows for `{table}`"))?;
		Ok(decode_cbor_json_or_null(&rows))
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
		instance: &ActiveActorInstance,
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

fn inspector_connections(ctx: &ActorContext) -> Vec<InspectorConnectionJson> {
	ctx
		.conns()
		.into_iter()
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

fn build_actor_inspector(
	ctx: ActorContext,
	callbacks: Arc<crate::actor::callbacks::ActorInstanceCallbacks>,
) -> Inspector {
	let get_workflow_history = callbacks.get_workflow_history.as_ref().map(|_| {
		let callbacks = callbacks.clone();
		let ctx = ctx.clone();
		Arc::new(move || -> futures::future::BoxFuture<'static, Result<Option<Vec<u8>>>> {
			let callbacks = callbacks.clone();
			let ctx = ctx.clone();
			Box::pin(async move {
				let Some(callback) = callbacks.get_workflow_history.as_ref() else {
					return Ok(None);
				};
				callback(GetWorkflowHistoryRequest { ctx }).await
			})
		}) as Arc<
			dyn Fn() -> futures::future::BoxFuture<'static, Result<Option<Vec<u8>>>>
				+ Send
				+ Sync,
		>
	});
	let replay_workflow = callbacks.replay_workflow.as_ref().map(|_| {
		let callbacks = callbacks.clone();
		let ctx = ctx.clone();
		Arc::new(
			move |entry_id: Option<String>| -> futures::future::BoxFuture<
				'static,
				Result<Option<Vec<u8>>>,
			> {
			let callbacks = callbacks.clone();
			let ctx = ctx.clone();
			Box::pin(async move {
				let Some(callback) = callbacks.replay_workflow.as_ref() else {
					return Ok(None);
				};
				callback(ReplayWorkflowRequest { ctx, entry_id }).await
			})
		},
		) as Arc<
			dyn Fn(
					Option<String>,
				) -> futures::future::BoxFuture<'static, Result<Option<Vec<u8>>>>
				+ Send
				+ Sync,
		>
	});

	Inspector::with_workflow_callbacks(get_workflow_history, replay_workflow)
}

fn inspector_rpcs(instance: &ActiveActorInstance) -> Vec<String> {
	let mut rpcs: Vec<String> = instance.callbacks.actions.keys().cloned().collect();
	rpcs.sort();
	rpcs
}

fn inspector_request_url(request: &Request) -> Result<Url> {
	Url::parse(&format!("http://inspector{}", request.uri()))
		.context("parse inspector request url")
}

fn decode_cbor_json_or_null(payload: &[u8]) -> JsonValue {
	if payload.is_empty() {
		return JsonValue::Null;
	}

	ciborium::from_reader::<JsonValue, _>(Cursor::new(payload))
		.unwrap_or(JsonValue::Null)
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

fn not_found_response() -> HttpResponse {
	HttpResponse {
		status: StatusCode::NOT_FOUND.as_u16(),
		headers: HashMap::new(),
		body: Some(Vec::new()),
		body_stream: None,
	}
}

fn inspector_unauthorized_response() -> HttpResponse {
	inspector_error_response(
		StatusCode::UNAUTHORIZED,
		"auth",
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

fn inspector_anyhow_response(error: anyhow::Error) -> HttpResponse {
	let error = RivetError::extract(&error);
	let status = inspector_error_status(error.group(), error.code());
	inspector_error_response(status, error.group(), error.code(), error.message())
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
		("auth", "unauthorized") => StatusCode::UNAUTHORIZED,
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

fn request_has_inspector_access(
	request: &Request,
	configured_token: Option<&str>,
) -> bool {
	let provided_token = request
		.headers()
		.get(http::header::AUTHORIZATION)
		.and_then(|value| value.to_str().ok())
		.and_then(|value| value.strip_prefix("Bearer "));

	match configured_token {
		Some(configured_token) => provided_token == Some(configured_token),
		None if env::var("NODE_ENV").unwrap_or_else(|_| "development".to_owned()) != "production" => {
			tracing::warn!(
				path = %request.uri(),
				"allowing inspector request without configured token in development mode"
			);
			true
		}
		None => false,
	}
}

async fn build_http_request(request: HttpRequest) -> Result<Request> {
	let mut body = request.body.unwrap_or_default();
	if let Some(mut body_stream) = request.body_stream {
		while let Some(chunk) = body_stream.recv().await {
			body.extend_from_slice(&chunk);
		}
	}

	Request::from_parts(&request.method, &request.path, request.headers, body)
		.with_context(|| format!("build actor request for `{}`", request.path))
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

fn internal_server_error_response() -> HttpResponse {
	HttpResponse {
		status: http::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
		headers: HashMap::new(),
		body: Some(Vec::new()),
		body_stream: None,
	}
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

fn default_websocket_handler() -> WebSocketHandler {
	WebSocketHandler {
		on_message: Box::new(|_message: WebSocketMessage| Box::pin(async {})),
		on_close: Box::new(|_code, _reason| Box::pin(async {})),
		on_open: None,
	}
}

#[cfg(test)]
#[path = "../tests/modules/registry.rs"]
mod tests;
