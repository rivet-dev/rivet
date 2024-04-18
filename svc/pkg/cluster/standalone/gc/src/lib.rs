use futures_util::FutureExt;
use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct ServerRow {
	server_id: Uuid,
	datacenter_id: Uuid,
	pool_type: i64,
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

	let datacenter_ids = rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.base();

		async move {
			// Select all draining gg and ats servers
			let servers = sql_fetch_all!(
				[ctx, ServerRow, @tx tx]
				"
				SELECT server_id, datacenter_id, pool_type, drain_ts
				FROM db_cluster.datacenters AS d
				WHERE
					s.datacenter_id = d.datacenter_id AND
					pool_type = ANY($1) AND
					cloud_destroy_ts IS NULL AND
					drain_ts IS NOT NULL
				",
				&[backend::cluster::PoolType::Gg as i64, backend::cluster::PoolType::Ats as i64],
				ts,
			)
			.await?;

			let datacenters_res = op!([ctx] cluster_datacenter_get {
				datacenter_ids: servers
					.iter()
					.map(|server| server.datacenter_id.into())
					.collect::<Vec<_>>(),
			})
			.await?;

			let drained_servers = servers
				.into_iter()
				.map(|server| {
					let dc_id_proto = Some(server.datacenter_id.into());
					let datacenter = unwrap!(datacenters_res
						.datacenters
						.iter()
						.find(|dc| dc.datacenter_id == dc_id_proto));
					let pool = unwrap!(datacenter
						.pools
						.iter()
						.find(|pool| pool.pool_type == server.pool_type as i32));
					let drain_completed = server.drain_ts < ts - pool.drain_timeout as i64;

					Ok((server, drain_completed))
				})
				.filter(|res| {
					res.as_ref()
						.map_or(true, |(_, drain_completed)| *drain_completed)
				})
				.collect::<GlobalResult<Vec<_>>>()?;

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
		msg!([ctx] cluster::msg::datacenter_scale(datacenter_id) {
			datacenter_id: Some(datacenter_id.into()),
		})
		.await?;
	}

	Ok(())
}
