use chirp_workflow::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceType {
	pub hardware_id: String,
	pub memory: u32,
	pub disk: u32,
	pub vcpus: u32,
	pub transfer: u32,
	pub network_out: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub enum FirewallPreset {
	// TODO: Rename to game
	Job,
	Gg,
	Ats,
}

impl FirewallPreset {
	pub fn rules(
		&self,
		config: &rivet_config::Config,
	) -> GlobalResult<Vec<rivet_config::config::rivet::cluster_provision::FirewallRule>> {
		let provision_config = config.server()?.rivet.provision()?;
		Ok(match self {
			FirewallPreset::Job => provision_config.pools.pegboard.firewall_rules(),
			FirewallPreset::Gg => provision_config
				.pools
				.gg
				.firewall_rules(&config.server()?.rivet.guard),
			FirewallPreset::Ats => provision_config.pools.ats.firewall_rules(),
		})
	}
}

impl std::fmt::Display for FirewallPreset {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			FirewallPreset::Job => write!(f, "job"),
			FirewallPreset::Gg => write!(f, "gg"),
			FirewallPreset::Ats => write!(f, "ats"),
		}
	}
}
