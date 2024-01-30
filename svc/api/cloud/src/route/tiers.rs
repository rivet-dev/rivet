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
	// TODO: fill in user regions. `region_ids` doesn't actually do anything for now so its not important
	let res = op!([ctx] tier_list {
		region_ids: vec![Uuid::new_v4().into()],
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
