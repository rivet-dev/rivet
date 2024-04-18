use chirp_worker::prelude::*;

pub fn install() -> String {
	include_str!("../files/nomad_install.sh").to_string()
}

pub fn configure() -> String {
	let servers = &["127.0.0.1:5000", "127.0.0.1:5001", "127.0.0.1:5002"];

	include_str!("../files/nomad_configure.sh")
		// HACK: Hardcoded to Linode
		.replace("__PUBLIC_IFACE__", "eth0")
		// HACK: Hardcoded to Linode
		.replace("__VLAN_IFACE__", "eth1")
		.replace(
			"__SERVER_JOIN__",
			&servers
				.iter()
				.map(|x| format!("\"{x}\""))
				.collect::<Vec<_>>()
				.join(", "),
		)
		.replace(
			"__GG_VLAN_SUBNET__",
			&util::net::gg::vlan_ip_net().to_string(),
		)
		.replace(
			"__ATS_VLAN_SUBNET__",
			&util::net::ats::vlan_ip_net().to_string(),
		)
}
