use proto::backend::{
	self,
	pkg::{region::config_get::Region, *},
};
use rivet_operation::prelude::*;

fn universal_region(region: &Region) -> backend::region::UniversalRegion {
	use backend::region::UniversalRegion;

	match region.provider.as_str() {
		"local" => match region.provider_region.as_str() {
			"lcl1" => UniversalRegion::Local,
			_ => {
				tracing::error!(provider_region = ?region.provider_region, "unknown local region");
				UniversalRegion::Unknown
			}
		},
		"digitalocean" => match region.provider_region.as_str() {
			"ams1" | "ams2" | "ams3" => UniversalRegion::Amsterdam,
			"blr1" => UniversalRegion::Bangalore,
			"fra1" => UniversalRegion::Frankfurt,
			"lon1" => UniversalRegion::London,
			"nyc1" | "nyc2" | "nyc3" => UniversalRegion::NewYorkCity,
			"sfo1" | "sfo2" | "sfo3" => UniversalRegion::SanFrancisco,
			"sgp1" => UniversalRegion::Singapore,
			"tor1" => UniversalRegion::Toronto,
			_ => {
				tracing::error!(provider_region = ?region.provider_region, "unknown digitalocean region");
				UniversalRegion::Unknown
			}
		},
		"linode" => match region.provider_region.as_str() {
			"ap-west" => UniversalRegion::Mumbai,
			"ca-central" => UniversalRegion::Toronto,
			"ap-southeast" => UniversalRegion::Sydney,
			"us-central" => UniversalRegion::Dallas,
			"us-west" => UniversalRegion::SanFrancisco,
			"us-southeast" => UniversalRegion::Atlanta,
			"us-east" => UniversalRegion::NewYorkCity,
			"us-iad" => UniversalRegion::WashingtonDc,
			"eu-west" => UniversalRegion::London,
			"ap-south" => UniversalRegion::Singapore,
			"eu-central" => UniversalRegion::Frankfurt,
			"ap-northeast" => UniversalRegion::Tokyo,
			_ => {
				tracing::error!(provider_region = ?region.provider_region, "unknown linode region");
				UniversalRegion::Unknown
			}
		},
		_ => {
			tracing::error!(provider = ?region.provider, provider_region = ?region.provider_region, "unknown provider");
			UniversalRegion::Unknown
		}
	}
}

fn provider_display_name(region: &Region) -> &'static str {
	match region.provider.as_str() {
		"local" => "Local",
		"digitalocean" => "DigitalOcean",
		"linode" => "Linode",
		_ => "Unknown",
	}
}

/// See corresponding values in `region-resolve`.
// fn universal_region_short(universal_region: &backend::region::UniversalRegion) -> &'static str {
// 	use backend::region::UniversalRegion;

// 	match universal_region {
// 		UniversalRegion::Unknown => "ukn",
// 		UniversalRegion::Local => "lcl",

// 		UniversalRegion::Amsterdam => "ams",
// 		UniversalRegion::Atlanta => "atl",
// 		UniversalRegion::Bangalore => "blr",
// 		UniversalRegion::Dallas => "dfw",
// 		UniversalRegion::Frankfurt => "fra",
// 		UniversalRegion::London => "lon",
// 		UniversalRegion::Mumbai => "mba",
// 		UniversalRegion::Newark => "ewr",
// 		UniversalRegion::NewYorkCity => "nyc",
// 		UniversalRegion::SanFrancisco => "sfo",
// 		UniversalRegion::Singapore => "sgp",
// 		UniversalRegion::Sydney => "syd",
// 		UniversalRegion::Tokyo => "tok",
// 		UniversalRegion::Toronto => "tor",
// 		UniversalRegion::WashingtonDc => "dca",
// 	}
// }

