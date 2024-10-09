use std::time::Duration;

use chirp_workflow::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("pegboard-gc").await?;

	tokio::task::Builder::new()
		.name("pegboard_gc::health_checks")
		.spawn(rivet_health_checks::run_standalone(
			rivet_health_checks::Config {
				pools: Some(pools.clone()),
			},
		))?;

	tokio::task::Builder::new()
		.name("pegboard_gc::metrics")
		.spawn(rivet_metrics::run_standalone())?;

	let mut interval = tokio::time::interval(Duration::from_secs(15));
	loop {
		interval.tick().await;

		let ts = util::timestamp::now();
		pegboard_gc::run_from_env(ts, pools.clone()).await?;
	}
}
