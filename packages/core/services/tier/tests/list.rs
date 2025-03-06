use chirp_workflow::prelude::*;

#[workflow_test]
async fn list(ctx: TestCtx) {
	let default_cluster_id = ctx
		.config()
		.server()
		.unwrap()
		.rivet
		.default_cluster_id()
		.unwrap();
	let datacenters_res = ctx
		.op(cluster::ops::datacenter::list::Input {
			cluster_ids: vec![default_cluster_id],
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
