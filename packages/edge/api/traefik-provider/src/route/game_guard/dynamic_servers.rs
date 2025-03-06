use api_core_traefik_provider::types;
use api_helper::ctx::Ctx;
use cluster::types::GuardPublicHostname;
use ds::types::{EndpointType, GameGuardProtocol};
use fdb_util::{FormalKey, SNAPSHOT};
use foundationdb::{self as fdb, options::StreamingMode};
use futures_util::{StreamExt, TryStreamExt};
use rivet_operation::prelude::*;
use std::{
	collections::hash_map::DefaultHasher,
	fmt::Write,
	hash::{Hash, Hasher},
};

use crate::auth::Auth;

pub async fn build_ds(
	ctx: &Ctx<Auth>,
	server_id: Option<Uuid>,
	config: &mut types::TraefikConfigResponse,
) -> GlobalResult<Option<i64>> {
	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;

	let (dc_res, proxied_ports) = tokio::try_join!(
		ctx.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![dc_id],
		}),
		async move {
			ctx.fdb()
				.await?
				.run(|tx, _mc| async move {
					let proxied_ports_subspace = ds::keys::subspace()
						.subspace(&ds::keys::server::ProxiedPortsKey::subspace());

					tx.get_ranges_keyvalues(
						fdb::RangeOption {
							mode: StreamingMode::WantAll,
							..(&proxied_ports_subspace).into()
						},
						// NOTE: This is not SERIALIZABLE because we don't want to conflict with port updates
						// and its not important if its slightly stale
						SNAPSHOT,
					)
					.map(|res| match res {
						Ok(entry) => {
							let proxied_ports_key = ds::keys::subspace()
								.unpack::<ds::keys::server::ProxiedPortsKey>(entry.key())
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

							Ok(futures_util::stream::iter(
								proxied_ports_key
									.deserialize(entry.value())
									.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
							)
							.map(move |pp| Ok((proxied_ports_key.server_id, pp))))
						}
						Err(err) => Err(Into::<fdb::FdbBindingError>::into(err)),
					})
					.try_flatten()
					.try_collect::<Vec<(Uuid, ds::keys::server::ProxiedPort)>>()
					.await
				})
				.await
				.map_err(Into::into)
		}
	)?;

	let dc = unwrap!(dc_res.datacenters.first());
	let latest_ds_create_ts = proxied_ports.iter().map(|(_, pp)| pp.create_ts).max();

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
	for (server_id, proxied_port) in &proxied_ports {
		if let Err(err) = ds_register_proxied_port(*server_id, proxied_port, dc, config) {
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

#[tracing::instrument]
fn ds_register_proxied_port(
	server_id: Uuid,
	proxied_port: &ds::keys::server::ProxiedPort,
	dc: &cluster::types::Datacenter,
	traefik_config: &mut types::TraefikConfigResponse,
) -> GlobalResult<()> {
	let ingress_port = proxied_port.ingress_port_number;
	let server_id = server_id;
	let target_port_name = proxied_port.port_name.clone();
	let service_id = format!("ds:{server_id}:{target_port_name}");

	// Insert the relevant service
	match proxied_port.protocol {
		GameGuardProtocol::Http | GameGuardProtocol::Https => {
			traefik_config.http.services.insert(
				service_id.clone(),
				types::TraefikService {
					load_balancer: types::TraefikLoadBalancer {
						servers: vec![types::TraefikServer {
							url: Some(format!(
								"http://{}:{}",
								proxied_port.lan_hostname, proxied_port.source
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
							address: Some(format!(
								"{}:{}",
								proxied_port.lan_hostname, proxied_port.source
							)),
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
							address: Some(format!(
								"{}:{}",
								proxied_port.lan_hostname, proxied_port.source
							)),
						}],
						sticky: None,
					},
				},
			);
		}
	};

	// Insert the relevant router
	match proxied_port.protocol {
		GameGuardProtocol::Http => {
			add_http_port(
				server_id,
				proxied_port,
				traefik_config,
				&service_id,
				&dc.guard_public_hostname,
				false,
			)?;
		}
		GameGuardProtocol::Https => {
			add_http_port(
				server_id,
				proxied_port,
				traefik_config,
				&service_id,
				&dc.guard_public_hostname,
				true,
			)?;
		}
		GameGuardProtocol::Tcp => {
			traefik_config.tcp.routers.insert(
				format!("ds:{}:{}:tcp", server_id, target_port_name),
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
				format!("ds:{}:{}:tcp-tls", server_id, target_port_name),
				types::TraefikRouter {
					entry_points: vec![format!("lb-{ingress_port}-tcp")],
					rule: Some("HostSNI(`*`)".into()),
					priority: None,
					service: service_id,
					middlewares: vec![],
					tls: Some(types::TraefikTls::build(build_tls_domains(
						&dc.guard_public_hostname,
					)?)),
				},
			);
		}
		GameGuardProtocol::Udp => {
			traefik_config.udp.routers.insert(
				format!("ds:{}:{}:udp", server_id, target_port_name),
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
	server_id: Uuid,
	proxied_port: &ds::keys::server::ProxiedPort,
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
			server_id,
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
		let rule = format_http_rule(&hostname, proxied_port.ingress_port_number, path.as_deref())?;

		// Create unique hash to prevent collision with other ports
		let unique_key = (&server_id, &proxied_port.port_name, &rule);
		let mut hasher = DefaultHasher::new();
		unique_key.hash(&mut hasher);
		let hash = hasher.finish();

		// Strip path
		if let Some(path) = path {
			let mw_name = format!("ds:{}:{hash:x}:strip-path", server_id);
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
			format!("ds:{}:{hash:x}:{proto}", server_id),
			types::TraefikRouter {
				entry_points: vec![format!("lb-{}", proxied_port.ingress_port_number)],
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

fn format_http_rule(hostname: &str, port: u16, path: Option<&str>) -> GlobalResult<String> {
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
