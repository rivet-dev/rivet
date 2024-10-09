use std::collections::HashMap;

use chirp_workflow::prelude::*;
use indoc::formatdoc;

use super::{
	ok_server::OK_SERVER_PORT,
	vector::{TUNNEL_VECTOR_PORT, TUNNEL_VECTOR_TCP_JSON_PORT},
	TUNNEL_API_INTERNAL_PORT,
};

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
		name: "api-internal",
		port: TUNNEL_API_INTERNAL_PORT,
	},
	TunnelService {
		name: "vector",
		port: TUNNEL_VECTOR_PORT,
	},
	TunnelService {
		name: "vector-tcp-json",
		port: TUNNEL_VECTOR_TCP_JSON_PORT,
	},
	TunnelService {
		name: "pegboard-server",
		port: 5030,
	},
];

/// Service that gets exposed from the Traefik tunnel.
pub struct TunnelService {
	/// Name of the service for the subdomain. This is how the Traefik tunnel server knows where to
	/// route traffic.
	name: &'static str,

	/// The port to serve the service on locally.
	port: u16,
}

/// Installs Traefik, but does not create the Traefik service.
pub fn install() -> String {
	include_str!("../files/traefik.sh").to_string()
}

pub struct TlsCert {
	pub cert_pem: String,
	pub key_pem: String,
}

pub struct Instance {
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
/// Requires `install()`.
pub fn instance(config: Instance) -> String {
	let config_name = &config.name;

	let mut script = include_str!("../files/traefik_instance.sh")
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

pub fn tunnel(name: &str) -> GlobalResult<String> {
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

	Ok(instance(Instance {
		name: name.to_string(),
		static_config: tunnel_static_config(),
		dynamic_config: tunnel_dynamic_config(&util::env::var("RIVET_HOST_TUNNEL")?),
		tcp_server_transports,
	}))
}

fn tunnel_static_config() -> String {
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

fn tunnel_dynamic_config(host_tunnel: &str) -> String {
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
					address = "{host_tunnel}"
					tls = true
			"#
		))
	}

	config
}

pub async fn gg_static_config() -> GlobalResult<String> {
	let api_traefik_provider_token =
		&util::env::read_secret(&["rivet", "api_traefik_provider", "token"]).await?;
	let http_provider_endpoint = format!(
		"http://127.0.0.1:{port}/traefik-provider/config/game-guard?token={api_traefik_provider_token}&datacenter=___DATACENTER_ID___",
		port = TUNNEL_API_INTERNAL_PORT,
	);

	// Metrics are disabled since they're too high cardinality for Prometheus (both the # of
	// entrypoint & the frequently changing routers + services)
	let mut config = formatdoc!(
		r#"
		[entryPoints]
			[entryPoints.traefik]
				address = "127.0.0.1:9980"

			[entryPoints.lb-80]
				address = ":80"

			[entryPoints.lb-443]
				address = ":443"

		[api]
			insecure = true

		[providers]
			[providers.file]
				directory = "/etc/game_guard/dynamic"

			[providers.http]
				endpoint = "{http_provider_endpoint}"
				pollInterval = "0.5s"
		"#
	);

	// TCP ports
	for port in util::net::job::MIN_INGRESS_PORT_TCP..=util::net::job::MAX_INGRESS_PORT_TCP {
		config.push_str(&formatdoc!(
			r#"
			[entryPoints.lb-{port}-tcp]
				address = ":{port}/tcp"

			[entryPoints.lb-{port}-tcp.transport.respondingTimeouts]
				readTimeout = "12h"
				writeTimeout = "12h"
				idleTimeout = "30s"

			"#
		));
	}

	// UDP ports
	for port in util::net::job::MIN_INGRESS_PORT_UDP..=util::net::job::MAX_INGRESS_PORT_UDP {
		config.push_str(&formatdoc!(
			r#"
			[entryPoints.lb-{port}-udp]
				address = ":{port}/udp"

			[entryPoints.lb-{port}-udp.udp]
				timeout = "15s"
			"#
		));
	}

	Ok(config)
}

pub fn gg_dynamic_config(datacenter_id: Uuid) -> GlobalResult<String> {
	let domain_job = unwrap!(util::env::domain_job(), "dns not enabled");

	let main = format!("{datacenter_id}.{domain_job}");

	Ok(formatdoc!(
		r#"
		# Always returns 200 at /status
		[http.routers.ok-status]
			entryPoints = ["lb-80"]
			rule = "Host(`lobby.{main}`) && Path(`/status`)"
			service = "ok-service"

		[http.routers.ok-status-secure]
			entryPoints = ["lb-443"]
			rule = "Host(`lobby.{main}`) && Path(`/status`)"
			service = "ok-service"
		[[http.routers.ok-status-secure.tls.domains]]
			main = "{main}"
			sans = []

		[http.services.ok-service.loadBalancer]
			[[http.services.ok-service.loadBalancer.servers]]
			url = "http://127.0.0.1:{OK_SERVER_PORT}"
		"#
	))
}
