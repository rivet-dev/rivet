use chirp_workflow::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("monolith-workflow-worker").await?;

	tokio::task::Builder::new()
		.name("monolith_workflow_worker::health_checks")
		.spawn(rivet_health_checks::run_standalone(
			rivet_health_checks::Config {
				pools: Some(pools.clone()),
			},
		))?;

	tokio::task::Builder::new()
		.name("monolith_workflow_worker::metrics")
		.spawn(rivet_metrics::run_standalone())?;

	monolith_workflow_worker::run_from_env(pools).await?;

	Ok(())
}
