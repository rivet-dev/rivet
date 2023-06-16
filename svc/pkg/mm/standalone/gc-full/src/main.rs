use rivet_operation::prelude::*;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	mm_gc_full::run_from_env(util::timestamp::now()).await?;

	Ok(())
}
