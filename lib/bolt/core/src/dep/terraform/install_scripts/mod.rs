use crate::{context::ProjectContext, dep::terraform::servers::Server};
use anyhow::Result;
use indoc::formatdoc;
use maplit::hashmap;

use crate::dep::terraform;

pub mod components;

pub async fn gen(ctx: &ProjectContext, server: &Server) -> Result<String> {
	let k8s_infra = terraform::output::read_k8s_infra(ctx).await;
	let tls = terraform::output::read_tls(ctx).await;

	let mut script = Vec::new();
	script.push(components::common());
	script.push(components::node_exporter());
	script.push(components::sysctl());

	if server.pool_id == "gg" {
		script.push(components::traefik());

		script.push(components::traefik_instance(components::TraefikInstance {
			name: "game_guard".into(),
			static_config: gg_traefik_static_config(),
			dynamic_config: String::new(),
			tls_certs: hashmap! {
				"letsencrypt_rivet_job".into() => (*tls.tls_cert_letsencrypt_rivet_job).clone(),
			},
			tcp_server_transports: Default::default(),
		}));

		script.push(components::traefik_instance(components::TraefikInstance {
			name: "tunnel".into(),
			static_config: tunnel_traefik_static_config(),
			dynamic_config: tunnel_traefik_dynamic_config(&*k8s_infra.traefik_tunnel_external_ip),
			tls_certs: Default::default(),
			tcp_server_transports: hashmap! {
				"tunnel".into() => components::ServerTransport {
					certs: vec![
						(*tls.tls_cert_locally_signed_job).clone()
					]
				}
			},
		}));
	}

	if server.pool_id == "job" {
		script.push(components::traefik());

		script.push(components::traefik_instance(components::TraefikInstance {
			name: "tunnel".into(),
			static_config: tunnel_traefik_static_config(),
			dynamic_config: tunnel_traefik_dynamic_config(&*k8s_infra.traefik_tunnel_external_ip),
			tls_certs: Default::default(),
			tcp_server_transports: hashmap! {
				"tunnel".into() => components::ServerTransport {
					certs: vec![
						(*tls.tls_cert_locally_signed_job).clone()
					]
				}
			},
		}));

		script.push(components::docker()); // why do we need to install docker and cni plugins?
		script.push(components::cni_plugins());
		script.push(components::nomad(server));
	}

	if server.pool_id == "ats" {
		script.push(components::docker());
		script.push(components::traffic_server(ctx).await?);
	}

	let joined = script.join("\n\necho \"======\"\n\n");
	Ok(format!("#!/usr/bin/env bash\nset -eu\n\n{joined}"))
}

fn gg_traefik_static_config() -> String {
	let http_provider_endpoint = "foo bar"; // should point to the api-route in the core cluster for exposing game guard dynamic config. this should already exist!

	let mut config = formatdoc!(
		r#"
		[entryPoints]
			[entryPoints.lb-80]
				address = ":80"

			[entryPoints.lb-443]
				address = ":443"

		[metrics.prometheus]
			# See lib/chirp/metrics/src/buckets.rs
			buckets = [0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 25.0, 50.0, 100.0]
			addEntryPointsLabels = true
			addRoutersLabels = true
			addServicesLabels = true

		[providers]
			[providers.file]
				directory = "/etc/game_guard/dynamic"

			[providers.http]
				endpoint = "{http_provider_endpoint}"
				pollInterval = "1s"
		"#
	);

	// TCP ports
	for port in 20000..20512 {
		config.push_str(&formatdoc!(
			r#"
			[entryPoints.lb-{port}]
				address = ":{port}/tcp"

			[entryPoints.lb-{port}.transport.respondingTimeouts]
				readTimeout = "15s"
				writeTimeout = "15s"
				idleTimeout = "15s"

			"#
		));
	}

	// UDP ports
	for port in 26000..26512 {
		config.push_str(&formatdoc!(
			r#"
			[entryPoints.lb-{port}]
				address = ":{port}/udp"

			[entryPoints.lb-{port}.udp]
				timeout = "15s"
			"#
		));
	}

	config
}

fn tunnel_traefik_static_config() -> String {
	formatdoc!(
		r#"
		[entryPoints.nomad]
			address = "127.0.0.1:5000"

		[entryPoints.api_route]
			address = "127.0.0.1:5001"

		[providers]
			[providers.file]
				directory = "/etc/tunnel/dynamic"
		"#
	)
}

fn tunnel_traefik_dynamic_config(tunnel_external_ip: &str) -> String {
	formatdoc!(
		r#"
		[tcp.routers.nomad]
			entryPoints = ["nomad"]
			rule = "HostSNI(`*`)"
			service = "nomad"

		[tcp.routers.api_route]
			entryPoints = ["api_route"]
			rule = "HostSNI(`*`)"
			service = "api_route"

		[tcp.services.nomad.loadBalancer]
			serversTransport = "tunnel"

			[[tcp.services.nomad.loadBalancer.servers]]
				address = "{tunnel_external_ip}:5000"

		[tcp.services.api_route.loadBalancer]
			serversTransport = "tunnel"

			[[tcp.services.api_route.loadBalancer.servers]]
				address = "{tunnel_external_ip}:5001"
		"#,
	)
}
