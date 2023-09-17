use anyhow::{Result, Context};
use indoc::formatdoc;
use std::collections::HashMap;
use tokio::fs;

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

	// Write config to files
	let config = traffic_server_config(ctx).await?;
	let config_script = config.into_iter().map(|(k, v)| format!("cat << 'EOF' > /etc/trafficserver/{k}\n{v}\nEOF\n")).collect::<Vec<_>>().join("\n\n");

	let script = include_str!("files/traffic_server.sh")
		.replace("__GHCR_USERNAME__", &username)
		.replace("__GHCR_PASSWORD__", &password)
		.replace(
			"__IMAGE__",
			"ghcr.io/rivet-gg/apache-traffic-server:378f44b",
		)
		.replace("__CONFIG__", &config_script);

	Ok(script)
}

async fn traffic_server_config(ctx: & ProjectContext) -> Result<Vec<(String, String)>> {
	let config_dir = ctx
		.path()
		.join("infra")
		.join("misc")
		.join("game_guard")
		.join("traffic_server");

	// Static files
	let mut my_map = Vec::<(String, String)>::new();
	let mut static_dir = fs::read_dir(config_dir.join("etc/static")).await?;
	while let Some(entry) = static_dir.next_entry().await? {
		let key = entry.path().file_name().context("path.file_name")?.as_str().context("as_str")?.to_string();
		let value = fs::read_to_string(entry.path()).await?;
		my_map.push((key, value));
	}

	// Storage
	my_map.push(("storage.config".to_string(), format("/var/cache/trafficserver {volume_size}Gi")))

	// Remap
	let mut remap = String::new();
	remap.push_str(&format!("map /s3-cache ${s3_providers[s3_default_provider].endpoint_internal} @plugin=s3_auth.so @pparam=--config @pparam=/etc/trafficserver-s3-auth/s3_auth_v4_${s3_default_provider}.config\n"));
	for (provider_name, provider) in s3_proviers {
		remap.push_str(&format!("map /s3-cache/${provider_name} ${provider.endpoint_internal} @plugin=s3_auth.so @pparam=--config @pparam=/etc/trafficserver-s3-auth/s3_auth_v4_${provider_name}.config"));
	}

	my_map.push(("remap.config".to_string(), remap));

	// TODO: S3

	Ok(my_map)
}
