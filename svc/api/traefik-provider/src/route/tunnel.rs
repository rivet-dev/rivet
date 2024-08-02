use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{auth::Auth, types};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigQuery {
	token: String,
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
	let servers_res = ctx
		.op(cluster::ops::server::list::Input {
			filter: cluster::types::Filter {
				pool_types: Some(vec![cluster::types::PoolType::Gg]),
				..Default::default()
			},
			include_destroyed: false,
		})
		.await?;

	let public_ips = servers_res
		.servers
		.iter()
		.filter_map(|server| server.public_ip)
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
