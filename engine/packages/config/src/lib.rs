use std::{ops::Deref, path::Path, result::Result::Ok, sync::Arc};

use ::config as config_loader;
use anyhow::*;
use parking_lot::RwLock;

pub mod build_meta;
pub mod config;
pub mod defaults;
pub mod dynamic;
pub mod paths;
pub mod runtime_protocols;
pub mod secret;

pub use build_meta::BuildMeta;
pub use dynamic::DynamicConfigUpdate;
pub use runtime_protocols::{RuntimeProtocol, RuntimeProtocolKind, RuntimeProtocols};

struct ConfigData {
	/// The config as loaded from files and the environment. Never changes for the life of the
	/// process, so it stays reachable by reference through `Deref`.
	config: config::Root,
	/// The same config with any runtime changes applied, which reading is opt-in through
	/// [`Config::dynamic`].
	///
	/// This is a sync lock because it is read from sync `&self` accessors. Every critical section
	/// clones an `Arc` or swaps one in, so a guard is never held across an await.
	dynamic: RwLock<Arc<config::Root>>,
	/// Notifies long-lived workers when the dynamic config changes.
	dynamic_tx: tokio::sync::watch::Sender<Arc<config::Root>>,
	/// Build metadata for this process, stamped once at startup by the binary that has it.
	///
	/// This does not come from the config file or the environment. It lives here so that reading
	/// build metadata does not require depending on the crate that carries it, which is rebuilt on
	/// every commit. Unlike `dynamic`, it is process-local and is never shared.
	build_meta: Arc<BuildMeta>,
	/// Stores the current runtime protocols and notifies process-local subscribers of changes.
	protocols_tx: tokio::sync::watch::Sender<Arc<RuntimeProtocols>>,
}

#[derive(Clone)]
pub struct Config(Arc<ConfigData>);

