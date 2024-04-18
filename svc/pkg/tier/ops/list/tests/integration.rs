use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let datacenters_res = op!([ctx] cluster_datacenter_list {
		cluster_ids: vec![util_cluster::default_cluster_id().into()],
	})
	.await
	.unwrap();
	let cluster = datacenters_res.clusters.first().unwrap();

	let res = op!([ctx] tier_list {
		region_ids: cluster.datacenter_ids.clone(),
	})
	.await
	.unwrap();

	tracing::info!(?res);
}
