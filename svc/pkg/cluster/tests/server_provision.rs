use chirp_workflow::prelude::*;

mod common;
use common::{setup, Setup};

#[workflow_test]
async fn server_provision(ctx: TestCtx) {
	if !util::feature::server_provision() {
		return;
	}

	let server_id = Uuid::new_v4();
	let datacenter_id = Uuid::new_v4();
	let cluster_id = Uuid::new_v4();

	let dc = setup(
		&ctx,
		Setup {
			server_id,
			datacenter_id,
			cluster_id,
			pool_type: cluster::types::PoolType::Ats,
			drain_timeout: 0,
		},
	)
	.await;

	ctx.signal(cluster::workflows::datacenter::ServerCreate {
		server_id,
		pool_type: dc.pools.first().unwrap().pool_type,
		tags: vec!["test".to_string()],
	})
	.tag("datacenter_id", datacenter_id)
	.send()
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

	// Clean up afterwards so we don't litter
	ctx.signal(cluster::workflows::server::Destroy {})
		.tag("server_id", server_id)
		.send()
		.await
		.unwrap();
}