impl Config {
	/// Loads the config, stamping the build metadata and compiled protocol versions of the binary
	/// doing the loading.
	///
	/// Only the binary that carries those values can supply real ones. Anything else passes
	/// `Default::default()` and gets the placeholders documented on [`BuildMeta`] and
	/// [`RuntimeProtocols`].
	#[allow(
		clippy::disallowed_methods,
		reason = "configuration discovery runs at bootstrap before adapters exist"
	)]
	pub async fn load<P: AsRef<Path>>(
		paths: &[P],
		build_meta: BuildMeta,
		protocols: RuntimeProtocols,
	) -> Result<Self> {
		let mut settings = config_loader::Config::builder();

		// Start with default values
		settings = settings.add_source(config_loader::Config::try_from(&config::Root::default())?);

		if paths.is_empty() {
			let default_path = paths::system_config_dir();
			if default_path.exists() {
				// Add default config directory if it exists
				settings = add_source(settings, default_path)?;
			}
		} else {
			// Use provided paths
			for path in paths {
				settings = add_source(settings, path)?;
			}
		}

		// Add env source for overrides
		settings = settings.add_source(
			config_loader::Environment::with_prefix("RIVET")
				.try_parsing(true)
				.separator("__"),
		);

		// Read config
		let mut config_root = settings
			.build()
			.context("failed to build config")?
			.try_deserialize::<config::Root>()
			.context("failed to deserialize config")?;

		// Validate configuration at load time
		config_root.validate_and_set_defaults()?;

		Ok(Self::from_root_with_build_meta(
			config_root,
			build_meta,
			protocols,
		))
	}

	/// Builds a validated config with placeholder build metadata and protocol versions.
	pub fn try_from_root(mut config: config::Root) -> Result<Self> {
		config.validate_and_set_defaults()?;
		Ok(Self::from_root_with_build_meta(
			config,
			BuildMeta::default(),
			RuntimeProtocols::default(),
		))
	}

	/// Builds a config without validating or filling defaults.
	///
	/// This is for tests and embedded callers that intentionally construct partial roots. Runtime
	/// configuration should use [`Config::load`] or [`Config::try_from_root`] instead.
	pub fn from_root_unchecked(config: config::Root) -> Self {
		Self::from_root_with_build_meta(config, BuildMeta::default(), RuntimeProtocols::default())
	}

	pub fn from_root_with_build_meta(
		config: config::Root,
		build_meta: BuildMeta,
		protocols: RuntimeProtocols,
	) -> Self {
		let shared = Arc::new(config.clone());
		let (dynamic_tx, _) = tokio::sync::watch::channel(shared.clone());
		let (protocols_tx, _) = tokio::sync::watch::channel(Arc::new(protocols));
		Self(Arc::new(ConfigData {
			dynamic: RwLock::new(shared),
			dynamic_tx,
			build_meta: Arc::new(build_meta),
			protocols_tx,
			config,
		}))
	}

	/// Build metadata for this process.
	///
	/// Returns placeholder values for a process that did not stamp its own metadata.
	pub fn build_meta(&self) -> Arc<BuildMeta> {
		self.0.build_meta.clone()
	}

	pub fn protocols(&self) -> Arc<RuntimeProtocols> {
		self.0.protocols_tx.borrow().clone()
	}

	/// Subscribes to process-local runtime protocol snapshots, retaining only the latest value.
	///
	/// The initial value is already considered seen. Reconcile a cloned `borrow_and_update()`
	/// snapshot before awaiting `changed()`, including on startup at the latest version. Drop the
	/// borrow before async work so it cannot block publishers. Multiple updates may be coalesced.
	pub fn protocols_watch(&self) -> tokio::sync::watch::Receiver<Arc<RuntimeProtocols>> {
		self.0.protocols_tx.subscribe()
	}

	pub fn auth_required(&self) -> Result<&config::Auth> {
		self.0
			.config
			.auth
			.as_ref()
			.context("authentication is not configured")
	}

	/// Authentication configuration for callers using a validated runtime config.
	pub fn auth(&self) -> &config::Auth {
		self.auth_required()
			.expect("validated config must contain authentication configuration")
	}

	pub fn set_protocols(&self, protocol: RuntimeProtocols) {
		self.0.protocols_tx.send_if_modified(|current| {
			if **current == protocol {
				return false;
			}
			*current = Arc::new(protocol);
			true
		});
	}

	/// The config with runtime changes applied.
	///
	/// Reading a property through this instead of dereferencing the `Config` is what opts a call
	/// site into runtime reconfiguration: `config.runtime.worker_poll_interval()` returns the
	/// startup value, while `config.dynamic().runtime.worker_poll_interval()` returns the current
	/// value. Take this snapshot once per logical decision rather than holding it, since a later
	/// read can return different values.
	pub fn dynamic(&self) -> Arc<config::Root> {
		self.0.dynamic.read().clone()
	}

	/// Subscribes to dynamic config changes. The receiver always starts with the current config.
	pub fn dynamic_watch(&self) -> tokio::sync::watch::Receiver<Arc<config::Root>> {
		self.0.dynamic_tx.subscribe()
	}

	/// Validates a durable dynamic snapshot without changing the live config.
	pub fn validate_dynamic(&self, update: &DynamicConfigUpdate) -> Result<()> {
		Self::from_root_unchecked(self.0.config.clone()).apply_dynamic(update)?;
		Ok(())
	}

	/// Applies a runtime config change to every handle sharing this config, and returns the
	/// resulting config.
	///
	/// The result goes through the same validation as a config loaded from disk, and nothing is
	/// published to other readers if it fails. Cleared properties revert to the value loaded at
	/// startup.
	pub fn apply_dynamic(&self, update: &DynamicConfigUpdate) -> Result<Arc<config::Root>> {
		let base = &self.0.config;
		let mut guard = self.0.dynamic.write();
		let mut dynamic = (**guard).clone();

		if let Some(value) = update.worker_poll_interval_ms {
			dynamic.runtime.worker_poll_interval_ms =
				value.or(base.runtime.worker_poll_interval_ms);
		}
		if let Some(value) = update.worker_max_deduped_workflows_per_pull {
			dynamic.runtime.worker_max_deduped_workflows_per_pull =
				value.or(base.runtime.worker_max_deduped_workflows_per_pull);
		}
		if let Some(value) = update.worker_max_workflows_per_pull {
			dynamic.runtime.worker_max_workflows_per_pull =
				value.or(base.runtime.worker_max_workflows_per_pull);
		}
		if let Some(value) = update.worker_max_wake_condition_clears_per_pull {
			dynamic.runtime.worker_max_wake_condition_clears_per_pull =
				value.or(base.runtime.worker_max_wake_condition_clears_per_pull);
		}
		if let Some(value) = update.sqlite_server_prefetch {
			let base = base
				.pegboard
				.as_ref()
				.and_then(|pegboard| pegboard.sqlite_server_prefetch);
			dynamic
				.pegboard
				.get_or_insert_with(Default::default)
				.sqlite_server_prefetch = value.or(base);
		}
		if let Some(overrides) = &update.worker_max_wake_keys_per_workflow_name_per_pull {
			let base = base
				.runtime
				.worker_max_wake_keys_per_workflow_name_per_pull
				.as_ref();
			let map = dynamic
				.runtime
				.worker_max_wake_keys_per_workflow_name_per_pull
				.get_or_insert_with(Default::default);

			for (workflow_name, max) in overrides {
				match max {
					Some(max) => {
						map.insert(workflow_name.clone(), *max);
					}
					// Clearing one name reverts it to the value loaded at startup, which may be no
					// entry at all.
					None => match base.and_then(|base| base.get(workflow_name)) {
						Some(base_max) => {
							map.insert(workflow_name.clone(), *base_max);
						}
						None => {
							map.remove(workflow_name);
						}
					},
				}
			}
		}
		if let Some(overrides) = &update.worker_max_concurrent_workflows {
			let base = base.runtime.worker_max_concurrent_workflows.as_ref();
			let map = dynamic
				.runtime
				.worker_max_concurrent_workflows
				.get_or_insert_with(Default::default);

			for (workflow_name, max) in overrides {
				match max {
					Some(max) => {
						map.insert(workflow_name.clone(), *max);
					}
					// Clearing one name reverts it to the value loaded at startup, which may be no
					// entry at all.
					None => match base.and_then(|base| base.get(workflow_name)) {
						Some(base_max) => {
							map.insert(workflow_name.clone(), *base_max);
						}
						None => {
							map.remove(workflow_name);
						}
					},
				}
			}
		}

		if let Some(value) = update.depot_balancing_enabled {
			let enabled = value.unwrap_or(base.depot().balancing.enabled);
			dynamic
				.depot
				.get_or_insert_with(Default::default)
				.balancing
				.enabled = enabled;
		}

		// Runtime snapshots already contain the names derived at startup. Restore
		// map input before validation so these are not treated as name overrides.
		if let Some(topology) = &mut dynamic.topology {
			if let config::topology::DatacentersRepr::Map(datacenters) = &mut topology.datacenters {
				for (name, datacenter) in datacenters {
					if datacenter.name == *name {
						datacenter.name.clear();
					}
				}
			}
		}
		dynamic.validate_and_set_defaults()?;

		let dynamic = Arc::new(dynamic);
		*guard = dynamic.clone();
		self.0.dynamic_tx.send_replace(dynamic.clone());

		Ok(dynamic)
	}
}

