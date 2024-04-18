use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

mod common;
use common::{setup, Setup};

#[worker_test]
async fn server_dns_delete(ctx: TestCtx) {
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
			pool_type: backend::cluster::PoolType::Gg,
			drain_timeout: 0,
		},
	)
	.await;

	msg!([ctx] cluster::msg::server_provision(server_id) -> cluster::msg::server_dns_create {
		cluster_id: Some(cluster_id.into()),
		datacenter_id: Some(datacenter_id.into()),
		server_id: Some(server_id.into()),
		pool_type: dc.pools.first().unwrap().pool_type,
		provider: dc.provider as i32,
		tags: vec!["test".to_string()],
	})
	.await
	.unwrap();

	msg!([ctx] cluster::msg::server_destroy(server_id) -> cluster::msg::server_dns_delete {
		server_id: Some(server_id.into()),
		force: false,
	})
	.await
	.unwrap();

	tokio::time::sleep(std::time::Duration::from_secs(5)).await;

	let (exists,) = sql_fetch_one!(
		[ctx, (bool,)]
		"
		SELECT EXISTS (
			SELECT 1
			FROM db_cluster.servers_cloudflare
			WHERE server_id = $1
		)
		",
		server_id,
	)
	.await
	.unwrap();

	assert!(!exists, "dns record not deleted");
}
