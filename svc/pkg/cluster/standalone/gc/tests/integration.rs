use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

use ::cluster_gc::run_from_env;

const DRAIN_TIMEOUT: i64 = 1000 * 60 * 60;

#[tokio::test(flavor = "multi_thread")]
async fn basic() {
	if !util::feature::server_provision() {
		return;
	}

	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
		.init();

	let ctx = TestCtx::from_env("cluster-gc-test").await.unwrap();
	let pools = rivet_pools::from_env("cluster-gc-test").await.unwrap();

	let server_id = Uuid::new_v4();
	let datacenter_id = Uuid::new_v4();
	let cluster_id = Uuid::new_v4();

	let (dc_pools, provider) = setup(&ctx, server_id, datacenter_id, cluster_id).await;

	msg!([ctx] cluster::msg::server_provision(server_id) {
		datacenter_id: Some(datacenter_id.into()),
		server_id: Some(server_id.into()),
		pool_type: dc_pools.first().unwrap().pool_type,
		provider: provider as i32,
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

	// Start drain
	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.servers
		SET drain_ts = $2
		WHERE server_id = $1
		",
		server_id,
		util::timestamp::now(),
	)
	.await
	.unwrap();
	msg!([ctx] @wait cluster::msg::server_drain(server_id) {
		server_id: Some(server_id.into()),
	})
	.await
	.unwrap();

	let mut sub = subscribe!([ctx] cluster::msg::server_destroy(server_id))
		.await
		.unwrap();

	// Run GC
	let ts = util::timestamp::now() + DRAIN_TIMEOUT + 1;
	run_from_env(ts, pools).await.unwrap();

	// Check that destroy message was sent
	sub.next().await.unwrap();

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
) -> (Vec<backend::cluster::Pool>, backend::cluster::Provider) {
	let pool_type = backend::cluster::PoolType::Gg as i32;
	let pools = vec![backend::cluster::Pool {
		pool_type,
		hardware: vec![backend::cluster::Hardware {
			provider_hardware: util_cluster::test::LINODE_HARDWARE.to_string(),
		}],
		desired_count: 0,
		max_count: 0,
		drain_timeout: DRAIN_TIMEOUT as u64,
	}];
	let provider = backend::cluster::Provider::Linode;

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

		provider: provider as i32,
		provider_datacenter_id: "us-southeast".to_string(),
		provider_api_token: None,

		pools: pools.clone(),

		build_delivery_method: backend::cluster::BuildDeliveryMethod::TrafficServer as i32,
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
			pool_type,
			create_ts
		)
		VALUES ($1, $2, $3, $4)
		",
		server_id,
		datacenter_id,
		pool_type as i64,
		util::timestamp::now(),
	)
	.await
	.unwrap();

	(pools, provider)
}
