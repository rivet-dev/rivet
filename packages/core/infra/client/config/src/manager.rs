use std::{
	borrow::Cow,
	net::{IpAddr, Ipv4Addr},
	path::{Path, PathBuf},
	time::Duration,
};

use pegboard::protocol;
use schemars::JsonSchema;
use serde::Deserialize;
use url::Url;
use uuid::Uuid;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Config {
	// We store all configs under `client` in order to prevent the config from being mixed up with
	// the server config.
	pub client: Client,
}

impl Config {
	/// Builds a config that will be sent to the server.
	///
	/// This holds information that the server needs in order to orchestrate nodes.
	pub fn build_client_config(&self) -> pegboard::client_config::ClientConfig {
		pegboard::client_config::ClientConfig {
			network: pegboard::client_config::Network {
				bind_ip: IpAddr::V4(self.client.network.bind_ip),
				lan_hostname: self.client.network.lan_hostname.clone(),
				wan_hostname: self.client.network.wan_hostname.clone(),
				lan_port_range_min: self.client.network.lan_port_range_min(),
				lan_port_range_max: self.client.network.lan_port_range_max(),
				wan_port_range_min: self.client.network.wan_port_range_min(),
				wan_port_range_max: self.client.network.wan_port_range_max(),
			},
			reserved_resources: pegboard::client_config::ReservedResources {
				cpu: self.client.reserved_resources.cpu(),
				memory: self.client.reserved_resources.memory(),
			},
		}
	}
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Client {
	pub data_dir: Option<PathBuf>,
	pub cluster: Cluster,
	pub runner: Runner,
	#[serde(default)]
	pub images: Images,
	pub network: Network,
	#[serde(default)]
	pub cni: Cni,
	#[serde(default)]
	pub reserved_resources: ReservedResources,
	#[serde(default)]
	pub logs: Logs,
	#[serde(default)]
	pub metrics: Metrics,
	pub foundationdb: FoundationDb,
	#[serde(default)]
	pub vector: Option<Vector>,
}

impl Client {
	pub fn data_dir(&self) -> PathBuf {
		self.data_dir
			.clone()
			.unwrap_or_else(|| Path::new("/var/lib/rivet-client").to_path_buf())
	}
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Cluster {
	pub client_id: Uuid,
	pub datacenter_id: Uuid,
	pub api_endpoint: Url,
	pub pegboard_endpoint: Url,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Runner {
	pub flavor: protocol::ClientFlavor,
	/// Whether or not to use a mount for actor file systems.
	pub use_mounts: Option<bool>,

	/// WebSocket Port for runners on this machine to connect to.
	pub port: Option<u16>,

	pub container_runner_binary_path: Option<PathBuf>,
	pub isolate_runner_binary_path: Option<PathBuf>,
}

impl Runner {
	pub fn use_mounts(&self) -> bool {
		self.use_mounts.unwrap_or(true)
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(6080)
	}

	pub fn container_runner_binary_path(&self) -> PathBuf {
		self.container_runner_binary_path
			.clone()
			.unwrap_or_else(|| Path::new("/usr/local/bin/rivet-container-runner").into())
	}

	pub fn isolate_runner_binary_path(&self) -> PathBuf {
		self.isolate_runner_binary_path
			.clone()
			.unwrap_or_else(|| Path::new("/usr/local/bin/rivet-isolate-v8-runner").into())
	}
}

#[derive(Clone, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Images {
	pub pull_addresses: Option<Addresses>,
}

impl Images {
	pub fn pull_addresses(&self) -> Cow<Addresses> {
		self.pull_addresses
			.as_ref()
			.map(Cow::Borrowed)
			.unwrap_or_else(|| Cow::Owned(Addresses::Static(Vec::new())))
	}
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Network {
	/// Address to serve actor traffic on.
	///
	/// This will usually be the same as `actor_lan_ip` unless the node is accessed within the
	/// LAN by a different IP.
	pub bind_ip: Ipv4Addr,

	/// Address to access this node in a LAN.
	///
	/// This IP is used to route traffic from Game Guard.
	pub lan_hostname: String,

	/// Address to access this node publicly.
	///
	/// This IP is used when providing the actor's IP & port for host networking.
	pub wan_hostname: String,

	pub lan_port_range_min: Option<u16>,
	pub lan_port_range_max: Option<u16>,
	pub wan_port_range_min: Option<u16>,
	pub wan_port_range_max: Option<u16>,
}

impl Network {
	pub fn lan_port_range_min(&self) -> u16 {
		self.lan_port_range_min.unwrap_or(20000)
	}

	pub fn lan_port_range_max(&self) -> u16 {
		self.lan_port_range_max.unwrap_or(25999)
	}

	pub fn wan_port_range_min(&self) -> u16 {
		self.wan_port_range_min.unwrap_or(26000)
	}

	pub fn wan_port_range_max(&self) -> u16 {
		self.wan_port_range_max.unwrap_or(31999)
	}
}

#[derive(Clone, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Cni {
	pub network_interface: String,
	pub network_name: Option<String>,
	pub bin_path: Option<String>,
	pub config_path: Option<String>,
}

impl Cni {
	pub fn network_name(&self) -> String {
		self.network_name
			.clone()
			.unwrap_or_else(|| "rivet-actor".into())
	}

	pub fn bin_path(&self) -> String {
		self.bin_path
			.clone()
			.unwrap_or_else(|| "/opt/cni/bin".into())
	}

	pub fn config_path(&self) -> String {
		self.config_path
			.clone()
			.unwrap_or_else(|| "/opt/cni/config".into())
	}
}

#[derive(Clone, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReservedResources {
	// Millicores
	pub cpu: Option<u64>,
	// MiB
	pub memory: Option<u64>,
}

impl ReservedResources {
	pub fn cpu(&self) -> u64 {
		self.cpu.unwrap_or(0)
	}

	pub fn memory(&self) -> u64 {
		self.memory.unwrap_or(0)
	}
}

#[derive(Clone, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Logs {
	pub redirect_logs: Option<bool>,
	/// Log retention in seconds. Defaults to 10 days. Only applies with log redirection enabled.
	pub retention: Option<u64>,
}

impl Logs {
	pub fn redirect_logs(&self) -> bool {
		self.redirect_logs.unwrap_or(true)
	}

	pub fn retention(&self) -> Duration {
		Duration::from_secs(self.retention.unwrap_or(10 * 24 * 60 * 60))
	}
}

#[derive(Clone, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Metrics {
	pub port: Option<u16>,
}

impl Metrics {
	pub fn port(&self) -> u16 {
		self.port.unwrap_or(6090)
	}
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FoundationDb {
	pub cluster_description: String,
	pub cluster_id: String,
	pub addresses: Addresses,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Addresses {
	Dynamic { fetch_endpoint: Url },
	Static(Vec<String>),
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Vector {
	pub address: String,
}
