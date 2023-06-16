use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "region-list")]
async fn handle(
	_ctx: OperationContext<region::list::Request>,
) -> GlobalResult<region::list::Response> {
	let db = util_region::config::read().await;
	let mut region_ids = db
		.values()
		.map(|x| common::Uuid::from(x.id))
		.collect::<Vec<_>>();
	region_ids.sort_by_cached_key(|x| x.as_uuid());

	Ok(region::list::Response { region_ids })
}
