use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use napi::bindgen_prelude::{Buffer, Promise};
use napi::threadsafe_function::{ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction};
use napi::{Env, JsFunction, JsObject};
use napi_derive::napi;
use rivetkit_core::actor::callbacks::{
	ActionHandler, BeforeActionResponseCallback, LifecycleCallback, RequestCallback,
};
use rivetkit_core::{
	ActionRequest, ActorConfig, ActorFactory as CoreActorFactory, ActorInstanceCallbacks,
	FactoryRequest, OnBeforeActionResponseRequest, OnBeforeConnectRequest,
	OnConnectRequest, OnDestroyRequest, OnDisconnectRequest, OnRequestRequest,
	OnSleepRequest, OnStateChangeRequest, OnWakeRequest, OnWebSocketRequest,
	Request, Response, RunRequest,
};

use crate::actor_context::ActorContext;
use crate::connection::ConnHandle;
use crate::websocket::WebSocket;

type CallbackTsfn<T> = ThreadsafeFunction<T, ErrorStrategy::CalleeHandled>;

#[napi(object)]
pub struct JsHttpResponse {
	pub status: Option<u16>,
	pub headers: Option<HashMap<String, String>>,
	pub body: Option<Buffer>,
}

#[napi(object)]
pub struct JsActorConfig {
	pub name: Option<String>,
	pub icon: Option<String>,
	pub can_hibernate_websocket: Option<bool>,
	pub state_save_interval_ms: Option<u32>,
	pub create_vars_timeout_ms: Option<u32>,
	pub create_conn_state_timeout_ms: Option<u32>,
	pub on_before_connect_timeout_ms: Option<u32>,
	pub on_connect_timeout_ms: Option<u32>,
	pub on_sleep_timeout_ms: Option<u32>,
	pub on_destroy_timeout_ms: Option<u32>,
	pub action_timeout_ms: Option<u32>,
	pub run_stop_timeout_ms: Option<u32>,
	pub sleep_timeout_ms: Option<u32>,
	pub no_sleep: Option<bool>,
	pub sleep_grace_period_ms: Option<u32>,
	pub connection_liveness_timeout_ms: Option<u32>,
	pub connection_liveness_interval_ms: Option<u32>,
	pub max_queue_size: Option<u32>,
	pub max_queue_message_size: Option<u32>,
	pub preload_max_workflow_bytes: Option<f64>,
	pub preload_max_connections_bytes: Option<f64>,
}

#[napi(object)]
pub struct JsFactoryInitResult {
	pub state: Option<Buffer>,
	pub vars: Option<Buffer>,
}

#[derive(Clone)]
struct LifecyclePayload {
	ctx: rivetkit_core::ActorContext,
}

#[derive(Clone)]
struct FactoryInitPayload {
	ctx: rivetkit_core::ActorContext,
	input: Option<Vec<u8>>,
	is_new: bool,
}

#[derive(Clone)]
struct StateChangePayload {
	ctx: rivetkit_core::ActorContext,
	new_state: Vec<u8>,
}

#[derive(Clone)]
struct HttpRequestPayload {
	ctx: rivetkit_core::ActorContext,
	request: Request,
}

#[derive(Clone)]
struct WebSocketPayload {
	ctx: rivetkit_core::ActorContext,
	ws: rivetkit_core::WebSocket,
}

#[derive(Clone)]
struct BeforeConnectPayload {
	ctx: rivetkit_core::ActorContext,
	params: Vec<u8>,
}

#[derive(Clone)]
struct ConnectionPayload {
	ctx: rivetkit_core::ActorContext,
	conn: rivetkit_core::ConnHandle,
}

#[derive(Clone)]
struct ActionPayload {
	ctx: rivetkit_core::ActorContext,
	conn: rivetkit_core::ConnHandle,
	name: String,
	args: Vec<u8>,
}

#[derive(Clone)]
struct BeforeActionResponsePayload {
	ctx: rivetkit_core::ActorContext,
	name: String,
	args: Vec<u8>,
	output: Vec<u8>,
}

