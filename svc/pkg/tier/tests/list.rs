use chirp_workflow::prelude::*;

#[workflow_test]
async fn list(ctx: TestCtx) {
	let datacenters_res = ctx
		.op(cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster::util::default_cluster_id()],
		})
		.await
		.unwrap();
	let cluster = datacenters_res.clusters.first().unwrap();

	let res = ctx
		.op(tier::ops::list::Input {
			datacenter_ids: cluster.datacenter_ids.clone(),
			pegboard: false,
		})
		.await
		.unwrap();

	tracing::info!(?res);

	let res = ctx
		.op(tier::ops::list::Input {
			datacenter_ids: cluster.datacenter_ids.clone(),
			pegboard: true,
		})
		.await
		.unwrap();
	tracing::info!(?res);
}
