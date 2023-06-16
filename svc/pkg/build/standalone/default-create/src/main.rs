use rivet_operation::prelude::*;

#[tokio::main]
async fn main() -> GlobalResult<()> {
	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
		.init();

	build_default_create::run_from_env().await
}