struct CallbackBindings {
	on_init: Option<CallbackTsfn<FactoryInitPayload>>,
	on_wake: Option<CallbackTsfn<LifecyclePayload>>,
	on_sleep: Option<CallbackTsfn<LifecyclePayload>>,
	on_destroy: Option<CallbackTsfn<LifecyclePayload>>,
	on_state_change: Option<CallbackTsfn<StateChangePayload>>,
	on_request: Option<CallbackTsfn<HttpRequestPayload>>,
	on_websocket: Option<CallbackTsfn<WebSocketPayload>>,
	on_before_connect: Option<CallbackTsfn<BeforeConnectPayload>>,
	on_connect: Option<CallbackTsfn<ConnectionPayload>>,
	on_disconnect: Option<CallbackTsfn<ConnectionPayload>>,
	actions: HashMap<String, CallbackTsfn<ActionPayload>>,
	on_before_action_response: Option<CallbackTsfn<BeforeActionResponsePayload>>,
	run: Option<CallbackTsfn<LifecyclePayload>>,
}

#[napi]
pub struct NapiActorFactory {
	#[allow(dead_code)]
	inner: Arc<CoreActorFactory>,
}

impl NapiActorFactory {
	#[allow(dead_code)]
	pub(crate) fn actor_factory(&self) -> Arc<CoreActorFactory> {
		Arc::clone(&self.inner)
	}
}

#[napi]
impl NapiActorFactory {
	#[napi(constructor)]
	pub fn constructor(
		callbacks: JsObject,
		config: Option<JsActorConfig>,
	) -> napi::Result<Self> {
		let bindings = Arc::new(CallbackBindings::from_js(callbacks)?);
		let inner = Arc::new(CoreActorFactory::new(
			actor_config_from_js(config),
			move |request: FactoryRequest| {
				let bindings = Arc::clone(&bindings);
				Box::pin(async move {
					bindings.initialize(&request).await?;
					Ok(bindings.create_callbacks())
				})
			},
		));

		Ok(Self { inner })
	}
}

impl CallbackBindings {
	fn from_js(callbacks: JsObject) -> napi::Result<Self> {
		let actions = if let Some(actions) = callbacks.get::<_, JsObject>("actions")? {
			let mut mapped = HashMap::new();
			for name in JsObject::keys(&actions)? {
				let callback = actions
					.get::<_, JsFunction>(&name)?
					.ok_or_else(|| napi::Error::from_reason(format!("action `{name}` must be a function")))?;
				mapped.insert(name, create_tsfn(callback, build_action_payload)?);
			}
			mapped
		} else {
			HashMap::new()
		};

		Ok(Self {
			on_init: optional_tsfn(&callbacks, "onInit", build_factory_init_payload)?,
			on_wake: optional_tsfn(&callbacks, "onWake", build_lifecycle_payload)?,
			on_sleep: optional_tsfn(&callbacks, "onSleep", build_lifecycle_payload)?,
			on_destroy: optional_tsfn(&callbacks, "onDestroy", build_lifecycle_payload)?,
			on_state_change: optional_tsfn(
				&callbacks,
				"onStateChange",
				build_state_change_payload,
			)?,
			on_request: optional_tsfn(&callbacks, "onRequest", build_http_request_payload)?,
			on_websocket: optional_tsfn(
				&callbacks,
				"onWebSocket",
				build_websocket_payload,
			)?,
			on_before_connect: optional_tsfn(
				&callbacks,
				"onBeforeConnect",
				build_before_connect_payload,
			)?,
			on_connect: optional_tsfn(&callbacks, "onConnect", build_connection_payload)?,
			on_disconnect: optional_tsfn(
				&callbacks,
				"onDisconnect",
				build_connection_payload,
			)?,
			actions,
			on_before_action_response: optional_tsfn(
				&callbacks,
				"onBeforeActionResponse",
				build_before_action_response_payload,
			)?,
			run: optional_tsfn(&callbacks, "run", build_lifecycle_payload)?,
		})
	}

