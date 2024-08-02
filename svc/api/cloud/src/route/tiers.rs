use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: GET /region-tiers
pub async fn list_tiers(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::CloudGetRegionTiersResponse> {
	let datacenters_res = ctx.op(cluster::ops::datacenter::list::Input {
		cluster_ids: vec![cluster::util::default_cluster_id()],
	}).await?;
	let cluster = unwrap!(datacenters_res.clusters.first());

	let res = op!([ctx] tier_list {
		region_ids: cluster.datacenter_ids
			.iter()
			.cloned()
			.map(Into::into)
			.collect::<Vec<_>>(),
	})
	.await?;

	let region = unwrap!(res.regions.first());

	Ok(models::CloudGetRegionTiersResponse {
		tiers: region
			.tiers
			.clone()
			.into_iter()
			.map(ApiTryInto::api_try_into)
			.collect::<GlobalResult<Vec<_>>>()?,
	})
}
