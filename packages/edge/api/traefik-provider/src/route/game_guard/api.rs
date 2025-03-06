use api_core_traefik_provider::types;
use api_helper::ctx::Ctx;
use cluster::types::{Filter, PoolType};
use rivet_operation::prelude::*;

use crate::auth::Auth;

#[tracing::instrument(skip_all)]
pub async fn build_api(
	ctx: &Ctx<Auth>,
	config: &mut types::TraefikConfigResponse,
) -> GlobalResult<()> {
	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;

	let (dc_res, servers_res) = tokio::try_join!(
		ctx.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![dc_id],
		}),
		ctx.op(cluster::ops::server::list::Input {
			filter: Filter {
				datacenter_ids: Some(vec![dc_id]),
				pool_types: Some(vec![PoolType::Worker]),
				..Default::default()
			},
			include_destroyed: false,
			exclude_draining: true,
			exclude_no_vlan: true,
		}),
	)?;
	let dc = unwrap!(dc_res.datacenters.first());

	let mut middlewares = vec![];
	let service_id = "api".to_string();

	let port = ctx.config().server()?.rivet.api_public.port();
	config.http.services.insert(
		service_id.clone(),
		types::TraefikService {
			load_balancer: types::TraefikLoadBalancer {
				servers: servers_res
					.servers
					.iter()
					.map(|server| {
						Ok(types::TraefikServer {
							url: Some(format!("http://{}:{port}", unwrap!(server.lan_ip),)),
							address: None,
						})
					})
					.collect::<GlobalResult<_>>()?,
				sticky: None,
			},
		},
	);

	config.http.middlewares.insert(
		"api-compress".to_owned(),
		types::TraefikMiddlewareHttp::Compress {},
	);
	middlewares.push("api-compress".to_string());

	config.http.middlewares.insert(
		"api-in-flight".to_owned(),
		types::TraefikMiddlewareHttp::InFlightReq {
			// This number needs to be high to allow for parallel requests
			amount: 64,
			// TODO: Different strat?
			source_criterion: types::InFlightReqSourceCriterion::IpStrategy(types::IpStrategy {
				depth: 0,
				exclude_ips: None,
			}),
		},
	);
	middlewares.push("api-in-flight".to_string());

	let url = ctx.config().server()?.rivet.edge_api_url(&dc.name_id)?;
	let host = unwrap!(url.host());
	let rule = format!("Host(`{host}`)");

	let tls_domain = types::TraefikTlsDomain {
		main: host.to_string(),
		sans: Vec::new(),
	};

	config.http.routers.insert(
		"api-secure".to_string(),
		types::TraefikRouter {
			entry_points: vec!["lb-443".to_string()],
			rule: Some(rule),
			priority: None,
			service: service_id.to_string(),
			middlewares,
			tls: Some(types::TraefikTls::build(vec![tls_domain])),
		},
	);

	Ok(())
}
