use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "region-resolve")]
async fn handle(
	ctx: OperationContext<region::resolve::Request>,
) -> GlobalResult<region::resolve::Response> {
	let db = util_region::config::read().await;
	let regions = db
		.iter()
		.filter(|(x, _)| ctx.name_ids.contains(x))
		.map(|(name_id, region)| region::resolve::response::Region {
			region_id: Some(region.id.into()),
			name_id: name_id.clone(),
		})
		.collect::<Vec<_>>();

	Ok(region::resolve::Response { regions })
}
