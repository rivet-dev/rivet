use std::net::Ipv4Addr;

use ipnet::{Ipv4AddrRange, Ipv4Net};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClusterProvision {
	/// Configuration for server pools that use a margin for scaling.
	pub pools: ClusterPools,

	#[schemars(with = "Option<String>")]
	pub vlan_ip_net: Option<Ipv4Net>,

	/// The URL for the manager binary.
	pub manager_binary_url: Url,

	/// The URL for the container runner binary.
	pub container_runner_binary_url: Url,

	/// The URL for the isolate runner binary.
	pub isolate_runner_binary_url: Url,

	// The URL for the rivet edge server binary.
	pub edge_server_binary_url: Url,
}

impl ClusterProvision {
	pub fn vlan_ip_net(&self) -> Ipv4Net {
		self.vlan_ip_net
			.unwrap_or(Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 0), 16).unwrap())
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClusterPools {
	pub job: ClusterPoolJob,
	pub pegboard: ClusterPoolPegboard,
	pub gg: ClusterPoolGg,
	pub ats: ClusterPoolAts,
	pub fdb: ClusterPoolFdb,
	pub worker: ClusterPoolWorker,
	pub nats: ClusterPoolNats,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClusterPoolJob {
	pub autoscale_margin: u32,
	// All other properties are read from Pegboard since they're identical
}

/// These port range values will be pass to the Rivet Clients to choose ports & are used to
/// provision firewalls.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClusterPoolPegboard {
	pub autoscale_margin: u32,

	pub vlan_addr_range_min: Option<Ipv4Addr>,
	pub vlan_addr_range_max: Option<Ipv4Addr>,

	pub firewall_rules: Option<Vec<FirewallRule>>,

	pub min_lan_port: Option<u16>,
	pub max_lan_port: Option<u16>,
	pub min_wan_port: Option<u16>,
	pub max_wan_port: Option<u16>,
}

impl ClusterPoolPegboard {
	pub fn vlan_addr_range_min(&self) -> Ipv4Addr {
		self.vlan_addr_range_min
			.unwrap_or(Ipv4Addr::new(10, 0, 4, 1))
	}

	pub fn vlan_addr_range_max(&self) -> Ipv4Addr {
		self.vlan_addr_range_min
			.unwrap_or(Ipv4Addr::new(10, 0, 255, 254))
	}

	pub fn vlan_addr_range(&self) -> Ipv4AddrRange {
		Ipv4AddrRange::new(self.vlan_addr_range_min(), self.vlan_addr_range_max())
	}

