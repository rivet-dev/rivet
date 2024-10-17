use chirp_workflow::prelude::*;

use cluster::types::TlsState;

// How much time before the cert expires to renew it
const EXPIRE_PADDING: i64 = util::duration::days(30);

pub async fn start() -> GlobalResult<()> {
	let pools = rivet_pools::from_env().await?;

	run_from_env(pools).await
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?
		.wrap_new("cluster-datacenter-tls-renew");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		rivet_connection::Connection::new(client, pools, cache),
		"cluster-datacenter-tls-renew",
	)
	.await?;

	let updated_datacenter_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		UPDATE db_cluster.datacenter_tls
		SET state = $3
		WHERE
			state = $1 AND
			expire_ts < $2
		RETURNING datacenter_id
		",
		TlsState::Active as i32,
		util::timestamp::now() + EXPIRE_PADDING,
		TlsState::Renewing as i32,
	)
	.await?
	.into_iter()
	.map(|(datacenter_id,)| datacenter_id)
	.collect::<Vec<_>>();

	for datacenter_id in updated_datacenter_ids {
		ctx.signal(cluster::workflows::datacenter::TlsRenew {})
			.tag("datacenter_id", datacenter_id)
			.send()
			.await?;
	}

	Ok(())
}
