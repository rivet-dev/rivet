use anyhow::Result;
use indexmap::{indexmap, IndexMap};
use indoc::formatdoc;
use std::collections::HashMap;

use crate::{context::ProjectContext, dep::terraform, dep::terraform::servers::Server};

pub mod components;

pub async fn gen(
	ctx: &ProjectContext,
	server: &Server,
	all_servers: &HashMap<String, Server>,
	k8s_infra: &terraform::output::K8sInfra,
	tls: &terraform::output::Tls,
) -> Result<String> {
	let mut script = Vec::new();

	let mut prometheus_targets = IndexMap::new();

	// MARK: Common (pre)
	script.push(components::common());
	script.push(components::node_exporter());
	script.push(components::sysctl());
	script.push(components::traefik());
	script.push(components::traefik_tunnel(ctx, &k8s_infra, &tls));

	prometheus_targets.insert(
		"node_exporter".into(),
		components::VectorPrometheusTarget {
			endpoint: "http://127.0.0.1:9100/metrics".into(),
			scrape_interval: 15,
		},
	);

	// MARK: Game Guard
	if server.pool_id == "gg" {
		script.push(components::traefik());
		script.push(components::traefik_instance(components::TraefikInstance {
			name: "game_guard".into(),
			static_config: gg_traefik_static_config(
				server,
				&ctx.read_secret(&["rivet", "api_route", "token"]).await?,
			),
			dynamic_config: String::new(),
			tls_certs: indexmap! {
				"letsencrypt_rivet_job".into() => (*tls.tls_cert_letsencrypt_rivet_job).clone(),
			},
			tcp_server_transports: Default::default(),
		}));

		prometheus_targets.insert(
			"game_guard".into(),
			components::VectorPrometheusTarget {
				endpoint: "http://127.0.0.1:9980/metrics".into(),
				scrape_interval: 15,
			},
		);
	}

	// MARK: Job
	if server.pool_id == "job" {
		script.push(components::docker());
		script.push(components::lz4());
		script.push(components::skopeo());
		script.push(components::umoci());
		script.push(components::cnitool());
		script.push(components::cni_plugins());
		script.push(components::nomad(server));
		script.push(components::envoy());
		script.push(components::outbound_proxy(server, all_servers)?);

		prometheus_targets.insert(
			"nomad".into(),
			components::VectorPrometheusTarget {
				endpoint: "http://127.0.0.1:4646/v1/metrics?format=prometheus".into(),
				scrape_interval: 15,
			},
		);
	}

	// MARK: ATS
	if server.pool_id == "ats" {
		script.push(components::docker());
		script.push(components::traffic_server(ctx, server).await?);
	}

	// MARK: Common (post)
	if !prometheus_targets.is_empty() {
		script.push(components::vector(&components::VectorConfig {
			prometheus_targets,
		}));
	}

	let joined = script.join("\n\necho \"======\"\n\n");
	Ok(format!("#!/usr/bin/env bash\nset -eu\n\n{joined}"))
}

fn gg_traefik_static_config(server: &Server, api_route_token: &str) -> String {
	let http_provider_endpoint = format!(
		"http://127.0.0.1:5001/traefik/config/game-guard?token={api_route_token}&region={region}",
		region = server.region_id
	);

	let mut config = formatdoc!(
		r#"
		[entryPoints]
			[entryPoints.traefik]
				address = ":9980"

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
	for port in 20000..=31999 {
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
	for port in 20000..=31999 {
		config.push_str(&formatdoc!(
			r#"
			[entryPoints.lb-{port}-udp]
				address = ":{port}/udp"

			[entryPoints.lb-{port}-udp.udp]
				timeout = "15s"
			"#
		));
	}

	config
}
