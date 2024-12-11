use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Compression;

// TODO: Add back `deny_unknown_fields` after https://github.com/serde-rs/serde/issues/1600
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Build {
	pub script: String,
	pub bundler: Option<Bundler>,
	#[serde(default)]
	pub deno: Deno,
	#[serde(default)]
	pub unstable: Unstable,
}

impl Build {
	pub fn bundler(&self) -> Bundler {
		self.bundler.unwrap_or(Bundler::Deno)
	}
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Bundler {
	Deno,
	None,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Deno {
	pub config_path: Option<String>,
	pub import_map_url: Option<String>,
	pub lock_path: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Unstable {
	pub minify: Option<bool>,
	pub analyze_result: Option<bool>,
	pub esbuild_log_level: Option<String>,
	pub compression: Option<Compression>,
	pub dump_build: Option<bool>,
}

impl Unstable {
	pub fn minify(&self) -> bool {
		self.minify.unwrap_or(true)
	}

	pub fn analyze_result(&self) -> bool {
		self.analyze_result.unwrap_or(false)
	}

	pub fn esbuild_log_level(&self) -> String {
		self.esbuild_log_level
			.clone()
			.unwrap_or_else(|| "error".to_string())
	}

	pub fn compression(&self) -> Compression {
		// TODO: Change back to Lz4 default
		self.compression.unwrap_or(Compression::None)
	}

	pub fn dump_build(&self) -> bool {
		self.dump_build.unwrap_or(false)
	}
}
