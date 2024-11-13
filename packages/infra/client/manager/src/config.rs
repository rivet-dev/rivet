use std::{
	net::{IpAddr, Ipv4Addr, SocketAddr},
	path::{Path, PathBuf},
};

use pegboard::protocol;
use serde::Deserialize;
use url::Url;
use uuid::Uuid;

#[derive(Clone, Deserialize)]
pub struct Config {
	pub client_id: Uuid,
	pub datacenter_id: Uuid,

	pub vector_socket_addr: Option<SocketAddr>,

	pub flavor: protocol::ClientFlavor,

	/// Address to serve actor traffic on.
	///
	/// This will usually be the same as `actor_vlan_ip` unless the node is accessed within the
	/// VLAN by a different IP.
	pub actor_network_ip: Ipv4Addr,

	/// Address to access this node in a VLAN.
	///
	/// This IP is used to route traffic from Game Guard.
	pub actor_vlan_ip: IpAddr,

	/// Address to access this node publicly.
	///
	/// This IP is used when providing the actor's IP & port for host networking.
	pub actor_public_ip: IpAddr,

	#[serde(default = "default_redirect_logs")]
	pub redirect_logs: bool,

	pub pegboard_ws_endpoint: Url,

	pub api_public_endpoint: Url,

	#[serde(default = "default_working_path")]
	pub data_dir: PathBuf,

	#[serde(default = "default_container_runner_binary_path")]
	pub container_runner_binary_path: PathBuf,

	#[serde(default = "default_isolate_runner_binary_path")]
	pub isolate_runner_binary_path: PathBuf,

	#[serde(default = "default_reserved_cpu")]
	pub reserved_cpu: u64,

	#[serde(default = "default_reserved_memory")]
	pub reserved_memory: u64,
}

impl Config {
	/// Builds a config that will be sent to the server.
	///
	/// This holds information that the server needs in order to orchestrate nodes.
	pub fn build_client_config(&self) -> pegboard::client_config::ClientConfig {
		pegboard::client_config::ClientConfig {
			actor_network_ip: self.actor_network_ip,
			actor_vlan_ip: self.actor_vlan_ip,
			actor_public_ip: self.actor_public_ip,
			reserved_cpu: self.reserved_cpu,
			reserved_memory: self.reserved_memory,
		}
	}
}

fn default_working_path() -> PathBuf {
	Path::new("/var/lib/rivet-client").to_path_buf()
}

fn default_container_runner_binary_path() -> PathBuf {
	Path::new("/usr/local/bin/rivet-container-runner").into()
}

fn default_isolate_runner_binary_path() -> PathBuf {
	Path::new("/usr/local/bin/rivet-isolate-v8-runner").into()
}

fn default_redirect_logs() -> bool {
	true
}

fn default_reserved_cpu() -> u64 {
	0
}

fn default_reserved_memory() -> u64 {
	128
}
