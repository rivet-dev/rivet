use chirp_workflow::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("pegboard-ws").await?;

	tokio::task::Builder::new()
		.name("pegboard_ws::health_checks")
		.spawn(rivet_health_checks::run_standalone(
			rivet_health_checks::Config {
				pools: Some(pools.clone()),
			},
		))?;

	tokio::task::Builder::new()
		.name("pegboard_ws::metrics")
		.spawn(rivet_metrics::run_standalone())?;

	pegboard_ws::run_from_env(pools.clone()).await
}
