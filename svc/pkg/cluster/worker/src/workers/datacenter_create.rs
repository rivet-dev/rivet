use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "cluster-datacenter-create")]
async fn worker(
	ctx: &OperationContext<cluster::msg::datacenter_create::Message>,
) -> GlobalResult<()> {
	let mut config = unwrap_ref!(ctx.config).clone();
	let cluster_id = unwrap_ref!(config.cluster_id).as_uuid();
	let datacenter_id = unwrap_ref!(config.datacenter_id).as_uuid();

	// Ensure that the desired count is below the max count
	for pool in &mut config.pools {
		if pool.desired_count > pool.max_count {
			pool.desired_count = pool.max_count;
		}
	}

	// Copy pools config to write to db
	let pools = cluster::msg::datacenter_create::Pools {
		pools: config.pools.clone(),
	};

	let mut pools_buf = Vec::with_capacity(pools.encoded_len());
	pools.encode(&mut pools_buf)?;

	sql_execute!(
		[ctx]
		"
		INSERT INTO db_cluster.datacenters (
			datacenter_id,
			cluster_id,
			name_id,
			display_name,
			provider,
			provider_datacenter_id,
			pools,
			build_delivery_method,
			drain_timeout
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
		",
		datacenter_id,
		cluster_id,
		&config.name_id,
		&config.display_name,
		config.provider as i64,
		&config.provider_datacenter_id,
		pools_buf,
		config.build_delivery_method as i64,
		config.drain_timeout as i64
	)
	.await?;

	msg!([ctx] cluster::msg::datacenter_scale(datacenter_id) {
		datacenter_id: config.datacenter_id,
	})
	.await?;

	Ok(())
}
