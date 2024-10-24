use ::load_test_watch_requests::run_from_env;
use chirp_worker::prelude::*;
use tracing_subscriber::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn basic() {
	if !util::feature::dns() {
		return;
	}

	tracing_subscriber::registry()
		.with(
			tracing_logfmt::builder()
				.layer()
				.with_filter(tracing_subscriber::filter::LevelFilter::INFO),
		)
		.init();

	// TODO:
	run_from_env(util::timestamp::now()).await.unwrap();
}
