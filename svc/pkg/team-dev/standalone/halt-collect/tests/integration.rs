use chirp_worker::prelude::*;

#[tokio::test]
async fn basic() {
	// Run tests sequentially
	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
		.init();

	team_dev_halt_collect::run_from_env(util::timestamp::now())
		.await
		.unwrap();
}
