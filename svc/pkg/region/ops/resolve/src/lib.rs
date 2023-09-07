use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "region-resolve")]
async fn handle(
	ctx: OperationContext<region::resolve::Request>,
) -> GlobalResult<region::resolve::Response> {
	let res = op!([ctx] region_config_get {}).await?;
	let regions = res
		.regions
		.iter()
		.filter(|(x, _)| ctx.name_ids.contains(x))
		.map(|(name_id, region)| region::resolve::response::Region {
			region_id: region.id,
			name_id: name_id.clone(),
		})
		.collect::<Vec<_>>();

	Ok(region::resolve::Response { regions })
}
