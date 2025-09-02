use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::secret::Secret;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PubSub {
	Nats(Nats),
	PostgresNotify(Postgres),
	Memory(Memory),
}

impl Default for PubSub {
	fn default() -> Self {
		PubSub::Memory(Memory::default())
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct Postgres {
	/// URL to connect to Postgres with
	///
	/// See: https://docs.rs/postgres/0.19.10/postgres/config/struct.Config.html#url
	pub url: Secret<String>,
}

impl Default for Postgres {
	fn default() -> Self {
		Self {
			url: Secret::new("postgresql://postgres:postgres@127.0.0.1:5432/postgres".into()),
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Nats {
	pub addresses: Vec<String>,
	pub port: Option<u16>,
	#[serde(default)]
	pub username: Option<String>,
	#[serde(default)]
	pub password: Option<Secret<String>>,
}

impl Default for Nats {
	fn default() -> Self {
		Self {
			addresses: vec!["127.0.0.1:4222".to_string()],
			port: None,
			username: None,
			password: None,
		}
	}
}

impl Nats {
	pub fn port(&self) -> u16 {
		self.port.unwrap_or(4222)
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Memory {
	#[serde(default = "Memory::default_channel")]
	pub channel: String,
}

impl Default for Memory {
	fn default() -> Self {
		Self {
			channel: Self::default_channel(),
		}
	}
}

impl Memory {
	fn default_channel() -> String {
		"default".to_string()
	}
}
