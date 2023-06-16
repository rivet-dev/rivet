use rivet_operation::prelude::*;
use std::time::Duration;

fn main() -> GlobalResult<()> {
	rivet_runtime::run(start()).unwrap()
}

async fn start() -> GlobalResult<()> {
	// TODO: Handle ctrl-c

	let pools = rivet_pools::from_env("mm-gc").await?;
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;

	tokio::task::Builder::new()
		.name("mm_gc::health_checks")
		.spawn(rivet_health_checks::run_standalone(
			rivet_health_checks::Config {
				pools: Some(pools.clone()),
			},
		))
		.unwrap();

	tokio::task::Builder::new()
		.name("mm_gc::metrics")
		.spawn(rivet_metrics::run_standalone())
		.unwrap();

	let mut interval = tokio::time::interval(Duration::from_secs(15));
	loop {
		interval.tick().await;

		let ts = util::timestamp::now();
		let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("mm-gc");
		let ctx = OperationContext::new(
			"mm-gc".into(),
			std::time::Duration::from_secs(60),
			rivet_connection::Connection::new(client, pools.clone(), cache.clone()),
			Uuid::new_v4(),
			Uuid::new_v4(),
			ts,
			ts,
			(),
			Vec::new(),
		);

		mm_gc::run_from_env(ts, ctx).await?;
	}
}
