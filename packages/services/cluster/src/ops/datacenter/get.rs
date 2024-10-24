use std::convert::{TryFrom, TryInto};

use chirp_workflow::prelude::*;

use crate::types::{BuildDeliveryMethod, Datacenter, Pool, Provider};

#[derive(Debug)]
pub struct Input {
	pub datacenter_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub datacenters: Vec<Datacenter>,
}

#[derive(sqlx::FromRow)]
struct DatacenterRow {
	datacenter_id: Uuid,
	cluster_id: Uuid,
	name_id: String,
	display_name: String,
	provider: i64,
	provider_datacenter_id: String,
	provider_api_token: Option<String>,
	pools2: sqlx::types::Json<Vec<Pool>>,
	build_delivery_method: i64,
	prebakes_enabled: bool,
	create_ts: i64,
}

impl TryFrom<DatacenterRow> for Datacenter {
	type Error = GlobalError;

	fn try_from(value: DatacenterRow) -> GlobalResult<Self> {
		Ok(Datacenter {
			datacenter_id: value.datacenter_id,
			cluster_id: value.cluster_id,
			name_id: value.name_id,
			display_name: value.display_name,
			create_ts: value.create_ts,
			provider: unwrap!(Provider::from_repr(value.provider.try_into()?)),
			provider_datacenter_id: value.provider_datacenter_id,
			provider_api_token: value.provider_api_token,
			pools: value.pools2.0,
			build_delivery_method: unwrap!(BuildDeliveryMethod::from_repr(
				value.build_delivery_method.try_into()?
			)),
			prebakes_enabled: value.prebakes_enabled,
		})
	}
}

#[operation]
pub async fn cluster_datacenter_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let datacenters = ctx
		.cache()
		.fetch_all_json("cluster.datacenters2", input.datacenter_ids.clone(), {
			let ctx = ctx.clone();
			move |mut cache, datacenter_ids| {
				let ctx = ctx.clone();
				async move {
					let dcs = get_dcs(ctx, datacenter_ids).await?;
					for dc in dcs {
						let dc_id = dc.datacenter_id;
						cache.resolve(&dc_id, dc);
					}

					Ok(cache)
				}
			}
		})
		.await?;

	Ok(Output { datacenters })
}

async fn get_dcs(ctx: OperationCtx, datacenter_ids: Vec<Uuid>) -> GlobalResult<Vec<Datacenter>> {
	let dc_rows = sql_fetch_all!(
		[ctx, DatacenterRow]
		"
		SELECT
			datacenter_id,
			cluster_id,
			name_id,
			display_name,
			provider,
			provider_datacenter_id,
			provider_api_token,
			pools2,
			build_delivery_method,
			prebakes_enabled,
			create_ts
		FROM db_cluster.datacenters
		WHERE datacenter_id = ANY($1)
		",
		datacenter_ids,
	)
	.await?;

	dc_rows
		.into_iter()
		.map(TryInto::try_into)
		.collect::<GlobalResult<Vec<_>>>()
}
