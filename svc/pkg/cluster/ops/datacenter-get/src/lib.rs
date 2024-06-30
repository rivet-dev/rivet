use std::convert::{TryFrom, TryInto};

use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Datacenter {
	datacenter_id: Uuid,
	cluster_id: Uuid,
	name_id: String,
	display_name: String,
	provider: i64,
	provider_datacenter_id: String,
	provider_api_token: Option<String>,
	pools: Vec<u8>,
	build_delivery_method: i64,
	prebakes_enabled: bool,
	create_ts: i64,
}

impl TryFrom<Datacenter> for backend::cluster::Datacenter {
	type Error = GlobalError;

	fn try_from(value: Datacenter) -> GlobalResult<Self> {
		let pools = cluster::msg::datacenter_create::Pools::decode(value.pools.as_slice())?.pools;

		Ok(backend::cluster::Datacenter {
			datacenter_id: Some(value.datacenter_id.into()),
			cluster_id: Some(value.cluster_id.into()),
			name_id: value.name_id,
			display_name: value.display_name,
			create_ts: value.create_ts,
			provider: value.provider as i32,
			provider_datacenter_id: value.provider_datacenter_id,
			provider_api_token: value.provider_api_token,
			pools,
			build_delivery_method: value.build_delivery_method as i32,
			prebakes_enabled: value.prebakes_enabled,
		})
	}
}

#[operation(name = "cluster-datacenter-get")]
pub async fn handle(
	ctx: OperationContext<cluster::datacenter_get::Request>,
) -> GlobalResult<cluster::datacenter_get::Response> {
	let datacenter_ids = ctx
		.datacenter_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let datacenters = ctx
		.cache()
		.fetch_all_proto("cluster.datacenters", datacenter_ids, {
			let ctx = ctx.base();
			move |mut cache, datacenter_ids| {
				let ctx = ctx.clone();
				async move {
					let dcs = get_dcs(ctx, datacenter_ids).await?;
					for dc in dcs {
						let dc_id = unwrap!(dc.datacenter_id).as_uuid();
						cache.resolve(&dc_id, dc);
					}

					Ok(cache)
				}
			}
		})
		.await?;

	Ok(cluster::datacenter_get::Response { datacenters })
}

async fn get_dcs(
	ctx: OperationContext<()>,
	datacenter_ids: Vec<Uuid>,
) -> GlobalResult<Vec<backend::cluster::Datacenter>> {
	let configs = sql_fetch_all!(
		[ctx, Datacenter]
		"
		SELECT
			datacenter_id,
			cluster_id,
			name_id,
			display_name,
			provider,
			provider_datacenter_id,
			provider_api_token,
			pools,
			build_delivery_method,
			prebakes_enabled,
			create_ts
		FROM db_cluster.datacenters
		WHERE datacenter_id = ANY($1)
		",
		datacenter_ids,
	)
	.await?;

	let datacenters = configs
		.into_iter()
		.map(TryInto::try_into)
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(datacenters)
}
