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
	drain_timeout: i64,
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
			provider: value.provider as i32,
			provider_datacenter_id: value.provider_datacenter_id,
			provider_api_token: value.provider_api_token,
			pools,
			build_delivery_method: value.build_delivery_method as i32,
			drain_timeout: value.drain_timeout as u64,
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
			drain_timeout
		FROM db_cluster.datacenters
		WHERE datacenter_id = ANY($1)
		",
		datacenter_ids,
	)
	.await?;

	Ok(cluster::datacenter_get::Response {
		datacenters: configs
			.into_iter()
			.map(TryInto::try_into)
			.collect::<GlobalResult<Vec<_>>>()?,
	})
}
