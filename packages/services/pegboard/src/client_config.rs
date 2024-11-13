use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct ClientConfig {
	/// See pegboard_manager::config::Config::actor_network_ip.
	pub actor_network_ip: Ipv4Addr,

	/// See pegboard_manager::config::Config::actor_vlan_ip.
	pub actor_vlan_ip: IpAddr,

	/// See pegboard_manager::config::Config::actor_public_ip.
	pub actor_public_ip: IpAddr,

	// Millicores
	pub reserved_cpu: u64,

	// MiB
	pub reserved_memory: u64,
}
