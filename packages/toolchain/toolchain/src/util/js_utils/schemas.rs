pub mod build {
	use serde::{Deserialize, Serialize};
	use std::path::PathBuf;

	#[derive(Serialize)]
	#[serde(rename_all = "camelCase")]
	pub struct Input {
		pub project_root: PathBuf,
		pub entry_point: PathBuf,
		pub out_dir: PathBuf,
		pub bundle: Bundle,
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
