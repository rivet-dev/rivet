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

	// First run
	build_default_create::run_from_env().await.unwrap();

	// Second run, this should do nothing
	build_default_create::run_from_env().await.unwrap();
}
