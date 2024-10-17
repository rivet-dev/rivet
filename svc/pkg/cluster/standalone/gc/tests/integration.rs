use ::cluster_gc::run_from_env;
use chirp_workflow::prelude::*;
use serde_json::json;
use tracing_subscriber::prelude::*;

use cluster::types::{BuildDeliveryMethod, Hardware, Pool, PoolType, Provider};

const DRAIN_TIMEOUT: i64 = 1000 * 60 * 60;

#[tokio::test(flavor = "multi_thread")]
async fn basic() {
	if !util::feature::server_provision() {
		return;
	}

	tracing_subscriber::registry()
		.with(
			tracing_logfmt::builder()
				.layer()
				.with_filter(tracing_subscriber::filter::LevelFilter::INFO),
		)
		.init();

	let ctx = TestCtx::from_env("cluster-gc-test").await;
	let pools = rivet_pools::from_env().await.unwrap();

	let (server_id, _) = setup(&ctx).await;

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

	ctx.signal(cluster::workflows::server::Drain {})
		.tag("server_id", server_id)
		.send()
		.await
		.unwrap();

	// Run GC
	let ts = util::timestamp::now() + DRAIN_TIMEOUT + 1;
	run_from_env(ts, pools).await.unwrap();

	// Wait for server to be completely drained
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
					drain_complete_ts IS NOT NULL
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

	// Clean up afterwards so we don't litter
	ctx.signal(cluster::workflows::server::Destroy {})
		.tag("server_id", server_id)
		.send()
		.await
		.unwrap();
}

async fn setup(ctx: &TestCtx) -> (Uuid, Uuid) {
	let server_id = Uuid::new_v4();
	let datacenter_id = Uuid::new_v4();
	let cluster_id = Uuid::new_v4();

	let pool_type = PoolType::Gg;
	let pools = vec![Pool {
		pool_type,
		hardware: vec![Hardware {
			provider_hardware: cluster::util::test::LINODE_HARDWARE.to_string(),
		}],
		desired_count: 0,
		min_count: 0,
		max_count: 0,
		drain_timeout: DRAIN_TIMEOUT as u64,
	}];
	let provider = Provider::Linode;

	ctx.workflow(cluster::workflows::cluster::Input {
		cluster_id,
		name_id: util::faker::ident(),
		owner_team_id: None,
	})
	.tag("cluster_id", cluster_id)
	.dispatch()
	.await
	.unwrap();

	let mut create_sub = ctx
		.subscribe::<cluster::workflows::datacenter::CreateComplete>(&json!({
			"datacenter_id": datacenter_id,
		}))
		.await
		.unwrap();
	ctx.signal(cluster::workflows::cluster::DatacenterCreate {
		datacenter_id,
		name_id: util::faker::ident(),
		display_name: util::faker::ident(),

		provider,
		provider_datacenter_id: "us-southeast".to_string(),
		provider_api_token: None,

		pools: pools.clone(),

		build_delivery_method: BuildDeliveryMethod::TrafficServer,
		prebakes_enabled: false,
	})
	.tag("cluster_id", cluster_id)
	.send()
	.await
	.unwrap();

	create_sub.next().await.unwrap();

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
		pool_type as i32,
		util::timestamp::now(),
	)
	.await
	.unwrap();

	ctx.signal(cluster::workflows::datacenter::ServerCreate {
		server_id,
		pool_type,
		tags: vec!["test".to_string()],
	})
	.tag("datacenter_id", datacenter_id)
	.send()
	.await
	.unwrap();

	(server_id, datacenter_id)
}
