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
#[tracing::instrument(skip_all)]
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

	// Filter the datacenters that can be contacted
	let regions = dcs_res
		.datacenters
		.into_iter()
		.filter(|dc| crate::utils::filter_edge_dc(ctx.config(), dc).unwrap_or(false))
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

	// Filter the datacenters that can be contacted
	let datacenters = dcs_res
		.datacenters
		.into_iter()
		.filter(|dc| crate::utils::filter_edge_dc(ctx.config(), dc).unwrap_or(false))
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

#[tracing::instrument(skip_all)]
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

	// Get all datacenters
	let dcs_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: cluster_dcs.clone(),
		})
		.await?;

	// Filter the datacenters that can be contacted
	let filtered_dcs = dcs_res
		.datacenters
		.into_iter()
		.filter(|dc| crate::utils::filter_edge_dc(ctx.config(), dc).unwrap_or(false))
		.collect::<Vec<_>>();

	if filtered_dcs.is_empty() {
		bail!("no valid datacenters with worker and guard pools");
	}

	// Get recommended dc
	let dc = if let Some((lat, long)) = coords {
		let recommend_res = op!([ctx] region_recommend {
			region_ids: filtered_dcs
				.iter()
				.map(|dc| dc.datacenter_id)
				.map(Into::into)
				.collect(),
			coords: Some(backend::net::Coordinates {
				latitude: lat,
				longitude: long,
			}),
			..Default::default()
		})
		.await?;

		if recommend_res.regions.is_empty() {
			bail!("no regions found");
		}

		let region = unwrap!(recommend_res.regions.first());
		let datacenter_id = unwrap_ref!(region.region_id).as_uuid();

		// Find the datacenter in our filtered list
		filtered_dcs
			.iter()
			.find(|dc| dc.datacenter_id == datacenter_id)
			.cloned()
			.unwrap_or_else(|| filtered_dcs.first().unwrap().clone())
	} else {
		tracing::warn!("coords not provided to select region");
		filtered_dcs.first().unwrap().clone()
	};

	Ok(models::RegionsRecommendRegionResponse {
		region: Box::new(models::RegionsRegion {
			id: dc.name_id,
			name: dc.display_name,
		}),
	})
}
