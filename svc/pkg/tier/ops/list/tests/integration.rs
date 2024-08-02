use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let datacenters_res = chirp_workflow::compat::op(
		ctx.op_ctx(),
		cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster::util::default_cluster_id()],
		},
	)
	.await
	.unwrap();
	let cluster = datacenters_res.clusters.first().unwrap();

	let res = op!([ctx] tier_list {
		region_ids: cluster.datacenter_ids
			.iter()
			.cloned()
			.map(Into::into)
			.collect(),
	})
	.await
	.unwrap();

	tracing::info!(?res);
}
