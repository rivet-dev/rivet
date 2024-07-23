use ::load_test_mm::run_from_env;
use chirp_worker::prelude::*;
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

	if !util::feature::job_run() {
		return;
	}

	run_from_env(util::timestamp::now()).await.unwrap();
}
