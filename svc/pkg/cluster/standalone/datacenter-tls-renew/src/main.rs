use std::time::Duration;

use rivet_operation::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("cluster-datacenter-tls-renew").await?;

	tokio::task::Builder::new()
		.name("cluster_datacenter_tls_renew::health_checks")
		.spawn(rivet_health_checks::run_standalone(
			rivet_health_checks::Config {
				pools: Some(pools.clone()),
			},
		))?;

	tokio::task::Builder::new()
		.name("cluster_datacenter_tls_renew::metrics")
		.spawn(rivet_metrics::run_standalone())?;

	let mut interval = tokio::time::interval(Duration::from_secs(60 * 60));
	loop {
		interval.tick().await;

		cluster_datacenter_tls_renew::run_from_env(pools.clone()).await?;
	}
}
