use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_convert::{ApiInto, ApiTryInto};
use rivet_operation::prelude::*;
use serde_json::{json, Value};

use crate::auth::Auth;

// MARK: GET /clusters/{cluster_id}/datacenters
pub async fn list(
	ctx: Ctx<Auth>,
	cluster_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::AdminClustersListDatacentersResponse> {
	let datacenters_res = ctx
		.op(cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster_id],
		})
		.await?;

	let datacenter_ids = unwrap!(datacenters_res.clusters.first())
		.datacenter_ids
		.clone();

	let datacenters = ctx
		.op(cluster::ops::datacenter::get::Input { datacenter_ids })
		.await?
		.datacenters
		.into_iter()
		.map(ApiTryInto::api_try_into)
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::AdminClustersListDatacentersResponse { datacenters })
}

// MARK: POST /clusters/{cluster_id}/datacenters
pub async fn create(
	ctx: Ctx<Auth>,
	cluster_id: Uuid,
	body: models::AdminClustersCreateDatacenterRequest,
) -> GlobalResult<models::AdminClustersCreateDatacenterResponse> {
	// Make sure the cluster exists
	let clusters = ctx
		.op(cluster::ops::get::Input {
			cluster_ids: vec![cluster_id],
		})
		.await?
		.clusters;

	if clusters.is_empty() {
		bail_with!(CLUSTER_NOT_FOUND);
	}

	let datacenter_id = Uuid::new_v4();
	let drain_timeout = util::duration::hours(4).try_into()?;

	// When creating a datacenter, an empty pool of each type is added. This
	// is to make sure that the datacenter starts in a valid state.
	let pools = vec![
		cluster::types::Pool {
			pool_type: cluster::types::PoolType::Pegboard,
			hardware: vec![cluster::types::Hardware {
				provider_hardware: "g6-nanode-1".to_string(),
			}],
			desired_count: 0,
			min_count: 0,
			max_count: 0,
			drain_timeout,
		},
		cluster::types::Pool {
			pool_type: cluster::types::PoolType::Gg,
			hardware: vec![cluster::types::Hardware {
				provider_hardware: "g6-nanode-1".to_string(),
			}],
			desired_count: 0,
			min_count: 0,
			max_count: 0,
			drain_timeout,
		},
		cluster::types::Pool {
			pool_type: cluster::types::PoolType::Ats,
			hardware: vec![cluster::types::Hardware {
				provider_hardware: "g6-nanode-1".to_string(),
			}],
			desired_count: 0,
			min_count: 0,
			max_count: 0,
			drain_timeout,
		},
	];

	let mut sub = ctx
		.subscribe::<cluster::workflows::datacenter::CreateComplete>(&json!({
			"datacenter_id": datacenter_id,
		}))
		.await?;

	ctx.signal(cluster::workflows::cluster::DatacenterCreate {
		datacenter_id,
		name_id: body.name_id.clone(),
		display_name: body.display_name.clone(),

		provider: body.provider.api_into(),
		provider_datacenter_id: body.provider_datacenter_id.clone(),
		provider_api_token: None,

		pools,

		build_delivery_method: body.build_delivery_method.api_into(),
		prebakes_enabled: body.prebakes_enabled,
	})
	.tag("cluster_id", cluster_id)
	.send()
	.await?;

	sub.next().await?;

	Ok(models::AdminClustersCreateDatacenterResponse { datacenter_id })
}

// MARK: PUT /admin/clusters/{cluster_id}/datacenters/{datacenter_id}
pub async fn update(
	ctx: Ctx<Auth>,
	cluster_id: Uuid,
	datacenter_id: Uuid,
	body: models::AdminClustersUpdateDatacenterRequest,
) -> GlobalResult<Value> {
	let datacenters = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![datacenter_id],
		})
		.await?
		.datacenters;

	let datacenter = unwrap_with!(datacenters.first(), CLUSTER_DATACENTER_NOT_FOUND);

	// Make sure that the datacenter is part of the cluster
	if datacenter.cluster_id != cluster_id {
		bail_with!(CLUSTER_DATACENTER_NOT_IN_CLUSTER);
	}

	ctx.signal(cluster::workflows::datacenter::Update {
		pools: body
			.pools
			.iter()
			.cloned()
			.map(ApiInto::api_into)
			.collect::<Vec<_>>(),
		prebakes_enabled: body.prebakes_enabled,
	})
	.tag("datacenter_id", datacenter_id)
	.send()
	.await?;

	Ok(json!({}))
}
