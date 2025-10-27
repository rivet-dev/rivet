use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Topology {
	/// Must be included in `datacenters`
	pub datacenter_label: u16,
	/// List of all datacenters, including this datacenter.
	pub datacenters: Vec<Datacenter>,
}

impl Topology {
	pub fn dc_for_label(&self, label: u16) -> Option<&Datacenter> {
		self.datacenters
			.iter()
			.find(|dc| dc.datacenter_label == label)
	}

	pub fn dc_for_name(&self, name: &str) -> Option<&Datacenter> {
		self.datacenters.iter().find(|dc| dc.name == name)
	}

	pub fn leader_dc(&self) -> Result<&Datacenter> {
		self.datacenters
			.iter()
			.find(|dc| dc.is_leader)
			.context("topology must have a leader datacenter")
	}

	pub fn current_dc(&self) -> Result<&Datacenter> {
		self.dc_for_label(self.datacenter_label)
			.context("topology must have a own datacenter")
	}

	pub fn is_leader(&self) -> bool {
		self.current_dc()
			.ok()
			.map(|dc| dc.is_leader)
			.unwrap_or(false)
	}
}

impl Default for Topology {
	fn default() -> Self {
		Topology {
			datacenter_label: 1,
			datacenters: vec![Datacenter {
				name: "default".into(),
				datacenter_label: 1,
				is_leader: true,
				public_url: Url::parse(&format!(
					"http://127.0.0.1:{}",
					crate::defaults::ports::GUARD
				))
				.unwrap(),
				peer_url: Url::parse(&format!(
					"http://127.0.0.1:{}",
					crate::defaults::ports::API_PEER
				))
				.unwrap(),
				proxy_url: None,
				valid_hosts: None,
			}],
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Datacenter {
	pub name: String,
	pub datacenter_label: u16,
	pub is_leader: bool,
	/// Public origin that can be used to connect to this region.
	pub public_url: Url,
	/// URL of the api-peer service
	pub peer_url: Url,
	/// URL of the guard service that other datacenters can access privately. Goes to the same place as
	// public_url.
	pub proxy_url: Option<Url>,
	/// List of hosts that are valid to connect to this region with. This is used in regional
	/// endpoints to validate that incoming requests to this datacenter are going to a
	/// region-specific domain.
	///
	/// IMPORTANT: Do not use a global origin that routes to multiple different regions. This will
	/// cause unpredictable behavior when requests are expected to go to a specific region.
	#[serde(default)]
	pub valid_hosts: Option<Vec<String>>,
}

impl Datacenter {
	pub fn is_valid_regional_host(&self, host: &str) -> bool {
		if let Some(valid_hosts) = &self.valid_hosts {
			// Check if host is in the valid_hosts list
			valid_hosts.iter().any(|valid_host| valid_host == host)
		} else {
			// Ignore this behavior if not configured
			true
		}
	}

	pub fn proxy_url(&self) -> &Url {
		self.proxy_url.as_ref().unwrap_or(&self.public_url)
	}

	pub fn proxy_url_host(&self) -> Result<&str> {
		self.proxy_url().host_str().context("no host")
	}

	pub fn proxy_url_port(&self) -> Result<u16> {
		self.proxy_url()
			.port()
			.or_else(|| match self.proxy_url().scheme() {
				"http" => Some(80),
				"https" => Some(443),
				_ => None,
			})
			.context("unsupported URL scheme")
	}
}
