use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn nomad_node_registered(ctx: TestCtx) {
	if !util::feature::server_provision() {
		return;
	}

	let server_id = Uuid::new_v4();
	let datacenter_id = Uuid::new_v4();
	let cluster_id = Uuid::new_v4();

	let dc = setup(&ctx, server_id, datacenter_id, cluster_id).await;

	msg!([ctx] @notrace cluster::msg::server_provision(server_id) -> nomad::msg::monitor_node_registered {
		cluster_id: Some(cluster_id.into()),
		datacenter_id: Some(datacenter_id.into()),
		server_id: Some(server_id.into()),
		pool_type: dc.pools.first().unwrap().pool_type,
		provider: dc.provider,
		tags: vec!["test".to_string()],
	})
	.await
	.unwrap();

	// Clean up afterwards so we don't litter
	msg!([ctx] @wait cluster::msg::server_destroy(server_id) {
		server_id: Some(server_id.into()),
		force: false,
	})
	.await
	.unwrap();
}

async fn setup(
	ctx: &TestCtx,
	server_id: Uuid,
	datacenter_id: Uuid,
	cluster_id: Uuid,
) -> backend::cluster::Datacenter {
	let pool_type = backend::cluster::PoolType::Job as i32;

	msg!([ctx] cluster::msg::create(cluster_id) -> cluster::msg::create_complete {
		cluster_id: Some(cluster_id.into()),
		name_id: util::faker::ident(),
		owner_team_id: None,
	})
	.await
	.unwrap();

	let dc = backend::cluster::Datacenter {
		datacenter_id: Some(datacenter_id.into()),
		cluster_id: Some(cluster_id.into()),
		name_id: util::faker::ident(),
		display_name: util::faker::ident(),

		provider: backend::cluster::Provider::Linode as i32,
		provider_datacenter_id: "us-southeast".to_string(),
		provider_api_token: None,

		pools: vec![backend::cluster::Pool {
			pool_type,
			hardware: vec![backend::cluster::Hardware {
				provider_hardware: util_cluster::test::HARDWARE.to_string(),
			}],
			desired_count: 0,
			max_count: 0,
		}],

		build_delivery_method: backend::cluster::BuildDeliveryMethod::TrafficServer as i32,
		drain_timeout: 0,
	};

	msg!([ctx] cluster::msg::datacenter_create(datacenter_id) -> cluster::msg::datacenter_scale {
		config: Some(dc.clone()),
	})
	.await
	.unwrap();

	// Write new server to db
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_cluster.servers (
			server_id,
			datacenter_id,
			cluster_id,
			pool_type,
			create_ts
		)
		VALUES ($1, $2, $3, $4, $5)
		",
		server_id,
		datacenter_id,
		cluster_id,
		pool_type as i64,
		util::timestamp::now(),
	)
	.await
	.unwrap();

	dc
}
