use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "region-resolve-for-game")]
async fn handle(
	ctx: OperationContext<region::resolve_for_game::Request>,
) -> GlobalResult<region::resolve_for_game::Response> {
	let game_id = unwrap!(ctx.game_id);

	let region_list_res = op!([ctx] region_list_for_game {
		game_ids: vec![game_id],
	})
	.await?;

	let datacenters_res = chirp_workflow::compat::op(
		&ctx,
		cluster::ops::datacenter::get::Input {
			datacenter_ids: region_list_res
				.region_ids
				.iter()
				.map(|id| id.as_uuid())
				.collect(),
		},
	)
	.await?;

	let regions = datacenters_res
		.datacenters
		.iter()
		.filter(|dc| ctx.name_ids.contains(&dc.name_id))
		.map(|dc| region::resolve_for_game::response::Region {
			region_id: Some(dc.datacenter_id.into()),
			name_id: dc.name_id.clone(),
		})
		.collect::<Vec<_>>();

	// NOTE: Order of regions is not preserved from input
	Ok(region::resolve_for_game::Response { regions })
}
