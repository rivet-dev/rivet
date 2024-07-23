use chirp_worker::prelude::*;
use tracing_subscriber::prelude::*;

use ::cluster_fix_tls::run_from_env;

#[tokio::test(flavor = "multi_thread")]
async fn basic() {
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
