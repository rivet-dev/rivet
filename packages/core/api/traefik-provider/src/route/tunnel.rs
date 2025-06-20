use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{auth::Auth, types};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigQuery {
	token: Option<String>,
}

#[tracing::instrument(skip(ctx))]
pub async fn config(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	ConfigQuery { token }: ConfigQuery,
) -> GlobalResult<types::TraefikConfigResponseNullified> {
	ctx.auth().token(&token).await?;

	let mut config = types::TraefikConfigResponse::default();

	build_ip_allowlist(&ctx, &mut config).await?;

	tracing::info!(
		services = ?config.tcp.services.len(),
		routers = config.tcp.routers.len(),
		middlewares = ?config.tcp.middlewares.len(),
		"tunnel traefik config"
	);

	Ok(types::TraefikConfigResponseNullified {
		http: config.http.nullified(),
		tcp: config.tcp.nullified(),
		udp: config.udp.nullified(),
	})
}

/// Builds configuration for GG edge node routes.
#[tracing::instrument(skip(ctx))]
pub async fn build_ip_allowlist(
	ctx: &Ctx<Auth>,
	config: &mut types::TraefikConfigResponse,
) -> GlobalResult<()> {
	let servers = ctx
		.cache()
		.ttl(5000)
		.fetch_one_json("cluster.guard_ip_allow_list", "", {
			let ctx = (*ctx).clone();
			move |mut cache, key| {
				let ctx = ctx.clone();
				async move {
					let servers_res = ctx
						.op(cluster::ops::server::list::Input {
							filter: cluster::types::Filter {
								pool_types: Some(vec![
									cluster::types::PoolType::Gg,
									cluster::types::PoolType::Guard,
								]),
								..Default::default()
							},
							include_destroyed: false,
							// IMPORTANT: Returns installing servers
							exclude_installing: false,
							exclude_draining: true,
							exclude_no_vlan: true,
						})
						.await?;

					cache.resolve(&key, servers_res.servers);

					Ok(cache)
				}
			}
		})
		.await?;

	let public_ips = servers
		.iter()
		.flatten()
		.filter_map(|server| server.wan_ip)
		.map(|ip| ip.to_string())
		.collect::<Vec<_>>();

	config.tcp.middlewares.insert(
		"tunnel-ip-allowlist".to_owned(),
		types::TraefikMiddlewareHttp::IpAllowList {
			source_range: public_ips,
			ip_strategy: None,
		},
	);

	Ok(())
}
