use chirp_workflow::prelude::*;

#[tracing::instrument(skip_all)]
pub async fn run_from_env(pools: rivet_pools::Pools) -> GlobalResult<()> {
	let reg = cluster::registry().merge(linode::registry());

	let db = db::DatabasePostgres::from_pool(pools.crdb().unwrap());
	let worker = Worker::new(reg.handle(), db.clone());

	// Start worker
	worker.start(pools).await?;
	bail!("worker exited unexpectedly");
}
