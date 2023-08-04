use rivet_operation::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	{{snake pkg}}_{{snake name}}::run_from_env(util::timestamp::now()).await?;

	Ok(())
}
