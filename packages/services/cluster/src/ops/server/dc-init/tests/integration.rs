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

	pegboard_dc_init::run_from_env().await.unwrap();
}
