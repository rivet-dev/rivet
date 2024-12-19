use std::collections::HashMap;

use chirp_workflow::prelude::*;
use indoc::formatdoc;

use super::{
	ok_server::OK_SERVER_PORT,
	pegboard::TUNNEL_PEGBOARD_WS_PORT,
	vector::{TUNNEL_VECTOR_PORT, TUNNEL_VECTOR_TCP_JSON_PORT},
	TUNNEL_API_EDGE_PORT,
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
		name: "api-edge",
		port: TUNNEL_API_EDGE_PORT,
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
		port: TUNNEL_PEGBOARD_WS_PORT,
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

pub fn tunnel(config: &rivet_config::Config, name: &str) -> GlobalResult<String> {
	// Build transports for each service
	let tls_config = &config.server()?.tls()?;
	let mut tcp_server_transports = HashMap::new();
	for TunnelService { name, .. } in TUNNEL_SERVICES {
		tcp_server_transports.insert(
			name.to_string(),
			ServerTransport {
				server_name: format!("{name}.tunnel.rivet.gg"),
				root_cas: vec![tls_config.root_ca_cert_pem.read().clone()],
				certs: vec![TlsCert {
					cert_pem: tls_config.cert_locally_signed_job_cert_pem.read().clone(),
					key_pem: tls_config.cert_locally_signed_job_key_pem.read().clone(),
				}],
			},
		);
	}

	Ok(instance(Instance {
		name: name.to_string(),
		static_config: tunnel_static_config(),
		dynamic_config: tunnel_dynamic_config(&config.server()?.rivet.tunnel.public_host),
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

pub async fn gg_static_config(config: &rivet_config::Config) -> GlobalResult<String> {
	let gg_config = &config.server()?.rivet.guard;

	let http_provider_endpoint = if let Some(api_traefik_provider_token) =
		&config.server()?.rivet.api_edge.traefik_provider_token
	{
		format!(
			"http://127.0.0.1:{port}/traefik-provider/config/game-guard?token={token}&datacenter=___DATACENTER_ID___",
			port = TUNNEL_API_EDGE_PORT,
			token = api_traefik_provider_token.read(),
		)
	} else {
		format!(
			"http://127.0.0.1:{port}/traefik-provider/config/game-guard?datacenter=___DATACENTER_ID___",
			port = TUNNEL_API_EDGE_PORT,
		)
	};

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
	for port in gg_config.min_ingress_port_tcp()..=gg_config.max_ingress_port_tcp() {
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
	for port in gg_config.min_ingress_port_udp()..=gg_config.max_ingress_port_udp() {
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

pub fn gg_dynamic_config(
	config: &rivet_config::Config,
	datacenter_id: Uuid,
) -> GlobalResult<String> {
	let Some(domain_job) = config
		.server()?
		.rivet
		.dns
		.as_ref()
		.and_then(|x| x.domain_job.as_ref())
	else {
		// Don't return a config since we can't reserve a unique hostname
		return Ok(String::new());
	};

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
