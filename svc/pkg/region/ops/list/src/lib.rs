use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "region-list")]
async fn handle(
	ctx: OperationContext<region::list::Request>,
) -> GlobalResult<region::list::Response> {
	let datacenter_list_res = chirp_workflow::compat::op(
		&ctx,
		cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster::util::default_cluster_id()],
		},
	)
	.await?;
	let cluster = unwrap!(
		datacenter_list_res.clusters.first(),
		"default cluster doesn't exist"
	);

	Ok(region::list::Response {
		region_ids: cluster
			.datacenter_ids
			.iter()
			.cloned()
			.map(Into::into)
			.collect(),
	})
}
