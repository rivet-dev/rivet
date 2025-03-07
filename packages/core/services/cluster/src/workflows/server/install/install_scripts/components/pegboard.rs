use chirp_workflow::prelude::*;

use super::{fdb::FDB_VERSION, rivet::TUNNEL_API_EDGE_PORT};

// Cant import from pegboard::protocol because of circular dependencies
#[derive(Debug, Clone, Copy)]
pub enum ClientFlavor {
	Container,
	Isolate,
}

impl std::fmt::Display for ClientFlavor {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ClientFlavor::Container => write!(f, "container"),
			ClientFlavor::Isolate => write!(f, "isolate"),
		}
	}
}

pub async fn install(config: &rivet_config::Config, flavor: ClientFlavor) -> GlobalResult<String> {
	let provision_config = &config.server()?.rivet.provision()?;

	Ok(include_str!("../files/pegboard_install.sh")
		.replace("__FLAVOR__", &flavor.to_string())
		.replace(
			"__PEGBOARD_MANAGER_BINARY_URL__",
			provision_config.manager_binary_url.as_ref(),
		)
		.replace(
			"__CONTAINER_RUNNER_BINARY_URL__",
			provision_config.container_runner_binary_url.as_ref(),
		)
		.replace(
			"__ISOLATE_V8_RUNNER_BINARY_URL__",
			provision_config.isolate_runner_binary_url.as_ref(),
		)
		.replace("__FDB_VERSION__", FDB_VERSION))
}

pub fn configure(config: &rivet_config::Config, flavor: ClientFlavor) -> GlobalResult<String> {
	let provision_config = config.server()?.rivet.provision()?;

	let origin_api =
		util::url::to_string_without_slash(&config.server()?.rivet.api_public.public_origin());

	let pb_reserved_memory = match flavor {
		ClientFlavor::Container => server_spec::PEGBOARD_CONTAINER_RESERVE_MEMORY_MIB,
		ClientFlavor::Isolate => server_spec::PEGBOARD_ISOLATE_RESERVE_MEMORY_MIB,
	};

	Ok(include_str!("../files/pegboard_configure.sh")
		.replace("__FLAVOR__", &flavor.to_string())
		.replace("__ORIGIN_API__", &origin_api)
		.replace(
			"__TUNNEL_API_EDGE_API__",
			&format!("http://127.0.0.1:{TUNNEL_API_EDGE_PORT}"),
		)
		.replace(
			"__RESERVED_MEMORY__",
			&(server_spec::RESERVE_LB_MEMORY_MIB + pb_reserved_memory).to_string(),
		)
		// HACK: Hardcoded to Linode
		.replace("__PUBLIC_IFACE__", "eth0")
		// HACK: Hardcoded to Linode
		.replace("__VLAN_IFACE__", "eth1")
		.replace(
			"__GG_VLAN_SUBNET__",
			&provision_config.pools.gg.vlan_ip_net().to_string(),
		)
		.replace(
			"__ATS_VLAN_SUBNET__",
			&provision_config.pools.ats.vlan_ip_net().to_string(),
		)
		.replace(
			"__MIN_WAN_PORT__",
			&provision_config.pools.pegboard.min_wan_port().to_string(),
		)
		.replace(
			"__MAX_WAN_PORT__",
			&provision_config.pools.pegboard.max_wan_port().to_string(),
		))
}
