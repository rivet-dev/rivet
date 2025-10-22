use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Configuration for the cache layer.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Cache {
	pub driver: CacheDriver,
}

impl Default for Cache {
	fn default() -> Cache {
		Self {
			driver: CacheDriver::InMemory,
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CacheDriver {
	Redis,
	InMemory,
}
