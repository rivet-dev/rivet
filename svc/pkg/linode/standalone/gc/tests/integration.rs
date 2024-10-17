use ::linode_gc::run_from_env;
use chirp_workflow::prelude::*;
use tracing_subscriber::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn basic() {
	tracing_subscriber::registry()
		.with(
			tracing_logfmt::builder()
				.layer()
				.with_filter(tracing_subscriber::filter::LevelFilter::INFO),
		)
		.init();

	let pools = rivet_pools::from_env().await.unwrap();

	run_from_env(pools).await.unwrap();
}
