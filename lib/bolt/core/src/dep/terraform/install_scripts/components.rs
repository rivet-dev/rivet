use anyhow::Result;
use indoc::formatdoc;
use std::collections::HashMap;

use crate::{
	context::ProjectContext,
	dep::terraform::{output::Cert, servers::Server},
};

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

/// Installs Treafik, but does not create the Traefik service.
pub fn traefik() -> String {
	include_str!("files/traefik.sh").to_string()
}

pub struct TraefikInstance {
	pub name: String,
	pub static_config: String,
	pub dynamic_config: String,
	pub tls_certs: HashMap<String, Cert>,
}

/// Creates a Traefik instance.
///
/// Requires `traefik()`.
pub fn traefik_instance(config: TraefikInstance) -> String {
	let mut script = include_str!("files/traefik_instance.sh")
		.replace("__NAME__", &config.name)
		.replace("__STATIC_CONFIG__", &config.static_config)
		.replace("__DYNAMIC_CONFIG__", &config.dynamic_config);

	// Add TLS certs
	for (cert_id, cert) in config.tls_certs {
		script.push_str(&formatdoc!(
			r#"

			cat << 'EOF' > /etc/{name}/tls/{cert_id}_cert.pem
			{cert}
			EOF

			cat << 'EOF' > /etc/{name}/tls/{cert_id}_key.pem
			{key}
			EOF

			cat << 'EOF' > /etc/{name}/dynamic/tls/{cert_id}.toml
			[[tls.certificates]]
				certFile = "/etc/{name}/tls/{cert_id}_cert.pem"
				keyFile = "/etc/{name}/tls/{cert_id}_key.pem"
			EOF
			"#,
			name = config.name,
			cert = cert.cert_pem,
			key = cert.key_pem,
		));
	}

	script
}

pub async fn traffic_server(ctx: &ProjectContext) -> Result<String> {
	let username = ctx
		.read_secret(&["docker", "registry", "ghcr.io", "write", "username"])
		.await?;
	let password = ctx
		.read_secret(&["docker", "registry", "ghcr.io", "write", "password"])
		.await?;

	let script = include_str!("files/traffic_server.sh")
		.replace("__GHCR_USERNAME__", &username)
		.replace("__GHCR_PASSWORD__", &password)
		.replace(
			"__IMAGE__",
			"ghcr.io/rivet-gg/apache-traffic-server:378f44b",
		);

	Ok(script)
}
