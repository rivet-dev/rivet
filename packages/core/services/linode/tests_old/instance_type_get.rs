use chirp_worker::prelude::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let res = op!([ctx] linode_instance_type_get {
		hardware_ids: vec![util_cluster::test::LINODE_HARDWARE.to_string()],
	})
	.await
	.unwrap();

	tracing::info!(?res);
}
