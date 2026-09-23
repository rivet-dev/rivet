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
	/// every commit. Unlike `dynamic`, it is process-local and is never broadcast.
	build_meta: Arc<BuildMeta>,
	protocols: RwLock<Arc<RuntimeProtocols>>,
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
				.separator("__")
				.list_separator(",")
				.with_list_parse_key("foundationdb.addresses"),
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

	/// Builds a config with placeholder build metadata and protocol versions.
	pub fn from_root(config: config::Root) -> Self {
		Self::from_root_with_build_meta(config, BuildMeta::default(), RuntimeProtocols::default())
	}

	pub fn from_root_with_build_meta(
		config: config::Root,
		build_meta: BuildMeta,
		protocols: RuntimeProtocols,
	) -> Self {
		let shared = Arc::new(config.clone());
		let (dynamic_tx, _) = tokio::sync::watch::channel(shared.clone());
		Self(Arc::new(ConfigData {
			dynamic: RwLock::new(shared),
			dynamic_tx,
			build_meta: Arc::new(build_meta),
			protocols: RwLock::new(Arc::new(protocols)),
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
		self.0.protocols.read().clone()
	}

	pub fn auth_required(&self) -> Result<&config::Auth> {
		self.0
			.config
			.auth
			.as_ref()
			.context("authentication is not configured")
	}

	pub fn set_protocols(&self, protocol: RuntimeProtocols) {
		*self.0.protocols.write() = Arc::new(protocol);
	}

	/// The config with runtime changes applied.
	///
	/// Reading a property through this instead of dereferencing the `Config` is what opts a call
	/// site into runtime reconfiguration: `config.sqlite()` always returns the value loaded at
	/// startup, while `config.dynamic().sqlite()` returns the value currently in effect. Take this
	/// snapshot once per logical decision rather than holding it, since a later read can return
	/// different values.
	pub fn dynamic(&self) -> Arc<config::Root> {
		self.0.dynamic.read().clone()
	}

	/// Subscribes to dynamic config changes. The receiver always starts with the current config.
	pub fn dynamic_watch(&self) -> tokio::sync::watch::Receiver<Arc<config::Root>> {
		self.0.dynamic_tx.subscribe()
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

		if let Some(value) = update.max_storage_bytes {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.max_storage_bytes = value.or(base.sqlite().max_storage_bytes);
		}
		if let Some(value) = update.compaction_admission_percent {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.compaction_admission_percent = value.or(base.sqlite().compaction_admission_percent);
		}
		if let Some(value) = update.compaction_write_bytes_per_second {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.compaction_write_bytes_per_second = value.or(base.sqlite().compaction_write_bytes_per_second);
		}
		if let Some(value) = update.actor_write_bytes_per_second {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.actor_write_bytes_per_second = value.or(base.sqlite().actor_write_bytes_per_second);
		}
		if let Some(value) = update.actor_read_bytes_per_second {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.actor_read_bytes_per_second = value.or(base.sqlite().actor_read_bytes_per_second);
		}
		if let Some(value) = update.compaction_read_bytes_per_second {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.compaction_read_bytes_per_second = value.or(base.sqlite().compaction_read_bytes_per_second);
		}
		if let Some(value) = update.compaction_hot_fold_direct_to_shard {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.compaction_hot_fold_direct_to_shard =
				value.or(base.sqlite().compaction_hot_fold_direct_to_shard);
		}
		if let Some(value) = update.compaction_max_hot_drain_span_txids {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.compaction_max_hot_drain_span_txids =
				value.or(base.sqlite().compaction_max_hot_drain_span_txids);
		}

		if let Some(value) = update.compaction_stage_throttle_budget_multiplier {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.compaction_stage_throttle_budget_multiplier =
				value.or(base.sqlite().compaction_stage_throttle_budget_multiplier);
		}
		if let Some(value) = update.compaction_stage_throttle_admit_soft_util {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.compaction_stage_throttle_admit_soft_util =
				value.or(base.sqlite().compaction_stage_throttle_admit_soft_util);
		}
		if let Some(value) = update.compaction_stage_throttle_backoff_ms {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.compaction_stage_throttle_backoff_ms =
				value.or(base.sqlite().compaction_stage_throttle_backoff_ms);
		}
		if let Some(value) = update.compaction_install_throttle_budget_multiplier {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.compaction_install_throttle_budget_multiplier =
				value.or(base.sqlite().compaction_install_throttle_budget_multiplier);
		}
		if let Some(value) = update.compaction_install_throttle_admit_soft_util {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.compaction_install_throttle_admit_soft_util =
				value.or(base.sqlite().compaction_install_throttle_admit_soft_util);
		}
		if let Some(value) = update.compaction_reclaim_throttle_budget_multiplier {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.compaction_reclaim_throttle_budget_multiplier =
				value.or(base.sqlite().compaction_reclaim_throttle_budget_multiplier);
		}
		if let Some(value) = update.compaction_reclaim_throttle_admit_soft_util {
			dynamic
				.sqlite
				.get_or_insert_with(config::Sqlite::default)
				.compaction_reclaim_throttle_admit_soft_util =
				value.or(base.sqlite().compaction_reclaim_throttle_admit_soft_util);
		}
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
