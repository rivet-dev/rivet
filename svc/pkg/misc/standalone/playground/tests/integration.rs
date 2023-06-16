#[tokio::test(flavor = "multi_thread")]
async fn basic() {
	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
		.init();

	// First run
	let pools = rivet_pools::from_env("playground").await.unwrap();
	let shared_client = chirp_client::SharedClient::from_env(pools.clone()).unwrap();
	let cache = rivet_cache::CacheInner::from_env(pools.clone()).unwrap();

	playground::run_from_env(shared_client, pools, cache)
		.await
		.unwrap();

	// // Second run, this should do nothing
	// let pools = rivet_pools::from_env("playground").await.unwrap();
	// let shared_client = chirp_client::SharedClient::from_env(pools.clone()).unwrap();
	// playground::run_from_env(shared_client, pools)
	// 	.await
	// 	.unwrap();
}
