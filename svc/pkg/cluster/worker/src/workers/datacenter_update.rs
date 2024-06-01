use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "cluster-datacenter-update")]
async fn worker(
	ctx: &OperationContext<cluster::msg::datacenter_update::Message>,
) -> GlobalResult<()> {
	let datacenter_id = unwrap_ref!(ctx.datacenter_id).as_uuid();

	let datacenter_res = op!([ctx] cluster_datacenter_get {
		datacenter_ids: vec![datacenter_id.into()],
	})
	.await?;
	let datacenter = unwrap!(
		datacenter_res.datacenters.first(),
		"datacenter does not exist"
	);

	// Update pools config
	let mut new_pools = cluster::msg::datacenter_create::Pools {
		pools: datacenter.pools.clone(),
	};
	for pool in &ctx.pools {
		let current_pool = unwrap!(
			new_pools
				.pools
				.iter_mut()
				.find(|p| p.pool_type == pool.pool_type),
			"attempting to update pool that doesn't exist in current config"
		);

		// Update pool config
		if !pool.hardware.is_empty() {
			current_pool.hardware.clone_from(&pool.hardware);
		}
		if let Some(desired_count) = pool.desired_count {
			current_pool.desired_count = desired_count;
		}
		if let Some(min_count) = pool.min_count {
			current_pool.min_count = min_count;
		}
		if let Some(max_count) = pool.max_count {
			current_pool.max_count = max_count;
		}
		if let Some(drain_timeout) = pool.drain_timeout {
			current_pool.drain_timeout = drain_timeout;
		}
	}

	// Encode config
	let mut pools_buf = Vec::with_capacity(new_pools.encoded_len());
	new_pools.encode(&mut pools_buf)?;

	// Update pools
	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.datacenters
		SET pools = $2
		WHERE datacenter_id = $1
		",
		datacenter_id,
		pools_buf,
	)
	.await?;

	msg!([ctx] cluster::msg::datacenter_scale(datacenter_id) {
		datacenter_id: ctx.datacenter_id,
	})
	.await?;

	Ok(())
}
