use std::{collections::HashMap, path::PathBuf};

use deno_runtime::deno_permissions;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
	pub actors_path: PathBuf,
	pub fdb_cluster_path: PathBuf,
}

/// Config for running an isolate. Similar to runc config.
#[derive(Deserialize)]
pub struct ActorConfig {
	pub resources: Resources,
	pub ports: Vec<Port>,
	pub env: HashMap<String, String>,
	pub owner: ActorOwner,
	pub vector_socket_addr: Option<String>,
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
pub enum ActorOwner {
	DynamicServer { server_id: String },
}
