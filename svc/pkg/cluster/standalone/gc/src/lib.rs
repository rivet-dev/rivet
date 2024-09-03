use std::convert::TryInto;

use chirp_workflow::prelude::*;
use cluster::types::PoolType;
use futures_util::FutureExt;

#[derive(sqlx::FromRow)]
struct ServerRow {
	server_id: Uuid,
	datacenter_id: Uuid,
	pool_type2: Option<sqlx::types::Json<PoolType>>,
	pool_type: i64,
	drain_ts: i64,
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(ts: i64, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("cluster-gc");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		rivet_connection::Connection::new(client, pools, cache),
		"cluster-gc",
	)
	.await?;

	let datacenter_ids = rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();

		async move {
			// Select all draining servers
			let servers = sql_fetch_all!(
				[ctx, ServerRow, @tx tx]
				"
				SELECT server_id, datacenter_id, pool_type, pool_type2, drain_ts
				FROM db_cluster.servers
				WHERE
					drain_ts IS NOT NULL AND
					drain_complete_ts IS NULL AND
					cloud_destroy_ts IS NULL
				",
			)
			.await?;
			if servers.is_empty() {
				return Ok(Vec::new());
			}

			// Fetch relevant datacenters
			let datacenters_res = ctx
				.op(cluster::ops::datacenter::get::Input {
					datacenter_ids: servers
						.iter()
						.map(|server| server.datacenter_id)
						.collect::<Vec<_>>(),
				})
				.await?;

			// Determine which servers are finished draining via their drain timeout
			let drained_servers = servers
				.into_iter()
				.map(|server| {
					let pool_type = if let Some(pool_type) = server.pool_type2.clone() {
						pool_type.0
					} else {
						server.pool_type.try_into()?
					};
					let datacenter = unwrap!(datacenters_res
						.datacenters
						.iter()
						.find(|dc| dc.datacenter_id == server.datacenter_id));
					let pool = unwrap!(datacenter
						.pools
						.iter()
						.find(|pool| pool.pool_type == pool_type));
					let drain_completed = server.drain_ts < ts - pool.drain_timeout as i64;

					tracing::info!(
						server_id=?server.server_id,
						drain_ts=%server.drain_ts,
						pool_drain_timeout=%pool.drain_timeout,
						%drain_completed,
					);

					Ok((server, drain_completed))
				})
				.filter(|res| {
					res.as_ref()
						.map_or(true, |(_, drain_completed)| *drain_completed)
				})
				.collect::<GlobalResult<Vec<_>>>()?;

			if drained_servers.is_empty() {
				return Ok(Vec::new());
			}

			tracing::info!("{} servers done draining", drained_servers.len());

			// Update servers that have completed draining
			sql_execute!(
				[ctx, @tx tx]
				"
				UPDATE db_cluster.servers
				SET drain_complete_ts = $2
				WHERE
					server_id = ANY($1) AND
					cloud_destroy_ts IS NULL
				",
				drained_servers.iter().map(|(server, _)| server.server_id).collect::<Vec<_>>(),
				ts,
			)
			.await?;

			Ok(drained_servers
				.into_iter()
				.map(|(server, _)| server.datacenter_id)
				.collect::<Vec<_>>())
		}
		.boxed()
	})
	.await?;

	// Scale
	for datacenter_id in datacenter_ids {
		ctx.signal(cluster::workflows::datacenter::Scale {})
			.tag("datacenter_id", datacenter_id)
			.send()
			.await?;
	}

	Ok(())
}
