use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "region-list")]
async fn handle(
	ctx: OperationContext<region::list::Request>,
) -> GlobalResult<region::list::Response> {
	let datacenter_list_res = op!([ctx] cluster_datacenter_list {
		cluster_ids: vec![util_cluster::default_cluster_id().into()],
	})
	.await?;
	let cluster = unwrap!(
		datacenter_list_res.clusters.first(),
		"default cluster doesn't exist"
	);

	Ok(region::list::Response {
		region_ids: cluster.datacenter_ids.clone(),
	})
}
