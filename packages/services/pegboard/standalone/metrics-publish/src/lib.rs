use chirp_workflow::prelude::*;

pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let mut interval =
		tokio::time::interval(std::time::Duration::from_secs(15));
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

	let (client_ping, client_actors) = tokio::try_join!(
		sql_fetch_all!(
			[ctx, (Uuid, i64)]
			"
			SELECT last_ping_ts
			FROM db_pegboard.clients AS OF SYSTEM TIME '-1s'
			WHERE delete_ts IS NULL
			",
			ts - util::duration::seconds(30),
		),
		sql_fetch_all!(
			[ctx, (Uuid, i64)]
			"
			SELECT a.client_id, COUNT(*)
			FROM db_pegboard.actors AS a
			JOIN db_pegboard.clients AS c
			ON a.client_id = c.client_id
			AS OF SYSTEM TIME '-1s'
			WHERE c.delete_ts IS NULL
			GROUP BY a.client_id
			",
		),
	)?;

	for (client_id, last_ping_ts) in client_ping {
		pegboard::metrics::CLIENT_LAST_PING
			.with_label_values(&[&client_id.to_string()])
			.set(last_ping_ts);
	}

	for (client_id, count) in client_actors {
		pegboard::metrics::CLIENT_ACTORS_ALLOCATED
			.with_label_values(&[&client_id.to_string()])
			.set(count.try_into()?);
	}

	Ok(())
}
