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
	token: String,
	datacenter: Uuid,
}

#[tracing::instrument(skip(ctx))]
pub async fn config(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	ConfigQuery { token, datacenter }: ConfigQuery,
) -> GlobalResult<types::TraefikConfigResponseNullified> {
	ctx.auth().token(&token).await?;

	let mut config = types::TraefikConfigResponse::default();

	// Fetch configs and catch any errors
	tracing::info!(?config, "traefik config ds");
	tracing::info!("asdgaerwvsdfvasdf");

	build_ds(&ctx, datacenter, &mut config).await?;
	build_job(&ctx, datacenter, &mut config).await?;

	tracing::info!(?config, "traefik config ds");

	// tracing::info!(
	// 	http_services = ?config.http.services.len(),
	// 	http_routers = config.http.routers.len(),
	// 	http_middlewares = ?config.http.middlewares.len(),
	// 	tcp_services = ?config.tcp.services.len(),
	// 	tcp_routers = config.tcp.routers.len(),
	// 	tcp_middlewares = ?config.tcp.middlewares.len(),
	// 	udp_services = ?config.udp.services.len(),
	// 	udp_routers = config.udp.routers.len(),
	// 	udp_middlewares = ?config.udp.middlewares.len(),
	// 	"traefik config"
	// );

	Ok(types::TraefikConfigResponseNullified {
		http: config.http.nullified(),
		tcp: config.tcp.nullified(),
		udp: config.udp.nullified(),
	})
}
