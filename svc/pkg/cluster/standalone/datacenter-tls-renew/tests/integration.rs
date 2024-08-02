use tracing_subscriber::prelude::*;
use chirp_workflow::prelude::*;

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

	let _ctx = TestCtx::from_env("cluster-datacenter-tls-renew-test").await;
	let _pools = rivet_pools::from_env("cluster-datacenter-tls-renew-test")
		.await
		.unwrap();

	// TODO:
}
