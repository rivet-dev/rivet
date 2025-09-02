use anyhow::Result;
use gas::prelude::*;

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	let reg = pegboard::registry()?
		.merge(namespace::registry()?)?
		.merge(epoxy::registry()?)?;

	let db = db::DatabaseKv::from_pools(pools.clone()).await?;
	let worker = Worker::new(reg.handle(), db, config, pools);

	// Start worker
	worker.start(None).await
}
