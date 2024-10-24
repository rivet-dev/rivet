use std::collections::HashMap;

use deno_runtime::deno_permissions;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
	pub resources: Resources,
	pub ports: Vec<Port>,
	pub env: HashMap<String, String>,
	pub stakeholder: Stakeholder,
}

#[derive(Deserialize)]
pub struct Port {
	pub target: u16,
	pub protocol: Protocol,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
	Tcp,
	Udp,
}

impl From<Protocol> for deno_permissions::Protocol {
	fn from(value: Protocol) -> Self {
		match value {
			Protocol::Tcp => deno_permissions::Protocol::Tcp,
			Protocol::Udp => deno_permissions::Protocol::Udp,
		}
	}
}

#[derive(Deserialize)]
pub struct Resources {
	/// Bytes.
	pub memory: u64,
	/// Bytes.
	pub memory_max: u64,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stakeholder {
	DynamicServer { server_id: String },
}
