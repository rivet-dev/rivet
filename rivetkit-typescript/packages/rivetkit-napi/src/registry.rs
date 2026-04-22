use std::path::PathBuf;

use napi_derive::napi;
use parking_lot::Mutex;
use rivetkit_core::{CoreRegistry as NativeCoreRegistry, ServeConfig};

use crate::actor_factory::NapiActorFactory;
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
}

#[napi]
pub struct CoreRegistry {
	// Registration is a synchronous N-API boundary; the lock is released before
	// async serving begins.
	inner: Mutex<Option<NativeCoreRegistry>>,
}

#[napi]
impl CoreRegistry {
	#[napi(constructor)]
	pub fn new() -> Self {
		crate::init_tracing(None);
		tracing::debug!(class = "CoreRegistry", "constructed napi class");
		Self {
			inner: Mutex::new(Some(NativeCoreRegistry::new())),
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
			})
			.await
			.map_err(napi_anyhow_error)
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