	async fn initialize(&self, request: &FactoryRequest) -> Result<()> {
		let Some(callback) = &self.on_init else {
			return Ok(());
		};

		let promise = callback
			.call_async::<Promise<JsFactoryInitResult>>(Ok(FactoryInitPayload {
				ctx: request.ctx.clone(),
				input: request.input.clone(),
				is_new: request.is_new,
			}))
			.await
			.map_err(napi_to_anyhow)?;
		let result = promise.await.map_err(napi_to_anyhow)?;

		if let Some(state) = result.state {
			request.ctx.set_state(state.to_vec());
		}

		if let Some(vars) = result.vars {
			request.ctx.set_vars(vars.to_vec());
		}

		Ok(())
	}

	fn create_callbacks(&self) -> ActorInstanceCallbacks {
		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.on_wake = wrap_void_callback(&self.on_wake, |request: OnWakeRequest| {
			LifecyclePayload { ctx: request.ctx }
		});
		callbacks.on_sleep =
			wrap_void_callback(&self.on_sleep, |request: OnSleepRequest| {
				LifecyclePayload { ctx: request.ctx }
			});
		callbacks.on_destroy =
			wrap_void_callback(&self.on_destroy, |request: OnDestroyRequest| {
				LifecyclePayload { ctx: request.ctx }
			});
		callbacks.on_state_change = wrap_void_callback(
			&self.on_state_change,
			|request: OnStateChangeRequest| StateChangePayload {
				ctx: request.ctx,
				new_state: request.new_state,
			},
		);
		callbacks.on_request = wrap_request_callback(
			&self.on_request,
			|request: OnRequestRequest| HttpRequestPayload {
				ctx: request.ctx,
				request: request.request,
			},
		);
		callbacks.on_websocket = wrap_void_callback(
			&self.on_websocket,
			|request: OnWebSocketRequest| WebSocketPayload {
				ctx: request.ctx,
				ws: request.ws,
			},
		);
		callbacks.on_before_connect = wrap_void_callback(
			&self.on_before_connect,
			|request: OnBeforeConnectRequest| BeforeConnectPayload {
				ctx: request.ctx,
				params: request.params,
			},
		);
		callbacks.on_connect = wrap_void_callback(
			&self.on_connect,
			|request: OnConnectRequest| ConnectionPayload {
				ctx: request.ctx,
				conn: request.conn,
			},
		);
		callbacks.on_disconnect = wrap_void_callback(
			&self.on_disconnect,
			|request: OnDisconnectRequest| ConnectionPayload {
				ctx: request.ctx,
				conn: request.conn,
			},
		);
		callbacks.actions = self
			.actions
			.iter()
			.map(|(name, callback)| {
				(
					name.clone(),
					wrap_action_callback(callback, |request: ActionRequest| ActionPayload {
						ctx: request.ctx,
						conn: request.conn,
						name: request.name,
						args: request.args,
					}),
				)
			})
			.collect();
		callbacks.on_before_action_response = wrap_buffer_callback(
			&self.on_before_action_response,
			|request: OnBeforeActionResponseRequest| BeforeActionResponsePayload {
				ctx: request.ctx,
				name: request.name,
				args: request.args,
				output: request.output,
			},
		);
		callbacks.run = wrap_void_callback(&self.run, |request: RunRequest| {
			LifecyclePayload { ctx: request.ctx }
		});
		callbacks
	}
}

fn optional_tsfn<T, F>(
	callbacks: &JsObject,
	name: &str,
	build_args: F,
) -> napi::Result<Option<CallbackTsfn<T>>>
where
	T: Send + 'static,
	F: Fn(&Env, T) -> napi::Result<Vec<napi::JsUnknown>> + Send + Sync + 'static,
{
	let Some(callback) = callbacks.get::<_, JsFunction>(name)? else {
		return Ok(None);
	};
	create_tsfn(callback, build_args).map(Some)
}

