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

	cluster_default_update::run_from_env(false).await.unwrap();
}
