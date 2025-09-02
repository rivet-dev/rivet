use std::{ops::Deref, path::Path, result::Result::Ok, sync::Arc};

use ::config as config_loader;
use anyhow::*;

pub mod config;
pub mod defaults;
pub mod paths;
pub mod secret;

struct ConfigData {
	config: config::Root,
}

#[derive(Clone)]
pub struct Config(Arc<ConfigData>);

impl Config {
	pub async fn load<P: AsRef<Path>>(paths: &[P]) -> Result<Self> {
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
				.separator("__")
				.list_separator(","),
		);

		// Read config
		let mut config_root = settings
			.build()
			.context("failed to build config")?
			.try_deserialize::<config::Root>()
			.context("failed to deserialize config")?;

		// Validate configuration at load time
		config_root.validate_and_set_defaults()?;

		Ok(Self(Arc::new(ConfigData {
			config: config_root,
		})))
	}

	pub fn from_root(config: config::Root) -> Self {
		Self(Arc::new(ConfigData { config }))
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
		// Silently ignore non-existent paths
		return Ok(settings);
	}

	if path.is_dir() {
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
		settings = add_file_source(settings, path)?;
	} else {
		bail!("Invalid path: {}", path.display());
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
