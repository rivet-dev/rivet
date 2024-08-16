use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
};

use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;
use rivet_operation::prelude::*;
use rivet_pools::prelude::*;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{auth::Auth, types};

// TODO: Rename to ProxiedPort since this is not 1:1 with servers
#[derive(sqlx::FromRow, Clone, Debug, Serialize, Deserialize)]
struct DynamicServer {
	server_id: Uuid,
	datacenter_id: Uuid,
	label: String,
	nomad_ip: String,
	nomad_source: i64,
	port_number: i64,
	gg_port: i64,
	port_name: String,
	protocol: i64,
}

impl DynamicServer {
	fn parent_host(&self) -> GlobalResult<String> {
		Ok(format!(
			"lobby.{}.{}",
			self.datacenter_id,
			unwrap!(util::env::domain_job()),
		))
	}

	fn hostname(&self) -> GlobalResult<String> {
		util_ds::build_ds_hostname(self.server_id, &self.port_name, self.datacenter_id)
	}
}

pub async fn build_ds(
	ctx: &Ctx<Auth>,
	dc_id: Uuid,
	config: &mut types::TraefikConfigResponse,
) -> GlobalResult<()> {
	// TODO put in function, clean up
	// TODO: remove cache for now
	tracing::info!(?config, "config timeeee");

	// let dynamic_servers: Option<Vec<DynamicServer>> = ctx
	// 	.cache()
	// 	// TODO: Set this for longer, this should mean that no caching happens
	// 	.ttl(1)
	// 	.fetch_one_json("servers_ports", dc_id, |mut cache, dc_id| {
	// 		let ctx = ctx.clone();
	// 		async move {
	let dynamic_servers = sql_fetch_all!(
		[ctx, DynamicServer]
		"
		SELECT
			servers.server_id,
			servers.datacenter_id,
			internal_ports.nomad_label AS label,
			internal_ports.nomad_ip,
			internal_ports.nomad_source,
			docker_ports_protocol_game_guard.port_number,
			docker_ports_protocol_game_guard.gg_port,
			docker_ports_protocol_game_guard.port_name,
			docker_ports_protocol_game_guard.protocol
		FROM
			db_ds.internal_ports
		JOIN
			db_ds.servers
		ON
			internal_ports.server_id = servers.server_id
		JOIN
			db_ds.docker_ports_protocol_game_guard
				ON
					internal_ports.server_id = docker_ports_protocol_game_guard.server_id
				AND
					internal_ports.nomad_label = CONCAT('ds_', docker_ports_protocol_game_guard.port_name)
		WHERE
			servers.datacenter_id = $1 AND servers.stop_ts IS NULL
		",
		dc_id
	)
	.await?;
	// 		cache.resolve(&dc_id, rows);

	// 		Ok(cache)
	// 	}
	// })
	// .await?;

	tracing::info!(?config, "config timeeee2");

	// let dynamic_servers = unwrap!(dynamic_servers);
	tracing::info!(?dynamic_servers, "ds0time");

	// Process proxied ports
	for dynamic_server in &dynamic_servers {
		tracing::info!(?dynamic_server, "ds1time");

		let server_id = dynamic_server.server_id;
		let register_res = ds_register_proxied_port(server_id, dynamic_server, config);
		match register_res {
			Ok(_) => {}
			Err(err) => {
				tracing::error!(?err, "failed to register proxied port route")
			}
		}
	}

	tracing::info!(?config, "config timeeee3");

	config.http.middlewares.insert(
		"ds-rate-limit".to_owned(),
		types::TraefikMiddlewareHttp::RateLimit {
			average: 100,
			period: "5m".into(),
			burst: 256,
			source_criterion: types::InFlightReqSourceCriterion::IpStrategy(types::IpStrategy {
				depth: 0,
				exclude_ips: None,
			}),
		},
	);
	config.http.middlewares.insert(
		"ds-in-flight".to_owned(),
		types::TraefikMiddlewareHttp::InFlightReq {
			// This number needs to be high to allow for parallel requests
			amount: 4,
			source_criterion: types::InFlightReqSourceCriterion::IpStrategy(types::IpStrategy {
				depth: 0,
				exclude_ips: None,
			}),
		},
	);

	tracing::info!(?config, "config timeeee");

	// TODO: add middleware & services & ports
	// TODO: same as jobs, watch out for namespaces
	Ok(())
}

