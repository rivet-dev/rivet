use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use futures_util::{StreamExt, TryStreamExt};
use proto::backend;
use rivet_api::models;
use rivet_convert::{ApiInto, ApiTryInto};
use rivet_operation::prelude::*;
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};

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
	watch_index: WatchIndexQuery,
	query: GlobalQuery,
) -> GlobalResult<models::ActorGetActorResponse> {
	get_inner(&ctx, actor_id, watch_index, query).await
}

async fn get_inner(
	ctx: &Ctx<Auth>,
	actor_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: GlobalQuery,
) -> GlobalResult<models::ActorGetActorResponse> {
	let CheckOutput { env_id, .. } = ctx.auth().check(ctx.op_ctx(), &query, true).await?;

	// Get the server
	let servers_res = ctx
		.op(ds::ops::server::get::Input {
			server_ids: vec![actor_id],
		})
		.await?;
	let server = unwrap_with!(servers_res.servers.first(), ACTOR_NOT_FOUND);

	// Get the datacenter
	let dc_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![server.datacenter_id],
		})
		.await?;
	let dc = unwrap!(dc_res.datacenters.first());

	// Validate token can access server
	ensure_with!(server.env_id == env_id, ACTOR_NOT_FOUND);

	Ok(models::ActorGetActorResponse {
		actor: Box::new(ds::types::convert_actor_to_api(server.clone(), dc)?),
	})
}

pub async fn get_deprecated(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	actor_id: Uuid,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::ServersGetServerResponse> {
	let global = build_global_query_compat(&ctx, game_id, env_id).await?;
	let get_res = get_inner(&ctx, actor_id, watch_index, global).await?;

	let game_res = ctx
		.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		})
		.await?;
	let game = unwrap!(game_res.games.first());

	let dc_resolve_res = ctx
		.op(cluster::ops::datacenter::resolve_for_name_id::Input {
			cluster_id: game.cluster_id,
			name_ids: vec![get_res.actor.region.clone()],
		})
		.await?;
	let dc_id = unwrap!(dc_resolve_res.datacenters.first()).datacenter_id;

	let dc_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![dc_id],
		})
		.await?;
	let dc = unwrap!(dc_res.datacenters.first());

	Ok(models::ServersGetServerResponse {
		server: Box::new(legacy_convert_actor_to_server(*get_res.actor, dc)),
	})
}

