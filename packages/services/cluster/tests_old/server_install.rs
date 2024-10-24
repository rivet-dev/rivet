use chirp_workflow::prelude::*;

mod common;
use common::{setup, Setup};

#[workflow_test]
async fn server_install(ctx: TestCtx) {
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
			pool_type: backend::cluster::PoolType::Ats,
			drain_timeout: 0,
		},
	)
	.await;

	msg!([ctx] cluster::msg::server_provision(server_id) {
		datacenter_id: Some(datacenter_id.into()),
		server_id: Some(server_id.into()),
		pool_type: dc.pools.first().unwrap().pool_type,
		provider: dc.provider as i32,
		tags: vec!["test".to_string()],
	})
	.await
	.unwrap();

	// Wait for server to have an ip
	let public_ip = loop {
		tokio::time::sleep(std::time::Duration::from_secs(5)).await;

		let row = sql_fetch_optional!(
			[ctx, (String,)]
			"
			SELECT public_ip
			FROM db_cluster.servers
			WHERE
				server_id = $1 AND
				public_ip IS NOT NULL
			",
			server_id,
		)
		.await
		.unwrap();

		if let Some((public_ip,)) = row {
			break public_ip;
		}
	};

	// Wait for install to complete
	let mut sub = subscribe!([ctx] cluster::msg::server_install_complete(public_ip))
		.await
		.unwrap();
	sub.next().await.unwrap();

	// Clean up afterwards so we don't litter
	msg!([ctx] @wait cluster::msg::server_destroy(server_id) {
		server_id: Some(server_id.into()),
		force: false,
	})
	.await
	.unwrap();
}
