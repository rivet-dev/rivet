use proto::backend::{self, pkg::*};
use rivet_matchmaker_server::models;
use rivet_operation::prelude::*;

pub fn build_region(
	region: &backend::region::Region,
	recommend: Option<&region::recommend::response::Region>,
) -> models::RegionInfo {
	models::RegionInfo {
		region_id: region.name_id.clone(),
		provider_display_name: region.provider_display_name.clone(),
		region_display_name: region.region_display_name.clone(),
		datacenter_coord: models::Coord {
			latitude: region.latitude,
			longitude: region.longitude,
		},
		datacenter_distance_from_client: if let Some(recommend) = recommend {
			models::Distance {
				kilometers: recommend.distance_meters / 1000.0,
				miles: util::geo::convert::kilometers_to_miles(recommend.distance_meters / 1000.0),
			}
		} else {
			models::Distance {
				kilometers: 0.0,
				miles: 0.0,
			}
		},
	}
}

pub fn build_region_openapi(
	region: &backend::region::Region,
	recommend: Option<&region::recommend::response::Region>,
) -> rivet_api::models::MatchmakerRegionInfo {
	rivet_api::models::MatchmakerRegionInfo {
		region_id: region.name_id.clone(),
		provider_display_name: region.provider_display_name.clone(),
		region_display_name: region.region_display_name.clone(),
		datacenter_coord: Box::new(rivet_api::models::GeoCoord {
			latitude: region.latitude,
			longitude: region.longitude,
		}),
		datacenter_distance_from_client: Box::new(if let Some(recommend) = recommend {
			rivet_api::models::GeoDistance {
				kilometers: recommend.distance_meters / 1000.0,
				miles: util::geo::convert::kilometers_to_miles(recommend.distance_meters / 1000.0),
			}
		} else {
			rivet_api::models::GeoDistance {
				kilometers: 0.0,
				miles: 0.0,
			}
		}),
	}
}
