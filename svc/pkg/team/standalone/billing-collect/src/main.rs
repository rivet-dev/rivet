use rivet_operation::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	if util::env::is_billing_enabled() {
		team_billing_collect::run_from_env(util::timestamp::now()).await?;
	}

	Ok(())
}
