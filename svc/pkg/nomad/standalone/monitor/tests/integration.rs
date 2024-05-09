use ::nomad_monitor::run_from_env;
use chirp_worker::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn basic() {
	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
		.init();

	let pools = rivet_pools::from_env("nomad-monitor-test").await.unwrap();

	run_from_env(pools).await.unwrap();
}
