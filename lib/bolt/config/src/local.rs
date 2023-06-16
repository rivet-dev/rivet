use std::path::PathBuf;

use serde::Deserialize;

/// Configuration for the Bolt.local.toml file.
///
/// Use for configuring settings specific to the current development environment.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Local {
	#[serde(default)]
	pub namespace: Option<String>,
	#[serde(default)]
	pub additional_roots: Vec<PathBuf>,
	#[serde(default)]
	pub up: Up,
	#[serde(default)]
	pub generate: Generate,
	#[serde(default)]
	pub rust: Rust,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Up {}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Generate {
	/// Use this if you want rust-analyzer to work without timing out. This
	/// speeds up compilation.
	#[serde(default)]
	pub disable_cargo_workspace: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Rust {
	/// The `--jobs` field passed to cargo jobs. Defaults to your number of
	/// CPUs. Lower if restricted memory.
	#[serde(default)]
	pub num_jobs: Option<usize>,
	#[serde(default)]
	pub message_format: Option<String>,
}
