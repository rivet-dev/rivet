use ::telemetry_beacon::run_from_env;
use chirp_worker::prelude::*;
use tracing_subscriber::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn telemetry_beacon() {
	tracing_subscriber::registry()
		.with(
			tracing_logfmt::builder()
				.layer()
				.with_filter(tracing_subscriber::filter::LevelFilter::INFO),
		)
		.init();

	let config = rivet_config::Config::load::<String>(&[]).await.unwrap();
	let pools = rivet_pools::Pools::new(config.clone()).await.unwrap();
	run_from_env(config, pools, util::timestamp::now())
		.await
		.unwrap();
}
