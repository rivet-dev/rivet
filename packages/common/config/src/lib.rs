use ::config as config_loader;
use global_error::prelude::*;
use std::{ops::Deref, path::Path, sync::Arc};

pub mod config;
pub mod paths;
pub mod secret;

struct ConfigData {
	config: config::Root,
}

#[derive(Clone)]
pub struct Config(Arc<ConfigData>);

impl Config {
	pub async fn load<P: AsRef<Path>>(paths: &[P]) -> GlobalResult<Self> {
		let mut settings = config_loader::Config::builder();

		if paths.is_empty() {
			// Add default config directory
			settings = add_source(settings, paths::system_config_dir())?;
		} else {
			// Use config paths provided
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
		let config = unwrap!(
			unwrap!(settings.build(), "failed to build config").try_deserialize::<config::Root>(),
			"failed to deserialize config"
		);

		Ok(Self(Arc::new(ConfigData { config })))
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
/// If it's a file, it adds it directly.
fn add_source<P: AsRef<Path>>(
	mut settings: config_loader::ConfigBuilder<config_loader::builder::DefaultState>,
	path: P,
) -> GlobalResult<config_loader::ConfigBuilder<config_loader::builder::DefaultState>> {
	let path = path.as_ref();

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
) -> GlobalResult<config_loader::ConfigBuilder<config_loader::builder::DefaultState>> {
	let path = path.as_ref();
	let content = unwrap!(
		std::fs::read_to_string(path),
		"failed to read file: {}",
		path.display()
	);

	let format = match path.extension().and_then(std::ffi::OsStr::to_str) {
		Some("json") => config_loader::FileFormat::Json,
		Some("json5") | Some("jsonc") => {
			// Parse JSON5/JSONC and convert to regular JSON
			let value: serde_json::Value =
				json5::from_str(&content).map_err(|e| global_error::GlobalError::new(e))?;
			let json =
				serde_json::to_string(&value).map_err(|e| global_error::GlobalError::new(e))?;
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
