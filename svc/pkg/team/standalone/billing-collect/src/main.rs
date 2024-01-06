use rivet_operation::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	if util::env::billing().is_some() {
		team_billing_collect::run_from_env(util::timestamp::now()).await?;
	}

	Ok(())
}
