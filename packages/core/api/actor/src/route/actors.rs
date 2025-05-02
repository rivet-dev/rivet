use std::{
	collections::{HashMap, HashSet},
	time::Duration,
};

use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use futures_util::{StreamExt, TryStreamExt};
use proto::backend;
use rivet_api::{
	apis::{actors_api, configuration::Configuration},
	models,
};
use rivet_operation::prelude::*;
use serde::Deserialize;
use serde_json::json;
use tracing::Instrument;

use crate::{
	auth::{Auth, CheckOpts, CheckOutput},
	utils::build_global_query_compat,
};

use super::GlobalQuery;

#[derive(Debug, Clone, Deserialize)]
pub struct GlobalEndpointTypeQuery {
	#[serde(flatten)]
	global: GlobalQuery,
	endpoint_type: Option<models::ActorsEndpointType>,
}

// MARK: GET /actors/{}
#[tracing::instrument(skip_all)]
pub async fn get(
	ctx: Ctx<Auth>,
	actor_id: Uuid,
	watch_index: WatchIndexQuery,
	query: GlobalEndpointTypeQuery,
) -> GlobalResult<models::ActorsGetActorResponse> {
	get_inner(&ctx, actor_id, watch_index, query).await
}

async fn get_inner(
	ctx: &Ctx<Auth>,
	actor_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: GlobalEndpointTypeQuery,
) -> GlobalResult<models::ActorsGetActorResponse> {
	let CheckOutput { game_id, .. } = ctx
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

	// Fetch all datacenters
	let clusters_res = ctx
		.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		})
		.await?;
	let cluster_id = unwrap!(clusters_res.games.first()).cluster_id;
	let dc_list_res = ctx
		.op(cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster_id],
		})
		.await?;
	let cluster = unwrap!(dc_list_res.clusters.into_iter().next());
	let dcs_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: cluster.datacenter_ids,
		})
		.await?;

	// Filter the datacenters that can be contacted
	let filtered_datacenters = dcs_res
		.datacenters
		.into_iter()
		.filter(|dc| crate::utils::filter_edge_dc(ctx.config(), dc).unwrap_or(false))
		.collect::<Vec<_>>();

	if filtered_datacenters.is_empty() {
		bail!("no valid datacenters with worker and guard pools");
	}

	// Query every datacenter for the given actor
	let mut futures = filtered_datacenters
		.into_iter()
		.map(|dc| async {
			let dc = dc;

			let config = Configuration {
				client: rivet_pools::reqwest::client().await?,
				base_path: ctx.config().server()?.rivet.edge_api_url_str(&dc.name_id)?,
				bearer_access_token: ctx.auth().api_token.clone(),
				..Default::default()
			};

			// Pass the request to the edge api
			use actors_api::ActorsGetError::*;
			match actors_api::actors_get(
				&config,
				&actor_id.to_string(),
				query.global.project.as_deref(),
				query.global.environment.as_deref(),
				query.endpoint_type,
			)
			.await
			{
				Ok(res) => Ok(res),
				Err(rivet_api::apis::Error::ResponseError(content)) => match content.entity {
					Some(Status400(body))
					| Some(Status403(body))
					| Some(Status404(body))
					| Some(Status408(body))
					| Some(Status429(body))
					| Some(Status500(body)) => Err(GlobalError::bad_request_builder(&body.code)
						.http_status(content.status)
						.message(body.message)
						.build()),
					err => bail!("unknown error: {err:?}"),
				},
				Err(err) => bail!("request error: {err:?}"),
			}
		})
		.collect::<futures_util::stream::FuturesUnordered<_>>();
	let mut last_error = None;

	// Return first api response that succeeds
	while let Some(result) = futures.next().await {
		match result {
			Ok(value) => return Ok(value),
			Err(err) => last_error = Some(err),
		}
	}

	// Otherwise return the last error
	Err(unwrap!(last_error))
}

