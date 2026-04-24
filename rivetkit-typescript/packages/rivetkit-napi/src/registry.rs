use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use napi::JsObject;
use napi::bindgen_prelude::{Buffer, Env, Promise};
use napi::threadsafe_function::{ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction};
use napi_derive::napi;
use rivetkit_core::{
	CoreRegistry as NativeCoreRegistry, CoreServerlessRuntime, ServeConfig, ServerlessRequest,
	serverless::ServerlessStreamError,
};
use tokio::sync::{Mutex as TokioMutex, Notify};
use tokio_util::sync::CancellationToken as CoreCancellationToken;

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

/// Registry lifecycle state machine.
///
/// Mode A (`serve`) and Mode B (`handle_serverless_request` -> `Serverless(...)`)
/// are mutually exclusive per instance: both transition out of `Registering`.
/// `BuildingServerless` is a sentinel held across the `into_serverless_runtime`
/// `.await` so a concurrent `shutdown()` can observe an in-flight build and
/// either wait for it to settle into `Serverless(_)` (then tear it down) or
/// transition directly to `ShutDown` while the build-side checks terminal
/// state before installing.
enum RegistryState {
	Registering(NativeCoreRegistry),
	BuildingServerless,
	Serving,
	Serverless(CoreServerlessRuntime),
	ShuttingDown,
	ShutDown,
}

#[napi]
#[derive(Clone)]
pub struct CoreRegistry {
	state: Arc<TokioMutex<RegistryState>>,
	shutdown_token: CoreCancellationToken,
	/// Notified whenever the state transitions out of `BuildingServerless`
	/// (to `Serverless(_)` on success, or `ShutDown` on failure/shutdown).
	/// Lets concurrent `ensure_serverless_runtime` callers that arrive during
	/// a build wait for the build to settle and then re-check the fast path
	/// instead of erroring with a misleading mode-conflict.
	build_complete: Arc<Notify>,
}

#[napi]
impl CoreRegistry {
	#[napi(constructor)]
	pub fn new() -> Self {
		crate::init_tracing(None);
		tracing::debug!(class = "CoreRegistry", "constructed napi class");
		Self {
			state: Arc::new(TokioMutex::new(RegistryState::Registering(
				NativeCoreRegistry::new(),
			))),
			shutdown_token: CoreCancellationToken::new(),
			build_complete: Arc::new(Notify::new()),
		}
	}

	#[napi]
	pub fn register(&self, name: String, factory: &NapiActorFactory) -> napi::Result<()> {
		// Registration runs on the sync N-API thread before any async work.
		// `try_lock` must always succeed here: no other path holds the lock at
		// this point. If somehow contended, surface the structured error rather
		// than blocking.
		let mut guard = self
			.state
			.try_lock()
			.map_err(|_| registry_register_busy_error())?;
		match &mut *guard {
			RegistryState::Registering(registry) => {
				registry.register_shared(&name, factory.actor_factory());
				Ok(())
			}
			RegistryState::BuildingServerless
			| RegistryState::Serving
			| RegistryState::Serverless(_)
			| RegistryState::ShuttingDown
			| RegistryState::ShutDown => Err(registry_not_registering_error()),
		}
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
			let mut guard = self.state.lock().await;
			match std::mem::replace(&mut *guard, RegistryState::Serving) {
				RegistryState::Registering(registry) => registry,
				other => {
					// Restore prior state so later shutdown sees the right variant.
					*guard = other;
					return Err(registry_not_registering_error());
				}
			}
		};

