use chirp_workflow::prelude::*;

pub async fn install() -> GlobalResult<String> {
	Ok(include_str!("../files/pegboard_install.sh")
		.replace("__MANAGER_BINARY_URL__", &util::env::var("PEGBOARD_MANAGER_BINARY_URL")?)
		.replace("__CONTAINER_RUNNER_BINARY_URL__", &util::env::var("CONTAINER_RUNNER_BINARY_URL")?)
		.replace("__V8_ISOLATE_BINARY_URL__", &util::env::var("V8_ISOLATE_RUNNER_BINARY_URL")?))
}

pub fn configure(flavor: pegboard::protocol::ClientFlavor) -> GlobalResult<String> {
	Ok(include_str!("../files/pegboard_configure.sh")
		.replace("__FLAVOR__", &flavor.to_string())
		.replace("__ORIGIN_API__", util::env::origin_api())
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
		))
}
