use chirp_workflow::prelude::*;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> GlobalResult<()> {
	tracing_subscriber::registry()
		.with(
			tracing_logfmt::builder()
				.layer()
				.with_filter(tracing_subscriber::filter::LevelFilter::INFO),
		)
		.init();

	cluster_workflow_backfill::run_from_env().await
}