pub async fn get_deprecated(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	actor_id: Uuid,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::ServersGetServerResponse> {
	let global = build_global_query_compat(&ctx, game_id, env_id).await?;
	let get_res = get_inner(
		&ctx,
		actor_id,
		watch_index,
		GlobalEndpointTypeQuery {
			global,
			endpoint_type: None,
		},
	)
	.await?;

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
		server: Box::new(legacy_convert_actor_to_server(*get_res.actor, dc)?),
	})
}

// MARK: POST /actors
#[tracing::instrument(skip_all)]
pub async fn create(
	ctx: Ctx<Auth>,
	body: models::ActorsCreateActorRequest,
	query: GlobalEndpointTypeQuery,
) -> GlobalResult<models::ActorsCreateActorResponse> {
	let CheckOutput { game_id, .. } = ctx
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

	let clusters_res = ctx
		.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		})
		.await?;
	let cluster_id = unwrap!(clusters_res.games.first()).cluster_id;
	let dc_name_id = resolve_dc(&ctx, cluster_id, body.region.clone()).await?;

	let config = Configuration {
		client: rivet_pools::reqwest::client().await?,
		base_path: ctx.config().server()?.rivet.edge_api_url_str(&dc_name_id)?,
		bearer_access_token: ctx.auth().api_token.clone(),
		..Default::default()
	};

	// Pass the request to the edge api
	use actors_api::ActorsCreateError::*;
	match actors_api::actors_create(
		&config,
		body,
		query.global.project.as_deref(),
		query.global.environment.as_deref(),
		query.endpoint_type,
	)
	.instrument(tracing::info_span!("proxy request"))
	.await
	{
		Ok(res) => Ok(res),
		Err(rivet_api::apis::Error::ResponseError(content)) => match content.entity {
			Some(Status400(body))
			| Some(Status403(body))
			| Some(Status404(body))
			| Some(Status408(body))
			| Some(Status429(body))
			| Some(Status500(body)) => Err(GlobalError::bad_request_builder(&body.code)
				.http_status(content.status)
				.message(body.message)
				.build()),
			_ => bail!("unknown error: {:?} {:?}", content.status, content.content),
		},
		Err(err) => bail!("request error: {err:?}"),
	}
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
		models::ActorsCreateActorRequest {
			region: Some(dc.name_id.clone()),
			lifecycle: body.lifecycle.map(|l| {
				Box::new(models::ActorsLifecycle {
					kill_timeout: l.kill_timeout,
					durable: Some(false),
				})
			}),
			network: Some(Box::new(models::ActorsCreateActorNetworkRequest {
				mode: body.network.mode.map(|n| match n {
					models::ServersNetworkMode::Host => models::ActorsNetworkMode::Host,
					models::ServersNetworkMode::Bridge => models::ActorsNetworkMode::Bridge,
				}),
				ports: Some(
					body.network
						.ports
						.into_iter()
						.map(|(k, p)| {
							(
								k,
								models::ActorsCreateActorPortRequest {
									internal_port: p.internal_port,
									protocol: match p.protocol {
										models::ServersPortProtocol::Http => {
											models::ActorsPortProtocol::Http
										}
										models::ServersPortProtocol::Https => {
											models::ActorsPortProtocol::Https
										}
										models::ServersPortProtocol::Tcp => {
											models::ActorsPortProtocol::Tcp
										}
										models::ServersPortProtocol::TcpTls => {
											models::ActorsPortProtocol::TcpTls
										}
										models::ServersPortProtocol::Udp => {
											models::ActorsPortProtocol::Udp
										}
									},
									routing: p.routing.map(|r| {
										Box::new(models::ActorsPortRouting {
											// Temporarily disabled
											// guard: r.game_guard.map(|_| {
											// 	Box::new(models::ActorsGuardRouting::default())
											// }),
											guard: r.game_guard.map(|_| json!({})),
											host: r.host.map(|_| json!({})),
										})
									}),
								},
							)
						})
						.collect(),
				),
				wait_ready: None,
			})),
			resources: Some(Box::new(models::ActorsResources {
				cpu: body.resources.cpu,
				memory: body.resources.memory,
			})),
			runtime: Some(Box::new(models::ActorsCreateActorRuntimeRequest {
				environment: body.runtime.environment,
				network: None,
			})),
			build: Some(body.runtime.build),
			build_tags: None,
			tags: body.tags,
		},
		GlobalEndpointTypeQuery {
			global,
			endpoint_type: None,
		},
	)
	.await?;

	Ok(models::ServersCreateServerResponse {
		server: Box::new(legacy_convert_actor_to_server(*create_res.actor, &dc)?),
	})
}

