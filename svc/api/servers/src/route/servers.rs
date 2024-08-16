use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend::{self, pkg::*};
use rivet_api::models;
use rivet_convert::{ApiFrom, ApiInto, ApiTryFrom, ApiTryInto};
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{assert, auth::Auth};

// MARK: GET /games/{}/environments/{}/servers/{}
pub async fn get(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	server_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::ServersGetServerResponse> {
	ctx.auth()
		.check_game(ctx.op_ctx(), game_id, env_id, true)
		.await?;

	// Get the server
	let get_res = op!([ctx] ds_server_get {
		server_ids: vec![server_id.into()],
	})
	.await?;
	let server = unwrap_with!(get_res.servers.first(), SERVERS_SERVER_NOT_FOUND).clone();

	// Validate token can access server
	ensure_with!(
		unwrap!(server.env_id).as_uuid() == env_id && unwrap!(server.env_id).as_uuid() == env_id,
		SERVERS_SERVER_NOT_FOUND
	);

	Ok(models::ServersGetServerResponse {
		server: Box::new(models::ServersServer::api_try_from(server)?),
	})
}

// MARK: POST /games/{}/environments/{}/servers
pub async fn create(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	body: models::ServersCreateServerRequest,
) -> GlobalResult<models::ServersCreateServerResponse> {
	ctx.auth()
		.check_game(ctx.op_ctx(), game_id, env_id, true)
		.await?;

	let clusters_get = ctx
		.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		})
		.await?;
	let cluster_id = unwrap!(clusters_get.games.first()).cluster_id;

	let datacenters = ctx
		.op(cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster_id],
		})
		.await?;
	ensure_with!(
		unwrap!(datacenters.clusters.first())
			.datacenter_ids
			.contains(&body.datacenter),
		CLUSTER_DATACENTER_NOT_FOUND
	);

	let tags = serde_json::from_value(body.tags.unwrap_or_default())?;

	tracing::info!(?tags, "creating server with tags");

	let server = op!([ctx] ds_server_create {
		env_id: Some(env_id.into()),
		datacenter_id: Some(body.datacenter.into()),
		cluster_id: Some(cluster_id.into()),
		tags: tags,
		resources: Some((*body.resources).api_into()),
		kill_timeout_ms: body.lifecycle.as_ref().and_then(|x| x.kill_timeout).unwrap_or_default(),
		image_id: Some(body.runtime.build.into()),
		args: body.runtime.arguments.unwrap_or_default(),
		network_mode: backend::ds::NetworkMode::api_from(
			body.network.mode.unwrap_or_default(),
		) as i32,
		environment: body.runtime.environment.unwrap_or_default(),
		network_ports: unwrap!(body.network
			.ports
			.into_iter()
			.map(|(s, p)| Ok((s, dynamic_servers::server_create::Port {
				internal_port: p.internal_port,
				routing: Some(if let Some(routing) = p.routing {
					match *routing {
						models::ServersPortRouting {
							game_guard: Some(_),
							host: None,
						} => dynamic_servers::server_create::port::Routing::GameGuard(
							backend::ds::GameGuardRouting {
								protocol: backend::ds::GameGuardProtocol::api_from(p.protocol) as i32,
							},
						),
						models::ServersPortRouting {
							game_guard: None,
							host: Some(_),
						} => dynamic_servers::server_create::port::Routing::Host(backend::ds::HostRouting {
							protocol: backend::ds::HostProtocol::api_try_from(p.protocol)? as i32,
						}),
						models::ServersPortRouting { .. } => {
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

	Ok(models::ServersCreateServerResponse {
		server: Box::new(unwrap!(server).api_try_into()?),
	})
}

// MARK: DELETE /games/{}/environments/{}/servers/{}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteQuery {
	override_kill_timeout: Option<i64>,
}

pub async fn destroy(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	server_id: Uuid,
	query: DeleteQuery,
) -> GlobalResult<serde_json::Value> {
	ctx.auth()
		.check_game(ctx.op_ctx(), game_id, env_id, true)
		.await?;

	assert::server_for_env(&ctx, server_id, game_id, env_id).await?;

	op!([ctx] ds_server_delete {
		server_id: Some(server_id.into()),
		override_kill_timeout_ms: query.override_kill_timeout.unwrap_or_default(),
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: GET /games/{}/environments/{}/servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListQuery {
	tags_json: Option<String>,
	include_destroyed: Option<bool>,
	cursor: Option<Uuid>,
}

pub async fn list_servers(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: ListQuery,
) -> GlobalResult<models::ServersListServersResponse> {
	ctx.auth()
		.check_game(ctx.op_ctx(), game_id, env_id, true)
		.await?;

	let list_res = op!([ctx] ds_server_list_for_env {
		env_id: Some(env_id.into()),
		tags: query.tags_json.as_deref().map_or(Ok(HashMap::new()), serde_json::from_str)?,
		include_destroyed: query.include_destroyed.unwrap_or(false),
		cursor: query.cursor.map(|x| x.into()),
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
			let server = models::ServersServer::api_try_from(server)?;
			Ok(server)
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::ServersListServersResponse { servers })
}
