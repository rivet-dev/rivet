use std::net::IpAddr;

use chirp_workflow::prelude::*;
use futures_util::{StreamExt, TryStreamExt};
use rivet_operation::prelude::proto::backend::pkg::*;

use crate::types::PoolType;

#[derive(Debug)]
pub struct Input {
	pub datacenter_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub datacenters: Vec<Datacenter>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Datacenter {
	pub datacenter_id: Uuid,
	pub coords: Coordinates,
}

// TODO: Move to a common types lib
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coordinates {
	pub longitude: f64,
	pub latitude: f64,
}

#[operation]
pub async fn cluster_datacenter_location_get(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<Output> {
	let datacenters = ctx
		.cache()
		.fetch_all_json(
			"cluster.datacenters.location",
			input.datacenter_ids.clone(),
			{
				let ctx = ctx.clone();
				move |mut cache, datacenter_ids| {
					let ctx = ctx.clone();
					async move {
						let dcs = query_dcs(ctx, datacenter_ids).await?;
						for dc in dcs {
							let dc_id = dc.datacenter_id;
							cache.resolve(&dc_id, dc);
						}

						Ok(cache)
					}
				}
			},
		)
		.await?;

	Ok(Output { datacenters })
}

async fn query_dcs(ctx: OperationCtx, datacenter_ids: Vec<Uuid>) -> GlobalResult<Vec<Datacenter>> {
	// NOTE: if there is no active GG node in a datacenter, we cannot retrieve its location
	// Fetch the gg node public ip for each datacenter (there may be more than one, hence `DISTINCT`)
	let server_rows = sql_fetch_all!(
		[ctx, (Uuid, IpAddr)]
		"
		SELECT DISTINCT
			datacenter_id, public_ip
		FROM db_cluster.servers
		WHERE
			datacenter_id = ANY($1) AND
			pool_type2 = $2 AND
			public_ip IS NOT NULL AND
			cloud_destroy_ts IS NULL
		-- For consistency
		ORDER BY public_ip DESC
		",
		&datacenter_ids,
		serde_json::to_string(&PoolType::Gg)?,
	)
	.await?;

	let coords_res = futures_util::stream::iter(server_rows)
		.map(|(datacenter_id, public_ip)| {
			let ctx = ctx.clone();

			async move {
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
						.and_then(|info| info.coords.as_ref())
						.map(|coords| Coordinates {
							longitude: coords.longitude,
							latitude: coords.latitude,
						}),
				))
			}
		})
		.buffer_unordered(8)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(coords_res
		.into_iter()
		.filter_map(|(datacenter_id, coords)| {
			coords.map(|coords| Datacenter {
				datacenter_id,
				coords,
			})
		})
		.collect::<Vec<_>>())
}
