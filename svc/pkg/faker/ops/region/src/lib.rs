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
	let region = region_get
		.regions
		.iter()
		.find(|x| x.name_id == util::env::primary_region());
	let region = unwrap!(region, "primary region not listed in region list");

	Ok(faker::region::Response {
		region_id: region.region_id,
		region: Some(region.clone()),
	})
}