		registry
			.serve_with_config(
				ServeConfig {
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
				},
				self.shutdown_token.clone(),
			)
			.await
			.map_err(napi_anyhow_error)
	}

	/// Trip the shutdown token and tear down any live serverless runtime.
	///
	/// Idempotent. Safe to call when neither mode has been activated.
	/// Does not block on the `serve()` future; TS awaits that promise
	/// separately to avoid re-entrancy.
	#[napi]
	pub async fn shutdown(&self) -> napi::Result<()> {
		tracing::debug!(class = "CoreRegistry", "shutdown requested");
		// Trip the cancel first, outside the lock, so a `serve_with_config`
		// already past the state transition observes cancel promptly.
		self.shutdown_token.cancel();

		let (runtime, was_building) = {
			let mut guard = self.state.lock().await;
			match std::mem::replace(&mut *guard, RegistryState::ShuttingDown) {
				RegistryState::Registering(_) | RegistryState::Serving => (None, false),
				RegistryState::Serverless(runtime) => (Some(runtime), false),
				RegistryState::BuildingServerless => {
					// An `ensure_serverless_runtime` call is mid-build. Its
					// post-build re-check will observe `shutdown_token` and
					// tear down the runtime itself before settling state.
					(None, true)
				}
				RegistryState::ShuttingDown | RegistryState::ShutDown => {
					// Already in progress / done.
					*guard = RegistryState::ShutDown;
					return Ok(());
				}
			}
		};

		if let Some(runtime) = runtime {
			runtime.shutdown().await;
		}

		if !was_building {
			let mut guard = self.state.lock().await;
			*guard = RegistryState::ShutDown;
		}
		// Wake any `ensure_serverless_runtime` waiters parked on
		// `BuildingServerless`. They re-check state and observe the shutdown.
		// Also covers the case where `was_building` is true: the builder
		// itself is not a waiter, but future callers that arrive while the
		// builder is draining need to see `ShuttingDown` and error promptly.
		self.build_complete.notify_waiters();
		Ok(())
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
		// Loop handles the "another caller is mid-build" case: arm the notify
		// before re-checking so we can't miss a wakeup, then wait for the
		// builder to transition out of `BuildingServerless`.
		loop {
			{
				let guard = self.state.lock().await;
				if let RegistryState::Serverless(runtime) = &*guard {
					return Ok(runtime.clone());
				}
				if matches!(
					*guard,
					RegistryState::ShuttingDown | RegistryState::ShutDown
				) {
					return Err(registry_shut_down_error());
				}
				if matches!(*guard, RegistryState::Serving) {
					return Err(registry_wrong_mode_error());
				}
				if matches!(*guard, RegistryState::BuildingServerless) {
					// Another caller is building. Arm the notification before
					// dropping the lock so a completion we race against still
					// wakes us.
					let notify = self.build_complete.clone();
					let notified = notify.notified();
					tokio::pin!(notified);
					notified.as_mut().enable();
					drop(guard);
					notified.await;
					continue;
				}
				// RegistryState::Registering(_): fall through to build.
			}

			// Transition Registering -> BuildingServerless, drop lock, build,
			// re-acquire, install or tear down based on terminal state.
			let registry = {
				let mut guard = self.state.lock().await;
				match std::mem::replace(&mut *guard, RegistryState::BuildingServerless) {
					RegistryState::Registering(registry) => registry,
					other => {
						// State changed under us between fast-path and here;
						// restore and re-evaluate.
						*guard = other;
						continue;
					}
				}
			};
			return self.build_serverless_runtime(registry, config).await;
		}
	}

	async fn build_serverless_runtime(
		&self,
		registry: NativeCoreRegistry,
		config: JsServeConfig,
	) -> napi::Result<CoreServerlessRuntime> {
		let build_result = registry
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
			.await;

		// Re-acquire the lock and re-check state. Shutdown may have run during
		// the build. If so, tear down the freshly-built runtime rather than
		// installing it, preventing an orphaned runtime post-shutdown.
		let mut guard = self.state.lock().await;
		let result = match build_result {
			Ok(runtime) => {
				if self.shutdown_token.is_cancelled()
					|| matches!(
						*guard,
						RegistryState::ShuttingDown | RegistryState::ShutDown
					) {
					// Drop the lock while we drain the envoy.
					drop(guard);
					runtime.shutdown().await;
					let mut guard = self.state.lock().await;
					*guard = RegistryState::ShutDown;
					drop(guard);
					Err(registry_shut_down_error())
				} else {
					*guard = RegistryState::Serverless(runtime.clone());
					drop(guard);
					Ok(runtime)
				}
			}
			Err(error) => {
				// Build failed. The inner `NativeCoreRegistry` was consumed by
				// `into_serverless_runtime` and cannot be restored. Any future
				// call on this `CoreRegistry` must observe a terminal state
				// with a clear error, not the misleading `wrong_mode` that
				// leaving `BuildingServerless` would produce.
				*guard = RegistryState::ShutDown;
				drop(guard);
				Err(napi_anyhow_error(error))
			}
		};
		// Wake any `ensure_serverless_runtime` callers parked on
		// `BuildingServerless`. They re-check state and either get the cached
		// runtime or the shutdown error.
		self.build_complete.notify_waiters();
		result
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

fn registry_not_registering_error() -> napi::Error {
	napi_anyhow_error(
		NapiInvalidState {
			state: "core registry".to_owned(),
			reason: "already serving or shut down".to_owned(),
		}
		.build(),
	)
}

fn registry_wrong_mode_error() -> napi::Error {
	napi_anyhow_error(
		NapiInvalidState {
			state: "core registry".to_owned(),
			reason: "mode conflict: another run mode is already active".to_owned(),
		}
		.build(),
	)
}

fn registry_shut_down_error() -> napi::Error {
	napi_anyhow_error(
		NapiInvalidState {
			state: "core registry".to_owned(),
			reason: "shut down".to_owned(),
		}
		.build(),
	)
}

fn registry_register_busy_error() -> napi::Error {
	napi_anyhow_error(
		NapiInvalidState {
			state: "core registry".to_owned(),
			reason: "register called concurrently with serve or shutdown".to_owned(),
		}
		.build(),
	)
}
