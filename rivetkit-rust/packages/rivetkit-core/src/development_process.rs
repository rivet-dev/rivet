use std::path::PathBuf;

use anyhow::Result;
use rivetkit_engine_process::download;

use crate::engine_process::{EngineProcessManager, EngineResolverConfig};
use crate::registry::{ServeConfig, parse_engine_port};
use crate::services_process::{
	SERVICES_POOL_NAME, ServicesProcessConfig, ServicesProcessManager, skip_unsupported_services,
};

const RUN_SERVICES_ENV: &str = "RIVET_RUN_SERVICES";
const RUN_ENGINE_HOST_ENV: &str = "RIVET_RUN_ENGINE_HOST";
const RUN_ENGINE_PORT_ENV: &str = "RIVET_RUN_ENGINE_PORT";
const ENGINE_BINARY_PATH_ENV: &str = "RIVET_ENGINE_BINARY_PATH";
const SERVICES_BINARY_ENV: &str = "RIVET_SERVICES_BINARY";

/// Local development settings after applying, in order, the explicit
/// `ServeConfig` value, the environment, and the default.
///
/// Every host (TypeScript through NAPI and Rust through `ServeConfig`) resolves
/// through this one function so local Engine and Services startup behaves the
/// same everywhere. Hosts pass only values the user set in code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocalProcessSettings {
	pub(crate) engine_binary_path: Option<PathBuf>,
	pub(crate) engine_host: Option<String>,
	pub(crate) engine_port: Option<u16>,
	pub(crate) engine_version: Option<String>,
	pub(crate) auto_download: bool,
	pub(crate) start_services: bool,
	/// Whether Services was explicitly requested in config or the environment.
	/// When false, a platform with no Services build runs without Services.
	pub(crate) services_required: bool,
	pub(crate) services_binary_path: Option<PathBuf>,
}

impl LocalProcessSettings {
	pub(crate) fn resolve(config: &ServeConfig) -> Result<Self> {
		Self::resolve_with(config, |key| std::env::var(key).ok())
	}

	pub(crate) fn resolve_with(
		config: &ServeConfig,
		env: impl Fn(&str) -> Option<String>,
	) -> Result<Self> {
		let env_value = |key: &str| env(key).filter(|value| !value.trim().is_empty());

		let engine_port = match config.engine_port {
			Some(port) => Some(port),
			None => env_value(RUN_ENGINE_PORT_ENV)
				.map(|value| parse_engine_port(&value))
				.transpose()?,
		};

		// Services defaults on because this only runs when RivetKit manages a
		// local Engine. An explicit choice in config or the environment makes a
		// missing Services build an error instead of a warning.
		let (start_services, services_required) =
			match (config.start_services, env_value(RUN_SERVICES_ENV)) {
				(Some(start), _) => (start, start),
				(None, Some(value)) => {
					let start = download::is_truthy(&value);
					(start, start)
				}
				(None, None) => (true, false),
			};

		Ok(Self {
			engine_binary_path: config
				.engine_binary_path
				.clone()
				.or_else(|| env_value(ENGINE_BINARY_PATH_ENV).map(PathBuf::from)),
			engine_host: config
				.engine_host
				.clone()
				.or_else(|| env_value(RUN_ENGINE_HOST_ENV)),
			engine_port,
			engine_version: rivetkit_engine_process::engine_version_override(
				config.engine_version.clone(),
				&env,
			),
			auto_download: download::resolve_auto_download(config.engine_auto_download, true, &env),
			start_services,
			services_required,
			services_binary_path: config
				.services_binary_path
				.clone()
				.or_else(|| env_value(SERVICES_BINARY_ENV).map(PathBuf::from)),
		})
	}
}

/// Owns the processes RivetKit starts for a local development environment.
///
/// The Engine and actor hosts have deliberately different lifetimes: the Engine
/// is reusable and outlives the application process, while Services is
/// tied to this host and shuts down with it. Keeping the orchestration here
/// prevents serverful and serverless runtimes from growing separate startup
/// behavior without forcing those incompatible lifetime rules into one generic
/// subprocess abstraction.
#[derive(Debug)]
pub(crate) struct DevelopmentProcessManager {
	_engine: EngineProcessManager,
	services: Option<ServicesProcessManager>,
}

impl DevelopmentProcessManager {
	pub(crate) async fn start(config: &ServeConfig) -> Result<Self> {
		let settings = LocalProcessSettings::resolve(config)?;
		let engine = EngineProcessManager::start_or_reuse(
			EngineResolverConfig::from_parts(
				&config.endpoint,
				settings.engine_binary_path.clone(),
				settings.engine_host.clone(),
				settings.engine_port,
				settings.auto_download,
			)
			.with_engine_version(settings.engine_version.clone()),
		)
		.await?;

		let services_config = ServicesProcessConfig {
			binary_path: settings.services_binary_path.clone(),
			endpoint: config.endpoint.clone(),
			token: config.token.clone(),
			namespace: config.namespace.clone(),
			pool_name: SERVICES_POOL_NAME.to_owned(),
			engine_protocol_version: rivet_envoy_client::protocol::PROTOCOL_VERSION,
			auto_download: settings.auto_download,
		};
		let services = if settings.start_services
			&& !skip_unsupported_services(&services_config, settings.services_required)
		{
			Some(ServicesProcessManager::start(services_config).await?)
		} else {
			None
		};

		Ok(Self {
			_engine: engine,
			services,
		})
	}

	pub(crate) async fn shutdown(mut self) {
		if let Some(services) = self.services.take() {
			services.shutdown().await;
		}
	}
}

#[cfg(test)]
#[path = "../tests/development_process.rs"]
mod tests;
