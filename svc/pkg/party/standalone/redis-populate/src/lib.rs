use rivet_operation::prelude::*;

#[tracing::instrument]
pub async fn run_from_env() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("party_redis_populate").await?;
	let mut redis = pools.redis("redis-party")?;

	redis::cmd("EVAL")
		.arg(include_str!("../redis-scripts/main.lua"))
		.arg(0)
		.arg(util::timestamp::now())
		.query_async(&mut redis)
		.await?;

	tracing::info!("eval complete");

	Ok(())
}
