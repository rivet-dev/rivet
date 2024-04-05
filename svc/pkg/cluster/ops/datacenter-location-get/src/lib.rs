use std::collections::HashMap;

use futures_util::{StreamExt, TryStreamExt};
use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "cluster-datacenter-location-get")]
pub async fn handle(
	ctx: OperationContext<cluster::datacenter_location_get::Request>,
) -> GlobalResult<cluster::datacenter_location_get::Response> {
	let datacenter_ids = ctx
		.datacenter_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Fetch the gg node public ip for each datacenter (there may be more than one, hence `DISTINCT`)
	let gg_node_rows = sql_fetch_all!(
		[ctx, (Uuid, Option<String>,)]
		"
		SELECT DISTINCT
			datacenter_id, public_ip
		FROM db_cluster.servers
		WHERE
			datacenter_id = ANY($1) AND
			pool_type = $2
		",
		&datacenter_ids,
		backend::cluster::PoolType::Gg as i64,
	)
	.await?
	.into_iter()
	.filter_map(|(datacenter_id, public_ip)| public_ip.map(|ip| (datacenter_id, ip)));

	let coords_res = futures_util::stream::iter(gg_node_rows)
		.map(|(datacenter_id, public_ip)| {
			let ctx = ctx.base();

			async move {
				// Fetch IP info of GG node (this is cached inside `ip_info`)
				let ip_info_res = op!([ctx] ip_info {
					ip: public_ip,
					provider: ip::info::Provider::IpInfoIo as i32,
				})
				.await?;

				GlobalResult::Ok((
					datacenter_id,
					ip_info_res
						.ip_info
						.as_ref()
						.and_then(|info| info.coords.clone()),
				))
			}
		})
		.buffer_unordered(8)
		.try_collect::<Vec<_>>()
		.await?;

	// Fill in default values
	let mut datacenter_locations = datacenter_ids
		.into_iter()
		.map(|datacenter_id| {
			(
				datacenter_id,
				cluster::datacenter_location_get::response::Datacenter {
					datacenter_id: Some(datacenter_id.into()),
					coords: None,
				},
			)
		})
		.collect::<HashMap<_, _>>();

	// Insert coords
	for (datacenter_id, coords) in coords_res {
		let entry = datacenter_locations
			.entry(datacenter_id)
			.or_insert_with(|| cluster::datacenter_location_get::response::Datacenter {
				datacenter_id: Some(datacenter_id.into()),
				coords: None,
			});

		entry.coords = coords;
	}

	Ok(cluster::datacenter_location_get::Response {
		datacenters: datacenter_locations.into_values().collect::<Vec<_>>(),
	})
}