// MARK: DELETE /actors/{}
#[derive(Debug, Clone, Deserialize)]
pub struct DeleteQuery {
	#[serde(flatten)]
	global: GlobalQuery,
	override_kill_timeout: Option<i64>,
}

#[tracing::instrument(skip_all)]
pub async fn destroy(
	ctx: Ctx<Auth>,
	actor_id: Uuid,
	query: DeleteQuery,
) -> GlobalResult<serde_json::Value> {
	let CheckOutput { game_id, .. } = ctx
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

	// Fetch all datacenters
	let clusters_res = ctx
		.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		})
		.await?;
	let cluster_id = unwrap!(clusters_res.games.first()).cluster_id;
	let dc_list_res = ctx
		.op(cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster_id],
		})
		.await?;
	let cluster = unwrap!(dc_list_res.clusters.into_iter().next());
	let dcs_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: cluster.datacenter_ids,
		})
		.await?;

	// Filter the datacenters that can be contacted
	let filtered_datacenters = dcs_res
		.datacenters
		.into_iter()
		.filter(|dc| crate::utils::filter_edge_dc(ctx.config(), dc).unwrap_or(false))
		.collect::<Vec<_>>();

	if filtered_datacenters.is_empty() {
		bail!("no valid datacenters with worker and guard pools");
	}

	// Query every datacenter
	let mut futures = filtered_datacenters
		.into_iter()
		.map(|dc| async {
			let dc = dc;

			let config = Configuration {
				client: rivet_pools::reqwest::client().await?,
				base_path: ctx.config().server()?.rivet.edge_api_url_str(&dc.name_id)?,
				bearer_access_token: ctx.auth().api_token.clone(),
				..Default::default()
			};

			// Pass the request to the edge api
			use actors_api::ActorsDestroyError::*;
			match actors_api::actors_destroy(
				&config,
				&actor_id.to_string(),
				query.global.project.as_deref(),
				query.global.environment.as_deref(),
				query.override_kill_timeout,
			)
			.instrument(tracing::info_span!("proxy request"))
			.await
			{
				Ok(res) => Ok(res),
				Err(rivet_api::apis::Error::ResponseError(content)) => match content.entity {
					Some(Status400(body))
					| Some(Status403(body))
					| Some(Status404(body))
					| Some(Status408(body))
					| Some(Status429(body))
					| Some(Status500(body)) => Err(GlobalError::bad_request_builder(&body.code)
						.http_status(content.status)
						.message(body.message)
						.build()),
					err => bail!("unknown error: {err:?}"),
				},
				Err(err) => bail!("request error: {err:?}"),
			}
		})
		.collect::<futures_util::stream::FuturesUnordered<_>>();
	let mut last_error = None;

	// Return first api response that succeeds
	while let Some(result) = futures.next().await {
		match result {
			Ok(value) => return Ok(value),
			Err(err) => last_error = Some(err),
		}
	}

	// Otherwise return the last error
	Err(unwrap!(last_error))
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
#[tracing::instrument(skip_all)]
pub async fn upgrade(
	ctx: Ctx<Auth>,
	actor_id: Uuid,
	body: models::ActorsUpgradeActorRequest,
	query: GlobalQuery,
) -> GlobalResult<serde_json::Value> {
	let CheckOutput { game_id, .. } = ctx
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

	// Fetch all datacenters
	let clusters_res = ctx
		.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		})
		.await?;
	let cluster_id = unwrap!(clusters_res.games.first()).cluster_id;
	let dc_list_res = ctx
		.op(cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster_id],
		})
		.await?;
	let cluster = unwrap!(dc_list_res.clusters.into_iter().next());
	let dcs_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: cluster.datacenter_ids,
		})
		.await?;

	// Filter the datacenters that can be contacted
	let filtered_datacenters = dcs_res
		.datacenters
		.into_iter()
		.filter(|dc| crate::utils::filter_edge_dc(ctx.config(), dc).unwrap_or(false))
		.collect::<Vec<_>>();

	if filtered_datacenters.is_empty() {
		bail!("no valid datacenters with worker and guard pools");
	}

	// Query every datacenter
	let mut futures = filtered_datacenters
		.into_iter()
		.map(|dc| async {
			let dc = dc;

			let config = Configuration {
				client: rivet_pools::reqwest::client().await?,
				base_path: ctx.config().server()?.rivet.edge_api_url_str(&dc.name_id)?,
				bearer_access_token: ctx.auth().api_token.clone(),
				..Default::default()
			};

			// Pass the request to the edge api
			use actors_api::ActorsUpgradeError::*;
			match actors_api::actors_upgrade(
				&config,
				&actor_id.to_string(),
				body.clone(),
				query.project.as_deref(),
				query.environment.as_deref(),
			)
			.instrument(tracing::info_span!("proxy request"))
			.await
			{
				Ok(res) => Ok(res),
				Err(rivet_api::apis::Error::ResponseError(content)) => match content.entity {
					Some(Status400(body))
					| Some(Status403(body))
					| Some(Status404(body))
					| Some(Status408(body))
					| Some(Status429(body))
					| Some(Status500(body)) => Err(GlobalError::bad_request_builder(&body.code)
						.http_status(content.status)
						.message(body.message)
						.build()),
					err => bail!("unknown error: {err:?}"),
				},
				Err(err) => bail!("request error: {err:?}"),
			}
		})
		.collect::<futures_util::stream::FuturesUnordered<_>>();
	let mut last_error = None;

	// Return first api response that succeeds
	while let Some(result) = futures.next().await {
		match result {
			Ok(value) => return Ok(value),
			Err(err) => last_error = Some(err),
		}
	}

	// Otherwise return the last error
	Err(unwrap!(last_error))
}

