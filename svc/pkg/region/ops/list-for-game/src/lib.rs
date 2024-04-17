use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "region-list-for-game")]
async fn handle(
	ctx: OperationContext<region::list_for_game::Request>,
) -> GlobalResult<region::list_for_game::Response> {
	let clusters_res = op!([ctx] cluster_get_for_game {
		game_ids: ctx.game_ids.clone(),
	})
	.await?;

	let datacenter_list_res = op!([ctx] cluster_datacenter_list {
		cluster_ids: clusters_res
			.games
			.iter()
			.map(|game| Ok(unwrap!(game.cluster_id)))
			.collect::<GlobalResult<Vec<_>>>()?,
	})
	.await?;
	let datacenter_ids = datacenter_list_res
		.clusters
		.iter()
		.flat_map(|cluster| &cluster.datacenter_ids)
		.cloned()
		.collect::<Vec<_>>();

	Ok(region::list_for_game::Response {
		region_ids: datacenter_ids,
	})
}
