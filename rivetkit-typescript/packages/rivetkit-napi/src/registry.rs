use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use napi::JsObject;
use napi::bindgen_prelude::{Buffer, Env, Promise};
use napi::threadsafe_function::{ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction};
use napi_derive::napi;
use parking_lot::Mutex;
use rivetkit_core::{
	CoreRegistry as NativeCoreRegistry, CoreServerlessRuntime, ServeConfig, ServerlessRequest,
	serverless::ServerlessStreamError,
};

use crate::actor_factory::NapiActorFactory;
use crate::cancellation_token::CancellationToken;
use crate::{NapiInvalidState, napi_anyhow_error};

#[napi(object)]
pub struct JsServeConfig {
	pub version: u32,
	pub endpoint: String,
	pub token: Option<String>,
	pub namespace: String,
	pub pool_name: String,
	pub engine_binary_path: Option<String>,
	pub handle_inspector_http_in_runtime: Option<bool>,
	pub serverless_base_path: Option<String>,
	pub serverless_package_version: String,
	pub serverless_client_endpoint: Option<String>,
	pub serverless_client_namespace: Option<String>,
	pub serverless_client_token: Option<String>,
	pub serverless_validate_endpoint: bool,
	pub serverless_max_start_payload_bytes: u32,
}

#[napi(object)]
pub struct JsServerlessRequest {
	pub method: String,
	pub url: String,
	pub headers: HashMap<String, String>,
	pub body: Buffer,
}

#[napi(object)]
pub struct JsServerlessResponseHead {
	pub status: u16,
	pub headers: HashMap<String, String>,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsServerlessStreamError {
	pub group: String,
	pub code: String,
	pub message: String,
}

#[derive(Clone)]
enum ServerlessStreamEvent {
	Chunk {
		chunk: Vec<u8>,
	},
	End {
		error: Option<JsServerlessStreamError>,
	},
}

#[napi]
#[derive(Clone)]
pub struct CoreRegistry {
	// Registration is a synchronous N-API boundary; the lock is released before
	// async serving begins.
	inner: Arc<Mutex<Option<NativeCoreRegistry>>>,
	serverless_runtime: Arc<Mutex<Option<CoreServerlessRuntime>>>,
}

#[napi]
impl CoreRegistry {
	#[napi(constructor)]
	pub fn new() -> Self {
		crate::init_tracing(None);
		tracing::debug!(class = "CoreRegistry", "constructed napi class");
		Self {
			inner: Arc::new(Mutex::new(Some(NativeCoreRegistry::new()))),
			serverless_runtime: Arc::new(Mutex::new(None)),
		}
	}

	#[napi]
	pub fn register(&self, name: String, factory: &NapiActorFactory) -> napi::Result<()> {
		let mut guard = self.inner.lock();
		let registry = guard
			.as_mut()
			.ok_or_else(|| registry_already_serving_error())?;
		registry.register_shared(&name, factory.actor_factory());
		Ok(())
	}

	#[napi]
	pub async fn serve(&self, config: JsServeConfig) -> napi::Result<()> {
		tracing::debug!(
			class = "CoreRegistry",
			version = config.version,
			endpoint = %config.endpoint,
			namespace = %config.namespace,
			pool_name = %config.pool_name,
			starting_engine = config.engine_binary_path.is_some(),
			"serving native registry"
		);
		let registry = {
			let mut guard = self.inner.lock();
			guard
				.take()
				.ok_or_else(|| registry_already_serving_error())?
		};

		registry
			.serve_with_config(ServeConfig {
				version: config.version,
				endpoint: config.endpoint,
				token: config.token,
				namespace: config.namespace,
				pool_name: config.pool_name,
				engine_binary_path: config.engine_binary_path.map(PathBuf::from),
				handle_inspector_http_in_runtime: config
					.handle_inspector_http_in_runtime
					.unwrap_or(false),
				serverless_base_path: config.serverless_base_path,
				serverless_package_version: config.serverless_package_version,
				serverless_client_endpoint: config.serverless_client_endpoint,
				serverless_client_namespace: config.serverless_client_namespace,
				serverless_client_token: config.serverless_client_token,
				serverless_validate_endpoint: config.serverless_validate_endpoint,
				serverless_max_start_payload_bytes: config.serverless_max_start_payload_bytes
					as usize,
			})
			.await
			.map_err(napi_anyhow_error)
	}

