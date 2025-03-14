use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;
use serde::Deserialize;

use crate::{
	auth::{Auth, CheckOpts, CheckOutput},
	utils::build_global_query_compat,
};

use super::GlobalQuery;

// MARK: GET /regions
pub async fn list(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: GlobalQuery,
) -> GlobalResult<models::RegionsListRegionsResponse> {
	let CheckOutput { game_id, .. } = ctx
		.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query,
				allow_service_token: true,
				opt_auth: true,
			},
		)
		.await?;

	let cluster_res = ctx
		.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		})
		.await?;
	let cluster_id = unwrap!(cluster_res.games.first()).cluster_id;

	let cluster_dcs_res = ctx
		.op(cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster_id],
		})
		.await?;
	let cluster_dcs = unwrap!(cluster_dcs_res.clusters.first())
		.datacenter_ids
		.clone();

	let mut dcs_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: cluster_dcs,
		})
		.await?;
	dcs_res.datacenters.sort_by_key(|x| x.name_id.clone());

	let regions = dcs_res
		.datacenters
		.into_iter()
		.map(|dc| models::RegionsRegion {
			id: dc.name_id,
			name: dc.display_name,
		})
		.collect::<Vec<_>>();

	Ok(models::RegionsListRegionsResponse { regions })
}

pub async fn list_deprecated(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::ServersListDatacentersResponse> {
	let query = build_global_query_compat(&ctx, game_id, env_id).await?;
	let CheckOutput { game_id, .. } = ctx
		.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query,
				allow_service_token: true,
				opt_auth: true,
			},
		)
		.await?;

	let cluster_res = ctx
		.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		})
		.await?;
	let cluster_id = unwrap!(cluster_res.games.first()).cluster_id;

	let cluster_dcs_res = ctx
		.op(cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster_id],
		})
		.await?;
	let cluster_dcs = unwrap!(cluster_dcs_res.clusters.first())
		.datacenter_ids
		.clone();

	let mut dcs_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: cluster_dcs,
		})
		.await?;
	dcs_res.datacenters.sort_by_key(|x| x.name_id.clone());

	let datacenters = dcs_res
		.datacenters
		.into_iter()
		.map(|dc| models::ServersDatacenter {
			id: dc.datacenter_id,
			slug: dc.name_id,
			name: dc.display_name,
		})
		.collect::<Vec<_>>();

	Ok(models::ServersListDatacentersResponse { datacenters })
}

// MARK: GET /regions/recommend
#[derive(Debug, Clone, Deserialize)]
pub struct RecommendQuery {
	#[serde(flatten)]
	global: GlobalQuery,
	lat: Option<f64>,
	long: Option<f64>,
}

pub async fn recommend(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: RecommendQuery,
) -> GlobalResult<models::RegionsRecommendRegionResponse> {
	let CheckOutput { game_id, .. } = ctx
		.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query.global,
				allow_service_token: true,
				opt_auth: true,
			},
		)
		.await?;

	// Resolve coords
	let coords = match (query.lat, query.long) {
		(Some(lat), Some(long)) => Some((lat, long)),
		(None, None) => ctx.coords(),
		_ => bail_with!(API_BAD_QUERY, error = "must have both `lat` and `long`"),
	};

	let cluster_res = ctx
		.op(cluster::ops::get_for_game::Input {
			game_ids: vec![game_id],
		})
		.await?;
	let cluster_id = unwrap!(cluster_res.games.first()).cluster_id;

	let cluster_dcs_res = ctx
		.op(cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster_id],
		})
		.await?;
	let cluster_dcs = &unwrap!(cluster_dcs_res.clusters.first()).datacenter_ids;

	// Get recommended dc
	let datacenter_id = if let Some((lat, long)) = coords {
		let recommend_res = op!([ctx] region_recommend {
			region_ids: cluster_dcs
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
		let region = unwrap!(recommend_res.regions.first());

		unwrap_ref!(region.region_id).as_uuid()
	} else {
		tracing::warn!("coords not provided to select region");

		*unwrap!(cluster_dcs.first())
	};

	// Fetch dc
	let dcs_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![datacenter_id],
		})
		.await?;
	let dc = unwrap!(dcs_res.datacenters.into_iter().next());

	Ok(models::RegionsRecommendRegionResponse {
		region: Box::new(models::RegionsRegion {
			id: dc.name_id,
			name: dc.display_name,
		}),
	})
}
