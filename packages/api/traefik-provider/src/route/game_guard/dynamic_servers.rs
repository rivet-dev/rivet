use api_helper::ctx::Ctx;
use cluster::types::GuardPublicHostname;
use ds::{
	types::EndpointType,
	types::{GameGuardProtocol, PortAuthorization, PortAuthorizationType},
};
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
	collections::hash_map::DefaultHasher,
	fmt::Write,
	hash::{Hash, Hasher},
};

use crate::{auth::Auth, types};

#[derive(sqlx::FromRow, Clone, Debug, Serialize, Deserialize)]
struct DynamicServerProxiedPort {
	server_id: Uuid,
	datacenter_id: Uuid,
	create_ts: i64,
	guard_public_hostname_dns_parent: Option<String>,
	guard_public_hostname_static: Option<String>,

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

pub async fn build_ds(
	ctx: &Ctx<Auth>,
	dc_id: Uuid,
	server_id: Option<Uuid>,
	config: &mut types::TraefikConfigResponse,
) -> GlobalResult<Option<i64>> {
	let proxied_ports = ctx
		.cache()
		.ttl(60_000)
		.fetch_one_json("ds_proxied_ports2", dc_id, |mut cache, dc_id| async move {
			let rows = sql_fetch_all!(
				[ctx, DynamicServerProxiedPort]
				"
				SELECT
					s.server_id,
					s.datacenter_id,
					s.create_ts,
					dc.guard_public_hostname_dns_parent,
					dc.guard_public_hostname_static,
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
				JOIN db_cluster.datacenters AS dc
				ON s.datacenter_id = dc.datacenter_id
				JOIN db_ds.server_ports_gg AS gg
				ON
					pp.server_id = gg.server_id AND
					pp.label = gg.port_name
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

	let latest_ds_create_ts = proxied_ports.iter().map(|pp| pp.create_ts).max();

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

	// TODO(RVT-4349, RVT-4172): Retry requests in case the actor's server has not started yet
	config.http.middlewares.insert(
		"ds-retry".to_owned(),
		types::TraefikMiddlewareHttp::Retry {
			attempts: 4,
			initial_interval: "250ms".into(),
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

	Ok(latest_ds_create_ts)
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
	let guard_public_hostname = GuardPublicHostname::from_columns(
		config,
		proxied_port.datacenter_id,
		proxied_port.guard_public_hostname_dns_parent.clone(),
		proxied_port.guard_public_hostname_static.clone(),
	)?;

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
			add_http_port(
				config,
				proxied_port,
				traefik_config,
				&service_id,
				&guard_public_hostname,
				false,
			)?;
		}
		GameGuardProtocol::Https => {
			add_http_port(
				config,
				proxied_port,
				traefik_config,
				&service_id,
				&guard_public_hostname,
				true,
			)?;
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
						&guard_public_hostname,
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

fn add_http_port(
	config: &rivet_config::Config,
	proxied_port: &DynamicServerProxiedPort,
	traefik_config: &mut types::TraefikConfigResponse,
	service_id: &str,
	guard_public_hostname: &GuardPublicHostname,
	is_https: bool,
) -> GlobalResult<()> {
	// Choose endpoint types to expose routes for
	let supported_endpoint_types = match guard_public_hostname {
		GuardPublicHostname::DnsParent(_) => vec![EndpointType::Hostname, EndpointType::Path],
		GuardPublicHostname::Static(_) => vec![EndpointType::Path],
	};

	// Add routes for each endpoint type
	for endpoint_type in supported_endpoint_types {
		let (hostname, path) = ds::util::build_ds_hostname_and_path(
			proxied_port.server_id,
			&proxied_port.port_name,
			if is_https {
				GameGuardProtocol::Https
			} else {
				GameGuardProtocol::Http
			},
			endpoint_type,
			guard_public_hostname,
		)?;

		let mut middlewares = vec![
			"ds-rate-limit".to_string(),
			"ds-in-flight".to_string(),
			"ds-retry".to_string(),
		];
		let rule = format_http_rule(
			proxied_port,
			&hostname,
			proxied_port.gg_port,
			path.as_deref(),
		)?;

		// Create unique hash to prevent collision with other ports
		let unique_key = (&proxied_port.server_id, &proxied_port.label, &rule);
		let mut hasher = DefaultHasher::new();
		unique_key.hash(&mut hasher);
		let hash = hasher.finish();

		// Strip path
		if let Some(path) = path {
			let mw_name = format!("ds:{}:{hash:x}:strip-path", proxied_port.server_id);
			traefik_config.http.middlewares.insert(
				mw_name.clone(),
				types::TraefikMiddlewareHttp::StripPrefix {
					prefixes: vec![path],
				},
			);
			middlewares.push(mw_name);
		}

		// Build router
		let proto = if is_https { "https" } else { "http" };

		traefik_config.http.routers.insert(
			format!("ds:{}:{hash:x}:{proto}", proxied_port.server_id),
			types::TraefikRouter {
				entry_points: vec![format!("lb-{}", proxied_port.gg_port)],
				rule: Some(rule),
				priority: None,
				service: service_id.to_string(),
				middlewares,
				tls: if is_https {
					Some(types::TraefikTls::build(build_tls_domains(
						&guard_public_hostname,
					)?))
				} else {
					None
				},
			},
		);
	}

	Ok(())
}

fn format_http_rule(
	proxied_port: &DynamicServerProxiedPort,
	hostname: &str,
	port: i64,
	path: Option<&str>,
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

	match (hostname, path) {
		(hostname, Some(path)) => {
			// Matches both the host without the port (i.e. default port like
			// port 80 or 443) and host with the port.
			//
			// Matches both the path without trailing slash (e.g. `/foo`) and subpaths (e.g. `/foo/bar`), but not `/foobar`.
			write!(&mut rule, "(Host(`{hostname}`) || Host(`{hostname}:{port}`)) && (Path(`{path}`) || PathPrefix(`{path}/`))")?;
		}
		(hostname, None) => {
			write!(&mut rule, "Host(`{hostname}`)")?;
		}
	}

	match authorization {
		PortAuthorization::None => {}
		PortAuthorization::Bearer(token) => {
			write!(
				&mut rule,
				"&& Header(`Authorization`, `Bearer {}`)",
				escape_input(&token)
			)?;
		}
		PortAuthorization::Query(key, value) => {
			write!(
				&mut rule,
				"&& Query(`{}`, `{}`)",
				escape_input(&key),
				escape_input(&value)
			)?;
		}
	}

	rule.push(')');

	Ok(rule)
}

fn build_tls_domains(
	guard_public_hostname: &GuardPublicHostname,
) -> GlobalResult<Vec<types::TraefikTlsDomain>> {
	let (main, sans) = match guard_public_hostname {
		GuardPublicHostname::DnsParent(parent) => (parent.clone(), vec![format!("*.{parent}")]),
		// This will only work if there is an SSL cert provided for the exact name of the static
		// DNS address.
		//
		// This will not work if passing an IP address.
		GuardPublicHostname::Static(static_) => (static_.clone(), vec![static_.clone()]),
	};

	// Derive TLS config. Jobs can specify their own ingress rules, so we
	// need to derive which domains to use for the job.
	//
	// A parent wildcard SSL mode will use the parent domain as the SSL
	// name.
	let mut domains = Vec::new();
	domains.push(types::TraefikTlsDomain { main, sans });

	Ok(domains)
}

fn escape_input(input: &str) -> String {
	input.replace("`", "\\`")
}
