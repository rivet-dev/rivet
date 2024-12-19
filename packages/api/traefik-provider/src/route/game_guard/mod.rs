use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use dynamic_servers::build_ds;
use job::build_job;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{auth::Auth, types};

pub mod dynamic_servers;
pub mod job;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigQuery {
	token: Option<String>,
	datacenter: Uuid,
	server: Option<Uuid>,
}

#[tracing::instrument(skip(ctx))]
pub async fn config(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	ConfigQuery {
		token,
		datacenter,
		server,
	}: ConfigQuery,
) -> GlobalResult<types::TraefikConfigResponseNullified> {
	ctx.auth().token(&token).await?;

	// Fetch configs and catch any errors
	let mut config = types::TraefikConfigResponse::default();
	build_job(&ctx, datacenter, &mut config).await?;
	let latest_ds_create_ts = build_ds(&ctx, datacenter, server, &mut config).await?;

	// Publish message when the request is complete
	if let Some(latest_ds_create_ts) = latest_ds_create_ts {
		ctx.msg(ds::workflows::server::pegboard::TraefikPoll {
			server_id: server,
			latest_ds_create_ts,
		})
		.tag("datacenter_id", datacenter)
		.send()
		.await?;
	}

	tracing::debug!(
		http_services = ?config.http.services.len(),
		http_routers = config.http.routers.len(),
		http_middlewares = ?config.http.middlewares.len(),
		tcp_services = ?config.tcp.services.len(),
		tcp_routers = config.tcp.routers.len(),
		tcp_middlewares = ?config.tcp.middlewares.len(),
		udp_services = ?config.udp.services.len(),
		udp_routers = config.udp.routers.len(),
		udp_middlewares = ?config.udp.middlewares.len(),
		"traefik config"
	);

	Ok(types::TraefikConfigResponseNullified {
		http: config.http.nullified(),
		tcp: config.tcp.nullified(),
		udp: config.udp.nullified(),
	})
}
