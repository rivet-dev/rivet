use chirp_workflow::prelude::*;

pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	run_from_env(config, pools).await?;

	Ok(())
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
) -> GlobalResult<()> {
	let reg = ds::registry()?.merge(pegboard::registry()?)?;

	let db = db::DatabaseFdbSqliteNats::from_pools(pools.clone())?;
	let worker = Worker::new(reg.handle(), db);

	// Start worker
	worker.start(config, pools).await?;
	bail!("worker exited unexpectedly");
}
