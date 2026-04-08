use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Configuration for the cache layer.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Cache {
	pub enabled: bool,
	pub driver: Option<CacheDriver>,
}

impl Default for Cache {
	fn default() -> Cache {
		Self {
			enabled: true,
			driver: None,
		}
	}
}

impl Cache {
	pub fn driver(&self) -> CacheDriver {
		self.driver.clone().unwrap_or(CacheDriver::InMemory)
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CacheDriver {
	InMemory,
}
