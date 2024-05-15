use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use util_cluster::metrics;

#[worker(name = "cluster-nomad-node-registered")]
async fn worker(
	ctx: &OperationContext<nomad::msg::monitor_node_registered::Message>,
) -> GlobalResult<()> {
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();
	let nomad_join_ts = util::timestamp::now();

	let (datacenter_id, old_nomad_node_id, install_complete_ts) = sql_fetch_one!(
		[ctx, (Uuid, Option<String>, i64)]
		"
		UPDATE db_cluster.servers
		SET
			nomad_node_id = $2,
			nomad_join_ts = $3
		WHERE
			server_id = $1
		RETURNING datacenter_id, nomad_node_id, install_complete_ts
		",
		&server_id,
		&ctx.node_id,
		nomad_join_ts,
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

	// Insert metrics
	insert_metrics(ctx, datacenter_id, nomad_join_ts, install_complete_ts).await?;

	Ok(())
}

async fn insert_metrics(
	ctx: &OperationContext<nomad::msg::monitor_node_registered::Message>,
	datacenter_id: Uuid,
	nomad_join_ts: i64,
	install_complete_ts: i64,
) -> GlobalResult<()> {
	let datacenters_res = op!([ctx] cluster_datacenter_get {
		datacenter_ids: vec![datacenter_id.into()],
	})
	.await?;
	let dc = unwrap!(datacenters_res.datacenters.first());

	let datacenter_id = datacenter_id.to_string();
	let cluster_id = unwrap_ref!(dc.cluster_id).as_uuid().to_string();
	let dt = (nomad_join_ts - install_complete_ts) as f64 / 1000.0;

	metrics::NOMAD_JOIN_DURATION
		.with_label_values(&[
			cluster_id.as_str(),
			datacenter_id.as_str(),
			&dc.provider_datacenter_id,
			&dc.name_id,
		])
		.observe(dt);

	Ok(())
}
