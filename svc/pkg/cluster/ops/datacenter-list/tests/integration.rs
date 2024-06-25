use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let datacenter_id = Uuid::new_v4();
	let cluster_id = Uuid::new_v4();

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

		pools: Vec::new(),

		build_delivery_method: backend::cluster::BuildDeliveryMethod::TrafficServer as i32,
		prebakes_enabled: false,
	})
	.await
	.unwrap();

	let res = op!([ctx] cluster_datacenter_list {
		cluster_ids: vec![cluster_id.into()],
	})
	.await
	.unwrap();
	let cluster = res.clusters.first().unwrap();

	assert_eq!(1, cluster.datacenter_ids.len());
	assert_eq!(
		datacenter_id,
		cluster.datacenter_ids.first().unwrap().as_uuid(),
	);
}