// MARK: POST /actors
pub async fn create(
	ctx: Ctx<Auth>,
	body: models::ActorCreateActorRequest,
	query: GlobalQuery,
) -> GlobalResult<models::ActorCreateActorResponse> {
	let CheckOutput { game_id, env_id } = ctx.auth().check(ctx.op_ctx(), &query, true).await?;

	let (clusters_res, game_configs_res, build_id) = tokio::try_join!(
		ctx.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		}),
		ctx.op(ds::ops::game_config::get::Input {
			game_ids: vec![game_id],
		}),
		resolve_build_id(&ctx, env_id, body.build, body.build_tags.flatten()),
	)?;
	let cluster_id = unwrap!(clusters_res.games.first()).cluster_id;
	let game_config = unwrap!(game_configs_res.game_configs.first());

	let datacenter_id = resolve_dc_id(&ctx, cluster_id, body.region.clone()).await?;

	let tags = unwrap_with!(
		serde_json::from_value(body.tags.unwrap_or_default()).ok(),
		API_BAD_BODY,
		error = "`tags` must be `Map<String, String>`"
	);

	tracing::info!(?tags, "creating server with tags");

	let server_id = Uuid::new_v4();

	let mut ready_sub = ctx
		.subscribe::<ds::workflows::server::Ready>(("server_id", server_id))
		.await?;
	let mut fail_sub = ctx
		.subscribe::<ds::workflows::server::Failed>(("server_id", server_id))
		.await?;
	let mut destroy_sub = ctx
		.subscribe::<ds::workflows::server::DestroyStarted>(("server_id", server_id))
		.await?;

	let network = body.network.unwrap_or_default();

	ctx.workflow(ds::workflows::server::Input {
		server_id,
		env_id,
		datacenter_id,
		cluster_id,
		runtime: game_config.runtime,
		tags,
		resources: (*body.resources).api_into(),
		lifecycle: body.lifecycle.map(|x| (*x).api_into()).unwrap_or_else(|| {
			ds::types::ServerLifecycle {
				kill_timeout_ms: 0,
				durable: false,
			}
		}),
		image_id: build_id,
		root_user_enabled: game_config.root_user_enabled,
		// args: body.runtime.arguments.unwrap_or_default(),
		args: Vec::new(),
		network_mode: network.mode.unwrap_or_default().api_into(),
		environment: body.runtime.and_then(|r| r.environment).unwrap_or_default(),
		network_ports: unwrap!(network
			.ports
			.unwrap_or_default()
			.into_iter()
			.map(|(s, p)| GlobalResult::Ok((
				s.clone(),
				ds::workflows::server::Port {
					internal_port: p.internal_port.map(TryInto::try_into).transpose()?,
					routing: if let Some(routing) = p.routing {
						match *routing {
							models::ActorPortRouting {
								guard: Some(gg),
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
								guard: None,
								host: Some(_),
							} => ds::types::Routing::Host {
								protocol: match p.protocol.api_try_into() {
									Err(err) if GlobalError::is(&err, formatted_error::code::ACTOR_FAILED_TO_CREATE) => {
										// Add location
										bail_with!(
											ACTOR_FAILED_TO_CREATE,
											error = format!("network.ports[{s:?}].protocol: Host port protocol must be either TCP or UDP.")
										);
									}
									x => x?,
								},
							},
							models::ActorPortRouting { .. } => {
								bail_with!(
									ACTOR_FAILED_TO_CREATE,
									error = format!("network.ports[{s:?}].routing: Must specify either `guard` or `host` routing type.")
								);
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

	// Wait for ready, fail, or destroy
	tokio::select! {
		res = ready_sub.next() => { res?; },
		res = fail_sub.next() => {
			let msg = res?;
			bail_with!(ACTOR_FAILED_TO_CREATE, error = msg.message);
		}
		res = destroy_sub.next() => {
			res?;
			bail_with!(ACTOR_FAILED_TO_CREATE, error = "Actor failed before reaching a ready state.");
		}
	}

	let servers_res = ctx
		.op(ds::ops::server::get::Input {
			server_ids: vec![server_id],
		})
		.await?;
	let server = unwrap_with!(servers_res.servers.first(), ACTOR_NOT_FOUND);

	let dc_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![server.datacenter_id],
		})
		.await?;
	let dc = unwrap!(dc_res.datacenters.first());

	Ok(models::ActorCreateActorResponse {
		actor: Box::new(ds::types::convert_actor_to_api(server.clone(), dc)?),
	})
}

pub async fn create_deprecated(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	body: models::ServersCreateServerRequest,
) -> GlobalResult<models::ServersCreateServerResponse> {
	// Resolve region slug
	let dc_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![body.datacenter],
		})
		.await?;
	let dc = unwrap!(dc_res.datacenters.first());

	let global = build_global_query_compat(&ctx, game_id, env_id).await?;
	let create_res = create(
		ctx,
		models::ActorCreateActorRequest {
			region: Some(dc.name_id.clone()),
			lifecycle: body.lifecycle.map(|l| {
				Box::new(models::ActorLifecycle {
					kill_timeout: l.kill_timeout,
					durable: Some(false),
				})
			}),
			network: Some(Box::new(models::ActorCreateActorNetworkRequest {
				mode: body.network.mode.map(|n| match n {
					models::ServersNetworkMode::Host => models::ActorNetworkMode::Host,
					models::ServersNetworkMode::Bridge => models::ActorNetworkMode::Bridge,
				}),
				ports: Some(
					body.network
						.ports
						.into_iter()
						.map(|(k, p)| {
							(
								k,
								models::ActorCreateActorPortRequest {
									internal_port: p.internal_port,
									protocol: match p.protocol {
										models::ServersPortProtocol::Http => {
											models::ActorPortProtocol::Http
										}
										models::ServersPortProtocol::Https => {
											models::ActorPortProtocol::Https
										}
										models::ServersPortProtocol::Tcp => {
											models::ActorPortProtocol::Tcp
										}
										models::ServersPortProtocol::TcpTls => {
											models::ActorPortProtocol::TcpTls
										}
										models::ServersPortProtocol::Udp => {
											models::ActorPortProtocol::Udp
										}
									},
									routing: p.routing.map(|r| {
										Box::new(models::ActorPortRouting {
											guard: r.game_guard.map(|_| {
												Box::new(models::ActorGuardRouting::default())
											}),
											host: r.host.map(|_| json!({})),
										})
									}),
								},
							)
						})
						.collect(),
				),
			})),
			resources: Box::new(models::ActorResources {
				cpu: body.resources.cpu,
				memory: body.resources.memory,
			}),
			runtime: Some(Box::new(models::ActorCreateActorRuntimeRequest {
				environment: body.runtime.environment,
			})),
			build: Some(body.runtime.build),
			build_tags: None,
			tags: body.tags,
		},
		global,
	)
	.await?;

	Ok(models::ServersCreateServerResponse {
		server: Box::new(legacy_convert_actor_to_server(*create_res.actor, &dc)),
	})
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
		ctx.auth().check(ctx.op_ctx(), &query.global, true).await?;

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

	// Get server after sub is created
	let server = assert::server_for_env(&ctx, actor_id, game_id, env_id).await?;

	// Already destroyed
	if server.destroy_ts.is_some() {
		return Ok(json!({}));
	}

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

// MARK: POST /actors/{}/upgrade
pub async fn upgrade(
	ctx: Ctx<Auth>,
	actor_id: Uuid,
	body: models::ActorUpgradeActorRequest,
	query: GlobalQuery,
) -> GlobalResult<serde_json::Value> {
	let CheckOutput { game_id, env_id } = ctx.auth().check(ctx.op_ctx(), &query, false).await?;

	assert::server_for_env(&ctx, actor_id, game_id, env_id).await?;

	let build_id = resolve_build_id(&ctx, env_id, body.build, body.build_tags.flatten()).await?;

	let mut sub = ctx
		.subscribe::<ds::workflows::server::UpgradeStarted>(("server_id", actor_id))
		.await?;

	ctx.signal(ds::workflows::server::Upgrade { image_id: build_id })
		.tag("server_id", actor_id)
		.send()
		.await?;

	sub.next().await?;

	Ok(json!({}))
}

// MARK: POST /actors/upgrade
pub async fn upgrade_all(
	ctx: Ctx<Auth>,
	body: models::ActorUpgradeAllActorsRequest,
	query: GlobalQuery,
) -> GlobalResult<models::ActorUpgradeAllActorsResponse> {
	let CheckOutput { env_id, .. } = ctx.auth().check(ctx.op_ctx(), &query, false).await?;

	let tags = unwrap_with!(body.tags, API_BAD_BODY, error = "missing property `tags`");

	ensure_with!(
		tags.as_object().map(|x| x.len()).unwrap_or_default() <= 64,
		API_BAD_BODY,
		error = "Too many tags (max 64)."
	);

	let tags = unwrap_with!(
		serde_json::from_value::<HashMap<String, String>>(tags).ok(),
		API_BAD_BODY,
		error = "`tags` must be `Map<String, String>`"
	);

	for (k, v) in &tags {
		ensure_with!(
			k.len() <= 256,
			API_BAD_BODY,
			error = format!(
				"tags[{:?}]: Tag label too large (max 256 bytes).",
				&k[..256]
			),
		);

		ensure_with!(
			v.len() <= 1024,
			API_BAD_BODY,
			error = format!("tags[{k:?}]: Tag value too large (max 1024 bytes)."),
		);
	}

	let build_id = resolve_build_id(&ctx, env_id, body.build, body.build_tags.flatten()).await?;

	// Work in batches
	let mut count = 0;
	let mut cursor = None;
	loop {
		let list_res = ctx
			.op(ds::ops::server::list_for_env::Input {
				env_id,
				tags: tags.clone(),
				include_destroyed: false,
				cursor,
				limit: 10_000,
			})
			.await?;

		count += list_res.server_ids.len();
		cursor = list_res.server_ids.last().cloned();

		let subs = futures_util::stream::iter(list_res.server_ids.clone())
			.map(|server_id| {
				ctx.subscribe::<ds::workflows::server::UpgradeStarted>(("server_id", server_id))
			})
			.buffer_unordered(32)
			.try_collect::<Vec<_>>()
			.await?;

		futures_util::stream::iter(list_res.server_ids)
			.map(|server_id| {
				ctx.signal(ds::workflows::server::Upgrade { image_id: build_id })
					.tag("server_id", server_id)
					.send()
			})
			.buffer_unordered(32)
			.try_collect::<Vec<_>>()
			.await?;

		futures_util::stream::iter(subs)
			.map(|mut sub| async move { sub.next().await })
			.buffer_unordered(32)
			.try_collect::<Vec<_>>()
			.await?;

		if count % 10_000 != 0 {
			break;
		}
	}

	Ok(models::ActorUpgradeAllActorsResponse {
		count: count.try_into()?,
	})
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
	watch_index: WatchIndexQuery,
	query: ListQuery,
) -> GlobalResult<models::ActorListActorsResponse> {
	list_actors_inner(&ctx, watch_index, query).await
}

async fn list_actors_inner(
	ctx: &Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: ListQuery,
) -> GlobalResult<models::ActorListActorsResponse> {
	let CheckOutput { env_id, .. } = ctx.auth().check(ctx.op_ctx(), &query.global, true).await?;

	let include_destroyed = query.include_destroyed.unwrap_or(false);

	let tags = unwrap_with!(
		query
			.tags_json
			.as_deref()
			.map_or(Ok(HashMap::new()), serde_json::from_str)
			.ok(),
		API_BAD_QUERY_PARAMETER,
		parameter = "tags_json",
		error = "must be `Map<String, String>`"
	);

	let list_res = ctx
		.op(ds::ops::server::list_for_env::Input {
			env_id,
			tags,
			include_destroyed,
			cursor: query.cursor,
			limit: if include_destroyed { 64 } else { 10_000 },
		})
		.await?;

	let servers_res = ctx
		.op(ds::ops::server::get::Input {
			server_ids: list_res.server_ids.clone(),
		})
		.await?;

	let datacenter_ids = servers_res
		.servers
		.iter()
		.map(|s| s.datacenter_id)
		.collect::<HashSet<Uuid>>()
		.into_iter()
		.collect::<Vec<_>>();
	let dc_res = ctx
		.op(cluster::ops::datacenter::get::Input { datacenter_ids })
		.await?;

	let servers = servers_res
		.servers
		.into_iter()
		.map(|a| {
			let dc = unwrap!(dc_res
				.datacenters
				.iter()
				.find(|dc| dc.datacenter_id == a.datacenter_id));
			ds::types::convert_actor_to_api(a, &dc)
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::ActorListActorsResponse { actors: servers })
}

pub async fn list_servers_deprecated(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	watch_index: WatchIndexQuery,
	query: ListQuery,
) -> GlobalResult<models::ServersListServersResponse> {
	let global = build_global_query_compat(&ctx, game_id, env_id).await?;
	let actors_res = list_actors_inner(&ctx, watch_index, ListQuery { global, ..query }).await?;

	let clusters_res = ctx
		.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		})
		.await?;
	let cluster_id = unwrap!(clusters_res.games.first()).cluster_id;

	let dc_name_ids = actors_res
		.actors
		.iter()
		.map(|s| s.region.clone())
		.collect::<HashSet<String>>()
		.into_iter()
		.collect::<Vec<_>>();
	let dc_resolve_res = ctx
		.op(cluster::ops::datacenter::resolve_for_name_id::Input {
			cluster_id,
			name_ids: dc_name_ids,
		})
		.await?;

	let dc_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: dc_resolve_res
				.datacenters
				.iter()
				.map(|x| x.datacenter_id)
				.collect::<Vec<_>>(),
		})
		.await?;

	Ok(models::ServersListServersResponse {
		servers: actors_res
			.actors
			.into_iter()
			.map(|a| {
				let dc = unwrap!(dc_res.datacenters.iter().find(|dc| dc.name_id == a.region));
				GlobalResult::Ok(legacy_convert_actor_to_server(a, dc))
			})
			.collect::<Result<Vec<_>, _>>()?,
	})
}

fn legacy_convert_actor_to_server(
	a: models::ActorActor,
	datacenter: &cluster::types::Datacenter,
) -> models::ServersServer {
	models::ServersServer {
		created_at: a.created_at,
		datacenter: datacenter.datacenter_id,
		destroyed_at: a.destroyed_at,
		environment: Uuid::nil(),
		id: a.id,
		lifecycle: Box::new(models::ServersLifecycle {
			kill_timeout: a.lifecycle.kill_timeout,
		}),
		network: Box::new(models::ServersNetwork {
			mode: Some(match a.network.mode {
				models::ActorNetworkMode::Host => models::ServersNetworkMode::Host,
				models::ActorNetworkMode::Bridge => models::ServersNetworkMode::Bridge,
			}),
			ports: a
				.network
				.ports
				.into_iter()
				.map(|(k, p)| {
					(
						k,
						models::ServersPort {
							internal_port: p.internal_port,
							protocol: match p.protocol {
								models::ActorPortProtocol::Http => {
									models::ServersPortProtocol::Http
								}
								models::ActorPortProtocol::Https => {
									models::ServersPortProtocol::Https
								}
								models::ActorPortProtocol::Tcp => models::ServersPortProtocol::Tcp,
								models::ActorPortProtocol::TcpTls => {
									models::ServersPortProtocol::TcpTls
								}
								models::ActorPortProtocol::Udp => models::ServersPortProtocol::Udp,
							},
							public_hostname: p.public_hostname,
							public_port: p.public_port,
							routing: Box::new(models::ServersPortRouting {
								game_guard: p.routing.guard.map(|_| json!({})),
								host: p.routing.host.map(|_| json!({})),
							}),
						},
					)
				})
				.collect(),
		}),
		resources: Box::new(models::ServersResources {
			cpu: a.resources.cpu,
			memory: a.resources.memory,
		}),
		runtime: Box::new(models::ServersRuntime {
			arguments: a.runtime.arguments,
			build: a.runtime.build,
			environment: a.runtime.environment,
		}),
		started_at: a.started_at,
		tags: a.tags,
	}
}

async fn resolve_build_id(
	ctx: &Ctx<Auth>,
	env_id: Uuid,
	build_id: Option<Uuid>,
	build_tags: Option<serde_json::Value>,
) -> GlobalResult<Uuid> {
	match (build_id, build_tags) {
		(Some(build_id), None) => Ok(build_id),
		// Resolve build from tags
		(None, Some(build_tags)) => {
			let build_tags = unwrap_with!(
				serde_json::from_value::<HashMap<String, String>>(build_tags).ok(),
				API_BAD_BODY,
				error = "`build_tags` must be `Map<String, String>`"
			);

			ensure_with!(
				build_tags.len() < 64,
				API_BAD_BODY,
				error = "Too many build tags (max 64)."
			);

			for (k, v) in &build_tags {
				ensure_with!(
					k.len() < 128,
					API_BAD_BODY,
					error = format!(
						"build_tags[{:?}]: Build tag label too large (max 128 bytes).",
						&k[..128]
					)
				);
				ensure_with!(
					v.len() < 256,
					API_BAD_BODY,
					error =
						format!("build_tags[{k:?}]: Build tag value too large (max 256 bytes).")
				);
			}

			let builds_res = ctx
				.op(build::ops::resolve_for_tags::Input {
					env_id,
					tags: build_tags,
				})
				.await?;

			let build = unwrap_with!(builds_res.builds.first(), BUILD_NOT_FOUND_WITH_TAGS);

			Ok(build.build_id)
		}
		_ => {
			bail_with!(
				API_BAD_BODY,
				error = "must have either `build` or `buildTags`"
			);
		}
	}
}

async fn resolve_dc_id(
	ctx: &Ctx<Auth>,
	cluster_id: Uuid,
	region: Option<String>,
) -> GlobalResult<Uuid> {
	if let Some(region) = region {
		let dcs_res = ctx
			.op(cluster::ops::datacenter::resolve_for_name_id::Input {
				cluster_id,
				name_ids: vec![region],
			})
			.await?;
		let dc = unwrap_with!(
			dcs_res.datacenters.first(),
			ACTOR_FAILED_TO_CREATE,
			error = "Region not found."
		);

		Ok(dc.datacenter_id)
	}
	// Auto-select the closest region
	else {
		let clusters_res = ctx
			.op(cluster::ops::datacenter::list::Input {
				cluster_ids: vec![cluster_id],
			})
			.await?;
		let cluster = unwrap!(clusters_res.clusters.first());

		if let Some((lat, long)) = ctx.coords() {
			let recommend_res = op!([ctx] region_recommend {
				region_ids: cluster
					.datacenter_ids
					.iter()
					.cloned()
					.map(Into::into)
					.collect(),
				coords: Some(backend::net::Coordinates {
					latitude: lat,
					longitude: long,
				}),
				..Default::default()
			})
			.await?;
			let primary_region = unwrap!(recommend_res.regions.first());
			let primary_region_id = unwrap_ref!(primary_region.region_id).as_uuid();

			Ok(primary_region_id)
		} else {
			tracing::warn!("coords not provided to select region");

			let datacenter_id = *unwrap_with!(
				cluster.datacenter_ids.first(),
				ACTOR_FAILED_TO_CREATE,
				error = "No regions found."
			);

			Ok(datacenter_id)
		}
	}
}
