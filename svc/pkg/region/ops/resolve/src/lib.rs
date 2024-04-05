use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "region-resolve")]
async fn handle(
	ctx: OperationContext<region::resolve::Request>,
) -> GlobalResult<region::resolve::Response> {
	let datacenter_list_res = op!([ctx] cluster_datacenter_list {
		cluster_ids: vec![util::env::default_cluster_id().into()],
	})
	.await?;
	let cluster = unwrap!(
		datacenter_list_res.clusters.first(),
		"default cluster doesn't exist"
	);

	let datacenters_res = op!([ctx] cluster_datacenter_get {
		datacenter_ids: cluster.datacenter_ids.clone(),
	})
	.await?;

	let regions = datacenters_res
		.datacenters
		.iter()
		.filter(|dc| ctx.name_ids.contains(&dc.name_id))
		.map(|dc| region::resolve::response::Region {
			region_id: dc.datacenter_id,
			name_id: dc.name_id.clone(),
		})
		.collect::<Vec<_>>();

	// NOTE: Order of regions is not preserved from input
	Ok(region::resolve::Response { regions })
}
