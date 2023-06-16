use rivet_operation::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("job-run-eval-update-monitor").await?;
	let shared_client = chirp_client::SharedClient::from_env(pools.clone())?;
	let redis_job = pools.redis("redis-job")?;

	tokio::task::Builder::new()
		.name("job_run_eval_update_monitor::health_checks")
		.spawn(rivet_health_checks::run_standalone(
			rivet_health_checks::Config {
				pools: Some(pools.clone()),
			},
		))?;

	tokio::task::Builder::new()
		.name("job_run_eval_update_monitor::metrics")
		.spawn(rivet_metrics::run_standalone())?;

	job_run_eval_update_monitor::start(shared_client, redis_job).await?;

	Ok(())
}
