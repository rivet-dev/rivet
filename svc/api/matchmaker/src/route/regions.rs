use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend;
use rivet_matchmaker_server::models;
use rivet_operation::prelude::*;
use std::collections::HashSet;

use crate::{auth::Auth, fetch::game::fetch_ns, utils};

// MARK: GET /regions
pub async fn list(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::ListRegionsResponse> {
	let coords = ctx.coords();

	// Mock response
	if ctx.auth().game_ns_dev_option()?.is_some() {
		return Ok(models::ListRegionsResponse {
			regions: vec![models::RegionInfo {
				region_id: util_mm::consts::DEV_REGION_ID.into(),
				provider_display_name: util_mm::consts::DEV_PROVIDER_NAME.into(),
				region_display_name: util_mm::consts::DEV_REGION_NAME.into(),
				datacenter_coord: models::Coord {
					latitude: 0.0,
					longitude: 0.0,
				},
				datacenter_distance_from_client: models::Distance {
					kilometers: 0.0,
					miles: 0.0,
				},
			}],
		});
	}

	let game_ns = ctx.auth().game_ns(&ctx).await?;

	let ns_data = fetch_ns(&ctx, &game_ns).await?;

	// Fetch version data
	let version_res = op!([ctx] mm_config_version_get {
		version_ids: vec![ns_data.version_id.into()],
	})
	.await?;
	let version_data = unwrap!(version_res.versions.first());
	let version_config = unwrap_ref!(version_data.config);

	// Find all enabled region IDs in all requested lobby groups
	let enabled_region_ids = version_config
		.lobby_groups
		.iter()
		.flat_map(|lg| {
			lg.regions
				.iter()
				.filter_map(|r| r.region_id.as_ref())
				.map(common::Uuid::as_uuid)
				.collect::<Vec<_>>()
		})
		.collect::<HashSet<Uuid>>()
		.into_iter()
		.map(Into::<common::Uuid>::into)
		.collect::<Vec<_>>();

	let (region_res, recommend_res) = tokio::try_join!(
		// List regions
		op!([ctx] region_get {
			region_ids: enabled_region_ids.clone(),
		}),
		// Fetch recommended region if coords are provided
		async {
			if let Some((lat, long)) = coords {
				let res = op!([ctx] region_recommend {
					region_ids: enabled_region_ids.clone(),
					coords: Some(backend::net::Coordinates {
						latitude: lat,
						longitude: long,
					}),
					..Default::default()
				})
				.await?;
				GlobalResult::Ok(Some(res))
			} else {
				Ok(None)
			}
		}
	)?;

	let regions = region_res
		.regions
		.iter()
		.map(|region| {
			let recommend = if let Some(res) = &recommend_res {
				Some(unwrap!(res
					.regions
					.iter()
					.find(|recommend| recommend.region_id == region.region_id)))
			} else {
				None
			};

			utils::build_region(region, recommend)
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::ListRegionsResponse { regions })
}
