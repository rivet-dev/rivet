use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct CargoConfig {
	pub package: CargoPackage,
	pub dependencies: HashMap<String, CargoDependency>,
	#[serde(default)]
	pub dev_dependencies: HashMap<String, CargoDependency>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct CargoPackage {
	pub name: String,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged, rename_all = "kebab-case")]
pub enum CargoDependency {
	Path { path: String },
	Unknown(serde_json::Value),
}
