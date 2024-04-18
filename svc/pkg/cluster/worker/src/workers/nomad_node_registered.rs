use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "cluster-nomad-node-registered")]
async fn worker(
	ctx: &OperationContext<nomad::msg::monitor_node_registered::Message>,
) -> GlobalResult<()> {
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();

	let (datacenter_id,) = sql_fetch_one!(
		[ctx, (Uuid,)]
		"
		UPDATE db_cluster.servers
		SET
			nomad_node_id = $2,
			nomad_join_ts = $3
		WHERE
			server_id = $1 AND
			nomad_node_id IS NULL
		RETURNING datacenter_id
		",
		&server_id,
		&ctx.node_id,
		util::timestamp::now(),
	)
	.await?;

	// Scale to get rid of tainted servers
	msg!([ctx] cluster::msg::datacenter_scale(datacenter_id) {
		datacenter_id: Some(datacenter_id.into()),
	})
	.await?;

	Ok(())
}
