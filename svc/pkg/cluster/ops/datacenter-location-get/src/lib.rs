use std::net::IpAddr;

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

	// NOTE: if there is no active GG node in a datacenter, we cannot retrieve its location
	// Fetch the gg node public ip for each datacenter (there may be more than one, hence `DISTINCT`)
	let server_rows = sql_fetch_all!(
		[ctx, (Uuid, Option<IpAddr>,)]
		"
		SELECT DISTINCT
			datacenter_id, public_ip
		FROM db_cluster.servers
		WHERE
			datacenter_id = ANY($1) AND
			pool_type = $2 AND
			cloud_destroy_ts IS NULL
		-- For consistency
		ORDER BY public_ip DESC
		",
		&datacenter_ids,
		backend::cluster::PoolType::Gg as i64,
	)
	.await?;

	let coords_res = futures_util::stream::iter(server_rows)
		.map(|(datacenter_id, public_ip)| {
			let ctx = ctx.base();

			async move {
				if let Some(public_ip) = public_ip {
					// Fetch IP info of GG node (this is cached inside `ip_info`)
					let ip_info_res = op!([ctx] ip_info {
						ip: public_ip.to_string(),
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
				} else {
					GlobalResult::Ok((datacenter_id, None))
				}
			}
		})
		.buffer_unordered(8)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(cluster::datacenter_location_get::Response {
		datacenters: coords_res
			.into_iter()
			.map(
				|(datacenter_id, coords)| cluster::datacenter_location_get::response::Datacenter {
					datacenter_id: Some(datacenter_id.into()),
					coords,
				},
			)
			.collect::<Vec<_>>(),
	})
}
