use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend;
use rivet_api::models;
use rivet_convert::{ApiFrom, ApiInto, ApiTryFrom};
use rivet_operation::prelude::{proto::backend::pkg::cluster, *};
use serde_json::{json, Value};

use crate::auth::Auth;

// MARK: GET /clusters/{cluster_id}/datacenters
pub async fn list(
	ctx: Ctx<Auth>,
	cluster_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::AdminClustersListDatacentersResponse> {
	let response = op!([ctx] cluster_datacenter_list {
		cluster_ids: vec![cluster_id.into()],
	})
	.await?;

	let datacenter_ids = unwrap!(response.clusters.first()).datacenter_ids.clone();

	let datacenters = op!([ctx] cluster_datacenter_get {
		datacenter_ids: datacenter_ids.into_iter().map(Into::into).collect()
	})
	.await?
	.datacenters
	.into_iter()
	.map(models::AdminClustersDatacenter::api_try_from)
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
	let clusters = op!([ctx] cluster_get {
		cluster_ids: vec![cluster_id.into()],
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
		backend::cluster::Pool {
			pool_type: backend::cluster::PoolType::Job as i32,
			hardware: vec![backend::cluster::Hardware {
				provider_hardware: "g6-nanode-1".to_string(),
			}],
			desired_count: 0,
			min_count: 0,
			max_count: 0,
			drain_timeout,
		},
		backend::cluster::Pool {
			pool_type: backend::cluster::PoolType::Gg as i32,
			hardware: vec![backend::cluster::Hardware {
				provider_hardware: "g6-nanode-1".to_string(),
			}],
			desired_count: 0,
			min_count: 0,
			max_count: 0,
			drain_timeout,
		},
		backend::cluster::Pool {
			pool_type: backend::cluster::PoolType::Ats as i32,
			hardware: vec![backend::cluster::Hardware {
				provider_hardware: "g6-nanode-1".to_string(),
			}],
			desired_count: 0,
			min_count: 0,
			max_count: 0,
			drain_timeout,
		},
	];

	msg!([ctx] cluster::msg::datacenter_create(datacenter_id) -> cluster::msg::datacenter_scale {
		datacenter_id: Some(datacenter_id.into()),
		cluster_id: Some(cluster_id.into()),
		name_id: body.name_id.clone(),
		display_name: body.display_name.clone(),

		provider: backend::cluster::Provider::api_from(body.provider) as i32,
		provider_datacenter_id: body.provider_datacenter_id.clone(),
		provider_api_token: None,

		pools: pools,

		build_delivery_method: backend::cluster::BuildDeliveryMethod::api_from(body.build_delivery_method) as i32,
		prebakes_enabled: body.prebakes_enabled,
	})
	.await?;

	Ok(models::AdminClustersCreateDatacenterResponse { datacenter_id })
}

// MARK: PUT /admin/clusters/{cluster_id}/datacenters/{datacenter_id}
pub async fn update(
	ctx: Ctx<Auth>,
	cluster_id: Uuid,
	datacenter_id: Uuid,
	body: models::AdminClustersUpdateDatacenterRequest,
) -> GlobalResult<Value> {
	// Make sure that the datacenter is part of the cluster
	let datacenters = op!([ctx] cluster_datacenter_get {
		datacenter_ids: vec![datacenter_id.into()],
	})
	.await?
	.datacenters;

	let datacenter = match datacenters.first() {
		Some(d) => d,
		None => bail_with!(CLUSTER_DATACENTER_NOT_FOUND),
	};

	if datacenter.cluster_id != Some(cluster_id.into()) {
		bail_with!(CLUSTER_DATACENTER_NOT_IN_CLUSTER);
	}

	msg!([ctx] cluster::msg::datacenter_update(datacenter_id) -> cluster::msg::datacenter_scale {
		datacenter_id: Some(datacenter_id.into()),
		pools: body.pools
			.iter()
			.cloned()
			.map(ApiInto::api_into)
			.collect::<Vec<_>>(),
		prebakes_enabled: body.prebakes_enabled,
	})
	.await
	.unwrap();

	Ok(json!({}))
}
