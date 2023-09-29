use rivet_operation::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("job-run-alloc-plan-monitor").await?;
	let shared_client = chirp_client::SharedClient::from_env(pools.clone())?;
	let redis_job = pools.redis("job")?;

	tokio::task::Builder::new()
		.name("job_run_alloc_plan_monitor::health_checks")
		.spawn(rivet_health_checks::run_standalone(
			rivet_health_checks::Config {
				pools: Some(pools.clone()),
			},
		))?;

	tokio::task::Builder::new()
		.name("job_run_alloc_plan_monitor::metrics")
		.spawn(rivet_metrics::run_standalone())?;

	job_run_alloc_plan_monitor::start(shared_client, redis_job).await?;

	Ok(())
}