fn create_tsfn<T, F>(
	callback: JsFunction,
	build_args: F,
) -> napi::Result<CallbackTsfn<T>>
where
	T: Send + 'static,
	F: Fn(&Env, T) -> napi::Result<Vec<napi::JsUnknown>> + Send + Sync + 'static,
{
	let build_args = Arc::new(build_args);
	callback.create_threadsafe_function(
		0,
		move |ctx: ThreadSafeCallContext<T>| build_args(&ctx.env, ctx.value),
	)
}

fn actor_config_from_js(config: Option<JsActorConfig>) -> ActorConfig {
	let mut actor_config = ActorConfig::default();
	let Some(config) = config else {
		return actor_config;
	};

	actor_config.name = config.name;
	actor_config.icon = config.icon;
	if let Some(can_hibernate_websocket) = config.can_hibernate_websocket {
		actor_config.can_hibernate_websocket =
			rivetkit_core::CanHibernateWebSocket::Bool(can_hibernate_websocket);
	}
	if let Some(value) = config.state_save_interval_ms {
		actor_config.state_save_interval = duration_ms(value);
	}
	if let Some(value) = config.create_vars_timeout_ms {
		actor_config.create_vars_timeout = duration_ms(value);
	}
	if let Some(value) = config.create_conn_state_timeout_ms {
		actor_config.create_conn_state_timeout = duration_ms(value);
	}
	if let Some(value) = config.on_before_connect_timeout_ms {
		actor_config.on_before_connect_timeout = duration_ms(value);
	}
	if let Some(value) = config.on_connect_timeout_ms {
		actor_config.on_connect_timeout = duration_ms(value);
	}
	if let Some(value) = config.on_sleep_timeout_ms {
		actor_config.on_sleep_timeout = duration_ms(value);
	}
	if let Some(value) = config.on_destroy_timeout_ms {
		actor_config.on_destroy_timeout = duration_ms(value);
	}
	if let Some(value) = config.action_timeout_ms {
		actor_config.action_timeout = duration_ms(value);
	}
	if let Some(value) = config.run_stop_timeout_ms {
		actor_config.run_stop_timeout = duration_ms(value);
	}
	if let Some(value) = config.sleep_timeout_ms {
		actor_config.sleep_timeout = duration_ms(value);
	}
	if let Some(value) = config.no_sleep {
		actor_config.no_sleep = value;
	}
	if let Some(value) = config.sleep_grace_period_ms {
		actor_config.sleep_grace_period = Some(duration_ms(value));
	}
	if let Some(value) = config.connection_liveness_timeout_ms {
		actor_config.connection_liveness_timeout = duration_ms(value);
	}
	if let Some(value) = config.connection_liveness_interval_ms {
		actor_config.connection_liveness_interval = duration_ms(value);
	}
	if let Some(value) = config.max_queue_size {
		actor_config.max_queue_size = value;
	}
	if let Some(value) = config.max_queue_message_size {
		actor_config.max_queue_message_size = value;
	}
	actor_config.preload_max_workflow_bytes =
		config.preload_max_workflow_bytes.map(|value| value as u64);
	actor_config.preload_max_connections_bytes =
		config.preload_max_connections_bytes.map(|value| value as u64);

	actor_config
}

fn duration_ms(value: u32) -> Duration {
	Duration::from_millis(u64::from(value))
}

fn wrap_void_callback<Req, Payload, Map>(
	callback: &Option<CallbackTsfn<Payload>>,
	map: Map,
) -> Option<LifecycleCallback<Req>>
where
	Req: Send + 'static,
	Payload: Send + 'static,
	Map: Fn(Req) -> Payload + Send + Sync + 'static,
{
	let callback = callback.clone()?;
	let map = Arc::new(map);
	Some(Box::new(move |request| {
		let callback = callback.clone();
		let map = Arc::clone(&map);
		Box::pin(async move { call_void(&callback, (map.as_ref())(request)).await })
	}))
}

