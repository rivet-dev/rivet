use ::nomad_monitor::run_from_env;
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

	let pools = rivet_pools::from_env("nomad-monitor-test").await.unwrap();

	run_from_env(pools).await.unwrap();
}