// MARK: POST /actors/upgrade
#[tracing::instrument(skip_all)]
pub async fn upgrade_all(
	ctx: Ctx<Auth>,
	body: models::ActorsUpgradeAllActorsRequest,
	query: GlobalQuery,
) -> GlobalResult<models::ActorsUpgradeAllActorsResponse> {
	let CheckOutput { game_id, .. } = ctx
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

	let tags = unwrap_with!(&body.tags, API_BAD_BODY, error = "missing property `tags`");

	ensure_with!(
		tags.as_object().map(|x| x.len()).unwrap_or_default() <= 8,
		API_BAD_BODY,
		error = "Too many tags (max 8)."
	);

	let tags = unwrap_with!(
		serde_json::from_value::<HashMap<String, String>>(tags.clone()).ok(),
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

	// Fetch all datacenters
	let clusters_res = ctx
		.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		})
		.await?;
	let cluster_id = unwrap!(clusters_res.games.first()).cluster_id;
	let dc_list_res = ctx
		.op(cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster_id],
		})
		.await?;
	let cluster = unwrap!(dc_list_res.clusters.into_iter().next());
	let dcs_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: cluster.datacenter_ids,
		})
		.await?;

	// Filter the datacenters that can be contacted
	let filtered_datacenters = dcs_res
		.datacenters
		.into_iter()
		.filter(|dc| crate::utils::filter_edge_dc(ctx.config(), dc).unwrap_or(false))
		.collect::<Vec<_>>();

	if filtered_datacenters.is_empty() {
		bail!("no valid datacenters with worker and guard pools");
	}

	// Query every datacenter
	let futures = filtered_datacenters
		.into_iter()
		.map(|dc| async {
			let dc = dc;

			let config = Configuration {
				client: rivet_pools::reqwest::client().await?,
				base_path: ctx.config().server()?.rivet.edge_api_url_str(&dc.name_id)?,
				bearer_access_token: ctx.auth().api_token.clone(),
				..Default::default()
			};

			// Pass the request to the edge api
			use actors_api::ActorsUpgradeAllError::*;
			match actors_api::actors_upgrade_all(
				&config,
				body.clone(),
				query.project.as_deref(),
				query.environment.as_deref(),
			)
			.instrument(tracing::info_span!("proxy request"))
			.await
			{
				Ok(res) => Ok(res),
				Err(rivet_api::apis::Error::ResponseError(content)) => match content.entity {
					Some(Status400(body))
					| Some(Status403(body))
					| Some(Status404(body))
					| Some(Status408(body))
					| Some(Status429(body))
					| Some(Status500(body)) => Err(GlobalError::bad_request_builder(&body.code)
						.http_status(content.status)
						.message(body.message)
						.build()),
					err => bail!("unknown error: {err:?}"),
				},
				Err(err) => bail!("request error: {err:?}"),
			}
		})
		.collect::<Vec<_>>();

	// Aggregate results
	let count = futures_util::stream::iter(futures)
		.buffer_unordered(16)
		.try_fold(0, |a, res| std::future::ready(Ok(a + res.count)))
		.await?;

	Ok(models::ActorsUpgradeAllActorsResponse { count })
}

