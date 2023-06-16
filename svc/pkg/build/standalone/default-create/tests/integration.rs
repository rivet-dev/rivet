#[tokio::test(flavor = "multi_thread")]
async fn basic() {
	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
		.init();

	// First run
	build_default_create::run_from_env().await.unwrap();

	// Second run, this should do nothing
	build_default_create::run_from_env().await.unwrap();
}
