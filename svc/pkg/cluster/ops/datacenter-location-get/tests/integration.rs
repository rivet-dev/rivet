use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn basic(ctx: TestCtx) {
	let datacenter_id = Uuid::new_v4();
	let cluster_id = Uuid::new_v4();
	let server_id = Uuid::new_v4();

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

	// Insert fake server record
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_cluster.servers (
			server_id,
			datacenter_id,
			pool_type,
			create_ts,
			public_ip
		)
		VALUES ($1, $2, $3, $4, $5)
		",
		server_id,
		datacenter_id,
		backend::cluster::PoolType::Gg as i64,
		util::timestamp::now(),
		// Google
		"172.217.12.110",
	)
	.await
	.unwrap();

	let locations_res = op!([ctx] cluster_datacenter_location_get {
		datacenter_ids: vec![datacenter_id.into()],
	})
	.await
	.unwrap();

	assert_eq!(
		1,
		locations_res.datacenters.len(),
		"wrong number of locations"
	);

	let location = locations_res.datacenters.first().unwrap();
	let coords = location.coords.as_ref().unwrap();

	assert_eq!(37.3394, coords.latitude);
	assert_eq!(-121.8950, coords.longitude);
}
