#[tokio::test(flavor = "multi_thread")]
async fn basic() {
	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
		.init();

	upload_provider_fill::run_from_env().await.unwrap();
}
