use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use std::collections::HashSet;

#[operation(name = "cluster-server-destroy-with-filter")]
pub async fn handle(
	ctx: OperationContext<cluster::server_destroy_with_filter::Request>,
) -> GlobalResult<cluster::server_destroy_with_filter::Response> {
	let filter = unwrap!(ctx.filter.clone());

	let servers_res = op!([ctx] cluster_server_list {
		filter: Some(filter)
	})
	.await?;

	// Flag as destroyed
	let server_ids = servers_res
		.servers
		.iter()
		.filter_map(|x| x.server_id)
		.map(|x| x.as_uuid())
		.collect::<Vec<_>>();
	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.servers
		SET cloud_destroy_ts = $2
		WHERE server_id = ANY($1)
		",
		&server_ids,
		util::timestamp::now(),
	)
	.await?;

	// Destroy server
	for server_id in &server_ids {
		msg!([ctx] cluster::msg::server_destroy(server_id) {
			server_id: Some(server_id.clone().into()),
			force: false,
		})
		.await?;
	}

	// Trigger scale event
	let dc_ids = servers_res
		.servers
		.iter()
		.filter_map(|x| x.datacenter_id)
		.map(|x| x.as_uuid())
		.collect::<HashSet<_>>();
	for dc_id in dc_ids {
		msg!([ctx] cluster::msg::datacenter_scale(dc_id) {
			datacenter_id: Some(dc_id.into()),
		})
		.await?;
	}

	Ok(cluster::server_destroy_with_filter::Response {})
}