#[tracing::instrument(skip(config))]
fn ds_register_proxied_port(
	run_id: Uuid,
	proxied_port: &DynamicServer,
	config: &mut types::TraefikConfigResponse,
) -> GlobalResult<()> {
	let ingress_port = proxied_port.gg_port.clone();
	let target_nomad_port_label = proxied_port.label.clone();
	let service_id = format!("ds-run:{}:{}", run_id, target_nomad_port_label);
	let proxy_protocol = unwrap!(backend::ds::GameGuardProtocol::from_i32(
		proxied_port.protocol as i32
	));

	// Insert the relevant service
	match proxy_protocol {
		backend::ds::GameGuardProtocol::Http | backend::ds::GameGuardProtocol::Https => {
			config.http.services.insert(
				service_id.clone(),
				types::TraefikService {
					load_balancer: types::TraefikLoadBalancer {
						servers: vec![types::TraefikServer {
							url: Some(format!(
								"http://{}:{}",
								proxied_port.nomad_ip, proxied_port.nomad_source
							)),
							address: None,
						}],
						sticky: None,
					},
				},
			);
		}
		backend::ds::GameGuardProtocol::Tcp | backend::ds::GameGuardProtocol::TcpTls => {
			config.tcp.services.insert(
				service_id.clone(),
				types::TraefikService {
					load_balancer: types::TraefikLoadBalancer {
						servers: vec![types::TraefikServer {
							url: None,
							address: Some(format!(
								"{}:{}",
								proxied_port.nomad_ip, proxied_port.nomad_source
							)),
						}],
						sticky: None,
					},
				},
			);
		}
		backend::ds::GameGuardProtocol::Udp => {
			config.udp.services.insert(
				service_id.clone(),
				types::TraefikService {
					load_balancer: types::TraefikLoadBalancer {
						servers: vec![types::TraefikServer {
							url: None,
							address: Some(format!(
								"{}:{}",
								proxied_port.nomad_ip, proxied_port.nomad_source
							)),
						}],
						sticky: None,
					},
				},
			);
		}
	};

	// Insert the relevant router
	match proxy_protocol {
		backend::ds::GameGuardProtocol::Http => {
			// Generate config
			let middlewares = http_router_middlewares();
			let rule = format_http_rule(proxied_port)?;

			// Hash key
			let unique_key = (&run_id, &target_nomad_port_label, &rule, &middlewares);
			let mut hasher = DefaultHasher::new();
			unique_key.hash(&mut hasher);
			let hash = hasher.finish();

			config.http.routers.insert(
				format!("ds-run:{run_id}:{hash:x}:http"),
				types::TraefikRouter {
					entry_points: vec![format!("lb-{ingress_port}")],
					rule: Some(rule),
					priority: None,
					service: service_id.clone(),
					middlewares,
					tls: None,
				},
			);
		}
		backend::ds::GameGuardProtocol::Https => {
			// Generate config
			let middlewares = http_router_middlewares();
			let rule = format_http_rule(proxied_port)?;

			// Hash key
			let unique_key = (&run_id, &target_nomad_port_label, &rule, &middlewares);
			let mut hasher = DefaultHasher::new();
			unique_key.hash(&mut hasher);
			let hash = hasher.finish();

			config.http.routers.insert(
				format!("ds-run:{run_id}:{hash:x}:https"),
				types::TraefikRouter {
					entry_points: vec![format!("lb-{ingress_port}")],
					rule: Some(rule),
					priority: None,
					service: service_id.clone(),
					middlewares,
					tls: Some(types::TraefikTls::build(build_tls_domains(proxied_port)?)),
				},
			);
		}
		backend::ds::GameGuardProtocol::Tcp => {
			config.tcp.routers.insert(
				format!("ds-run:{}:{}:tcp", run_id, target_nomad_port_label),
				types::TraefikRouter {
					entry_points: vec![format!("lb-{ingress_port}-tcp")],
					rule: Some("HostSNI(`*`)".into()),
					priority: None,
					service: service_id,
					middlewares: vec![],
					tls: None,
				},
			);
		}
		backend::ds::GameGuardProtocol::TcpTls => {
			config.tcp.routers.insert(
				format!("ds-run:{}:{}:tcp-tls", run_id, target_nomad_port_label),
				types::TraefikRouter {
					entry_points: vec![format!("lb-{ingress_port}-tcp")],
					rule: Some("HostSNI(`*`)".into()),
					priority: None,
					service: service_id,
					middlewares: vec![],
					tls: Some(types::TraefikTls::build(build_tls_domains(proxied_port)?)),
				},
			);
		}
		backend::ds::GameGuardProtocol::Udp => {
			config.udp.routers.insert(
				format!("ds-run:{}:{}:udp", run_id, target_nomad_port_label),
				types::TraefikRouter {
					entry_points: vec![format!("lb-{ingress_port}-udp")],
					rule: None,
					priority: None,
					service: service_id,
					middlewares: vec![],
					tls: None,
				},
			);
		}
	}

	Ok(())
}

fn format_http_rule(proxied_port: &DynamicServer) -> GlobalResult<String> {
	Ok(format!("Host(`{}`)", proxied_port.hostname()?))
}

fn build_tls_domains(proxied_port: &DynamicServer) -> GlobalResult<Vec<types::TraefikTlsDomain>> {
	// Derive TLS config. Jobs can specify their own ingress rules, so we
	// need to derive which domains to use for the job.
	//
	// A parent wildcard SSL mode will use the parent domain as the SSL
	// name.
	let mut domains = Vec::new();
	let parent_host = proxied_port.parent_host()?;
	domains.push(types::TraefikTlsDomain {
		main: parent_host.to_owned(),
		sans: vec![format!("*.{}", parent_host)],
	});

	Ok(domains)
}

fn http_router_middlewares() -> Vec<String> {
	let middlewares = vec!["ds-rate-limit".to_string(), "ds-in-flight".to_string()];

	middlewares
}
