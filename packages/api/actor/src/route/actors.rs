use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_convert::{ApiInto, ApiTryInto};
use rivet_operation::prelude::*;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

use crate::{
	assert,
	auth::{Auth, CheckOutput},
	utils::build_global_query_compat,
};

use super::GlobalQuery;

// MARK: GET /actors/{}
pub async fn get(
	ctx: Ctx<Auth>,
	actor_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: GlobalQuery,
) -> GlobalResult<models::ActorGetActorResponse> {
	let CheckOutput { env_id, .. } = ctx.auth().check(ctx.op_ctx(), &query, false).await?;

	// Get the server
	let servers_res = ctx
		.op(ds::ops::server::get::Input {
			server_ids: vec![actor_id],
		})
		.await?;
	let server = unwrap_with!(servers_res.servers.first(), SERVERS_SERVER_NOT_FOUND);

	// Validate token can access server
	ensure_with!(server.env_id == env_id, SERVERS_SERVER_NOT_FOUND);

	Ok(models::ActorGetActorResponse {
		actor: Box::new(server.clone().api_try_into()?),
	})
}

pub async fn get_deprecated(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	actor_id: Uuid,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::ActorGetActorResponse> {
	let global = build_global_query_compat(&ctx, game_id, env_id).await?;
	get(ctx, actor_id, watch_index, global).await
}

// MARK: POST /actors
pub async fn create(
	ctx: Ctx<Auth>,
	body: models::ActorCreateActorRequest,
	query: GlobalQuery,
) -> GlobalResult<models::ActorCreateActorResponse> {
	let CheckOutput { game_id, env_id } = ctx.auth().check(ctx.op_ctx(), &query, false).await?;

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
		.subscribe::<ds::workflows::server::CreateComplete>(("server_id", server_id))
		.await?;
	let mut fail_sub = ctx
		.subscribe::<ds::workflows::server::CreateFailed>(("server_id", server_id))
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
					internal_port: p.internal_port.map(TryInto::try_into).transpose()?,
					routing: if let Some(routing) = p.routing {
						match *routing {
							models::ActorPortRouting {
								game_guard: Some(gg),
								host: None,
							} => ds::types::Routing::GameGuard {
								protocol: p.protocol.api_into(),
								authorization: match gg.authorization.as_deref() {
									Some(models::ActorPortAuthorization {
										bearer: Some(token),
										..
									}) => ds::types::PortAuthorization::Bearer(token.clone()),
									Some(models::ActorPortAuthorization {
										query: Some(query),
										..
									}) => ds::types::PortAuthorization::Query(
										query.key.clone(),
										query.value.clone(),
									),
									_ => ds::types::PortAuthorization::None,
								},
							},
							models::ActorPortRouting {
								game_guard: None,
								host: Some(_),
							} => ds::types::Routing::Host {
								protocol: p.protocol.api_try_into()?,
							},
							models::ActorPortRouting { .. } => {
								bail_with!(SERVERS_MUST_SPECIFY_ROUTING_TYPE)
							}
						}
					} else {
						ds::types::Routing::GameGuard {
							protocol: p.protocol.api_into(),
							authorization: ds::types::PortAuthorization::None,
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

	Ok(models::ActorCreateActorResponse {
		actor: Box::new(server.clone().api_try_into()?),
	})
}

pub async fn create_deprecated(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	body: models::ActorCreateActorRequest,
) -> GlobalResult<models::ActorCreateActorResponse> {
	let global = build_global_query_compat(&ctx, game_id, env_id).await?;
	create(ctx, body, global).await
}

// MARK: DELETE /actors/{}
#[derive(Debug, Clone, Deserialize)]
pub struct DeleteQuery {
	#[serde(flatten)]
	global: GlobalQuery,
	override_kill_timeout: Option<i64>,
}

pub async fn destroy(
	ctx: Ctx<Auth>,
	actor_id: Uuid,
	query: DeleteQuery,
) -> GlobalResult<serde_json::Value> {
	let CheckOutput { game_id, env_id } =
		ctx.auth().check(ctx.op_ctx(), &query.global, false).await?;

	assert::server_for_env(&ctx, actor_id, game_id, env_id).await?;

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
		.subscribe::<ds::workflows::server::DestroyStarted>(("server_id", actor_id))
		.await?;

	ctx.signal(ds::workflows::server::Destroy {
		override_kill_timeout_ms: query.override_kill_timeout,
	})
	.tag("server_id", actor_id)
	.send()
	.await?;

	sub.next().await?;

	Ok(json!({}))
}

pub async fn destroy_deprecated(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	actor_id: Uuid,
	query: DeleteQuery,
) -> GlobalResult<serde_json::Value> {
	let global = build_global_query_compat(&ctx, game_id, env_id).await?;
	destroy(
		ctx,
		actor_id,
		DeleteQuery {
			global,
			override_kill_timeout: query.override_kill_timeout,
		},
	)
	.await
}

// MARK: GET /actors
#[derive(Debug, Clone, Deserialize)]
pub struct ListQuery {
	#[serde(flatten)]
	global: GlobalQuery,
	tags_json: Option<String>,
	include_destroyed: Option<bool>,
	cursor: Option<Uuid>,
}

pub async fn list_actors(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: ListQuery,
) -> GlobalResult<models::ActorListActorsResponse> {
	let CheckOutput { env_id, .. } = ctx.auth().check(ctx.op_ctx(), &query.global, false).await?;

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

	Ok(models::ActorListActorsResponse { actors: servers })
}

pub async fn list_actors_deprecated(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	watch_index: WatchIndexQuery,
	query: ListQuery,
) -> GlobalResult<models::ActorListActorsResponse> {
	let global = build_global_query_compat(&ctx, game_id, env_id).await?;
	list_actors(ctx, watch_index, ListQuery { global, ..query }).await
}
