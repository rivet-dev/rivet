use futures_util::TryFutureExt;
use std::cmp::{Ordering, PartialOrd};

use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(Debug, Clone)]
enum OriginKind {
	Coords(f64, f64),
	Ip(String),
}

impl PartialEq for OriginKind {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(OriginKind::Coords(lat_a, long_a), OriginKind::Coords(lat_b, long_b)) => {
				// Round to 3 digits. See `cache_key` format below.
				let k = 1000.0f64;
				((lat_a * k) as usize, (long_a * k) as usize)
					== ((lat_b * k) as usize, (long_b * k) as usize)
			}
			(OriginKind::Ip(ip_a), OriginKind::Ip(ip_b)) => ip_a == ip_b,
			_ => false,
		}
	}
}

#[operation(name = "region-recommend")]
async fn handle(
	ctx: OperationContext<region::recommend::Request>,
) -> GlobalResult<region::recommend::Response> {
	let region_ids = ctx
		.region_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();
	let coords = unwrap_ref!(ctx.coords);

	#[allow(deprecated)]
	let origin = if let Some(coords) = &ctx.coords {
		OriginKind::Coords(coords.latitude, coords.longitude)
	} else if let Some(origin_ip) = &ctx.origin_ip {
		OriginKind::Ip(origin_ip.clone())
	} else {
		bail!("lat & long or origin ip not provided")
	};

	let regions = list_regions(&ctx, &origin, &region_ids).await?;

	Ok(region::recommend::Response { regions })
}

async fn list_regions(
	ctx: &OperationContext<region::recommend::Request>,
	origin: &OriginKind,
	region_ids: &[Uuid],
) -> GlobalResult<Vec<region::recommend::response::Region>> {
	let ((lat, long), regions_res) = tokio::try_join!(
		// Look up IP info
		async {
			match origin {
				OriginKind::Coords(lat, long) => GlobalResult::Ok((*lat, *long)),
				OriginKind::Ip(origin_ip) => {
					// Fetch origin IP info
					let res = op!([ctx] ip_info {
						ip: origin_ip.to_owned(),
					})
					.await?;

					let ip_info =
						unwrap_ref!(res.ip_info, "cannot recommend regions to a bogon ip");
					let coords = unwrap_ref!(ip_info.coords);

					GlobalResult::Ok((coords.latitude, coords.longitude))
				}
			}
		},
		// Fetch the location of all the servers
		op!([ctx] region_get {
			region_ids: region_ids
				.iter()
				.cloned()
				.map(Into::<common::Uuid>::into)
				.collect(),
		})
		.map_err(Into::<GlobalError>::into),
	)?;
	ensure!(
		regions_res.regions.len() == region_ids.len(),
		"region not found"
	);

	// Sort the regions by ascending distance to the origin
	let origin_location = geoutils::Location::new(lat, long);
	let mut region_distances = regions_res
		.regions
		.iter()
		.map(|region| {
			let coords = unwrap_ref!(region.coords);

			Ok((
				(coords.latitude, coords.longitude),
				unwrap!(
					geoutils::Location::new(coords.latitude, coords.longitude)
						.distance_to(&origin_location)
						.ok(),
					"failed to calculate distance to region"
				)
				.meters(),
				unwrap_ref!(region.region_id).as_uuid(),
			))
		})
		.collect::<GlobalResult<Vec<_>>>()?;
	region_distances.sort_by(|(_, dist_a, _), (_, dist_b, _)| {
		dist_a.partial_cmp(dist_b).unwrap_or(Ordering::Equal)
	});

	// Serialize response
	let regions = region_distances
		.into_iter()
		.map(|((latitude, longitude), distance_meters, region_id)| {
			region::recommend::response::Region {
				region_id: Some(region_id.into()),
				coords: Some(backend::net::Coordinates {
					latitude,
					longitude,
				}),
				distance_meters,
			}
		})
		.collect::<Vec<_>>();

	Ok(regions)
}
