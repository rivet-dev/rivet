use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let datacenter_id = Uuid::new_v4();
	let cluster_id = Uuid::new_v4();
	let dc_name_id = util::faker::ident();

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
		name_id: dc_name_id.clone(),
		display_name: util::faker::ident(),

		provider: backend::cluster::Provider::Linode as i32,
		provider_datacenter_id: "us-southeast".to_string(),
		provider_api_token: None,

		pools: Vec::new(),

		build_delivery_method: backend::cluster::BuildDeliveryMethod::TrafficServer as i32,
	})
	.await
	.unwrap();

	let res = op!([ctx] cluster_datacenter_resolve_for_name_id {
		cluster_id: Some(cluster_id.into()),
		name_ids: vec![dc_name_id],
	})
	.await
	.unwrap();

	let datacenter = res.datacenters.first().expect("datacenter not found");
	assert_eq!(
		datacenter_id,
		datacenter.datacenter_id.unwrap().as_uuid(),
		"wrong datacenter returned"
	);
}
