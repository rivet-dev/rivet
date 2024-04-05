use rivet_operation::prelude::*;

#[tokio::main]
async fn main() -> GlobalResult<()> {
	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
		.init();

	// TODO: When running bolt up, this service gets created first before `cluster-worker` so the messages
	// sent from here are received but effectively forgotten because `cluster-worker` gets restarted
	// immediately afterwards.
	tokio::time::sleep(std::time::Duration::from_secs(3)).await;

	cluster_default_update::run_from_env(false).await
}
