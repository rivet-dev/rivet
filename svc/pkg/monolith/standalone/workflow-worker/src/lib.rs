use chirp_workflow::prelude::*;

#[tracing::instrument(skip_all)]
pub async fn run_from_env(pools: rivet_pools::Pools) -> GlobalResult<()> {
	let reg = foo_worker::registry();

	let db = db::DatabasePostgres::from_pool(pools.crdb().unwrap());
	let worker = Worker::new(reg.handle(), db.clone());

	// Start worker
	match worker.start(pools).await {
		Ok(_) => {
			bail!("worker exited unexpectedly")
		}
		Err(err) => {
			return Err(err);
		}
	}
}
