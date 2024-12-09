use std::{
	collections::HashMap,
	ops::Deref,
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod build;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config(Arc<Root>);

impl Config {
	pub async fn config_path(path: Option<&Path>) -> Result<PathBuf> {
		let path = path.unwrap_or_else(|| Path::new("."));
		let jsonc_path = path.join("rivet.jsonc");
		let json_path = path.join("rivet.json");

		let file_path = match (jsonc_path.exists(), json_path.exists()) {
			(true, true) => bail!("Both rivet.jsonc and rivet.json exist. Please remove one."),
			(false, false) => bail!("Neither rivet.jsonc nor rivet.json exist."),
			(true, false) => jsonc_path,
			(false, true) => json_path,
		};

		Ok(file_path)
	}

	pub async fn load(path: Option<&Path>) -> Result<Self> {
		let file_path = Self::config_path(path).await?;
		let content = tokio::fs::read_to_string(&file_path)
			.await
			.with_context(|| anyhow!("failed to open config: {}", file_path.display()))?;

		let parsed_value = jsonc_parser::parse_to_serde_value(&content, &Default::default())
			.map_err(|err| anyhow!("Failed to parse {}: {err}", file_path.display()))?
			.with_context(|| format!("Config file is empty: {}", file_path.display()))?;
		let root: Root = serde_json::from_value::<Root>(parsed_value)
			.map_err(|err| anyhow!("Invalid config {}: {err}", file_path.display()))?;

		Ok(Config(Arc::new(root)))
	}
}

impl Deref for Config {
	type Target = Root;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref()
	}
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Root {
	pub builds: Vec<Build>,
	pub unstable: Option<Unstable>,
}

impl Root {
	pub fn unstable(&self) -> Unstable {
		self.unstable.clone().unwrap_or_default()
	}
}

// TODO: Add back `deny_unknown_fields` after https://github.com/serde-rs/serde/issues/1600
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Build {
	pub tags: HashMap<String, String>,
	#[serde(flatten)]
	pub runtime: build::Runtime,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Unstable {
	#[serde(default)]
	pub manager: ManagerUnstable,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ManagerUnstable {
	pub enable: Option<bool>,
}

impl ManagerUnstable {
	pub fn enable(&self) -> bool {
		self.enable.unwrap_or(true)
	}
}
