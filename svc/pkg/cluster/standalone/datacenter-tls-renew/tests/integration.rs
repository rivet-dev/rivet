use chirp_worker::prelude::*;
use tracing_subscriber::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn basic() {
	if !util::feature::server_provision() {
		return;
	}

	tracing_subscriber::registry()
		.with(
			tracing_logfmt::builder()
				.layer()
				.with_filter(tracing_subscriber::filter::LevelFilter::INFO),
		)
		.init();

	let _ctx = TestCtx::from_env("cluster-gc-test").await.unwrap();
	let _pools = rivet_pools::from_env("cluster-gc-test").await.unwrap();

	// TODO:
}
