use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "cluster-nomad-node-registered")]
async fn worker(
	ctx: &OperationContext<nomad::msg::monitor_node_registered::Message>,
) -> GlobalResult<()> {
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();

	let (datacenter_id, old_nomad_node_id) = sql_fetch_one!(
		[ctx, (Uuid, Option<String>)]
		"
		UPDATE db_cluster.servers
		SET
			nomad_node_id = $2,
			nomad_join_ts = $3
		WHERE
			server_id = $1
		RETURNING datacenter_id, nomad_node_id
		",
		&server_id,
		&ctx.node_id,
		util::timestamp::now(),
	)
	.await?;

	if let Some(old_nomad_node_id) = old_nomad_node_id {
		tracing::warn!(%old_nomad_node_id, "nomad node id was already set");
	}

	// Scale to get rid of tainted servers
	msg!([ctx] cluster::msg::datacenter_scale(datacenter_id) {
		datacenter_id: Some(datacenter_id.into()),
	})
	.await?;

	Ok(())
}