// MARK: GET /actors
#[derive(Debug, Clone, Deserialize)]
pub struct ListQuery {
	#[serde(flatten)]
	global_endpoint_type: GlobalEndpointTypeQuery,
	tags_json: Option<String>,
	include_destroyed: Option<bool>,
	cursor: Option<String>,
}

#[tracing::instrument(skip_all)]
pub async fn list_actors(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
	query: ListQuery,
) -> GlobalResult<models::ActorsListActorsResponse> {
	list_actors_inner(&ctx, watch_index, query).await
}

async fn list_actors_inner(
	ctx: &Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: ListQuery,
) -> GlobalResult<models::ActorsListActorsResponse> {
	let CheckOutput { game_id, .. } = ctx
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

	// Validate tags
	if let Some(tags) = &query.tags_json {
		let tags = unwrap_with!(
			serde_json::from_str::<HashMap<String, String>>(tags).ok(),
			API_BAD_QUERY_PARAMETER,
			parameter = "tags_json",
			error = "`tags` must be `Map<String, String>`"
		);

		ensure_with!(
			tags.len() <= 8,
			API_BAD_QUERY_PARAMETER,
			parameter = "tags_json",
			error = "Too many tags (max 8)."
		);

		for (k, v) in &tags {
			ensure_with!(
				!k.is_empty(),
				API_BAD_QUERY_PARAMETER,
				parameter = "tags_json",
				error = "tags_json[]: Tag label cannot be empty."
			);
			ensure_with!(
				k.len() <= 32,
				API_BAD_QUERY_PARAMETER,
				parameter = "tags_json",
				error = format!(
					"tags_json[{:?}]: Tag label too large (max 32 bytes).",
					util::safe_slice(k, 0, 32),
				),
			);
			ensure_with!(
				!v.is_empty(),
				API_BAD_QUERY_PARAMETER,
				parameter = "tags_json",
				error = format!("tags_json[{k:?}]: Tag value cannot be empty.")
			);
			ensure_with!(
				v.len() <= 1024,
				API_BAD_QUERY_PARAMETER,
				parameter = "tags_json",
				error = format!("tags_json[{k:?}]: Tag value too large (max 1024 bytes)."),
			);
		}
	}

	// Fetch all datacenters
	let clusters_res = ctx
		.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		})
		.await?;
	let cluster_id = unwrap!(clusters_res.games.first()).cluster_id;
	let dc_list_res = ctx
		.op(cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster_id],
		})
		.await?;
	let cluster = unwrap!(dc_list_res.clusters.into_iter().next());
	let dcs_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: cluster.datacenter_ids,
		})
		.await?;

	// Filter the datacenters that can be contacted
	let filtered_datacenters = dcs_res
		.datacenters
		.into_iter()
		.filter(|dc| crate::utils::filter_edge_dc(ctx.config(), dc).unwrap_or(false))
		.collect::<Vec<_>>();

	if filtered_datacenters.is_empty() {
		bail!("no valid datacenters with worker and guard pools");
	}

	// Query every datacenter
	let futures = filtered_datacenters
		.into_iter()
		.map(|dc| async {
			let dc = dc;

			let config = Configuration {
				client: rivet_pools::reqwest::client().await?,
				base_path: ctx.config().server()?.rivet.edge_api_url_str(&dc.name_id)?,
				bearer_access_token: ctx.auth().api_token.clone(),
				..Default::default()
			};

			// Pass the request to the edge api
			let timeout_res = tokio::time::timeout(
				Duration::from_secs(30),
				actors_api::actors_list(
					&config,
					query.global_endpoint_type.global.project.as_deref(),
					query.global_endpoint_type.global.environment.as_deref(),
					query.global_endpoint_type.endpoint_type,
					query.tags_json.as_deref(),
					query.include_destroyed,
					query.cursor.as_deref(),
				)
				.instrument(tracing::info_span!("proxy request")),
			)
			.await;

			use actors_api::ActorsListError::*;
			match timeout_res {
				Ok(timeout_res) => match timeout_res {
					Ok(res) => Ok(res),
					Err(rivet_api::apis::Error::ResponseError(content)) => match content.entity {
						Some(Status400(body))
						| Some(Status403(body))
						| Some(Status404(body))
						| Some(Status408(body))
						| Some(Status429(body))
						| Some(Status500(body)) => {
							return Err(GlobalError::bad_request_builder(&body.code)
								.http_status(content.status)
								.message(body.message)
								.build())
						}
						err => bail!("unknown error: {err:?}"),
					},
					Err(err) => bail!("request error: {err:?}"),
				},
				Err(_) => {
					tracing::error!(dc=?dc.name_id, "timed out requesting dc");
					bail_with!(API_REQUEST_TIMEOUT);
				}
			}
		})
		.collect::<Vec<_>>();

	let mut results = futures_util::stream::iter(futures)
		.buffer_unordered(16)
		.collect::<Vec<_>>()
		.await;

	// Aggregate results
	let mut actors = Vec::new();
	for res in &mut results {
		match res {
			Ok(res) => actors.extend(std::mem::take(&mut res.actors)),
			Err(err) => tracing::error!(?err, "failed to request edge dc"),
		}
	}

	// Error only if all requests failed
	if !results.is_empty() && results.iter().all(|res| res.is_err()) {
		return Err(unwrap!(unwrap!(results.into_iter().next()).err()));
	}

	// Sort by create ts desc
	//
	// This is an ISO 8601 string and is safely sortable
	actors.sort_by_cached_key(|x| std::cmp::Reverse(x.created_at.clone()));

	// Shorten array since returning all actors from all regions could end up returning `regions *
	// 32` results, which is a lot.
	actors.truncate(32);

	// TODO: Subtracting a ms might skip an actor in a rare edge case, need to build compound
	// cursor of [created_at, actor_id] that we pass to the fdb range
	let cursor = actors.last().map(|x| {
		let datetime = x
			.created_at
			.parse::<chrono::DateTime<chrono::Utc>>()
			.unwrap_or_default();
		let unix_ts = datetime.timestamp_millis() - 1;
		unix_ts.to_string()
	});

	Ok(models::ActorsListActorsResponse {
		actors,
		pagination: Box::new(models::Pagination { cursor }),
	})
}

