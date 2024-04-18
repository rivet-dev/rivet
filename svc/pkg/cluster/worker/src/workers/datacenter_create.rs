use chirp_worker::prelude::*;
use futures_util::FutureExt;
use proto::backend::{self, pkg::*};

#[worker(name = "cluster-datacenter-create")]
async fn worker(
	ctx: &OperationContext<cluster::msg::datacenter_create::Message>,
) -> GlobalResult<()> {
	let cluster_id = unwrap_ref!(ctx.cluster_id).as_uuid();
	let datacenter_id = unwrap_ref!(ctx.datacenter_id).as_uuid();

	let mut pools = ctx.pools.clone();

	// Cap the desired count below the max count
	for pool in &mut pools {
		if pool.desired_count > pool.max_count {
			pool.desired_count = pool.max_count;
		}
	}

	// Copy pools config to write to db
	let pools = cluster::msg::datacenter_create::Pools { pools };

	let mut pools_buf = Vec::with_capacity(pools.encoded_len());
	pools.encode(&mut pools_buf)?;

	rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();
		let pools_buf = pools_buf.clone();

		async move {
			sql_execute!(
				[ctx, @tx tx]
				"
				INSERT INTO db_cluster.datacenters (
					datacenter_id,
					cluster_id,
					name_id,
					display_name,
					provider,
					provider_datacenter_id,
					provider_api_token,
					pools,
					build_delivery_method,
					create_ts
				)
				VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
				",
				datacenter_id,
				cluster_id,
				&ctx.name_id,
				&ctx.display_name,
				ctx.provider as i64,
				&ctx.provider_datacenter_id,
				&ctx.provider_api_token,
				pools_buf,
				ctx.build_delivery_method as i64,
				util::timestamp::now(),
			)
			.await?;

			// Insert TLS record
			sql_execute!(
				[ctx, @tx tx]
				"
				INSERT INTO db_cluster.datacenter_tls (
					datacenter_id,
					state,
					expire_ts
				)
				VALUES ($1, $2, 0)
				",
				datacenter_id,
				backend::cluster::TlsState::Creating as i64,
			)
			.await?;

			Ok(())
		}
		.boxed()
	})
	.await?;

	// Start TLS issuing process
	msg!([ctx] cluster::msg::datacenter_tls_issue(datacenter_id) {
		datacenter_id: ctx.datacenter_id,
		renew: false,
	})
	.await?;

	// Scale servers
	msg!([ctx] cluster::msg::datacenter_scale(datacenter_id) {
		datacenter_id: ctx.datacenter_id,
	})
	.await?;

	Ok(())
}
