use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "cluster-datacenter-taint")]
async fn worker(
	ctx: &OperationContext<cluster::msg::datacenter_taint::Message>,
) -> GlobalResult<()> {
	let datacenter_id = unwrap_ref!(ctx.datacenter_id).as_uuid();

	// Taint server records. These will be incrementally drained and destroyed by `cluster-datacenter-scale`
	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.servers
		SET taint_ts = $2
		WHERE
			datacenter_id = $1 AND
			taint_ts IS NULL
		",
		&datacenter_id,
		util::timestamp::now(),
	)
	.await?;

	// Trigger rescale
	msg!([ctx] cluster::msg::datacenter_scale(datacenter_id) {
		datacenter_id: Some(datacenter_id.into()),
	})
	.await?;

	Ok(())
}
