use tracing_subscriber::prelude::*;
use chirp_workflow::prelude::*;

#[tokio::main]
async fn main() -> GlobalResult<()> {
	tracing_subscriber::registry()
		.with(
			tracing_logfmt::builder()
				.layer()
				.with_filter(tracing_subscriber::filter::LevelFilter::INFO),
		)
		.init();

	// TODO: When running bolt up, this service gets created first before `cluster-worker` so the messages
	// sent from here are received but effectively forgotten because `cluster-worker` gets restarted
	// immediately afterwards. This server will be replaced with a bolt infra step
	tokio::time::sleep(std::time::Duration::from_secs(3)).await;

	cluster_default_update::run_from_env(false).await
}
