pub mod build {
	use serde::{Deserialize, Serialize};
	use std::path::PathBuf;

	#[derive(Serialize)]
	#[serde(rename_all = "camelCase")]
	pub struct Input {
		pub entry_point: PathBuf,
		pub out_dir: PathBuf,
		pub deno: Deno,
		pub bundle: Bundle,
	}

	#[derive(Serialize)]
	#[serde(rename_all = "camelCase")]
	pub struct Deno {
		#[serde(skip_serializing_if = "Option::is_none")]
		pub config_path: Option<String>,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub import_map_url: Option<String>,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub lock_path: Option<String>,
	}

	#[derive(Serialize)]
	#[serde(rename_all = "camelCase")]
	pub struct Bundle {
		pub minify: bool,
		pub analyze_result: bool,
		pub log_level: String,
	}

	#[derive(Deserialize)]
	#[serde(rename_all = "camelCase")]
	pub struct Output {
		pub files: Vec<String>,
		pub analyzed_metafile: Option<String>,
	}
}
