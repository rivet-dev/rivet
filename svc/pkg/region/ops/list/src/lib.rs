use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "region-list")]
async fn handle(
	ctx: OperationContext<region::list::Request>,
) -> GlobalResult<region::list::Response> {
	let res = op!([ctx] region_config_get {}).await?;
	let mut region_ids = res.regions.values().flat_map(|x| x.id).collect::<Vec<_>>();
	region_ids.sort_by_cached_key(|x| x.as_uuid());

	Ok(region::list::Response { region_ids })
}
