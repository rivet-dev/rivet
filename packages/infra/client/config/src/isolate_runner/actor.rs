use std::collections::HashMap;

use pegboard::protocol;
use serde::{Deserialize, Serialize};

/// Config for running an isolate. Similar to runc config.
#[derive(Serialize, Deserialize)]
pub struct Config {
	pub resources: Resources,
	pub ports: Vec<Port>,
	pub env: HashMap<String, String>,
	pub owner: protocol::ActorOwner,
	pub vector_socket_addr: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Port {
	pub target: u16,
	pub protocol: protocol::TransportProtocol,
}

#[derive(Serialize, Deserialize)]
pub struct Resources {
	/// Bytes.
	pub memory: u64,
	/// Bytes.
	pub memory_max: u64,
}
