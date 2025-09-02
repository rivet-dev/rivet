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
				name: "local".into(),
				datacenter_label: 1,
				is_leader: true,
				api_peer_url: Url::parse(&format!(
					"http://127.0.0.1:{}",
					crate::defaults::ports::API_PEER
				))
				.unwrap(),
				guard_url: Url::parse(&format!(
					"http://127.0.0.1:{}",
					crate::defaults::ports::GUARD
				))
				.unwrap(),
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
	/// Url of the api-peer service
	pub api_peer_url: Url,
	/// Url of the peer's guard server
	pub guard_url: Url,
}
