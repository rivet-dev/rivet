use std::collections::HashMap;

use anyhow::{Context, Result};
use indoc::formatdoc;
use indoc::indoc;
use serde_json::json;
use tokio::fs;

use crate::{
	context::ProjectContext,
	dep::terraform::{net, output::Cert, servers::Server},
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

pub fn lz4() -> String {
	format!("apt-get install -y lz4")
}

pub fn skopeo() -> String {
	format!("apt-get install -y skopeo")
}

pub fn umoci() -> String {
	formatdoc!(
		r#"
		curl -Lf -o /usr/bin/umoci "https://github.com/opencontainers/umoci/releases/download/v0.4.7/umoci.amd64"
		chmod +x /usr/bin/umoci
		"#
	)
}

pub fn cnitool() -> String {
	formatdoc!(
		r#"
		curl -Lf -o /usr/bin/cnitool "https://github.com/rivet-gg/cni/releases/download/v1.1.2-build3/cnitool"
		chmod +x /usr/bin/cnitool
		"#
	)
}

pub fn cni_plugins() -> String {
	include_str!("files/cni_plugins.sh").to_string()
}

pub fn nomad(server: &Server) -> String {
	let servers = &["127.0.0.1:5000"];

	include_str!("files/nomad.sh")
		.replace("__REGION_ID__", &server.region_id)
		.replace("__NODE_NAME__", &server.name)
		.replace("__VLAN_IP__", &server.vlan_ip.to_string())
		// Hardcoded to Linode
		.replace("__VLAN_IFACE__", "eth1")
		.replace(
			"__SERVER_JOIN__",
			&servers
				.iter()
				.map(|x| format!("\"{x}\""))
				.collect::<Vec<_>>()
				.join(", "),
		)
		.replace("__GG_VLAN_SUBNET__", &net::gg::vlan_ip_net().to_string())
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
	pub server_name: String,
	pub root_cas: Vec<String>,
	pub certs: Vec<Cert>,
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

	// Add TLS certs
	for (cert_id, cert) in config.tls_certs {
		script.push_str(&formatdoc!(
			r#"
			cat << 'EOF' > /etc/{config_name}/tls/{cert_id}_cert.pem
			{cert}
			EOF

			cat << 'EOF' > /etc/{config_name}/tls/{cert_id}_key.pem
			{key}
			EOF

			cat << 'EOF' > /etc/{config_name}/dynamic/tls/{cert_id}.toml
			[[tls.certificates]]
				certFile = "/etc/{config_name}/tls/{cert_id}_cert.pem"
				keyFile = "/etc/{config_name}/tls/{cert_id}_key.pem"
			EOF
			"#,
			cert = cert.cert_pem,
			key = cert.key_pem,
		));
	}

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

const TUNNEL_SERVICES: &[&'static str] = &["nomad", "api-route", "vector"];

pub fn traefik_tunnel(
	_ctx: &ProjectContext,
	k8s_infra: &crate::dep::terraform::output::K8sInfra,
	tls: &crate::dep::terraform::output::Tls,
) -> String {
	// Build transports for each service
	let mut tcp_server_transports = HashMap::new();
	for service in TUNNEL_SERVICES {
		tcp_server_transports.insert(
			service.to_string(),
			ServerTransport {
				server_name: format!("{service}.tunnel.rivet.gg"),
				root_cas: vec![(*tls.root_ca_cert_pem).clone()],
				certs: vec![(*tls.tls_cert_locally_signed_job).clone()],
			},
		);
	}

	traefik_instance(TraefikInstance {
		name: "tunnel".into(),
		static_config: tunnel_traefik_static_config(),
		dynamic_config: tunnel_traefik_dynamic_config(&*k8s_infra.traefik_tunnel_external_ip),
		tls_certs: Default::default(),
		tcp_server_transports,
	})
}

fn tunnel_traefik_static_config() -> String {
	let mut config = formatdoc!(
		r#"
		[providers]
			[providers.file]
				directory = "/etc/tunnel/dynamic"
		"#
	);

	for (i, service) in TUNNEL_SERVICES.iter().enumerate() {
		config.push_str(&formatdoc!(
			r#"
			[entryPoints.{service}]
				address = "127.0.0.1:{port}"
			"#,
			port = 5000 + i
		))
	}

	config
}

fn tunnel_traefik_dynamic_config(tunnel_external_ip: &str) -> String {
	let mut config = String::new();
	for service in TUNNEL_SERVICES.iter() {
		config.push_str(&formatdoc!(
			r#"
			[tcp.routers.{service}]
				entryPoints = ["{service}"]
				rule = "HostSNI(`*`)"  # Match all ingress, unrelated to the outbound TLS
				service = "{service}"

			[tcp.services.{service}.loadBalancer]
				serversTransport = "{service}"

				[[tcp.services.{service}.loadBalancer.servers]]
					address = "{tunnel_external_ip}:5000"
					tls = true
			"#
		))
	}

	config
}

pub struct VectorConfig {
	pub prometheus_targets: HashMap<String, VectorPrometheusTarget>,
}

pub struct VectorPrometheusTarget {
	pub endpoint: String,
	pub scrape_interval: usize,
}

pub fn vector(config: &VectorConfig) -> String {
	let sources = config
		.prometheus_targets
		.keys()
		.map(|x| format!("\"prometheus_{x}\""))
		.collect::<Vec<_>>()
		.join(", ");

	let mut config_str = formatdoc!(
		r#"
		[api]
			enabled = true

		[sinks.vector_sink]
			type = "vector"
			inputs = [{sources}]
			address = "127.0.0.1:5002"
			healthcheck.enabled = false
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

	include_str!("files/vector.sh").replace("__VECTOR_CONFIG__", &config_str)
}

pub async fn traffic_server(ctx: &ProjectContext, server: &Server) -> Result<String> {
	// Write config to files
	let config = traffic_server_config(ctx, server).await?;
	let mut config_scripts = config
		.into_iter()
		.map(|(k, v)| format!("cat << 'EOF' > /etc/trafficserver/{k}\n{v}\nEOF\n"))
		.collect::<Vec<_>>();

	// Update default storage config size to be entire filesystem size
	config_scripts.push(
		indoc!(
			"
		df -h / |
		awk 'NR==2 {print $2}' |
		xargs -I {} sed -i 's/64G/{}/' /etc/trafficserver/storage.config
		"
		)
		.to_string(),
	);

	let script = include_str!("files/traffic_server.sh")
		.replace(
			"__IMAGE__",
			"ghcr.io/rivet-gg/apache-traffic-server:9934dc2",
		)
		.replace("__CONFIG__", &config_scripts.join("\n\n"));

	Ok(script)
}

async fn traffic_server_config(
	ctx: &ProjectContext,
	server: &Server,
) -> Result<Vec<(String, String)>> {
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
			let value = value.replace("__VLAN_IP__", &server.vlan_ip.to_string());
			config_files.push((key, value));
		}
	}

	// Storage (default value of 64 gets overwritten in config script)
	let volume_size = 64;
	config_files.push((
		"storage.config".to_string(),
		format!("/var/cache/trafficserver {volume_size}G"),
	));

	// Remap & S3
	let mut remap = String::new();
	let (default_s3_provider, _) = ctx.default_s3_provider()?;
	if ctx.ns().s3.providers.minio.is_some() {
		let output = gen_s3_provider(ctx, s3_util::Provider::Minio, default_s3_provider).await?;
		remap.push_str(&output.append_remap);
		config_files.extend(output.config_files);
	}
	if ctx.ns().s3.providers.backblaze.is_some() {
		let output =
			gen_s3_provider(ctx, s3_util::Provider::Backblaze, default_s3_provider).await?;
		remap.push_str(&output.append_remap);
		config_files.extend(output.config_files);
	}
	if ctx.ns().s3.providers.aws.is_some() {
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

	// Build plugin chain
	let plugins = format!("@plugin=tslua.so @pparam=/etc/trafficserver/strip_headers.lua @plugin=s3_auth.so @pparam=--config @pparam=s3_auth_v4_{provider_name}.config");

	// Add remap
	remap.push_str(&format!(
		"map /s3-cache/{provider_name} {endpoint_external} {plugins}\n",
		endpoint_external = config.endpoint_external
	));

	// Add default route
	if default_s3_provider == provider {
		remap.push_str(&format!(
			"map /s3-cache {endpoint_external} {plugins}\n",
			endpoint_external = config.endpoint_external,
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
			s3_host = config.endpoint_external.split_once("://").unwrap().1,
			s3_region = config.region,
		),
	));

	Ok(GenRemapS3ProviderOutput {
		append_remap: remap,
		config_files,
	})
}

pub fn envoy() -> String {
	include_str!("files/envoy.sh").to_string()
}

pub fn outbound_proxy(server: &Server, all_servers: &HashMap<String, Server>) -> Result<String> {
	// Build ATS endpoints
	let mut ats_servers = all_servers
		.values()
		.filter(|x| server.region_id == x.region_id && x.pool_id == "ats")
		.collect::<Vec<_>>();
	// Use the same sorting as ATS for consistent Maglev hashing
	ats_servers.sort_by_key(|x| x.index);
	let ats_endpoints = ats_servers
		.iter()
		.map(|x| {
			json!({
				"endpoint": {
					"address": {
						"socket_address": {
							"address": x.vlan_ip.to_string(),
							"port_value": 8080
						}
					}
				}
			})
		})
		.collect::<Vec<_>>();

	// Build config
	let config = json!({
		"static_resources": {
			"listeners": [{
				"name": "ats",
				"address": {
					"socket_address": {
						"address": "0.0.0.0",
						"port_value": 8080
					}
				},
				"filter_chains": [{
					"filters": [{
						"name": "envoy.filters.network.http_connection_manager",
						"typed_config": {
							"@type": "type.googleapis.com/envoy.extensions.filters.network.http_connection_manager.v3.HttpConnectionManager",
							"stat_prefix": "ingress_http",
							"route_config": {
								"name": "local_route",
								"virtual_hosts": [
									{
										"name": "backend",
										"domains": ["*"],
										"routes": [{
											"match": { "prefix": "/" },
											"route": {
												"cluster": "ats_backend",
												"hash_policy": [{
													"header": { "header_name": ":path" }
												}]
											}
										}]
									}
								]
							},
							"http_filters": [
								{
									"name": "envoy.filters.http.router",
									"typed_config": {
										"@type": "type.googleapis.com/envoy.extensions.filters.http.router.v3.Router"
									}
								}
							]
						}
					}]
				}]
			}],
			"clusters": [{
				"name": "ats_backend",
				"connect_timeout": "0.25s",
				// Use consistent hashing to reliably send the same request to the same server
				//
				// In order for this to work, the load balancer must be configured with the same:
				// - Table size
				// - List of backend nodes (in the same order)
				// - Hash key for each endpoint (uses the host by default)
				//
				// See https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/upstream/load_balancing/load_balancers#arch-overview-load-balancing-types-maglev
				"lb_policy": "MAGLEV",
				"maglev_lb_config": {
					// Ensure the same table size for consistent hashing across load balancers
					"table_size": 65537
				},
				"load_assignment": {
					"cluster_name": "ats_backend",
					"endpoints": [
						{
							"lb_endpoints": ats_endpoints
						}
					]
				}
			}]
		}
	});

	let yaml_config = serde_yaml::to_string(&config)?;
	Ok(include_str!("files/outbound_proxy.sh").replace("__ENVOY_CONFIG__", &yaml_config))
}
