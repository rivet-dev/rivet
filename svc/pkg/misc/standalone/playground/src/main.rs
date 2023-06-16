use rivet_operation::prelude::*;

#[tokio::main]
async fn main() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("playground").await?;
	let shared_client = chirp_client::SharedClient::from_env(pools.clone())?;
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;

	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
		.init();

	tokio::task::Builder::new()
		.name("playground::health_checks")
		.spawn(rivet_health_checks::run_standalone(
			rivet_health_checks::Config {
				pools: Some(pools.clone()),
			},
		))
		.unwrap();

	tokio::task::Builder::new()
		.name("playground::metrics")
		.spawn(rivet_metrics::run_standalone())
		.unwrap();

	playground::run_from_env(shared_client, pools, cache).await
}
