use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Auth {
	pub admin_token: String,
}

impl Default for Auth {
	fn default() -> Self {
		Auth {
			admin_token: "admin".to_string(),
		}
	}
}
