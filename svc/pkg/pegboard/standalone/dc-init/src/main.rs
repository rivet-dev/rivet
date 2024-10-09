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

	// TODO: This functionality should be put in the cluster dc workflow and backfilled, but we don't have
	// the ability to update workflows yet
	pegboard_dc_init::run_from_env().await
}
