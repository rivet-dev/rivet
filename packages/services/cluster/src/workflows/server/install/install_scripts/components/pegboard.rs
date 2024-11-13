use chirp_workflow::prelude::*;

pub async fn install(config: &rivet_config::Config) -> GlobalResult<String> {
	let provision_config = &config.server()?.rivet.provision()?;

	Ok(include_str!("../files/pegboard_install.sh")
		.replace(
			"__PEGBOARD_MANAGER_BINARY_URL__",
			&provision_config.manager_binary_url.to_string(),
		)
		.replace(
			"__CONTAINER_RUNNER_BINARY_URL__",
			&provision_config.container_runner_binary_url.to_string(),
		)
		.replace(
			"__V8_ISOLATE_BINARY_URL__",
			&provision_config.isolate_runner_binary_url.to_string(),
		))
}

pub fn configure(
	config: &rivet_config::Config,
	flavor: pegboard::protocol::ClientFlavor,
) -> GlobalResult<String> {
	let origin_api = config
		.server()?
		.rivet
		.api_public
		.public_origin()
		.to_string();

	Ok(include_str!("../files/pegboard_configure.sh")
		.replace("__FLAVOR__", &flavor.to_string())
		.replace("__ORIGIN_API__", &origin_api)
		// HACK: Hardcoded to Linode
		.replace("__PUBLIC_IFACE__", "eth0")
		// HACK: Hardcoded to Linode
		.replace("__VLAN_IFACE__", "eth1")
		.replace(
			"__GG_VLAN_SUBNET__",
			&util::net::gg::vlan_ip_net().to_string(),
		)
		.replace(
			"__ATS_VLAN_SUBNET__",
			&util::net::ats::vlan_ip_net().to_string(),
		)
		.replace(
			"__MIN_HOST_PORT__",
			&util::net::job::MIN_HOST_PORT_TCP.to_string(),
		)
		.replace(
			"__MAX_HOST_PORT__",
			&util::net::job::MAX_HOST_PORT_TCP.to_string(),
		))
}
