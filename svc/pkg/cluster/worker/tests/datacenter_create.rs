use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn datacenter_create(ctx: TestCtx) {
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

	// Check if tls record exists
	let (exists,) = sql_fetch_one!(
		[ctx, (bool,)]
		"
		SELECT EXISTS (
			SELECT 1
			FROM db_cluster.datacenter_tls
			WHERE datacenter_id = $1
		)
		",
		datacenter_id,
	)
	.await
	.unwrap();

	assert!(exists, "no tls record");
}
