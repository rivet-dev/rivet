use rivet_operation::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("monolith-worker").await?;

	monolith_worker::run_from_env(pools).await?;

	Ok(())
}
