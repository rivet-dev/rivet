use std::collections::HashMap;

use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use futures_util::{StreamExt, TryStreamExt};
use rivet_api::models;
use rivet_convert::{ApiInto, ApiTryInto};
use rivet_operation::prelude::*;
use serde::Deserialize;
use serde_json::json;

use crate::{
	assert,
	auth::{Auth, CheckOpts, CheckOutput},
};

use super::GlobalQuery;

#[derive(Debug, Clone, Deserialize)]
pub struct GlobalEndpointTypeQuery {
	#[serde(flatten)]
	global: GlobalQuery,
	endpoint_type: Option<models::ActorEndpointType>,
}

// MARK: GET /actors/{}
pub async fn get(
	ctx: Ctx<Auth>,
	actor_id: Uuid,
	watch_index: WatchIndexQuery,
	query: GlobalEndpointTypeQuery,
) -> GlobalResult<models::ActorGetActorResponse> {
	get_inner(&ctx, actor_id, watch_index, query).await
}

async fn get_inner(
	ctx: &Ctx<Auth>,
	actor_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: GlobalEndpointTypeQuery,
) -> GlobalResult<models::ActorGetActorResponse> {
	let CheckOutput { env_id, .. } = ctx
		.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query.global,
				allow_service_token: true,
				opt_auth: false,
			},
		)
		.await?;

	// Get the actor
	let actors_res = ctx
		.op(pegboard::ops::actor::get::Input {
			actor_ids: vec![actor_id],
			endpoint_type: query.endpoint_type.map(ApiInto::api_into),
		})
		.await?;
	let actor = unwrap_with!(actors_res.actors.first(), ACTOR_NOT_FOUND);

	// Get the datacenter
	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;
	let dc_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![dc_id],
		})
		.await?;
	let dc = unwrap!(dc_res.datacenters.first());

	// Validate token can access actor
	ensure_with!(actor.env_id == env_id, ACTOR_NOT_FOUND);

	Ok(models::ActorGetActorResponse {
		actor: Box::new(pegboard::types::convert_actor_to_api(actor.clone(), dc)?),
	})
}

