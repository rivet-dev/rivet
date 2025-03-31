use std::collections::HashMap;

use chirp_workflow::prelude::*;
use components::pegboard::ClientFlavor;

use crate::types::PoolType;

pub mod components;

const TUNNEL_NAME: &str = "tunnel";
const GG_TRAEFIK_INSTANCE_NAME: &str = "game_guard";

// This script installs all of the software that doesn't need to know anything about the server running
// it (doesn't need to know server id, datacenter id, vlan ip, etc)
pub async fn gen_install(
	config: &rivet_config::Config,
	pool_type: PoolType,
	initialize_immediately: bool,
	server_token: &str,
	tunnel_root_ca: &str,
	tunnel_cert: &components::traefik::TlsCert,
) -> GlobalResult<String> {
	// MARK: Common (pre)
	let mut script = vec![
		components::common(),
		components::node_exporter::install(),
		components::sysctl::install(),
		components::traefik::install(),
		// NOTE: TLS certs expire in a year, prebakes expire in 6 months
		components::traefik::tunnel(config, TUNNEL_NAME, tunnel_root_ca, tunnel_cert)?,
		components::rivet::fetch_tunnel_tls(initialize_immediately, server_token, TUNNEL_NAME)?,
		components::vector::install(),
	];

	// MARK: Specific pool components
	match pool_type {
		PoolType::Job => {
			script.push(components::docker::install());
			script.push(components::lz4::install());
			script.push(components::skopeo::install());
			script.push(components::umoci::install());
			script.push(components::cni::tool());
			script.push(components::cni::plugins());
			script.push(components::nomad::install());
		}
		PoolType::Gg => {
			script.push(components::ok_server::install(initialize_immediately));
		}
		PoolType::Ats => {
			script.push(components::docker::install());
			script.push(components::traffic_server::install());
		}
		PoolType::Pegboard | PoolType::PegboardIsolate => {
			script.push(components::docker::install());
			script.push(components::lz4::install());
			script.push(components::skopeo::install());
			script.push(components::umoci::install());
			script.push(components::cni::tool());
			script.push(components::cni::plugins());

			let flavor = match pool_type {
				PoolType::Pegboard => ClientFlavor::Container,
				PoolType::PegboardIsolate => ClientFlavor::Isolate,
				_ => unreachable!(),
			};
			script.push(components::pegboard::install(config, flavor).await?);
		}
		PoolType::Fdb => {
			script.push(components::python::install());
			script.push(components::fdb::install(initialize_immediately));
		}
		PoolType::Worker => {
			script.push(components::rivet::worker::install(config)?);
		}
		PoolType::Nats => {
			script.push(components::docker::install());
			script.push(components::nats::install(config)?);
		}
		PoolType::Guard => {
			script.push(components::ok_server::install(initialize_immediately));
			script.push(components::rivet::guard::install(config)?);
		}
	}

	// MARK: Common (post)
	script.push(components::rivet::create_hook(
		TUNNEL_NAME,
		initialize_immediately,
	)?);

	let joined = script.join("\n\necho \"======\"\n\n");
	Ok(format!("#!/usr/bin/env bash\nset -eu\n\n{joined}"))
}

// This script is run by systemd on startup and gets the server's data from the Rivet API
pub async fn gen_hook(server_token: &str) -> GlobalResult<String> {
	let script = [components::rivet::fetch_info(server_token)?];

	let joined = script.join("\n\necho \"======\"\n\n");
	Ok(format!("#!/usr/bin/env bash\nset -eu\n\n{joined}"))
}

// This script is templated on the server itself after fetching server data from the Rivet API (see gen_hook).
// After being templated, it is run.
pub async fn gen_initialize(
	config: &rivet_config::Config,
	pool_type: PoolType,
	server_token: &str,
) -> GlobalResult<String> {
	let mut script = Vec::new();

	let mut prometheus_targets = HashMap::new();

	// MARK: Common (pre)
	prometheus_targets.insert(
		"node_exporter".into(),
		components::vector::PrometheusTarget {
			endpoint: "http://127.0.0.1:9100/metrics".into(),
			scrape_interval: 15,
		},
	);

	// MARK: Specific pool components
	match pool_type {
		PoolType::Job => {
			script.push(components::nomad::configure(config)?);

			prometheus_targets.insert(
				"nomad".into(),
				components::vector::PrometheusTarget {
					endpoint: "http://127.0.0.1:4646/v1/metrics?format=prometheus".into(),
					scrape_interval: 15,
				},
			);
		}
		PoolType::Gg => {
			script.push(components::rivet::fetch_api_route(
				server_token,
				GG_TRAEFIK_INSTANCE_NAME,
			)?);
			script.push(components::rivet::fetch_gg_tls(
				server_token,
				GG_TRAEFIK_INSTANCE_NAME,
			)?);
			script.push(components::traefik::instance(
				components::traefik::Instance {
					name: GG_TRAEFIK_INSTANCE_NAME.to_string(),
					static_config: components::traefik::gg_static_config(config).await?,
					dynamic_config: components::traefik::gg_dynamic_config(config)?,
					tcp_server_transports: Default::default(),
				},
			)?);
		}
		PoolType::Ats => {
			script.push(components::traffic_server::configure(config).await?);
		}
		PoolType::Pegboard => {
			script.push(components::pegboard::configure(
				config,
				ClientFlavor::Container,
			)?);

			prometheus_targets.insert(
				"pegboard".into(),
				components::vector::PrometheusTarget {
					// Should match port from pb manager config
					endpoint: "http://127.0.0.1:6090".into(),
					scrape_interval: 15,
				},
			);
		}
		PoolType::PegboardIsolate => {
			script.push(components::pegboard::configure(
				config,
				ClientFlavor::Isolate,
			)?);

			prometheus_targets.insert(
				"pegboard".into(),
				components::vector::PrometheusTarget {
					// Should match port from pb manager config
					endpoint: "http://127.0.0.1:6090".into(),
					scrape_interval: 15,
				},
			);
		}
		PoolType::Fdb => {
			script.push(components::fdb::configure());
			prometheus_targets.insert(
				"foundationdb".into(),
				components::vector::PrometheusTarget {
					endpoint: "http://127.0.0.1:9161".into(),
					scrape_interval: 15,
				},
			);
		}
		PoolType::Worker => {
			script.push(components::rivet::worker::configure(config)?);

			prometheus_targets.insert(
				"worker".into(),
				components::vector::PrometheusTarget {
					endpoint: "http://127.0.0.1:8091".into(),
					scrape_interval: 15,
				},
			);
		}
		PoolType::Nats => {
			script.push(components::nats::fetch_routes(config, server_token)?);
		}
		PoolType::Guard => {
			script.push(components::rivet::fetch_api_route(
				server_token,
				GG_TRAEFIK_INSTANCE_NAME,
			)?);
			script.push(components::rivet::guard::fetch_tls(server_token)?);
			script.push(components::rivet::guard::configure(config)?);
		}
	}

	// MARK: Common (post)
	if !prometheus_targets.is_empty() {
		script.push(components::vector::configure(
			&components::vector::Config { prometheus_targets },
			pool_type,
		)?);
	}

	let joined = script.join("\n\necho \"======\"\n\n");
	Ok(format!("#!/usr/bin/env bash\nset -eu\n\n{joined}"))
}