pub async fn list_servers_deprecated(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	watch_index: WatchIndexQuery,
	query: ListQuery,
) -> GlobalResult<models::ServersListServersResponse> {
	let global = build_global_query_compat(&ctx, game_id, env_id).await?;
	let actors_res = list_actors_inner(
		&ctx,
		watch_index,
		ListQuery {
			global_endpoint_type: GlobalEndpointTypeQuery {
				global,
				..query.global_endpoint_type
			},
			..query
		},
	)
	.await?;

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
				legacy_convert_actor_to_server(a, dc)
			})
			.collect::<Result<Vec<_>, _>>()?,
	})
}

fn legacy_convert_actor_to_server(
	a: models::ActorsActor,
	datacenter: &cluster::types::Datacenter,
) -> GlobalResult<models::ServersServer> {
	Ok(models::ServersServer {
		created_at: a
			.created_at
			.parse::<chrono::DateTime<chrono::Utc>>()?
			.timestamp_millis(),
		datacenter: datacenter.datacenter_id,
		destroyed_at: a
			.destroyed_at
			.map(|ts| {
				GlobalResult::Ok(
					ts.parse::<chrono::DateTime<chrono::Utc>>()?
						.timestamp_millis(),
				)
			})
			.transpose()?,
		environment: Uuid::nil(),
		id: a.id,
		lifecycle: Box::new(models::ServersLifecycle {
			kill_timeout: a.lifecycle.kill_timeout,
		}),
		network: Box::new(models::ServersNetwork {
			mode: Some(match a.network.mode {
				models::ActorsNetworkMode::Host => models::ServersNetworkMode::Host,
				models::ActorsNetworkMode::Bridge => models::ServersNetworkMode::Bridge,
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
								models::ActorsPortProtocol::Http => {
									models::ServersPortProtocol::Http
								}
								models::ActorsPortProtocol::Https => {
									models::ServersPortProtocol::Https
								}
								models::ActorsPortProtocol::Tcp => models::ServersPortProtocol::Tcp,
								models::ActorsPortProtocol::TcpTls => {
									models::ServersPortProtocol::TcpTls
								}
								models::ActorsPortProtocol::Udp => models::ServersPortProtocol::Udp,
							},
							public_hostname: p.hostname,
							public_port: p.port,
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
		started_at: a
			.started_at
			.map(|ts| {
				GlobalResult::Ok(
					ts.parse::<chrono::DateTime<chrono::Utc>>()?
						.timestamp_millis(),
				)
			})
			.transpose()?,
		tags: a.tags,
	})
}

#[tracing::instrument(skip_all)]
async fn resolve_dc(
	ctx: &Ctx<Auth>,
	cluster_id: Uuid,
	region: Option<String>,
) -> GlobalResult<String> {
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

		Ok(dc.name_id.clone())
	}
	// Auto-select the closest region
	else {
		let clusters_res = ctx
			.op(cluster::ops::datacenter::list::Input {
				cluster_ids: vec![cluster_id],
			})
			.await?;
		let cluster = unwrap!(clusters_res.clusters.first());

		let datacenter_id = if let Some((lat, long)) = ctx.coords() {
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

			primary_region_id
		} else {
			tracing::warn!("coords not provided to select region");

			let datacenter_id = *unwrap_with!(
				cluster.datacenter_ids.first(),
				ACTOR_FAILED_TO_CREATE,
				error = "No regions found."
			);

			datacenter_id
		};

		let dc_res = ctx
			.op(cluster::ops::datacenter::get::Input {
				datacenter_ids: vec![datacenter_id],
			})
			.await?;
		let dc = unwrap_with!(
			dc_res.datacenters.first(),
			ACTOR_FAILED_TO_CREATE,
			error = "Region not found."
		);

		Ok(dc.name_id.clone())
	}
}
