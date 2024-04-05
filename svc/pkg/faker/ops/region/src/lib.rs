use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "faker-region")]
async fn handle(
	ctx: OperationContext<faker::region::Request>,
) -> GlobalResult<faker::region::Response> {
	let region_list = op!([ctx] region_list {
		..Default::default()
	})
	.await?;

	// Get the region data
	let region_get = op!([ctx] region_get {
		region_ids: region_list.region_ids.clone(),
	})
	.await?;
	let region = unwrap!(region_get.regions.first());

	Ok(faker::region::Response {
		region_id: region.region_id,
		region: Some(region.clone()),
	})
}
