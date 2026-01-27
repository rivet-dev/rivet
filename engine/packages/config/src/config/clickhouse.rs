use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::secret::Secret;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClickHouse {
	/// URL to the HTTP access port for ClickHouse.
	pub http_url: Url,
	/// URL to the native access port for ClickHouse.
	pub native_url: Url,
	pub username: String,
	#[serde(default)]
	pub password: Option<Secret<String>>,
	#[serde(default)]
	pub secure: bool,
}