// MARK: POST /actors
pub async fn create(
	ctx: Ctx<Auth>,
	body: models::ActorCreateActorRequest,
	query: GlobalEndpointTypeQuery,
) -> GlobalResult<models::ActorCreateActorResponse> {
	let CheckOutput { game_id, env_id } = ctx
		.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query.global,
				allow_service_token: true,
				opt_auth: false,
			},
		)
		.await?;

	let (game_configs_res, build) = tokio::try_join!(
		ctx.op(pegboard::ops::game_config::get::Input {
			game_ids: vec![game_id],
		}),
		resolve_build(&ctx, game_id, env_id, body.build, body.build_tags.flatten()),
	)?;
	let game_config = unwrap!(game_configs_res.game_configs.first());

	let tags = unwrap_with!(
		serde_json::from_value(body.tags.unwrap_or_default()).ok(),
		API_BAD_BODY,
		error = "`tags` must be `Map<String, String>`"
	);

	let resources = match build.kind {
		build::types::BuildKind::DockerImage | build::types::BuildKind::OciBundle => {
			let resources = unwrap_with!(
				body.resources,
				API_BAD_BODY,
				error = "`resources` must be set for actors using Docker builds"
			);

			(*resources).api_into()
		}
		build::types::BuildKind::JavaScript => {
			ensure_with!(
				body.resources.is_none(),
				API_BAD_BODY,
				error = "actors using JavaScript builds cannot set `resources`"
			);

			pegboard::types::ActorResources::default_isolate()
		}
	};

	let actor_id = Uuid::new_v4();

	tracing::info!(?actor_id, ?tags, "creating actor with tags");

	let mut ready_sub = ctx
		.subscribe::<pegboard::workflows::actor::Ready>(("actor_id", actor_id))
		.await?;
	let mut fail_sub = ctx
		.subscribe::<pegboard::workflows::actor::Failed>(("actor_id", actor_id))
		.await?;
	let mut destroy_sub = ctx
		.subscribe::<pegboard::workflows::actor::DestroyStarted>(("actor_id", actor_id))
		.await?;

	let network = body.network.unwrap_or_default();

	ctx.workflow(pegboard::workflows::actor::Input {
		actor_id,
		env_id,
		tags,
		resources,
		lifecycle: body.lifecycle.map(|x| (*x).api_into()).unwrap_or_else(|| {
			pegboard::types::ActorLifecycle {
				kill_timeout_ms: 0,
				durable: false,
			}
		}),
		image_id: build.build_id,
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
				pegboard::workflows::actor::Port {
					internal_port: p.internal_port.map(TryInto::try_into).transpose()?,
					routing: if let Some(routing) = p.routing {
						match *routing {
							models::ActorPortRouting {
								guard: Some(_gg),
								host: None,
							} => pegboard::types::Routing::GameGuard {
								protocol: p.protocol.api_into(),
							},
							models::ActorPortRouting {
								guard: None,
								host: Some(_),
							} => pegboard::types::Routing::Host {
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
						pegboard::types::Routing::GameGuard {
							protocol: p.protocol.api_into(),
						}
					}
				}
			)))
			.collect::<GlobalResult<HashMap<_, _>>>()),
	})
	.tag("actor_id", actor_id)
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

	let actors_res = ctx
		.op(pegboard::ops::actor::get::Input {
			actor_ids: vec![actor_id],
			endpoint_type: query.endpoint_type.map(ApiInto::api_into),
		})
		.await?;
	let actor = unwrap_with!(actors_res.actors.first(), ACTOR_NOT_FOUND);

	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;
	let dc_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![dc_id],
		})
		.await?;
	let dc = unwrap!(dc_res.datacenters.first());

	Ok(models::ActorCreateActorResponse {
		actor: Box::new(pegboard::types::convert_actor_to_api(actor.clone(), dc)?),
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
	let CheckOutput { game_id, env_id } = ctx
		.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query.global,
				allow_service_token: true,
				opt_auth: false,
			},
		)
		.await?;

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
		.subscribe::<pegboard::workflows::actor::DestroyStarted>(("actor_id", actor_id))
		.await?;

	// Get actor after sub is created
	let actor = assert::actor_for_env(&ctx, actor_id, game_id, env_id, None).await?;

	// Already destroyed
	if actor.destroy_ts.is_some() {
		return Ok(json!({}));
	}

	ctx.signal(pegboard::workflows::actor::Destroy {
		override_kill_timeout_ms: query.override_kill_timeout,
	})
	.to_workflow::<pegboard::workflows::actor::Workflow>()
	.tag("actor_id", actor_id)
	.send()
	.await?;

	sub.next().await?;

	Ok(json!({}))
}

// MARK: POST /actors/{}/upgrade
pub async fn upgrade(
	ctx: Ctx<Auth>,
	actor_id: Uuid,
	body: models::ActorUpgradeActorRequest,
	query: GlobalQuery,
) -> GlobalResult<serde_json::Value> {
	let CheckOutput { game_id, env_id } = ctx
		.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query,
				allow_service_token: true,
				opt_auth: false,
			},
		)
		.await?;

	assert::actor_for_env(&ctx, actor_id, game_id, env_id, None).await?;

	let build = resolve_build(&ctx, game_id, env_id, body.build, body.build_tags.flatten()).await?;

	// TODO: Add back once we figure out how to cleanly handle if a wf is already complete when
	// upgrading
	// let mut sub = ctx
	// 	.subscribe::<pegboard::workflows::actor::UpgradeStarted>(("actor_id", actor_id))
	// 	.await?;

	ctx.signal(pegboard::workflows::actor::Upgrade {
		image_id: build.build_id,
	})
	.to_workflow::<pegboard::workflows::actor::Workflow>()
	.tag("actor_id", actor_id)
	.send()
	.await?;

	// sub.next().await?;

	Ok(json!({}))
}

