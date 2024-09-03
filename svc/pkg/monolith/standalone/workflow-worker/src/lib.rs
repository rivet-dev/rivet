use chirp_workflow::prelude::*;

#[tracing::instrument(skip_all)]
pub async fn run_from_env(pools: rivet_pools::Pools) -> GlobalResult<()> {
	let reg = cluster::registry()?.merge(linode::registry()?)?.merge(ds::registry()?)?.merge(job_run::registry()?)?;

	let db = db::DatabasePgNats::from_pools(pools.crdb()?, pools.nats()?);
	let worker = Worker::new(reg.handle(), db.clone());

	// Start worker
	worker.start_with_nats(pools).await?;
	bail!("worker exited unexpectedly");
}
