use api::build_api;
use api_core_traefik_provider::types;
use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use actor::build_actor;
use job::build_job;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};

use crate::auth::Auth;

pub mod api;
pub mod actor;
pub mod job;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigQuery {
	token: Option<String>,
	server: Option<Uuid>,
}

#[tracing::instrument(skip(ctx))]
pub async fn config(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	ConfigQuery { token, server }: ConfigQuery,
) -> GlobalResult<types::TraefikConfigResponseNullified> {
	ctx.auth().token(&token).await?;

	// Fetch configs and catch any errors
	let mut config = types::TraefikConfigResponse::default();
	build_job(&ctx, &mut config).await?;
	let latest_actor_create_ts = build_actor(&ctx, &mut config).await?;

	build_api(&ctx, &mut config).await?;

	// Publish message when the request is complete
	if let Some(latest_actor_create_ts) = latest_actor_create_ts {
		let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;
		ctx.msg(pegboard::workflows::actor::TraefikPoll {
			server_id: server,
			latest_actor_create_ts,
		})
		.tag("datacenter_id", dc_id)
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
