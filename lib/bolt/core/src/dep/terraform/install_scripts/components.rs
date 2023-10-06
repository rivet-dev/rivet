use anyhow::{Context, Result};
use indoc::formatdoc;
use std::collections::HashMap;
use tokio::fs;

use crate::{
	config::ns,
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

// Q: is this running in server or client mode?
pub fn nomad(server: &Server) -> String {
	// just do one for now -> refers to load balancer.

	let servers = &["foo", "bar"]; // TODO how will these be populated? are these the nomad leader servers? or just need to know about the load balancer?

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
	pub tcp_server_transports: HashMap<String, ServerTransport>,
}

pub struct ServerTransport {
	pub certs: Vec<Cert>,
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

	for (transport_id, transport) in config.tcp_server_transports {
		for (i, cert) in transport.certs.iter().enumerate() {
			script.push_str(&formatdoc!(
				r#"
				cat << 'EOF' > /etc/{name}/tls/transport_{transport_id}_{i}_cert.pem
				{cert}
				EOF

				cat << 'EOF' > /etc/{name}/tls/transport_{transport_id}_{i}_key.pem
				{key}
				EOF

				cat << 'EOF' > /etc/{name}/dynamic/transport_{transport_id}.toml
				[[tcp.serversTransports.tunnel_nomad_client.certificates]]
					certFile = "/etc/{name}/tls/transport_{transport_id}_{i}_cert.pem"
					keyFile = "/etc/{name}/tls/transport_{transport_id}_{i}_key.pem"
				EOF
				"#,
				name = config.name,
				cert = cert.cert_pem,
				key = cert.key_pem,
			));
		}
	}

	script
}

pub async fn traffic_server(ctx: &ProjectContext) -> Result<String> {
	// Write config to files
	let config = traffic_server_config(ctx).await?;
	let config_script = config
		.into_iter()
		.map(|(k, v)| format!("cat << 'EOF' > /etc/trafficserver/{k}\n{v}\nEOF\n"))
		.collect::<Vec<_>>()
		.join("\n\n");

	let script = include_str!("files/traffic_server.sh")
		.replace(
			"__IMAGE__",
			"ghcr.io/rivet-gg/apache-traffic-server:378f44b",
		)
		.replace("__CONFIG__", &config_script);

	Ok(script)
}

async fn traffic_server_config(ctx: &ProjectContext) -> Result<Vec<(String, String)>> {
	let config_dir = ctx
		.path()
		.join("infra")
		.join("misc")
		.join("game_guard")
		.join("traffic_server");

	// Static files
	let mut config_files = Vec::<(String, String)>::new();
	let mut static_dir = fs::read_dir(config_dir.join("etc")).await?;
	while let Some(entry) = static_dir.next_entry().await? {
		let meta = entry.metadata().await?;
		if meta.is_file() {
			let key = entry
				.path()
				.file_name()
				.context("path.file_name")?
				.to_str()
				.context("as_str")?
				.to_string();
			let value = fs::read_to_string(entry.path()).await?;
			config_files.push((key, value));
		}
	}

	// Storage
	let volume_size = 64; // TODO: Don't hardcode this
	config_files.push((
		"storage.config".to_string(),
		format!("/var/cache/trafficserver {volume_size}G"),
	));

	// Remap & S3
	let mut remap = String::new();
	let (default_s3_provider, _) = ctx.default_s3_provider()?;
	if let Some(p) = &ctx.ns().s3.providers.minio {
		let output = gen_s3_provider(ctx, s3_util::Provider::Minio, default_s3_provider).await?;
		remap.push_str(&output.append_remap);
		config_files.extend(output.config_files);
	}
	if let Some(p) = &ctx.ns().s3.providers.backblaze {
		let output =
			gen_s3_provider(ctx, s3_util::Provider::Backblaze, default_s3_provider).await?;
		remap.push_str(&output.append_remap);
		config_files.extend(output.config_files);
	}
	if let Some(p) = &ctx.ns().s3.providers.aws {
		let output = gen_s3_provider(ctx, s3_util::Provider::Aws, default_s3_provider).await?;
		remap.push_str(&output.append_remap);
		config_files.extend(output.config_files);
	}
	config_files.push(("remap.config".to_string(), remap));

	Ok(config_files)
}

struct GenRemapS3ProviderOutput {
	/// Append to remap.config
	append_remap: String,

	/// Concat with config files
	config_files: Vec<(String, String)>,
}

async fn gen_s3_provider(
	ctx: &ProjectContext,
	provider: s3_util::Provider,
	default_s3_provider: s3_util::Provider,
) -> Result<GenRemapS3ProviderOutput> {
	let mut remap = String::new();
	let provider_name = provider.as_str();
	let config = ctx.s3_config(provider).await?;
	let creds = ctx.s3_credentials(provider).await?;

	// Add remap
	remap.push_str(&format!("map /s3-cache/{provider_name} {endpoint_internal} @plugin=s3_auth.so @pparam=--config @pparam=s3_auth_v4_{provider_name}.config\n", endpoint_internal = config.endpoint_internal));

	// Add default route
	if default_s3_provider == provider {
		remap.push_str(&format!("map /s3-cache {endpoint_internal} @plugin=s3_auth.so @pparam=--config @pparam=s3_auth_v4_{provider_name}.config\n",
			endpoint_internal = config.endpoint_internal,
		));
	}

	// Add credentials
	let mut config_files = Vec::<(String, String)>::new();
	config_files.push((
		format!("s3_auth_v4_{provider_name}.config"),
		formatdoc!(
			r#"
			access_key={access_key}
			secret_key={secret_key}
			version=4
			v4-region-map=s3_region_map_{provider_name}.config
			"#,
			access_key = creds.access_key_id,
			secret_key = creds.access_key_secret,
		),
	));
	config_files.push((
		format!("s3_region_map_{provider_name}.config"),
		formatdoc!(
			r#"
		# Default region
		{s3_host}: {s3_region}
		"#,
			s3_host = config.endpoint_external,
			s3_region = config.region,
		),
	));

	Ok(GenRemapS3ProviderOutput {
		append_remap: remap,
		config_files,
	})
}
