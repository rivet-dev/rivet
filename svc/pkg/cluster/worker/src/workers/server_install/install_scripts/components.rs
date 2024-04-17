use std::collections::HashMap;

use chirp_worker::prelude::*;
use include_dir::{include_dir, Dir};
use indoc::{formatdoc, indoc};
use proto::backend;
use s3_util::Provider;

/// Service that gets exposed from the Traefik tunnel.
pub struct TunnelService {
	/// Name of the service for the subdomain. This is how the Treafik tunnel server knows where to
	/// route traffic.
	name: &'static str,

	/// The port to serve the service on locally.
	port: u16,
}

pub const TUNNEL_API_ROUTE_PORT: u16 = 5010;
pub const TUNNEL_VECTOR_PORT: u16 = 5020;
pub const TUNNEL_VECTOR_TCP_JSON_PORT: u16 = 5021;
pub const TUNNEL_SERVICES: &[TunnelService] = &[
	TunnelService {
		name: "nomad-server-0",
		port: 5000,
	},
	TunnelService {
		name: "nomad-server-1",
		port: 5001,
	},
	TunnelService {
		name: "nomad-server-2",
		port: 5002,
	},
	TunnelService {
		name: "api-route",
		port: TUNNEL_API_ROUTE_PORT,
	},
	TunnelService {
		name: "vector",
		port: TUNNEL_VECTOR_PORT,
	},
	TunnelService {
		name: "vector-tcp-json",
		port: TUNNEL_VECTOR_TCP_JSON_PORT,
	},
];

