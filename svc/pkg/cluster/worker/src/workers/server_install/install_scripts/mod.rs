use std::collections::HashMap;

use chirp_worker::prelude::*;
use indoc::formatdoc;
use proto::backend;

pub mod components;

// This script installs all of the software that doesn't need to know anything about the server running
// it (doesn't need to know server id, datacenter id, vlan ip, etc)
pub async fn gen_install(
	pool_type: backend::cluster::PoolType,
	initialize_immediately: bool,
) -> GlobalResult<String> {
	// MARK: Common (pre)
	let mut script = vec![
		components::common(),
		components::node_exporter(),
		components::sysctl(),
		components::traefik(),
		components::traefik_tunnel()?,
		components::vector_install(),
	];

	// MARK: Specific pool components
	match pool_type {
		backend::cluster::PoolType::Job => {
			script.push(components::docker());
			script.push(components::lz4());
			script.push(components::skopeo());
			script.push(components::umoci());
			script.push(components::cnitool());
			script.push(components::cni_plugins());
			script.push(components::nomad_install());
		}
		backend::cluster::PoolType::Gg => {}
		backend::cluster::PoolType::Ats => {
			script.push(components::docker());
			script.push(components::traffic_server_install());
		}
	}

	// MARK: Common (post)
	script.push(components::rivet_create_hook(initialize_immediately)?);

	let joined = script.join("\n\necho \"======\"\n\n");
	Ok(format!("#!/usr/bin/env bash\nset -eu\n\n{joined}"))
}

// This script is run by systemd on startup and gets the server's data from the Rivet API
pub async fn gen_hook(server_token: &str) -> GlobalResult<String> {
	let mut script = vec![components::rivet_fetch_info(server_token)?];

	let joined = script.join("\n\necho \"======\"\n\n");
	Ok(format!("#!/usr/bin/env bash\nset -eu\n\n{joined}"))
}

// This script is templated on the server itself after fetching server data from the Rivet API
// (see gen_hook) After being templated, it is run.
pub async fn gen_initialize(
	pool_type: backend::cluster::PoolType,
	initialize_immediately: bool,
	server_token: &str,
) -> GlobalResult<String> {
	let mut script = Vec::new();

	let mut prometheus_targets = HashMap::new();

	// MARK: Common (pre)
	prometheus_targets.insert(
		"node_exporter".into(),
		components::VectorPrometheusTarget {
			endpoint: "http://127.0.0.1:9100/metrics".into(),
			scrape_interval: 15,
		},
	);

	// MARK: Specific pool components
	match pool_type {
		backend::cluster::PoolType::Job => {
			script.push(components::nomad_configure());

			prometheus_targets.insert(
				"nomad".into(),
				components::VectorPrometheusTarget {
					endpoint: "http://127.0.0.1:4646/v1/metrics?format=prometheus".into(),
					scrape_interval: 15,
				},
			);
		}
		backend::cluster::PoolType::Gg => {
			let traefik_instance_name = "game_guard".to_string();

			script.push(components::traefik_instance(components::TraefikInstance {
				name: traefik_instance_name.clone(),
				static_config: gg_traefik_static_config().await?,
				dynamic_config: String::new(),
				tcp_server_transports: Default::default(),
			}));

			script.push(components::rivet_fetch_tls(
				initialize_immediately,
				server_token,
				&traefik_instance_name,
			)?);

			prometheus_targets.insert(
				"game_guard".into(),
				components::VectorPrometheusTarget {
					endpoint: "http://127.0.0.1:9980/metrics".into(),
					scrape_interval: 15,
				},
			);
		}
		backend::cluster::PoolType::Ats => {
			script.push(components::traffic_server_configure().await?);
		}
	}

	// MARK: Common (post)
	if !prometheus_targets.is_empty() {
		script.push(components::vector_configure(
			&components::VectorConfig { prometheus_targets },
			pool_type,
		));
	}

	let joined = script.join("\n\necho \"======\"\n\n");
	Ok(format!("#!/usr/bin/env bash\nset -eu\n\n{joined}"))
}

async fn gg_traefik_static_config() -> GlobalResult<String> {
	let api_traefik_provider_token =
		&util::env::read_secret(&["rivet", "api_traefik_provider", "token"]).await?;
	let http_provider_endpoint = format!(
		"http://127.0.0.1:{port}/traefik-provider/config/game-guard?token={api_traefik_provider_token}&datacenter=___DATACENTER_ID___",
		port = components::TUNNEL_API_INTERNAL_PORT,
	);

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
