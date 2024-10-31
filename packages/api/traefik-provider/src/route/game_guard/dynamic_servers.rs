use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
	fmt::Write,
};

use api_helper::ctx::Ctx;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use ds::types::{PortAuthorizationType, PortAuthorization, GameGuardProtocol};

use crate::{auth::Auth, types};

#[derive(sqlx::FromRow, Clone, Debug, Serialize, Deserialize)]
struct DynamicServerProxiedPort {
	server_id: Uuid,
	datacenter_id: Uuid,
	
	label: String,
	ip: String,

	source: i64,
	gg_port: i64,
	port_name: String,
	protocol: i64,

	auth_type: Option<i64>,
	auth_key: Option<String>,
	auth_value: Option<String>,
}

impl DynamicServerProxiedPort {
	fn parent_host(&self, config: &rivet_config::Config) -> GlobalResult<String> {
		Ok(format!(
			"lobby.{}.{}",
			self.datacenter_id,
			unwrap_ref!(config.server()?.rivet.dns()?.domain_job),
		))
	}

	fn hostname(&self, config: &rivet_config::Config) -> GlobalResult<String> {
		ds::util::build_ds_hostname(config, self.server_id, &self.port_name, self.datacenter_id)
	}
}

pub async fn build_ds(
	ctx: &Ctx<Auth>,
	dc_id: Uuid,
	config: &mut types::TraefikConfigResponse,
) -> GlobalResult<()> {
	let proxied_ports = ctx
		.cache()
		.ttl(60_000)
		.fetch_one_json("ds_proxied_ports", dc_id, |mut cache, dc_id| async move {
			let rows = sql_fetch_all!(
				[ctx, DynamicServerProxiedPort]
				"
				SELECT
					s.server_id,
					s.datacenter_id,
					pp.label,
					pp.ip,
					pp.source,
					gg.gg_port,
					gg.port_name,
					gg.protocol,
					gga.auth_type,
					gga.key AS auth_key,
					gga.value AS auth_value
				FROM db_ds.server_proxied_ports AS pp
				JOIN db_ds.servers AS s
				ON pp.server_id = s.server_id
				JOIN db_ds.server_ports_gg AS gg
				ON
					pp.server_id = gg.server_id AND
					pp.label = CONCAT('ds_', REPLACE(gg.port_name, '-', '_'))
				LEFT JOIN db_ds.server_ports_gg_auth AS gga
				ON
					gg.server_id = gga.server_id AND
					gg.port_name = gga.port_name
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

	// Process proxied ports
	for proxied_port in &proxied_ports {
		if let Err(err) = ds_register_proxied_port(ctx.config(), proxied_port, config) {
			tracing::error!(?err, "failed to register proxied port")
		}
	}

	tracing::info!(
		http_services = ?config.http.services.len(),
		http_routers = ?config.http.routers.len(),
		http_middlewares = ?config.http.middlewares.len(),
		tcp_services = ?config.tcp.services.len(),
		tcp_routers = ?config.tcp.routers.len(),
		tcp_middlewares = ?config.tcp.middlewares.len(),
		udp_services = ?config.udp.services.len(),
		udp_routers = ?config.udp.routers.len(),
		udp_middlewares = ?config.udp.middlewares.len(),
		"dynamic servers traefik config"
	);

	Ok(())
}

#[tracing::instrument(skip(config))]
fn ds_register_proxied_port(
	config: &rivet_config::Config,
	proxied_port: &DynamicServerProxiedPort,
	traefik_config: &mut types::TraefikConfigResponse,
) -> GlobalResult<()> {
	let ingress_port = proxied_port.gg_port;
	let server_id = proxied_port.server_id;
	let target_port_label = proxied_port.label.clone();
	let service_id = format!("ds:{server_id}:{target_port_label}");
	let proxy_protocol = unwrap!(GameGuardProtocol::from_repr(
		proxied_port.protocol.try_into()?
	));

	// Insert the relevant service
	match proxy_protocol {
		GameGuardProtocol::Http | GameGuardProtocol::Https => {
			traefik_config.http.services.insert(
				service_id.clone(),
				types::TraefikService {
					load_balancer: types::TraefikLoadBalancer {
						servers: vec![types::TraefikServer {
							url: Some(format!(
								"http://{}:{}",
								proxied_port.ip, proxied_port.source
							)),
							address: None,
						}],
						sticky: None,
					},
				},
			);
		}
		GameGuardProtocol::Tcp | GameGuardProtocol::TcpTls => {
			traefik_config.tcp.services.insert(
				service_id.clone(),
				types::TraefikService {
					load_balancer: types::TraefikLoadBalancer {
						servers: vec![types::TraefikServer {
							url: None,
							address: Some(format!("{}:{}", proxied_port.ip, proxied_port.source)),
						}],
						sticky: None,
					},
				},
			);
		}
		GameGuardProtocol::Udp => {
			traefik_config.udp.services.insert(
				service_id.clone(),
				types::TraefikService {
					load_balancer: types::TraefikLoadBalancer {
						servers: vec![types::TraefikServer {
							url: None,
							address: Some(format!("{}:{}", proxied_port.ip, proxied_port.source)),
						}],
						sticky: None,
					},
				},
			);
		}
	};

	// Insert the relevant router
	match proxy_protocol {
		GameGuardProtocol::Http => {
			// Generate config
			let middlewares = http_router_middlewares();
			let rule = format_http_rule(config, proxied_port)?;

			// Hash key
			let unique_key = (&server_id, &target_port_label, &rule, &middlewares);
			let mut hasher = DefaultHasher::new();
			unique_key.hash(&mut hasher);
			let hash = hasher.finish();

			traefik_config.http.routers.insert(
				format!("ds:{server_id}:{hash:x}:http"),
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
		GameGuardProtocol::Https => {
			// Generate config
			let middlewares = http_router_middlewares();
			let rule = format_http_rule(config, proxied_port)?;

			// Hash key
			let unique_key = (&server_id, &target_port_label, &rule, &middlewares);
			let mut hasher = DefaultHasher::new();
			unique_key.hash(&mut hasher);
			let hash = hasher.finish();

			traefik_config.http.routers.insert(
				format!("ds:{server_id}:{hash:x}:https"),
				types::TraefikRouter {
					entry_points: vec![format!("lb-{ingress_port}")],
					rule: Some(rule),
					priority: None,
					service: service_id.clone(),
					middlewares,
					tls: Some(types::TraefikTls::build(build_tls_domains(
						config,
						proxied_port,
					)?)),
				},
			);
		}
		GameGuardProtocol::Tcp => {
			traefik_config.tcp.routers.insert(
				format!("ds:{}:{}:tcp", server_id, target_port_label),
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
		GameGuardProtocol::TcpTls => {
			traefik_config.tcp.routers.insert(
				format!("ds:{}:{}:tcp-tls", server_id, target_port_label),
				types::TraefikRouter {
					entry_points: vec![format!("lb-{ingress_port}-tcp")],
					rule: Some("HostSNI(`*`)".into()),
					priority: None,
					service: service_id,
					middlewares: vec![],
					tls: Some(types::TraefikTls::build(build_tls_domains(
						config,
						proxied_port,
					)?)),
				},
			);
		}
		GameGuardProtocol::Udp => {
			traefik_config.udp.routers.insert(
				format!("ds:{}:{}:udp", server_id, target_port_label),
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

fn format_http_rule(
	config: &rivet_config::Config,
	proxied_port: &DynamicServerProxiedPort,
) -> GlobalResult<String> {
	let authorization = {
		let authorization_type = if let Some(auth_type) = proxied_port.auth_type {
			unwrap!(PortAuthorizationType::from_repr(auth_type.try_into()?))
		} else {
			PortAuthorizationType::None
		};

		match authorization_type {
			PortAuthorizationType::None => PortAuthorization::None,
			PortAuthorizationType::Bearer => {
				PortAuthorization::Bearer(unwrap!(proxied_port.auth_value.clone()))
			}
			PortAuthorizationType::Query => PortAuthorization::Query(
				unwrap!(proxied_port.auth_key.clone()),
				unwrap!(proxied_port.auth_value.clone()),
			),
		}
	};

	let mut rule = "(".to_string();
	
	write!(&mut rule, "Host(`{}`)", proxied_port.hostname(config)?)?;

	match authorization {
		PortAuthorization::None => {}
		PortAuthorization::Bearer(token) => {
			write!(&mut rule, "&& Header(`Authorization`, `Bearer {}`)", escape_input(&token))?;
		}
		PortAuthorization::Query(key, value) => {
			write!(&mut rule, "&& Query(`{}`, `{}`)", escape_input(&key), escape_input(&value))?;
		}
	}

	rule.push_str(")");

	Ok(rule)
}

fn build_tls_domains(
	config: &rivet_config::Config,
	proxied_port: &DynamicServerProxiedPort,
) -> GlobalResult<Vec<types::TraefikTlsDomain>> {
	// Derive TLS config. Jobs can specify their own ingress rules, so we
	// need to derive which domains to use for the job.
	//
	// A parent wildcard SSL mode will use the parent domain as the SSL
	// name.
	let mut domains = Vec::new();
	let parent_host = proxied_port.parent_host(config)?;
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

fn escape_input(input: &str) -> String {
	input.replace("`", "\\`")
}