fn wrap_request_callback<Map>(
	callback: &Option<CallbackTsfn<HttpRequestPayload>>,
	map: Map,
) -> Option<RequestCallback>
where
	Map: Fn(OnRequestRequest) -> HttpRequestPayload + Send + Sync + 'static,
{
	let callback = callback.clone()?;
	let map = Arc::new(map);
	Some(Box::new(move |request| {
		let callback = callback.clone();
		let map = Arc::clone(&map);
		Box::pin(async move { call_request(&callback, (map.as_ref())(request)).await })
	}))
}

fn wrap_action_callback<Map>(
	callback: &CallbackTsfn<ActionPayload>,
	map: Map,
) -> ActionHandler
where
	Map: Fn(ActionRequest) -> ActionPayload + Send + Sync + 'static,
{
	let callback = callback.clone();
	let map = Arc::new(map);
	Box::new(move |request| {
		let callback = callback.clone();
		let map = Arc::clone(&map);
		Box::pin(async move { call_buffer(&callback, (map.as_ref())(request)).await })
	})
}

fn wrap_buffer_callback<Payload, Map>(
	callback: &Option<CallbackTsfn<Payload>>,
	map: Map,
) -> Option<BeforeActionResponseCallback>
where
	Payload: Send + 'static,
	Map: Fn(OnBeforeActionResponseRequest) -> Payload + Send + Sync + 'static,
{
	let callback = callback.clone()?;
	let map = Arc::new(map);
	Some(Box::new(move |request| {
		let callback = callback.clone();
		let map = Arc::clone(&map);
		Box::pin(async move { call_buffer(&callback, (map.as_ref())(request)).await })
	}))
}

async fn call_void<T>(callback: &CallbackTsfn<T>, payload: T) -> Result<()>
where
	T: Send + 'static,
{
	let promise = callback
		.call_async::<Promise<()>>(Ok(payload))
		.await
		.map_err(napi_to_anyhow)?;
	promise.await.map_err(napi_to_anyhow)
}

async fn call_buffer<T>(callback: &CallbackTsfn<T>, payload: T) -> Result<Vec<u8>>
where
	T: Send + 'static,
{
	let promise = callback
		.call_async::<Promise<Buffer>>(Ok(payload))
		.await
		.map_err(napi_to_anyhow)?;
	let buffer = promise.await.map_err(napi_to_anyhow)?;
	Ok(buffer.to_vec())
}

async fn call_request(
	callback: &CallbackTsfn<HttpRequestPayload>,
	payload: HttpRequestPayload,
) -> Result<Response> {
	let promise = callback
		.call_async::<Promise<JsHttpResponse>>(Ok(payload))
		.await
		.map_err(napi_to_anyhow)?;
	let response = promise.await.map_err(napi_to_anyhow)?;
	parse_http_response(response)
}

fn parse_http_response(response: JsHttpResponse) -> Result<Response> {
	let mut parsed = Response::new(response.body.unwrap_or_else(|| Buffer::from(Vec::new())).to_vec());
	let status = response.status.unwrap_or(200);
	*parsed.status_mut() = status
		.try_into()
		.map_err(|error| anyhow!("invalid http response status `{status}`: {error}"))?;

	if let Some(headers) = response.headers {
		for (name, value) in headers {
			let header_name: http::header::HeaderName = name
				.parse()
				.map_err(|error| anyhow!("invalid response header name `{name}`: {error}"))?;
			let header_value: http::header::HeaderValue = value
				.parse()
				.map_err(|error| anyhow!("invalid response header `{name}` value: {error}"))?;
			parsed
				.headers_mut()
				.insert(header_name, header_value);
		}
	}

	Ok(parsed)
}

fn build_lifecycle_payload(
	env: &Env,
	payload: LifecyclePayload,
) -> napi::Result<Vec<napi::JsUnknown>> {
	let mut object = env.create_object()?;
	object.set("ctx", ActorContext::new(payload.ctx))?;
	Ok(vec![object.into_unknown()])
}

