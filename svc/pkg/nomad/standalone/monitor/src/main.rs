use chirp_workflow::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("nomad-monitor").await?;

	tokio::task::Builder::new()
		.name("nomad_monitor::health_checks")
		.spawn(rivet_health_checks::run_standalone(
			rivet_health_checks::Config {
				pools: Some(pools.clone()),
			},
		))?;

	tokio::task::Builder::new()
		.name("nomad_monitor::metrics")
		.spawn(rivet_metrics::run_standalone())?;

	nomad_monitor::run_from_env(pools).await
}
