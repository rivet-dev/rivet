use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
};

use api_helper::ctx::Ctx;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};

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
		ds::util::build_ds_hostname(self.server_id, &self.port_name, self.datacenter_id)
	}
}

pub async fn build_ds(
	ctx: &Ctx<Auth>,
	dc_id: Uuid,
	config: &mut types::TraefikConfigResponse,
) -> GlobalResult<()> {
	let dynamic_servers = ctx
		.cache()
		.ttl(60)
		.fetch_one_json("servers_ports", dc_id, |mut cache, dc_id| async move {
			let rows = sql_fetch_all!(
				[ctx, DynamicServer]
				"
				SELECT
					s.server_id,
					s.datacenter_id,
					ip.nomad_label AS label,
					ip.nomad_ip,
					ip.nomad_source,
					gg.port_number,
					gg.gg_port,
					gg.port_name,
					gg.protocol
				FROM db_ds.internal_ports AS ip
				JOIN db_ds.servers AS s
				ON ip.server_id = s.server_id
				JOIN db_ds.docker_ports_protocol_game_guard AS gg
				ON
					ip.server_id = gg.server_id AND
					ip.nomad_label = CONCAT('ds_', gg.port_name)
				WHERE
					s.datacenter_id = $1 AND
					s.destroy_ts IS NULL
				",
				dc_id
			)
			.await?;
			cache.resolve(&dc_id, rows);

			Ok(cache)
		})
		.await?
		.unwrap_or_default();

	// Process proxied ports
	for dynamic_server in &dynamic_servers {
		let server_id = dynamic_server.server_id;
		let register_res = ds_register_proxied_port(server_id, dynamic_server, config);
		match register_res {
			Ok(_) => {}
			Err(err) => {
				tracing::error!(?err, "failed to register proxied port route")
			}
		}
	}

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
	let ingress_port = proxied_port.gg_port;
	let target_nomad_port_label = proxied_port.label.clone();
	let service_id = format!("ds-run:{}:{}", run_id, target_nomad_port_label);
	let proxy_protocol = unwrap!(ds::types::GameGuardProtocol::from_repr(
		proxied_port.protocol.try_into()?
	));

	// Insert the relevant service
	match proxy_protocol {
		ds::types::GameGuardProtocol::Http | ds::types::GameGuardProtocol::Https => {
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
		ds::types::GameGuardProtocol::Tcp | ds::types::GameGuardProtocol::TcpTls => {
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
		ds::types::GameGuardProtocol::Udp => {
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
		ds::types::GameGuardProtocol::Http => {
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
		ds::types::GameGuardProtocol::Https => {
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
		ds::types::GameGuardProtocol::Tcp => {
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
		ds::types::GameGuardProtocol::TcpTls => {
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
		ds::types::GameGuardProtocol::Udp => {
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
