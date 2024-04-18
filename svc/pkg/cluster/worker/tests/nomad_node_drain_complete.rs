use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

mod common;
use common::{setup, Setup};

#[worker_test]
async fn nomad_node_drain_complete(ctx: TestCtx) {
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
			pool_type: backend::cluster::PoolType::Job,
			drain_timeout: 0,
		},
	)
	.await;

	msg!([ctx] @notrace cluster::msg::server_provision(server_id) -> nomad::msg::monitor_node_registered {
		datacenter_id: Some(datacenter_id.into()),
		server_id: Some(server_id.into()),
		pool_type: dc.pools.first().unwrap().pool_type,
		provider: dc.provider as i32,
		tags: vec!["test".to_string()],
	})
	.await
	.unwrap();

	msg!([ctx] @notrace cluster::msg::server_drain(server_id) -> cluster::msg::server_destroy {
		server_id: Some(server_id.into()),
	})
	.await
	.unwrap();
}
