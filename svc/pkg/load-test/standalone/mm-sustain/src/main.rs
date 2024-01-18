use rivet_operation::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	load_test_mm_sustain::run_from_env(util::timestamp::now()).await?;

	tracing::info!("finished");

	std::future::pending::<()>().await;

	Ok(())
}
