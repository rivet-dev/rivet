use chirp_workflow::prelude::*;
use pegboard::protocol::ClientFlavor;

/// How long to continue including a row in the query after a client has been deleted. This is so we can add
/// `inactive=true` for the prometheus metric.
const INACTIVE_THRESHOLD_MS: i64 = util::duration::hours(1);

pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
	loop {
		interval.tick().await;

		run_from_env(config.clone(), pools.clone(), util::timestamp::now()).await?;
	}
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	ts: i64,
) -> GlobalResult<()> {
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("pegboard-metrics-publish");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		config,
		rivet_connection::Connection::new(client, pools, cache),
		"pegboard-metrics-publish",
	)
	.await?;

	let client_actors = sql_fetch_all!(
		[ctx, (Uuid, Uuid, i64, bool, i64)]
		"
		SELECT
			c.datacenter_id,
			c.client_id,
			c.flavor,
			(c.drain_ts IS NOT NULL OR c.delete_ts IS NOT NULL) AS inactive,
			COUNT(a.client_id)
		FROM db_pegboard.clients AS c
		LEFT JOIN db_pegboard.actors AS a
		ON a.client_id = c.client_id
		AS OF SYSTEM TIME '-1s'
		WHERE c.delete_ts IS NULL OR c.delete_ts > $1
		GROUP BY c.client_id
		",
		ts - INACTIVE_THRESHOLD_MS,
	)
	.await?;

	for (datacenter_id, client_id, flavor, inactive, count) in client_actors {
		let flavor = unwrap!(ClientFlavor::from_repr(flavor.try_into()?));

		pegboard::metrics::CLIENT_ACTORS_ALLOCATED
			.with_label_values(&[
				&datacenter_id.to_string(),
				&client_id.to_string(),
				&flavor.to_string(),
				&inactive.to_string(),
			])
			.set(count.try_into()?);
	}

	Ok(())
}