fn build_factory_init_payload(
	env: &Env,
	payload: FactoryInitPayload,
) -> napi::Result<Vec<napi::JsUnknown>> {
	let mut object = env.create_object()?;
	object.set("ctx", ActorContext::new(payload.ctx))?;
	object.set("input", payload.input.map(Buffer::from))?;
	object.set("isNew", payload.is_new)?;
	Ok(vec![object.into_unknown()])
}

fn build_state_change_payload(
	env: &Env,
	payload: StateChangePayload,
) -> napi::Result<Vec<napi::JsUnknown>> {
	let mut object = env.create_object()?;
	object.set("ctx", ActorContext::new(payload.ctx))?;
	object.set("newState", Buffer::from(payload.new_state))?;
	Ok(vec![object.into_unknown()])
}

fn build_http_request_payload(
	env: &Env,
	payload: HttpRequestPayload,
) -> napi::Result<Vec<napi::JsUnknown>> {
	let mut object = env.create_object()?;
	object.set("ctx", ActorContext::new(payload.ctx))?;
	object.set("request", build_http_request(env, &payload.request)?)?;
	Ok(vec![object.into_unknown()])
}

fn build_websocket_payload(
	env: &Env,
	payload: WebSocketPayload,
) -> napi::Result<Vec<napi::JsUnknown>> {
	let mut object = env.create_object()?;
	object.set("ctx", ActorContext::new(payload.ctx))?;
	object.set("ws", WebSocket::new(payload.ws))?;
	Ok(vec![object.into_unknown()])
}

fn build_before_connect_payload(
	env: &Env,
	payload: BeforeConnectPayload,
) -> napi::Result<Vec<napi::JsUnknown>> {
	let mut object = env.create_object()?;
	object.set("ctx", ActorContext::new(payload.ctx))?;
	object.set("params", Buffer::from(payload.params))?;
	Ok(vec![object.into_unknown()])
}

fn build_connection_payload(
	env: &Env,
	payload: ConnectionPayload,
) -> napi::Result<Vec<napi::JsUnknown>> {
	let mut object = env.create_object()?;
	object.set("ctx", ActorContext::new(payload.ctx))?;
	object.set("conn", ConnHandle::new(payload.conn))?;
	Ok(vec![object.into_unknown()])
}

fn build_action_payload(
	env: &Env,
	payload: ActionPayload,
) -> napi::Result<Vec<napi::JsUnknown>> {
	let mut object = env.create_object()?;
	object.set("ctx", ActorContext::new(payload.ctx))?;
	object.set("conn", ConnHandle::new(payload.conn))?;
	object.set("name", payload.name)?;
	object.set("args", Buffer::from(payload.args))?;
	Ok(vec![object.into_unknown()])
}

fn build_before_action_response_payload(
	env: &Env,
	payload: BeforeActionResponsePayload,
) -> napi::Result<Vec<napi::JsUnknown>> {
	let mut object = env.create_object()?;
	object.set("ctx", ActorContext::new(payload.ctx))?;
	object.set("name", payload.name)?;
	object.set("args", Buffer::from(payload.args))?;
	object.set("output", Buffer::from(payload.output))?;
	Ok(vec![object.into_unknown()])
}

fn build_http_request(env: &Env, request: &Request) -> napi::Result<JsObject> {
	let mut object = env.create_object()?;
	object.set("method", request.method().to_string())?;
	object.set("uri", request.uri().to_string())?;
	object.set("headers", request_headers(request))?;
	object.set("body", Buffer::from(request.body().clone()))?;
	Ok(object)
}

fn request_headers(request: &Request) -> HashMap<String, String> {
	request
		.headers()
		.iter()
		.map(|(name, value)| {
			(
				name.to_string(),
				String::from_utf8_lossy(value.as_bytes()).into_owned(),
			)
		})
		.collect()
}

fn napi_to_anyhow(error: napi::Error) -> anyhow::Error {
	anyhow!(error.to_string())
}
