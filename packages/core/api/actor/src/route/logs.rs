use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use futures_util::StreamExt;
use rivet_api::{
	apis::{actor_logs_api, configuration::Configuration},
	models,
};
use rivet_operation::prelude::*;
use serde::Deserialize;

use crate::{
	auth::{Auth, CheckOpts, CheckOutput},
	utils::build_global_query_compat,
};

use super::GlobalQuery;

// MARK: GET /actors/{}/logs
#[derive(Debug, Deserialize)]
pub struct GetActorLogsQuery {
	#[serde(flatten)]
	pub global: GlobalQuery,
	pub stream: models::ActorLogStream,
}

pub async fn get_logs(
	ctx: Ctx<Auth>,
	server_id: Uuid,
	watch_index: WatchIndexQuery,
	query: GetActorLogsQuery,
) -> GlobalResult<models::ActorGetActorLogsResponse> {
	let CheckOutput { game_id, .. } = ctx
		.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query.global,
				allow_service_token: false,
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

	// Query every datacenter for the given actor
	let mut futures = dcs_res
		.datacenters
		.into_iter()
		.map(|dc| async {
			let dc = dc;

			let config = Configuration {
				base_path: ctx.config().server()?.rivet.edge_api_url_str(&dc.name_id)?,
				bearer_access_token: ctx.auth().api_token.clone(),
				..Default::default()
			};

			// Pass the request to the edge api
			use actor_logs_api::ActorLogsGetError::*;
			match actor_logs_api::actor_logs_get(
				&config,
				&server_id.to_string(),
				query.stream,
				query.global.project.as_deref(),
				query.global.environment.as_deref(),
				watch_index.watch_index.as_deref(),
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
					_ => bail!("unknown error"),
				},
				Err(err) => bail!("request error: {err:?}"),
			}
		})
		.collect::<futures_util::stream::FuturesUnordered<_>>();
	let mut first_error = None;

	// Return first api response that succeeds
	while let Some(result) = futures.next().await {
		match result {
			Ok(value) => return Ok(value),
			Err(err) => {
				if first_error.is_none() {
					first_error = Some(err);
				}
			}
		}
	}

	// Otherwise return the first error
	Err(unwrap!(first_error))
}

pub async fn get_logs_deprecated(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	server_id: Uuid,
	watch_index: WatchIndexQuery,
	query: GetActorLogsQuery,
) -> GlobalResult<models::ServersGetServerLogsResponse> {
	let global = build_global_query_compat(&ctx, game_id, env_id).await?;
	let logs_res = get_logs(
		ctx,
		server_id,
		watch_index,
		GetActorLogsQuery {
			global,
			stream: query.stream,
		},
	)
	.await?;
	Ok(models::ServersGetServerLogsResponse {
		lines: logs_res.lines,
		timestamps: logs_res.timestamps,
		watch: logs_res.watch,
	})
}
