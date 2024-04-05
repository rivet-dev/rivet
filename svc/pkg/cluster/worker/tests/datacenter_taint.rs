use std::time::Duration;

use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn datacenter_taint(ctx: TestCtx) {
	if !util::feature::server_provision() {
		return;
	}

	let server_id = Uuid::new_v4();
	let datacenter_id = Uuid::new_v4();
	let cluster_id = Uuid::new_v4();

	let dc = setup(&ctx, server_id, datacenter_id, cluster_id).await;

	// Manually create a server
	msg!([ctx] cluster::msg::server_provision(server_id) {
		cluster_id: Some(cluster_id.into()),
		datacenter_id: Some(datacenter_id.into()),
		server_id: Some(server_id.into()),
		pool_type: dc.pools.first().unwrap().pool_type,
		provider: dc.provider,
		tags: vec!["test".to_string()],
	})
	.await
	.unwrap();

	// Wait for server to have an ip
	loop {
		tokio::time::sleep(std::time::Duration::from_secs(5)).await;

		let (exists,) = sql_fetch_one!(
			[ctx, (bool,)]
			"
			SELECT EXISTS (
				SELECT 1
				FROM db_cluster.servers
				WHERE
					server_id = $1 AND
					public_ip IS NOT NULL
			)
			",
			server_id,
		)
		.await
		.unwrap();

		if exists {
			break;
		}
	}

	// Increase desired count (this wont provision anything, we manually created a server)
	msg!([ctx] cluster::msg::datacenter_update(datacenter_id) -> cluster::msg::datacenter_scale {
		datacenter_id: Some(datacenter_id.into()),
		pools: vec![cluster::msg::datacenter_update::PoolUpdate {
			pool_type: backend::cluster::PoolType::Job as i32,
			hardware: Vec::new(),
			desired_count: Some(1),
			max_count: Some(1),
		}],
		drain_timeout: None,
	})
	.await
	.unwrap();

	// TODO: The servers brought up by this taint (and subsequent scale) aren't tagged as "test" so they wont
	// be garbage collected if the test fails
	// Taint datacenter
	msg!([ctx] cluster::msg::datacenter_taint(datacenter_id) -> cluster::msg::datacenter_scale {
		datacenter_id: Some(datacenter_id.into()),
	})
	.await
	.unwrap();

	// Validate state
	let (taint_ts,) = sql_fetch_one!(
		[ctx, (Option<i64>,)]
		"
		SELECT taint_ts
		FROM db_cluster.servers
		WHERE server_id = $1
		",
		server_id,
	)
	.await
	.unwrap();

	taint_ts.expect("did not taint server");

	// Downscale datacenter (so it destroys the new server)
	msg!([ctx] cluster::msg::datacenter_update(datacenter_id) -> cluster::msg::datacenter_scale {
		datacenter_id: Some(datacenter_id.into()),
		pools: vec![cluster::msg::datacenter_update::PoolUpdate {
			pool_type: backend::cluster::PoolType::Job as i32,
			hardware: Vec::new(),
			desired_count: Some(0),
			max_count: Some(0),
		}],
		drain_timeout: None,
	})
	.await
	.unwrap();

	// Wait for datacenter scale to destroy servers
	tokio::time::sleep(Duration::from_secs(2)).await;
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
