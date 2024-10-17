use chirp_worker::prelude::*;
use tracing_subscriber::prelude::*;

use ::workflow_gc::run_from_env;

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

	// TODO:
	run_from_env(util::timestamp::now(), pools).await.unwrap();
}
