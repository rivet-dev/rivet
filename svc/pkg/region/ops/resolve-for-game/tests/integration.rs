use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let (cluster_id, datacenter_id, dc_name_id) = create_dc(&ctx).await;

	let game_id = Uuid::new_v4();
	msg!([ctx] cluster::msg::game_link(game_id, cluster_id) -> cluster::msg::game_link_complete {
		game_id: Some(game_id.into()),
		cluster_id: Some(cluster_id.into()),
	})
	.await
	.unwrap();

	let regions_res = op!([ctx] region_resolve_for_game {
		game_id: Some(game_id.into()),
		name_ids: vec![dc_name_id, "foo".to_string()],
	})
	.await
	.unwrap();

	assert_eq!(1, regions_res.regions.len(), "wrong number of regions");
	assert_eq!(
		datacenter_id,
		regions_res
			.regions
			.first()
			.unwrap()
			.region_id
			.unwrap()
			.as_uuid(),
		"wrong region id",
	);
}

async fn create_dc(ctx: &TestCtx) -> (Uuid, Uuid, String) {
	let datacenter_id = Uuid::new_v4();
	let dc_name_id = util::faker::ident();
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

	(cluster_id, datacenter_id, dc_name_id)
}
