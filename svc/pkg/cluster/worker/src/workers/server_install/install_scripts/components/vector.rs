use std::collections::HashMap;

use chirp_worker::prelude::*;
use proto::backend;

pub const TUNNEL_VECTOR_PORT: u16 = 5020;
pub const TUNNEL_VECTOR_TCP_JSON_PORT: u16 = 5021;

pub fn install() -> String {
	include_str!("../files/vector_install.sh").to_string()
}

pub struct Config {
	pub prometheus_targets: HashMap<String, PrometheusTarget>,
}

pub struct PrometheusTarget {
	pub endpoint: String,
	pub scrape_interval: usize,
}

pub fn configure(config: &Config, pool_type: backend::cluster::PoolType) -> String {
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

		[transforms.filter_metrics]
            type = "filter"
			inputs = [{sources}]
            condition = '!starts_with!(.name, "go_") && !starts_with!(.name, "promhttp_")'


		[transforms.add_meta]
			type = "remap"
			inputs = ["filter_metrics"]
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

			# Buffer to disk for durability & reduce memory usage
            buffer.type = "disk"
            buffer.max_size = 268435488  # 256 MB
            buffer.when_full = "block"
		"#
	);

	for (
		key,
		PrometheusTarget {
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

	include_str!("../files/vector_configure.sh").replace("__VECTOR_CONFIG__", &config_str)
}
