use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend::{self, pkg::*};
use rivet_api::models;
use rivet_convert::{ApiFrom, ApiInto, ApiTryFrom, ApiTryInto};
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{assert, auth::Auth};

// MARK: GET /games/{}/servers/{}
pub async fn get(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	server_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::GamesServersGetServerResponse> {
	ctx.auth().check_game(ctx.op_ctx(), game_id, true).await?;

	// Get the server
	let get_res = op!([ctx] ds_server_get {
		server_ids: vec![server_id.into()],
	})
	.await?;

	let server = models::GamesServersServer::api_try_from(
		unwrap_with!(get_res.servers.first(), SERVERS_SERVER_NOT_FOUND).clone(),
	)?;

	// Validate token can access server
	ensure_with!(server.game_id == game_id, SERVERS_SERVER_NOT_FOUND);

	Ok(models::GamesServersGetServerResponse {
		server: Box::new(server),
	})
}

// MARK: POST /games/{}/servers
pub async fn create(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	body: models::GamesServersCreateServerRequest,
) -> GlobalResult<models::GamesServersCreateServerResponse> {
	ctx.auth().check_game(ctx.op_ctx(), game_id, true).await?;

	let games = ctx
		.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		})
		.await?
		.games;

	let cluster_id = unwrap!(games.first()).cluster_id;

	let datacenters = ctx
		.op(cluster::ops::datacenter::resolve_for_name_id::Input {
			cluster_id,
			name_ids: vec![body.datacenter.clone()],
		})
		.await?
		.datacenters;

	if datacenters.is_empty() {
		bail_with!(CLUSTER_DATACENTER_NOT_FOUND);
	}

	let datacenter_id = unwrap!(datacenters.first()).datacenter_id;

	let tags = serde_json::from_value(body.tags.unwrap_or_default())?;

	tracing::info!(?tags, "creating server with tags");

	let server = op!([ctx] ds_server_create {
		game_id: Some(game_id.into()),
		datacenter_id: Some(datacenter_id.into()),
		cluster_id: Some(cluster_id.into()),
		tags: tags,
		resources: Some((*body.resources).api_into()),
		kill_timeout_ms: body.kill_timeout.unwrap_or_default(),
		webhook_url: body.webhook_url,
		image_id: Some(body.image_id.into()),
		args: body.arguments.unwrap_or_default(),
		network_mode: backend::ds::NetworkMode::api_from(
			body.network.mode.unwrap_or_default(),
		) as i32,
		environment: body.environment.unwrap_or_default(),
		network_ports: unwrap!(body.network
			.ports
			.into_iter()
			.map(|(s, p)| Ok((s, dynamic_servers::server_create::Port {
				internal_port: p.internal_port,
				routing: Some(if let Some(routing) = p.routing {
					match *routing {
						models::GamesServersPortRouting {
							game_guard: Some(_),
							host: None,
						} => dynamic_servers::server_create::port::Routing::GameGuard(
							backend::ds::GameGuardRouting {
								protocol: backend::ds::GameGuardProtocol::api_from(p.protocol) as i32,
							},
						),
						models::GamesServersPortRouting {
							game_guard: None,
							host: Some(_),
						} => dynamic_servers::server_create::port::Routing::Host(backend::ds::HostRouting {
							protocol: backend::ds::HostProtocol::api_try_from(p.protocol)? as i32,
						}),
						models::GamesServersPortRouting { .. } => {
							bail_with!(SERVERS_MUST_SPECIFY_ROUTING_TYPE)
						}
					}
				} else {
					dynamic_servers::server_create::port::Routing::GameGuard(backend::ds::GameGuardRouting {
						protocol: backend::ds::GameGuardProtocol::api_from(p.protocol) as i32,
					})
				})
			})))
			.collect::<GlobalResult<HashMap<_, _>>>()),
	})
	.await?
	.server;

	Ok(models::GamesServersCreateServerResponse {
		server: Box::new(unwrap!(server).api_try_into()?),
	})
}

// MARK: DELETE /games/{}/servers/{}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteQuery {
	override_kill_timeout: Option<i64>,
}

pub async fn destroy(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	server_id: Uuid,
	query: DeleteQuery,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().check_game(ctx.op_ctx(), game_id, true).await?;

	assert::server_for_game(&ctx, server_id, game_id).await?;

	op!([ctx] ds_server_delete {
		server_id: Some(server_id.into()),
		override_kill_timeout_ms: query.override_kill_timeout.unwrap_or_default(),
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: GET /games/{}/servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListQuery {
	tags: Option<String>,
}

pub async fn list_servers(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: ListQuery,
) -> GlobalResult<models::GamesServersListServersResponse> {
	ctx.auth().check_game(ctx.op_ctx(), game_id, true).await?;

	let list_res = op!([ctx] ds_server_list_for_game {
		game_id: Some(game_id.into()),
		tags: query.tags.as_deref().map_or(Ok(HashMap::new()), serde_json::from_str)?,
	})
	.await?;

	let servers_res = op!([ctx] ds_server_get {
		server_ids: list_res.server_ids.clone(),
	})
	.await?;

	let servers = servers_res
		.servers
		.into_iter()
		.map(|server| {
			let server = models::GamesServersServer::api_try_from(server)?;
			Ok(server)
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::GamesServersListServersResponse { servers })
}
