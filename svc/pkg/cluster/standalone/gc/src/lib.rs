use std::collections::HashMap;

use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Server {
	datacenter_id: Uuid,
	server_id: Uuid,
	drain_ts: i64,
}

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

	// Select all draining gg servers
	let gg_servers = sql_fetch_all!(
		[ctx, Server, &crdb]
		"
		SELECT datacenter_id, server_id, drain_ts
		FROM db_cluster.servers
		WHERE
			pool_type = $1 AND
			cloud_destroy_ts IS NULL AND
			drain_ts IS NOT NULL
		",
		backend::cluster::PoolType::Gg as i64
	)
	.await?;

	if gg_servers.is_empty() {
		return Ok(());
	}

	let datacenters_res = op!([ctx] cluster_datacenter_get {
		datacenter_ids: gg_servers
			.iter()
			.map(|server| server.datacenter_id.into())
			.collect::<Vec<_>>(),
	})
	.await?;

	// Collect into hashmap for better reads
	let datacenters = datacenters_res
		.datacenters
		.iter()
		.map(|dc| Ok((unwrap_ref!(dc.datacenter_id).as_uuid(), dc)))
		.collect::<GlobalResult<HashMap<_, _>>>()?;

	// Filter all gg servers that are finished draining
	let destroy_server_ids = gg_servers
		.iter()
		.map(|server| {
			let datacenter_config = unwrap!(datacenters.get(&server.datacenter_id));
			let drain_cutoff = ts - datacenter_config.drain_timeout as i64;

			Ok((server, drain_cutoff))
		})
		.filter_map(|res| match res {
			Ok((server, drain_cutoff)) => {
				if server.drain_ts < drain_cutoff {
					Some(Ok(server.server_id))
				} else {
					None
				}
			}
			Err(err) => Some(Err(err)),
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	// Mark as destroyed
	sql_execute!(
		[ctx, &crdb]
		"
		UPDATE db_cluster.servers
		SET cloud_destroy_ts = $2
		WHERE server_id = ANY($1)
		",
		&destroy_server_ids,
		util::timestamp::now(),
	)
	.await?;

	for server_id in destroy_server_ids {
		msg!([ctx] cluster::msg::server_destroy(server_id) {
			server_id: Some(server_id.into()),
			force: false,
		})
		.await?;
	}

	Ok(())
}
