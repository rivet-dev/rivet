use ::linode_gc::run_from_env;
use tracing_subscriber::prelude::*;
use chirp_workflow::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn basic() {
	tracing_subscriber::registry()
		.with(
			tracing_logfmt::builder()
				.layer()
				.with_filter(tracing_subscriber::filter::LevelFilter::INFO),
		)
		.init();

	let pools = rivet_pools::from_env("linode-gc-test").await.unwrap();

	run_from_env(pools).await.unwrap();
}
