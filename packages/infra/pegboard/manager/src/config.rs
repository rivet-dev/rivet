use std::{
	net::{Ipv4Addr, SocketAddr},
	path::{Path, PathBuf},
};

use pegboard::protocol;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone, Deserialize)]
pub struct Config {
	pub client_id: Uuid,
	pub datacenter_id: Uuid,
	pub network_ip: Ipv4Addr,
	pub vector_socket_addr: SocketAddr,
	pub flavor: protocol::ClientFlavor,
	#[serde(default = "default_redirect_logs")]
	pub redirect_logs: bool,

	pub api_endpoint: String,

	#[serde(default = "default_working_path")]
	pub working_path: PathBuf,
	#[serde(default = "default_container_runner_binary_path")]
	pub container_runner_binary_path: PathBuf,
	#[serde(default = "default_isolate_runner_binary_path")]
	pub isolate_runner_binary_path: PathBuf,
}

fn default_working_path() -> PathBuf {
	Path::new("/etc/pegboard").to_path_buf()
}

fn default_container_runner_binary_path() -> PathBuf {
	default_working_path().join("bin").join("container-runner")
}

fn default_isolate_runner_binary_path() -> PathBuf {
	default_working_path().join("bin").join("v8-isolate-runner")
}

fn default_redirect_logs() -> bool {
	true
}
