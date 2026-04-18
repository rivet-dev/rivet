use std::path::PathBuf;
use std::sync::Mutex;

use napi_derive::napi;
use rivetkit_core::{CoreRegistry as NativeCoreRegistry, ServeConfig};

use crate::actor_factory::NapiActorFactory;
use crate::napi_anyhow_error;

#[napi(object)]
pub struct JsServeConfig {
	pub version: u32,
	pub endpoint: String,
	pub token: Option<String>,
	pub namespace: String,
	pub pool_name: String,
	pub engine_binary_path: Option<String>,
	pub handle_inspector_http_in_runtime: Option<bool>,
}

#[napi]
pub struct CoreRegistry {
	inner: Mutex<Option<NativeCoreRegistry>>,
}

#[napi]
impl CoreRegistry {
	#[napi(constructor)]
	pub fn new() -> Self {
		Self {
			inner: Mutex::new(Some(NativeCoreRegistry::new())),
		}
	}

	#[napi]
	pub fn register(&self, name: String, factory: &NapiActorFactory) -> napi::Result<()> {
		let mut guard = self
			.inner
			.lock()
			.map_err(|_| napi::Error::from_reason("core registry mutex poisoned"))?;
		let registry = guard
			.as_mut()
			.ok_or_else(|| napi::Error::from_reason("core registry has already started serving"))?;
		registry.register_shared(&name, factory.actor_factory());
		Ok(())
	}

	#[napi]
	pub async fn serve(&self, config: JsServeConfig) -> napi::Result<()> {
		let registry = {
			let mut guard = self
				.inner
				.lock()
				.map_err(|_| napi::Error::from_reason("core registry mutex poisoned"))?;
			guard
				.take()
				.ok_or_else(|| napi::Error::from_reason("core registry is already serving"))?
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
			})
			.await
			.map_err(napi_anyhow_error)
	}
}
