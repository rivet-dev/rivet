use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "cluster-nomad-node-drain-complete")]
async fn worker(
	ctx: &OperationContext<nomad::msg::monitor_node_drain_complete::Message>,
) -> GlobalResult<()> {
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();

	// Set as completed draining. Will be destroyed by `cluster-datacenter-scale`
	let (datacenter_id,) = sql_fetch_one!(
		[ctx, (Uuid,)]
		"
		UPDATE db_cluster.servers
		SET drain_complete_ts = $2
		WHERE server_id = $1
		RETURNING datacenter_id
		",
		&server_id,
		util::timestamp::now(),
	)
	.await?;

	msg!([ctx] cluster::msg::datacenter_scale(datacenter_id) {
		datacenter_id: Some(datacenter_id.into()),
	})
	.await?;

	Ok(())
}
