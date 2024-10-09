use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_convert::{ApiInto, ApiTryInto};
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
	let servers_res = ctx
		.op(ds::ops::server::get::Input {
			server_ids: vec![server_id],
		})
		.await?;
	let server = unwrap_with!(servers_res.servers.first(), SERVERS_SERVER_NOT_FOUND);

	// Validate token can access server
	ensure_with!(server.env_id == env_id, SERVERS_SERVER_NOT_FOUND);

	Ok(models::ServersGetServerResponse {
		server: Box::new(server.clone().api_try_into()?),
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

	let (clusters_res, game_configs_res) = tokio::try_join!(
		ctx.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		}),
		ctx.op(ds::ops::game_config::get::Input {
			game_ids: vec![game_id],
		}),
	)?;
	let cluster_id = unwrap!(clusters_res.games.first()).cluster_id;
	let game_config = unwrap!(game_configs_res.game_configs.first());

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

	let server_id = Uuid::new_v4();

	let mut create_sub = ctx
		.subscribe::<ds::workflows::server::CreateComplete>(&json!({
			"server_id": server_id,
		}))
		.await?;
	let mut fail_sub = ctx
		.subscribe::<ds::workflows::server::CreateFailed>(&json!({
			"server_id": server_id,
		}))
		.await?;

	ctx.workflow(ds::workflows::server::Input {
		server_id,
		env_id,
		datacenter_id: body.datacenter,
		cluster_id,
		runtime: game_config.runtime,
		tags,
		resources: (*body.resources).api_into(),
		kill_timeout_ms: body
			.lifecycle
			.as_ref()
			.and_then(|x| x.kill_timeout)
			.unwrap_or_default(),
		image_id: body.runtime.build,
		root_user_enabled: game_config.root_user_enabled,
		args: body.runtime.arguments.unwrap_or_default(),
		network_mode: body.network.mode.unwrap_or_default().api_into(),
		environment: body.runtime.environment.unwrap_or_default(),
		network_ports: unwrap!(body
			.network
			.ports
			.into_iter()
			.map(|(s, p)| Ok((
				s,
				ds::workflows::server::Port {
					internal_port: p.internal_port,
					routing: if let Some(routing) = p.routing {
						match *routing {
							models::ServersPortRouting {
								game_guard: Some(_),
								host: None,
							} => ds::types::Routing::GameGuard {
								protocol: p.protocol.api_into(),
							},
							models::ServersPortRouting {
								game_guard: None,
								host: Some(_),
							} => ds::types::Routing::Host {
								protocol: p.protocol.api_try_into()?,
							},
							models::ServersPortRouting { .. } => {
								bail_with!(SERVERS_MUST_SPECIFY_ROUTING_TYPE)
							}
						}
					} else {
						ds::types::Routing::GameGuard {
							protocol: p.protocol.api_into(),
						}
					}
				}
			)))
			.collect::<GlobalResult<HashMap<_, _>>>()),
	})
	.tag("server_id", server_id)
	.dispatch()
	.await?;

	tokio::select! {
		res = create_sub.next() => { res?; },
		res = fail_sub.next() => {
			res?;
			bail_with!(SERVERS_SERVER_FAILED_TO_CREATE);
		}
	}

	let servers_res = ctx
		.op(ds::ops::server::get::Input {
			server_ids: vec![server_id],
		})
		.await?;
	let server = unwrap_with!(servers_res.servers.first(), SERVERS_SERVER_NOT_FOUND);

	Ok(models::ServersCreateServerResponse {
		server: Box::new(server.clone().api_try_into()?),
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

	ensure_with!(
		query.override_kill_timeout.unwrap_or(0) >= 0,
		API_BAD_QUERY_PARAMETER,
		parameter = "override_kill_timeout",
		error = "must be positive"
	);
	ensure_with!(
		query.override_kill_timeout.unwrap_or(0) < 2 * 60 * 60 * 1000,
		API_BAD_QUERY_PARAMETER,
		parameter = "override_kill_timeout",
		error = "cannot be longer than 2 hours"
	);

	let mut sub = ctx
		.subscribe::<ds::workflows::server::DestroyStarted>(&json!({
			"server_id": server_id,
		}))
		.await?;

	ctx.signal(ds::workflows::server::Destroy {
		override_kill_timeout_ms: query.override_kill_timeout,
	})
	.tag("server_id", server_id)
	.send()
	.await?;

	sub.next().await?;

	Ok(json!({}))
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

	let list_res = ctx
		.op(ds::ops::server::list_for_env::Input {
			env_id,
			tags: query
				.tags_json
				.as_deref()
				.map_or(Ok(HashMap::new()), serde_json::from_str)?,
			include_destroyed: query.include_destroyed.unwrap_or(false),
			cursor: query.cursor,
		})
		.await?;

	let servers_res = ctx
		.op(ds::ops::server::get::Input {
			server_ids: list_res.server_ids.clone(),
		})
		.await?;

	let servers = servers_res
		.servers
		.into_iter()
		.map(ApiTryInto::api_try_into)
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::ServersListServersResponse { servers })
}