	pub fn firewall_rules(&self) -> Vec<FirewallRule> {
		[
			FirewallRule::base_rules(),
			vec![
				// Ports available to Nomad jobs/actors using the host network
				FirewallRule {
					label: "host-tcp".into(),
					ports: format!("{}-{}", self.min_wan_port(), self.max_wan_port()),
					protocol: "tcp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
				FirewallRule {
					label: "host-udp".into(),
					ports: format!("{}-{}", self.min_wan_port(), self.max_wan_port()),
					protocol: "udp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
			],
		]
		.concat()
	}

	pub fn min_lan_port(&self) -> u16 {
		self.min_lan_port.unwrap_or(20000)
	}

	pub fn max_lan_port(&self) -> u16 {
		self.max_lan_port.unwrap_or(25999)
	}

	pub fn min_wan_port(&self) -> u16 {
		self.min_wan_port.unwrap_or(self.max_lan_port() + 1)
	}

	pub fn max_wan_port(&self) -> u16 {
		self.max_wan_port.unwrap_or(31999)
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClusterPoolGg {
	pub autoscale_margin: u32,

	#[schemars(with = "Option<String>")]
	pub vlan_ip_net: Option<Ipv4Net>,
	pub firewall_rules: Option<Vec<FirewallRule>>,
}

impl ClusterPoolGg {
	pub fn vlan_ip_net(&self) -> Ipv4Net {
		Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 0), 26).unwrap()
	}

	pub fn vlan_addr_range(&self) -> Ipv4AddrRange {
		self.vlan_ip_net().hosts()
	}

	pub fn firewall_rules(&self, gg: &super::Guard) -> Vec<FirewallRule> {
		[
			FirewallRule::base_rules(),
			vec![
				// HTTP(S)
				FirewallRule {
					label: "http-tcp".into(),
					ports: "80".into(),
					protocol: "tcp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
				FirewallRule {
					label: "http-udp".into(),
					ports: "80".into(),
					protocol: "udp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
				FirewallRule {
					label: "https-tcp".into(),
					ports: "443".into(),
					protocol: "tcp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
				FirewallRule {
					label: "https-udp".into(),
					ports: "443".into(),
					protocol: "udp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
				// Dynamic TCP
				FirewallRule {
					label: "dynamic-tcp".into(),
					ports: format!(
						"{}-{}",
						gg.min_ingress_port_tcp(),
						gg.max_ingress_port_tcp()
					),
					protocol: "tcp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
				// Dynamic UDP
				FirewallRule {
					label: "dynamic-udp".into(),
					ports: format!(
						"{}-{}",
						gg.min_ingress_port_udp(),
						gg.max_ingress_port_udp()
					),
					protocol: "udp".into(),
					inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
					inbound_ipv6_cidr: vec!["::/0".into()],
				},
			],
		]
		.concat()
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClusterPoolAts {
	pub autoscale_margin: u32,

	#[schemars(with = "Option<String>")]
	pub vlan_ip_net: Option<Ipv4Net>,
	pub firewall_rules: Option<Vec<FirewallRule>>,
}

impl ClusterPoolAts {
	pub fn vlan_ip_net(&self) -> Ipv4Net {
		Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 64), 26).unwrap()
	}

	pub fn vlan_addr_range(&self) -> Ipv4AddrRange {
		self.vlan_ip_net().hosts()
	}

	pub fn firewall_rules(&self) -> Vec<FirewallRule> {
		FirewallRule::base_rules()
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClusterPoolFdb {
	#[schemars(with = "Option<String>")]
	pub vlan_ip_net: Option<Ipv4Net>,
	pub firewall_rules: Option<Vec<FirewallRule>>,
}

impl ClusterPoolFdb {
	pub fn vlan_ip_net(&self) -> Ipv4Net {
		Ipv4Net::new(Ipv4Addr::new(10, 0, 2, 0), 26).unwrap()
	}

	pub fn vlan_addr_range(&self) -> Ipv4AddrRange {
		self.vlan_ip_net().hosts()
	}

	pub fn firewall_rules(&self) -> Vec<FirewallRule> {
		FirewallRule::base_rules()
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClusterPoolWorker {
	pub autoscale_margin: u32,

	#[schemars(with = "Option<String>")]
	pub vlan_ip_net: Option<Ipv4Net>,
	pub firewall_rules: Option<Vec<FirewallRule>>,
}

impl ClusterPoolWorker {
	pub fn vlan_ip_net(&self) -> Ipv4Net {
		Ipv4Net::new(Ipv4Addr::new(10, 0, 3, 0), 26).unwrap()
	}

	pub fn vlan_addr_range(&self) -> Ipv4AddrRange {
		self.vlan_ip_net().hosts()
	}

	pub fn firewall_rules(&self) -> Vec<FirewallRule> {
		FirewallRule::base_rules()
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClusterPoolNats {
	pub autoscale_margin: u32,

	#[schemars(with = "Option<String>")]
	pub vlan_ip_net: Option<Ipv4Net>,
	pub firewall_rules: Option<Vec<FirewallRule>>,
}

impl ClusterPoolNats {
	pub fn vlan_ip_net(&self) -> Ipv4Net {
		Ipv4Net::new(Ipv4Addr::new(10, 1, 0, 0), 26).unwrap()
	}

	pub fn vlan_addr_range(&self) -> Ipv4AddrRange {
		self.vlan_ip_net().hosts()
	}

	pub fn firewall_rules(&self) -> Vec<FirewallRule> {
		FirewallRule::base_rules()
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct FirewallRule {
	pub label: String,
	pub ports: String,
	pub protocol: String,
	pub inbound_ipv4_cidr: Vec<String>,
	pub inbound_ipv6_cidr: Vec<String>,
}

impl FirewallRule {
	pub fn base_rules() -> Vec<FirewallRule> {
		vec![FirewallRule {
			label: "ssh".into(),
			ports: "22".into(),
			protocol: "tcp".into(),
			inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
			inbound_ipv6_cidr: vec!["::/0".into()],
		}]
	}
}
