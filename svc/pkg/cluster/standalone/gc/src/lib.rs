use std::collections::HashMap;

use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[tracing::instrument(skip_all)]
pub async fn run_from_env(ts: i64, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("cluster-gc");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"cluster-gc".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
		Vec::new(),
	);
	let crdb = ctx.crdb().await?;

	// Update all draining gg and ats servers that have completed draining
	let datacenter_rows = sql_fetch_all!(
		[ctx, (Uuid,), &crdb]
		"
		WITH updated AS (
			UPDATE db_cluster.servers AS s
			SET drain_complete_ts = $2
			FROM db_cluster.datacenters AS d
			WHERE
				s.datacenter_id = d.datacenter_id AND
				pool_type = ANY($1) AND
				cloud_destroy_ts IS NULL AND
				drain_ts IS NOT NULL AND
				drain_ts < $2 - d.drain_timeout
			RETURNING s.datacenter_id
		)
		SELECT DISTINCT datacenter_id
		FROM updated
		",
		&[backend::cluster::PoolType::Gg as i64, backend::cluster::PoolType::Ats as i64],
		ts,
	)
	.await?;

	// Scale
	for (datacenter_id,) in datacenter_rows {
		msg!([ctx] cluster::msg::datacenter_scale(datacenter_id) {
			datacenter_id: Some(datacenter_id.into()),
		})
		.await?;
	}

	Ok(())
}
