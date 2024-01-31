use rivet_operation::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("load-test-sqlx").await?;
	let _shared_client = chirp_client::SharedClient::from_env(pools.clone())?;

	tokio::task::Builder::new()
		.name("load_test_sqlx::health_checks")
		.spawn(rivet_health_checks::run_standalone(
			rivet_health_checks::Config {
				pools: Some(pools.clone()),
			},
		))?;

	tokio::task::Builder::new()
		.name("load_test_sqlx::metrics")
		.spawn(rivet_metrics::run_standalone())?;

	load_test_sqlx::run_from_env(util::timestamp::now()).await?;

	Ok(())
}