// MARK: POST /actors/upgrade
pub async fn upgrade_all(
	ctx: Ctx<Auth>,
	body: models::ActorUpgradeAllActorsRequest,
	query: GlobalQuery,
) -> GlobalResult<models::ActorUpgradeAllActorsResponse> {
	let CheckOutput { game_id, env_id } = ctx
		.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query,
				allow_service_token: true,
				opt_auth: false,
			},
		)
		.await?;

	let tags = unwrap_with!(body.tags, API_BAD_BODY, error = "missing property `tags`");

	ensure_with!(
		tags.as_object().map(|x| x.len()).unwrap_or_default() <= 8,
		API_BAD_BODY,
		error = "Too many tags (max 8)."
	);

	let tags = unwrap_with!(
		serde_json::from_value::<HashMap<String, String>>(tags).ok(),
		API_BAD_BODY,
		error = "`tags` must be `Map<String, String>`"
	);

	for (k, v) in &tags {
		ensure_with!(
			!k.is_empty(),
			API_BAD_BODY,
			error = "tags[]: Tag label cannot be empty."
		);
		ensure_with!(
			k.len() <= 32,
			API_BAD_BODY,
			error = format!(
				"tags[{:?}]: Tag label too large (max 32 bytes).",
				util::safe_slice(k, 0, 32),
			),
		);
		ensure_with!(
			!v.is_empty(),
			API_BAD_BODY,
			error = format!("tags[{k:?}]: Tag value cannot be empty.")
		);
		ensure_with!(
			v.len() <= 1024,
			API_BAD_BODY,
			error = format!("tags[{k:?}]: Tag value too large (max 1024 bytes)."),
		);
	}

	let build = resolve_build(&ctx, game_id, env_id, body.build, body.build_tags.flatten()).await?;

	// Work in batches
	let mut count = 0;
	let mut created_before = None;
	loop {
		let list_res = ctx
			.op(pegboard::ops::actor::list_for_env::Input {
				env_id,
				tags: tags.clone(),
				include_destroyed: false,
				created_before,
				limit: 10_000,
			})
			.await?;

		count += list_res.actors.len();

		// TODO: Subtracting a ms might skip an actor in a rare edge case, need to build compound
		// cursor of [created_at, actor_id] that we pass to the fdb range
		created_before = list_res.actors.last().map(|x| x.create_ts - 1);

		// TODO: Add back once we figure out how to cleanly handle if a wf is already complete when
		// upgrading
		// let subs = futures_util::stream::iter(list_res.actor_ids.clone())
		// 	.map(|actor_id| {
		// 		ctx.subscribe::<pegboard::workflows::actor::UpgradeStarted>(("actor_id", actor_id))
		// 	})
		// 	.buffer_unordered(32)
		// 	.try_collect::<Vec<_>>()
		// 	.await?;

		futures_util::stream::iter(list_res.actors)
			.map(|actor| {
				ctx.signal(pegboard::workflows::actor::Upgrade {
					image_id: build.build_id,
				})
				.to_workflow::<pegboard::workflows::actor::Workflow>()
				.tag("actor_id", actor.actor_id)
				.send()
			})
			.buffer_unordered(32)
			.try_collect::<Vec<_>>()
			.await?;

		// futures_util::stream::iter(subs)
		// 	.map(|mut sub| async move { sub.next().await })
		// 	.buffer_unordered(32)
		// 	.try_collect::<Vec<_>>()
		// 	.await?;

		if count < 10_000 {
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
	global_endpoint_type: GlobalEndpointTypeQuery,
	tags_json: Option<String>,
	include_destroyed: Option<bool>,
	/// Before create timestamp
	cursor: Option<String>,
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
	let CheckOutput { env_id, .. } = ctx
		.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query.global_endpoint_type.global,
				allow_service_token: true,
				opt_auth: false,
			},
		)
		.await?;

	let created_before = query.cursor.map(|x| x.parse::<i64>()).transpose()?;

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
		.op(pegboard::ops::actor::list_for_env::Input {
			env_id,
			tags,
			include_destroyed,
			created_before,
			limit: 32,
		})
		.await?;

	let mut actors_res = ctx
		.op(pegboard::ops::actor::get::Input {
			actor_ids: list_res.actors.iter().map(|x| x.actor_id).collect(),
			endpoint_type: query
				.global_endpoint_type
				.endpoint_type
				.map(ApiInto::api_into),
		})
		.await?;
	actors_res.actors.sort_by_key(|x| -x.create_ts);

	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;
	let dc_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![dc_id],
		})
		.await?;
	let dc = unwrap!(dc_res.datacenters.first());

	// TODO: Subtracting a ms might skip an actor in a rare edge case, need to build compound
	// cursor of [created_at, actor_id] that we pass to the fdb range
	let cursor = actors_res
		.actors
		.last()
		.map(|x| (x.create_ts - 1).to_string());

	let actors = actors_res
		.actors
		.into_iter()
		.map(|a| pegboard::types::convert_actor_to_api(a, &dc))
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::ActorListActorsResponse {
		actors: actors,
		pagination: Box::new(models::Pagination { cursor }),
	})
}