	#[napi(ts_return_type = "Promise<JsServerlessResponseHead>")]
	pub fn handle_serverless_request(
		&self,
		env: Env,
		req: JsServerlessRequest,
		on_stream_event: napi::JsFunction,
		cancel_token: &CancellationToken,
		config: JsServeConfig,
	) -> napi::Result<JsObject> {
		let stream_event = create_stream_event_tsfn(on_stream_event)?;
		let (deferred, promise) = env.create_deferred::<JsServerlessResponseHead, _>()?;
		let registry = self.clone();
		let cancel_token = cancel_token.inner().clone();

		napi::bindgen_prelude::spawn(async move {
			let runtime = match registry.ensure_serverless_runtime(config).await {
				Ok(runtime) => runtime,
				Err(error) => {
					deferred.reject(error);
					return;
				}
			};
			let response = runtime
				.handle_request(ServerlessRequest {
					method: req.method,
					url: req.url,
					headers: req
						.headers
						.into_iter()
						.map(|(key, value)| (key.to_ascii_lowercase(), value))
						.collect(),
					body: req.body.to_vec(),
					cancel_token,
				})
				.await;
			let head = JsServerlessResponseHead {
				status: response.status,
				headers: response.headers,
			};
			deferred.resolve(move |_| Ok(head));

			let mut body = response.body;
			while let Some(chunk) = body.recv().await {
				let event = match chunk {
					Ok(chunk) => ServerlessStreamEvent::Chunk { chunk },
					Err(error) => ServerlessStreamEvent::End {
						error: Some(JsServerlessStreamError::from(error)),
					},
				};
				if let Err(error) = deliver_stream_event(&stream_event, event).await {
					tracing::warn!(?error, "failed to deliver serverless stream event");
					return;
				}
			}
			if let Err(error) =
				deliver_stream_event(&stream_event, ServerlessStreamEvent::End { error: None })
					.await
			{
				tracing::warn!(?error, "failed to close serverless response stream");
			}
		});

		Ok(promise)
	}

	async fn ensure_serverless_runtime(
		&self,
		config: JsServeConfig,
	) -> napi::Result<CoreServerlessRuntime> {
		if let Some(runtime) = self.serverless_runtime.lock().as_ref().cloned() {
			return Ok(runtime);
		}

		let registry = {
			let mut guard = self.inner.lock();
			guard
				.take()
				.ok_or_else(|| registry_already_serving_error())?
		};
		let runtime = registry
			.into_serverless_runtime(ServeConfig {
				version: config.version,
				endpoint: config.endpoint,
				token: config.token,
				namespace: config.namespace,
				pool_name: config.pool_name,
				engine_binary_path: config.engine_binary_path.map(PathBuf::from),
				handle_inspector_http_in_runtime: config
					.handle_inspector_http_in_runtime
					.unwrap_or(true),
				serverless_base_path: config.serverless_base_path,
				serverless_package_version: config.serverless_package_version,
				serverless_client_endpoint: config.serverless_client_endpoint,
				serverless_client_namespace: config.serverless_client_namespace,
				serverless_client_token: config.serverless_client_token,
				serverless_validate_endpoint: config.serverless_validate_endpoint,
				serverless_max_start_payload_bytes: config.serverless_max_start_payload_bytes
					as usize,
			})
			.await
			.map_err(napi_anyhow_error)?;
		*self.serverless_runtime.lock() = Some(runtime.clone());
		Ok(runtime)
	}
}

fn create_stream_event_tsfn(
	callback: napi::JsFunction,
) -> napi::Result<ThreadsafeFunction<ServerlessStreamEvent, ErrorStrategy::CalleeHandled>> {
	callback.create_threadsafe_function(0, |ctx: ThreadSafeCallContext<ServerlessStreamEvent>| {
		let mut object = ctx.env.create_object()?;
		match ctx.value {
			ServerlessStreamEvent::Chunk { chunk } => {
				object.set("kind", "chunk")?;
				object.set(
					"chunk",
					ctx.env.create_buffer_with_data(chunk)?.into_unknown(),
				)?;
			}
			ServerlessStreamEvent::End { error } => {
				object.set("kind", "end")?;
				if let Some(error) = error {
					let mut error_object = ctx.env.create_object()?;
					error_object.set("group", error.group)?;
					error_object.set("code", error.code)?;
					error_object.set("message", error.message)?;
					object.set("error", error_object)?;
				}
			}
		}
		Ok(vec![object.into_unknown()])
	})
}

async fn deliver_stream_event(
	callback: &ThreadsafeFunction<ServerlessStreamEvent, ErrorStrategy::CalleeHandled>,
	event: ServerlessStreamEvent,
) -> napi::Result<()> {
	let promise = callback.call_async::<Promise<()>>(Ok(event)).await?;
	promise.await
}

impl From<ServerlessStreamError> for JsServerlessStreamError {
	fn from(value: ServerlessStreamError) -> Self {
		Self {
			group: value.group,
			code: value.code,
			message: value.message,
		}
	}
}

fn registry_already_serving_error() -> napi::Error {
	napi_anyhow_error(
		NapiInvalidState {
			state: "core registry".to_owned(),
			reason: "already serving".to_owned(),
		}
		.build(),
	)
}