fn universal_region_display_name(
	universal_region: &backend::region::UniversalRegion,
) -> &'static str {
	use backend::region::UniversalRegion;

	match universal_region {
		UniversalRegion::Unknown => "Unknown",
		UniversalRegion::Local => "Local",

		UniversalRegion::Amsterdam => "Amsterdam",
		UniversalRegion::Atlanta => "Atlanta",
		UniversalRegion::Bangalore => "Bangalore",
		UniversalRegion::Dallas => "Dallas",
		UniversalRegion::Frankfurt => "Frankfurt",
		UniversalRegion::London => "London",
		UniversalRegion::Mumbai => "Mumbai",
		UniversalRegion::Newark => "Newark",
		UniversalRegion::NewYorkCity => "New York City",
		UniversalRegion::SanFrancisco => "San Francisco",
		UniversalRegion::Singapore => "Singapore",
		UniversalRegion::Sydney => "Sydney",
		UniversalRegion::Tokyo => "Tokyo",
		UniversalRegion::Toronto => "Toronto",
		UniversalRegion::WashingtonDc => "wdc",
	}
}

fn universal_region_coords(universal_region: &backend::region::UniversalRegion) -> (f64, f64) {
	use backend::region::UniversalRegion;

	match universal_region {
		UniversalRegion::Unknown => (0.0, 0.0),
		UniversalRegion::Local => (32.23239, -110.96132),

		UniversalRegion::Amsterdam => (52.36730, 4.89982),
		UniversalRegion::Atlanta => (33.74819, -84.39086),
		UniversalRegion::Bangalore => (12.97740, 77.57423),
		UniversalRegion::Dallas => (32.77557, -96.79560),
		UniversalRegion::Frankfurt => (50.11044, 8.68183),
		UniversalRegion::London => (51.50335, -0.07940),
		UniversalRegion::Mumbai => (18.94010, 72.83466),
		UniversalRegion::Newark => (40.735717094562006, -74.1724228101556),
		UniversalRegion::NewYorkCity => (40.71298, -74.00720),
		UniversalRegion::SanFrancisco => (37.77938, -122.41843),
		UniversalRegion::Singapore => (1.27980, 103.83728),
		UniversalRegion::Sydney => (-33.87271, 151.20569),
		UniversalRegion::Tokyo => (35.68951, 139.69170),
		UniversalRegion::Toronto => (43.65161, -79.38313),
		UniversalRegion::WashingtonDc => (38.89212213251763, -77.00908542245845),
	}
}

fn convert_region(
	name_id: &str,
	region: &Region,
	primary_region_id: Uuid,
) -> GlobalResult<backend::region::Region> {
	let universal_region = universal_region(region);
	let provider_display_name = provider_display_name(region).to_owned();

	let region_display_name = universal_region_display_name(&universal_region).to_owned();
	let (latitude, longitude) = universal_region_coords(&universal_region);
	Ok(backend::region::Region {
		region_id: region.id,
		enabled: true,
		nomad_region: "global".into(),
		nomad_datacenter: name_id.to_owned(),
		provider: region.provider.clone(),
		provider_region: region.provider_region.clone(),
		// TODO: Replace with more intelligent method of determining the CDN region
		cdn_region_id: Some(primary_region_id.into()),
		universal_region: universal_region as i32,
		provider_display_name,
		region_display_name,
		name_id: name_id.to_owned(),
		latitude,
		longitude,
	})
}

#[operation(name = "region-get")]
async fn handle(
	ctx: OperationContext<region::get::Request>,
) -> GlobalResult<region::get::Response> {
	let res = op!([ctx] region_config_get {}).await?;
	let regions = &res.regions;
	let primary_region = unwrap!(
		regions.get(util::env::primary_region()),
		"missing primary region"
	);

	let regions = regions
		.iter()
		.filter(|(_, x)| {
			x.id.as_ref()
				.map_or(false, |id| ctx.region_ids.contains(id))
		})
		.map(|(name_id, region)| {
			convert_region(name_id, region, unwrap_ref!(primary_region.id).as_uuid())
		})
		.collect::<GlobalResult<Vec<backend::region::Region>>>()?;

	Ok(region::get::Response { regions })
}
