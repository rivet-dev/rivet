use crate::dep::terraform::servers::Server;

pub fn common() -> String {
	vec![
		format!("apt-get update -y"),
		format!("apt-get install -y apt-transport-https ca-certificates gnupg2 software-properties-common curl jq unzip"),
	].join("\n")
}

pub fn node_exporter() -> String {
	include_str!("files/node_exporter.sh").to_string()
}
pub fn sysctl() -> String {
	include_str!("files/sysctl.sh").to_string()
}

pub fn docker() -> String {
	include_str!("files/docker.sh").to_string()
}

pub fn cni_plugins() -> String {
	include_str!("files/cni_plugins.sh").to_string()
}

pub fn nomad(server: &Server) -> String {
	let servers = &["foo", "bar"];

	include_str!("files/nomad.sh")
		.replace("__REGION_ID__", &server.region_id)
		.replace("__NODE_NAME__", &server.name)
		.replace("__VLAN_ADDR__", &server.vlan_ip.to_string())
		// Hardcoded to Linode
		.replace("__PUBLIC_IFACE__", "eth1")
		.replace(
			"__SERVER_JOIN__",
			&servers
				.iter()
				.map(|x| format!("\"{x}\""))
				.collect::<Vec<_>>()
				.join(", "),
		)
}
