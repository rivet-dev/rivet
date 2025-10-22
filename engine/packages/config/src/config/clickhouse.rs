use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
	pub provision_users: HashMap<String, ClickHouseUser>,
	#[serde(default)]
	pub secure: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClickHouseUser {
	pub username: String,
	pub password: Secret<String>,
	pub role: ClickHouseUserRole,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub enum ClickHouseUserRole {
	Admin,
	Write,
	ReadOnly,
}

impl ClickHouseUserRole {
	pub fn to_clickhouse_role(&self) -> &'static str {
		use ClickHouseUserRole::*;
		match self {
			Admin => "admin",
			Write => "write",
			ReadOnly => "readonly",
		}
	}
}