impl Deref for Config {
	type Target = config::Root;

	fn deref(&self) -> &Self::Target {
		&self.0.config
	}
}

/// Adds a source to the config builder. If the path is a directory, it reads all config files.
/// If it's a file, it adds it directly. If the path doesn't exist, it's silently ignored.
#[allow(
	clippy::disallowed_methods,
	clippy::disallowed_types,
	reason = "configuration files are read at bootstrap before adapters exist"
)]
fn add_source<P: AsRef<Path>>(
	mut settings: config_loader::ConfigBuilder<config_loader::builder::DefaultState>,
	path: P,
) -> Result<config_loader::ConfigBuilder<config_loader::builder::DefaultState>> {
	let path = path.as_ref();

	if !path.exists() {
		tracing::debug!(path=%path.display(), "ignoring non-existent config path");

		// Silently ignore non-existent paths
		return Ok(settings);
	}

	if path.is_dir() {
		tracing::debug!(path=%path.display(), "loading config from directory");

		for entry in std::fs::read_dir(path)? {
			let entry = entry?;
			let path = entry.path();
			if path.is_file() {
				if let Some(extension) = path.extension().and_then(std::ffi::OsStr::to_str) {
					if ["json", "json5", "jsonc", "yaml", "yml"].contains(&extension) {
						settings = add_file_source(settings, &path)?;
					}
				}
			}
		}
	} else if path.is_file() {
		tracing::debug!(path=%path.display(), "loading config from file");

		settings = add_file_source(settings, path)?;
	} else {
		bail!(
			"Invalid Rivet config path: {}. Ensure the path exists and is either a directory with config files or a specific config file.",
			path.display()
		);
	}

	Ok(settings)
}

/// Adds a single file source to the config builder.
#[allow(
	clippy::disallowed_methods,
	clippy::disallowed_types,
	reason = "configuration files are read at bootstrap before adapters exist"
)]
fn add_file_source<P: AsRef<Path>>(
	settings: config_loader::ConfigBuilder<config_loader::builder::DefaultState>,
	path: P,
) -> Result<config_loader::ConfigBuilder<config_loader::builder::DefaultState>> {
	let path = path.as_ref();
	let content = std::fs::read_to_string(path)
		.with_context(|| format!("failed to read file: {}", path.display()))?;

	let format = match path.extension().and_then(std::ffi::OsStr::to_str) {
		Some("json") => config_loader::FileFormat::Json,
		Some("json5") | Some("jsonc") => {
			// Parse JSON5/JSONC and convert to regular JSON
			let value = match json5::from_str::<serde_json::Value>(&content) {
				Ok(x) => x,
				Err(err) => bail!("failed to parse config file at {}: {err}", path.display()),
			};
			let json = serde_json::to_string(&value)?;
			return Ok(settings.add_source(config_loader::File::from_str(
				&json,
				config_loader::FileFormat::Json,
			)));
		}
		Some("yaml") | Some("yml") => config_loader::FileFormat::Yaml,
		_ => bail!("Unsupported file format: {}", path.display()),
	};

	Ok(settings.add_source(config_loader::File::from_str(&content, format)))
}
