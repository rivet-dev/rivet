use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Compression;

// TODO: Add back `deny_unknown_fields` after https://github.com/serde-rs/serde/issues/1600
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Build {
	pub script: String,
	#[serde(default)]
	pub unstable: Unstable,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Unstable {
	pub minify: Option<bool>,
	pub analyze_result: Option<bool>,
	pub esbuild_log_level: Option<String>,
	pub compression: Option<Compression>,
	pub dump_build: Option<bool>,
	pub no_bundler: Option<bool>,
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

	pub fn no_bundler(&self) -> bool {
		self.no_bundler.unwrap_or(false)
	}
}
