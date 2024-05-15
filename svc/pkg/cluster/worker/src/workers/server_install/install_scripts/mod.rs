use std::collections::HashMap;

use chirp_worker::prelude::*;
use proto::backend;

pub mod components;

const TUNNEL_NAME: &str = "tunnel";
const GG_TRAEFIK_INSTANCE_NAME: &str = "game_guard";

// This script installs all of the software that doesn't need to know anything about the server running
// it (doesn't need to know server id, datacenter id, vlan ip, etc)
pub async fn gen_install(
	pool_type: backend::cluster::PoolType,
	initialize_immediately: bool,
	server_token: &str,
) -> GlobalResult<String> {
	// MARK: Common (pre)
	let mut script = vec![
		components::common(),
		components::node_exporter::install(),
		components::sysctl::install(),
		components::traefik::install(),
		components::traefik::tunnel(TUNNEL_NAME)?,
		components::vector::install(),
	];

	// MARK: Specific pool components
	match pool_type {
		backend::cluster::PoolType::Job => {
			script.push(components::docker::install());
			script.push(components::lz4::install());
			script.push(components::skopeo::install());
			script.push(components::umoci::install());
			script.push(components::cni::tool());
			script.push(components::cni::plugins());
			script.push(components::nomad::install());
		}
		backend::cluster::PoolType::Gg => {
			script.push(components::rivet::fetch_tls(
				initialize_immediately,
				server_token,
				GG_TRAEFIK_INSTANCE_NAME,
			)?);
		}
		backend::cluster::PoolType::Ats => {
			script.push(components::docker::install());
			script.push(components::traffic_server::install());
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
pub async fn gen_initialize(pool_type: backend::cluster::PoolType) -> GlobalResult<String> {
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
		backend::cluster::PoolType::Job => {
			script.push(components::nomad::configure());

			prometheus_targets.insert(
				"nomad".into(),
				components::vector::PrometheusTarget {
					endpoint: "http://127.0.0.1:4646/v1/metrics?format=prometheus".into(),
					scrape_interval: 15,
				},
			);
		}
		backend::cluster::PoolType::Gg => {
			script.push(components::traefik::instance(
				components::traefik::Instance {
					name: GG_TRAEFIK_INSTANCE_NAME.to_string(),
					static_config: components::traefik::gg_static_config().await?,
					dynamic_config: String::new(),
					tcp_server_transports: Default::default(),
				},
			));

			prometheus_targets.insert(
				GG_TRAEFIK_INSTANCE_NAME.into(),
				components::vector::PrometheusTarget {
					endpoint: "http://127.0.0.1:9980/metrics".into(),
					scrape_interval: 15,
				},
			);
		}
		backend::cluster::PoolType::Ats => {
			script.push(components::traffic_server::configure().await?);
		}
	}

	// MARK: Common (post)
	if !prometheus_targets.is_empty() {
		script.push(components::vector::configure(
			&components::vector::Config { prometheus_targets },
			pool_type,
		));
	}

	let joined = script.join("\n\necho \"======\"\n\n");
	Ok(format!("#!/usr/bin/env bash\nset -eu\n\n{joined}"))
}
