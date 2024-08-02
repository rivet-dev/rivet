use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "region-list-for-game")]
async fn handle(
	ctx: OperationContext<region::list_for_game::Request>,
) -> GlobalResult<region::list_for_game::Response> {
	let clusters_res = chirp_workflow::compat::op(
		&ctx,
		cluster::ops::get_for_game::Input {
			game_ids: ctx
				.game_ids
				.iter()
				.map(|id| id.as_uuid())
				.collect::<Vec<_>>(),
		},
	)
	.await?;

	let datacenter_list_res = chirp_workflow::compat::op(
		&ctx,
		cluster::ops::datacenter::list::Input {
			cluster_ids: clusters_res
				.games
				.iter()
				.map(|game| game.cluster_id)
				.collect::<Vec<_>>(),
		},
	)
	.await?;

	let datacenter_ids = datacenter_list_res
		.clusters
		.iter()
		.flat_map(|cluster| &cluster.datacenter_ids)
		.cloned()
		.map(Into::into)
		.collect::<Vec<_>>();

	Ok(region::list_for_game::Response {
		region_ids: datacenter_ids,
	})
}
