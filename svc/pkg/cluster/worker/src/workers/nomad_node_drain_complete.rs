use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "cluster-nomad-node-drain-complete")]
async fn worker(
	ctx: &OperationContext<nomad::msg::monitor_node_drain_complete::Message>,
) -> GlobalResult<()> {
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();

	let (datacenter_id,) = sql_fetch_one!(
		[ctx, (Uuid,)]
		"
		UPDATE db_cluster.servers
		SET cloud_destroy_ts = $2
		WHERE
			server_id = $1
		RETURNING datacenter_id
		",
		&server_id,
		util::timestamp::now(),
	)
	.await?;

	msg!([ctx] cluster::msg::server_destroy(server_id) {
		server_id: Some(server_id.into()),
		force: false,
	})
	.await?;

	// In the case of completely deleting an entire datacenter, we would first set the desired count of all
	// of the pools to 0. However, we cannot delete all of the GG nodes if there are still job nodes draining
	// because connections may still be open through it. So one GG node is is left in this case (see
	// `cluster-datacenter-scale`). This message is published so that `cluster-datacenter-scale` checks again
	// if there are still job nodes active, and if not, deletes the last GG node.
	msg!([ctx] cluster::msg::datacenter_scale(datacenter_id) {
		datacenter_id: Some(datacenter_id.into()),
	})
	.await?;

	Ok(())
}
