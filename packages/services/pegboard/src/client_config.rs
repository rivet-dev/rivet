use serde::{Deserialize, Serialize};

/// See corresponding documentation in `pegboard_manager::config::Config`
#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct ClientConfig {
	pub actor: Actor,
	pub reserved_resources: ReservedResources,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct Actor {
	pub network: ActorNetwork,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct ActorNetwork {
	pub bind_ip: String,
	pub lan_ip: String,
	pub wan_ip: String,
	pub lan_port_range_min: u16,
	pub lan_port_range_max: u16,
	pub wan_port_range_min: u16,
	pub wan_port_range_max: u16,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct ReservedResources {
	// Millicores
	pub cpu: u64,
	// Mib
	pub memory: u64,
}
