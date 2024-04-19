use chirp_worker::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn basic() {
	if !util::feature::server_provision() {
		return;
	}

	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
		.init();

	let _ctx = TestCtx::from_env("cluster-gc-test").await.unwrap();
	let _pools = rivet_pools::from_env("cluster-gc-test").await.unwrap();

	// TODO:
}
