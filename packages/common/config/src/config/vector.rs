use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorHttp {
	pub host: String,
	pub port: u16,
}

impl Default for VectorHttp {
	fn default() -> Self {
		Self {
			host: "127.0.0.1".into(),
			port: 5022,
		}
	}
}
