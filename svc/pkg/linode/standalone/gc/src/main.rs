use std::time::Duration;

use rivet_operation::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("linode-gc").await?;

	tokio::task::Builder::new()
		.name("linode_gc::health_checks")
		.spawn(rivet_health_checks::run_standalone(
			rivet_health_checks::Config {
				pools: Some(pools.clone()),
			},
		))?;

	tokio::task::Builder::new()
		.name("linode_gc::metrics")
		.spawn(rivet_metrics::run_standalone())?;

	let mut interval = tokio::time::interval(Duration::from_secs(15));
	loop {
		interval.tick().await;

		linode_gc::run_from_env(pools.clone()).await?;
	}
}
