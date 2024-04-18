use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn datacenter_update(ctx: TestCtx) {
	let datacenter_id = Uuid::new_v4();
	let cluster_id = Uuid::new_v4();
	let pools = vec![backend::cluster::Pool {
		pool_type: backend::cluster::PoolType::Ats as i32,
		hardware: Vec::new(),
		desired_count: 0,
		max_count: 0,
		drain_timeout: 0,
	}];

	msg!([ctx] cluster::msg::create(cluster_id) -> cluster::msg::create_complete {
		cluster_id: Some(cluster_id.into()),
		name_id: util::faker::ident(),
		owner_team_id: None,
	})
	.await
	.unwrap();

	msg!([ctx] cluster::msg::datacenter_create(datacenter_id) -> cluster::msg::datacenter_scale {
		datacenter_id: Some(datacenter_id.into()),
		cluster_id: Some(cluster_id.into()),
		name_id: util::faker::ident(),
		display_name: util::faker::ident(),

		provider: backend::cluster::Provider::Linode as i32,
		provider_datacenter_id: "us-southeast".to_string(),
		provider_api_token: None,

		pools: pools.clone(),

		build_delivery_method: backend::cluster::BuildDeliveryMethod::TrafficServer as i32,
	})
	.await
	.unwrap();

	msg!([ctx] cluster::msg::datacenter_update(datacenter_id) -> cluster::msg::datacenter_scale {
		datacenter_id: Some(datacenter_id.into()),
		pools: vec![cluster::msg::datacenter_update::PoolUpdate {
			pool_type: backend::cluster::PoolType::Ats as i32,
			hardware: Vec::new(),
			desired_count: Some(1),
			max_count: None,
			drain_timeout: None,
		}],
	})
	.await
	.unwrap();

	let datacenter_res = op!([ctx] cluster_datacenter_get {
		datacenter_ids: vec![datacenter_id.into()],
	})
	.await
	.unwrap();
	let updated_dc = datacenter_res.datacenters.first().unwrap();

	assert_ne!(
		pools.first().unwrap().desired_count,
		updated_dc.pools.first().unwrap().desired_count,
		"datacenter not updated",
	);
}
