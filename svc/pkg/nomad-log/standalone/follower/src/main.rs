use rivet_operation::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("nomad-log-follower").await?;
	let shared_client = chirp_client::SharedClient::from_env(pools.clone())?;

	tokio::task::Builder::new()
		.name("nomad_log_follower::health_checks")
		.spawn(rivet_health_checks::run_standalone(
			rivet_health_checks::Config {
				pools: Some(pools.clone()),
			},
		))?;

	tokio::task::Builder::new()
		.name("nomad_log_follower::metrics")
		.spawn(rivet_metrics::run_standalone())?;

	nomad_log_follower::start(shared_client).await?;

	Ok(())
}