async fn resolve_build(
	ctx: &Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	build_id: Option<Uuid>,
	build_tags: Option<serde_json::Value>,
) -> GlobalResult<build::types::Build> {
	match (build_id, build_tags) {
		(Some(build_id), None) => {
			let builds_res = ctx
				.op(build::ops::get::Input {
					build_ids: vec![build_id],
				})
				.await?;
			let build = unwrap_with!(builds_res.builds.into_iter().next(), BUILD_NOT_FOUND);

			// Ensure build belongs to this game/env
			if let Some(build_game_id) = build.game_id {
				ensure_with!(build_game_id == game_id, BUILD_NOT_FOUND);
			} else if let Some(build_env_id) = build.env_id {
				ensure_with!(build_env_id == env_id, BUILD_NOT_FOUND);
			}

			Ok(build)
		}
		// Resolve build from tags
		(None, Some(build_tags)) => {
			let build_tags = unwrap_with!(
				serde_json::from_value::<HashMap<String, String>>(build_tags).ok(),
				API_BAD_BODY,
				error = "`build_tags` must be `Map<String, String>`"
			);

			ensure_with!(
				build_tags.len() < 8,
				API_BAD_BODY,
				error = "Too many build tags (max 8)."
			);

			for (k, v) in &build_tags {
				ensure_with!(
					!k.is_empty(),
					API_BAD_BODY,
					error = "build_tags[]: Build tag label cannot be empty."
				);
				ensure_with!(
					k.len() < 32,
					API_BAD_BODY,
					error = format!(
						"build_tags[{:?}]: Build tag label too large (max 32 bytes).",
						util::safe_slice(k, 0, 32),
					)
				);
				ensure_with!(
					!v.is_empty(),
					API_BAD_BODY,
					error = format!("build_tags[{k:?}]: Build tag value cannot be empty.")
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

			let build = unwrap_with!(
				builds_res.builds.into_iter().next(),
				BUILD_NOT_FOUND_WITH_TAGS
			);

			// Ensure build belongs to this game/env
			if let Some(build_game_id) = build.game_id {
				ensure_with!(build_game_id == game_id, BUILD_NOT_FOUND);
			} else if let Some(build_env_id) = build.env_id {
				ensure_with!(build_env_id == env_id, BUILD_NOT_FOUND);
			}

			Ok(build)
		}
		_ => {
			bail_with!(
				API_BAD_BODY,
				error = "must have either `build` or `build_tags`"
			);
		}
	}
}
