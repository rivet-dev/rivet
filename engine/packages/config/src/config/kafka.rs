use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::secret::Secret;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Kafka {
	pub url: Url,
	pub username: String,
	pub password: Secret<String>,
	pub ca_pem: Secret<String>,
}