pub fn common() -> String {
	indoc!(
		"
		apt-get update -y
		apt-get install -y apt-transport-https ca-certificates gnupg2 software-properties-common curl jq unzip
		"
	).to_string()
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

pub fn lz4() -> String {
	"apt-get install -y lz4".to_string()
}

pub fn skopeo() -> String {
	"apt-get install -y skopeo".to_string()
}

pub fn umoci() -> String {
	indoc!(
		r#"
		curl -Lf -o /usr/bin/umoci "https://github.com/opencontainers/umoci/releases/download/v0.4.7/umoci.amd64"
		chmod +x /usr/bin/umoci
		"#
	).to_string()
}

pub fn cnitool() -> String {
	indoc!(
		r#"
		curl -Lf -o /usr/bin/cnitool "https://github.com/rivet-gg/cni/releases/download/v1.1.2-build3/cnitool"
		chmod +x /usr/bin/cnitool
		"#
	).to_string()
}

pub fn cni_plugins() -> String {
	include_str!("files/cni_plugins.sh").to_string()
}

pub fn nomad_install() -> String {
	include_str!("files/nomad_install.sh").to_string()
}

pub fn nomad_configure() -> String {
	let servers = &["127.0.0.1:5000", "127.0.0.1:5001", "127.0.0.1:5002"];

	include_str!("files/nomad_configure.sh")
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

/// Installs Traefik, but does not create the Traefik service.
pub fn traefik() -> String {
	include_str!("files/traefik.sh").to_string()
}

pub struct TlsCert {
	pub cert_pem: String,
	pub key_pem: String,
}

pub struct TraefikInstance {
	pub name: String,
	pub static_config: String,
	pub dynamic_config: String,
	pub tcp_server_transports: HashMap<String, ServerTransport>,
}

pub struct ServerTransport {
	pub server_name: String,
	pub root_cas: Vec<String>,
	pub certs: Vec<TlsCert>,
}

/// Creates a Traefik instance.
///
/// Requires `traefik()`.
pub fn traefik_instance(config: TraefikInstance) -> String {
	let config_name = &config.name;

	let mut script = include_str!("files/traefik_instance.sh")
		.replace("__NAME__", &config.name)
		.replace("__STATIC_CONFIG__", &config.static_config)
		.replace("__DYNAMIC_CONFIG__", &config.dynamic_config);

	for (transport_id, transport) in config.tcp_server_transports {
		// Build config
		let root_cas = transport
			.root_cas
			.iter()
			.enumerate()
			.map(|(i, _)| {
				format!("\"/etc/{config_name}/tls/transport_{transport_id}_root_ca_{i}_cert.pem\"",)
			})
			.collect::<Vec<_>>()
			.join(", ");
		let mut transport_config = formatdoc!(
			r#"
			[tcp.serversTransports.{transport_id}.tls]
				serverName = "{server_name}"
				rootCAs = [{root_cas}]
			"#,
			server_name = transport.server_name
		);

		// Write root CAs
		for (i, cert) in transport.root_cas.iter().enumerate() {
			script.push_str(&formatdoc!(
				r#"
				cat << 'EOF' > /etc/{config_name}/tls/transport_{transport_id}_root_ca_{i}_cert.pem
				{cert}
				EOF
				"#,
			));
		}

		// Write certs
		for (i, cert) in transport.certs.iter().enumerate() {
			script.push_str(&formatdoc!(
				r#"
				cat << 'EOF' > /etc/{config_name}/tls/transport_{transport_id}_cert_{i}_cert.pem
				{cert}
				EOF

				cat << 'EOF' > /etc/{config_name}/tls/transport_{transport_id}_cert_{i}_key.pem
				{key}
				EOF
				"#,
				cert = cert.cert_pem,
				key = cert.key_pem,
			));
			transport_config.push_str(&formatdoc!(
				r#"
				[[tcp.serversTransports.{transport_id}.tls.certificates]]
					certFile = "/etc/{config_name}/tls/transport_{transport_id}_cert_{i}_cert.pem"
					keyFile = "/etc/{config_name}/tls/transport_{transport_id}_cert_{i}_key.pem"
				"#
			))
		}

		// Write config
		script.push_str(&formatdoc!(
			r#"
			cat << 'EOF' > /etc/{config_name}/dynamic/transport_{transport_id}.toml
			{transport_config}
			EOF
			"#
		));
	}

	script
}

pub fn traefik_tunnel() -> GlobalResult<String> {
	// Build transports for each service
	let mut tcp_server_transports = HashMap::new();
	for TunnelService { name, .. } in TUNNEL_SERVICES {
		tcp_server_transports.insert(
			name.to_string(),
			ServerTransport {
				server_name: format!("{name}.tunnel.rivet.gg"),
				root_cas: vec![util::env::var("TLS_ROOT_CA_CERT_PEM")?],
				certs: vec![TlsCert {
					cert_pem: util::env::var("TLS_CERT_LOCALLY_SIGNED_JOB_CERT_PEM")?,
					key_pem: util::env::var("TLS_CERT_LOCALLY_SIGNED_JOB_KEY_PEM")?,
				}],
			},
		);
	}

	Ok(traefik_instance(TraefikInstance {
		name: "tunnel".into(),
		static_config: tunnel_traefik_static_config(),
		dynamic_config: tunnel_traefik_dynamic_config(&util::env::var(
			"K8S_TRAEFIK_TUNNEL_EXTERNAL_IP",
		)?),
		tcp_server_transports,
	}))
}

fn tunnel_traefik_static_config() -> String {
	let mut config = formatdoc!(
		r#"
		[providers]
			[providers.file]
				directory = "/etc/tunnel/dynamic"
		"#
	);

	for TunnelService { name, port } in TUNNEL_SERVICES.iter() {
		config.push_str(&formatdoc!(
			r#"
			[entryPoints.{name}]
				address = "127.0.0.1:{port}"
			"#,
		))
	}

	config
}

fn tunnel_traefik_dynamic_config(tunnel_external_ip: &str) -> String {
	let mut config = String::new();
	for TunnelService { name, .. } in TUNNEL_SERVICES.iter() {
		config.push_str(&formatdoc!(
			r#"
			[tcp.routers.{name}]
				entryPoints = ["{name}"]
				rule = "HostSNI(`*`)"  # Match all ingress, unrelated to the outbound TLS
				service = "{name}"

			[tcp.services.{name}.loadBalancer]
				serversTransport = "{name}"

				[[tcp.services.{name}.loadBalancer.servers]]
					address = "{tunnel_external_ip}:5000"
					tls = true
			"#
		))
	}

	config
}

pub fn vector_install() -> String {
	include_str!("files/vector_install.sh").to_string()
}

pub struct VectorConfig {
	pub prometheus_targets: HashMap<String, VectorPrometheusTarget>,
}

pub struct VectorPrometheusTarget {
	pub endpoint: String,
	pub scrape_interval: usize,
}

pub fn vector_configure(config: &VectorConfig, pool_type: backend::cluster::PoolType) -> String {
	let sources = config
		.prometheus_targets
		.keys()
		.map(|x| format!("\"prometheus_{x}\""))
		.collect::<Vec<_>>()
		.join(", ");

	let pool_type_str = match pool_type {
		backend::cluster::PoolType::Job => "job",
		backend::cluster::PoolType::Gg => "gg",
		backend::cluster::PoolType::Ats => "ats",
	};

	let mut config_str = formatdoc!(
		r#"
		[api]
			enabled = true

		[transforms.add_meta]
			type = "remap"
			inputs = [{sources}]
			source = '''
			.tags.server_id = "___SERVER_ID___"
			.tags.datacenter_id = "___DATACENTER_ID___"
			.tags.cluster_id = "___CLUSTER_ID___"
			.tags.pool_type = "{pool_type_str}"
			.tags.public_ip = "${{PUBLIC_IP}}"
			'''

		[sinks.vector_sink]
			type = "vector"
			inputs = ["add_meta"]
			address = "127.0.0.1:{TUNNEL_VECTOR_PORT}"
			healthcheck.enabled = false
			compression = true
		"#
	);

	for (
		key,
		VectorPrometheusTarget {
			endpoint,
			scrape_interval,
		},
	) in &config.prometheus_targets
	{
		config_str.push_str(&formatdoc!(
			r#"
			[sources.prometheus_{key}]
				type = "prometheus_scrape"
				endpoints = ["{endpoint}"]
				scrape_interval_secs = {scrape_interval}
			"#
		));
	}

	include_str!("files/vector_configure.sh").replace("__VECTOR_CONFIG__", &config_str)
}

const TRAFFIC_SERVER_IMAGE: &str = "ghcr.io/rivet-gg/apache-traffic-server:9934dc2";

pub fn traffic_server_install() -> String {
	include_str!("files/traffic_server_install.sh").replace("__IMAGE__", TRAFFIC_SERVER_IMAGE)
}

pub async fn traffic_server_configure() -> GlobalResult<String> {
	// Write config to files
	let config = traffic_server_config().await?;
	let mut config_scripts = config
		.into_iter()
		.map(|(k, v)| format!("cat << 'EOF' > /etc/trafficserver/{k}\n{v}\nEOF\n"))
		.collect::<Vec<_>>();

	// Update default storage config size to be entire filesystem size minus 4GB
	config_scripts.push(
		indoc!(
			r#"
			df -h / |
			awk 'NR==2 {gsub(/G/, "", $2); print $2 - 4 "G"}' |
			xargs -I {} sed -i 's/64G/{}/' /etc/trafficserver/storage.config
			"#
		)
		.to_string(),
	);

	let script = include_str!("files/traffic_server_configure.sh")
		.replace("__IMAGE__", TRAFFIC_SERVER_IMAGE)
		.replace("__CONFIG__", &config_scripts.join("\n\n"));

	Ok(script)
}

static TRAFFIC_SERVER_CONFIG_DIR: Dir<'_> = include_dir!(
	"$CARGO_MANIFEST_DIR/src/workers/server_install/install_scripts/files/traffic_server"
);

async fn traffic_server_config() -> GlobalResult<Vec<(String, String)>> {
	// Static files
	let mut config_files = Vec::new();
	collect_config_files(&TRAFFIC_SERVER_CONFIG_DIR, &mut config_files)?;

	// Storage (default value of 64 gets overwritten in config script)
	let volume_size = 64;
	config_files.push((
		"storage.config".to_string(),
		format!("/var/cache/trafficserver {volume_size}G"),
	));

	// Remap & S3
	let mut remap = String::new();
	let default_s3_provider = Provider::default()?;
	if s3_util::s3_provider_active("bucket-build", Provider::Minio) {
		let output = gen_s3_provider(Provider::Minio, default_s3_provider).await?;
		remap.push_str(&output.append_remap);
		config_files.extend(output.config_files);
	}
	if s3_util::s3_provider_active("bucket-build", Provider::Backblaze) {
		let output = gen_s3_provider(Provider::Backblaze, default_s3_provider).await?;
		remap.push_str(&output.append_remap);
		config_files.extend(output.config_files);
	}
	if s3_util::s3_provider_active("bucket-build", Provider::Aws) {
		let output = gen_s3_provider(Provider::Aws, default_s3_provider).await?;
		remap.push_str(&output.append_remap);
		config_files.extend(output.config_files);
	}
	config_files.push(("remap.config".to_string(), remap));

	Ok(config_files)
}

fn collect_config_files(
	dir: &include_dir::Dir,
	config_files: &mut Vec<(String, String)>,
) -> GlobalResult<()> {
	for entry in dir.entries() {
		match entry {
			include_dir::DirEntry::File(file) => {
				let key = unwrap!(unwrap!(file.path().file_name()).to_str()).to_string();

				let value = unwrap!(file.contents_utf8());
				config_files.push((key, value.to_string()));
			}
			include_dir::DirEntry::Dir(dir) => collect_config_files(dir, config_files)?,
		}
	}

	Ok(())
}

struct GenRemapS3ProviderOutput {
	/// Append to remap.config
	append_remap: String,

	/// Concat with config files
	config_files: Vec<(String, String)>,
}

async fn gen_s3_provider(
	provider: Provider,
	default_s3_provider: Provider,
) -> GlobalResult<GenRemapS3ProviderOutput> {
	let mut remap = String::new();
	let provider_name = provider.as_str();
	let endpoint_external = s3_util::s3_endpoint_external("bucket-build", provider)?;
	let region = s3_util::s3_region("bucket-build", provider)?;
	let (access_key_id, secret_access_key) = s3_util::s3_credentials("bucket-build", provider)?;

	// Build plugin chain
	let plugins = format!("@plugin=tslua.so @pparam=/etc/trafficserver/strip_headers.lua @plugin=s3_auth.so @pparam=--config @pparam=s3_auth_v4_{provider_name}.config");

	// Add remap
	remap.push_str(&format!(
		"map /s3-cache/{provider_name} {endpoint_external} {plugins}\n",
	));

	// Add default route
	if default_s3_provider == provider {
		remap.push_str(&format!("map /s3-cache {endpoint_external} {plugins}\n",));
	}

	// Add credentials
	let mut config_files = Vec::<(String, String)>::new();
	config_files.push((
		format!("s3_auth_v4_{provider_name}.config"),
		formatdoc!(
			r#"
			access_key={access_key_id}
			secret_key={secret_access_key}
			version=4
			v4-region-map=s3_region_map_{provider_name}.config
			"#,
		),
	));
	config_files.push((
		format!("s3_region_map_{provider_name}.config"),
		formatdoc!(
			r#"
			# Default region
			{s3_host}: {s3_region}
			"#,
			s3_host = endpoint_external.split_once("://").unwrap().1,
			s3_region = region,
		),
	));

	Ok(GenRemapS3ProviderOutput {
		append_remap: remap,
		config_files,
	})
}

pub fn rivet_create_hook(initialize_immediately: bool) -> GlobalResult<String> {
	let domain_main_api = unwrap!(util::env::domain_main_api(), "no cdn");
	let mut script =
		include_str!("files/rivet_create_hook.sh").replace("__DOMAIN_MAIN_API__", domain_main_api);

	if initialize_immediately {
		script.push_str("systemctl start rivet_hook\n");
	}

	Ok(script)
}

pub fn rivet_fetch_info(server_token: &str) -> GlobalResult<String> {
	let domain_main_api = unwrap!(util::env::domain_main_api(), "no cdn");

	Ok(include_str!("files/rivet_fetch_info.sh")
		.replace("__SERVER_TOKEN__", server_token)
		.replace("__DOMAIN_MAIN_API__", domain_main_api))
}

pub fn rivet_fetch_tls(
	initialize_immediately: bool,
	server_token: &str,
	traefik_instance_name: &str,
) -> GlobalResult<String> {
	let domain_main_api = unwrap!(util::env::domain_main_api(), "no cdn");

	let mut script = include_str!("files/rivet_fetch_tls.sh")
		.replace("__NAME__", traefik_instance_name)
		.replace("__SERVER_TOKEN__", server_token)
		.replace("__DOMAIN_MAIN_API__", domain_main_api);

	if initialize_immediately {
		script.push_str("systemctl start rivet_fetch_tls.timer\n");
	}

	Ok(script)
}
