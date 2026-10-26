//! Entry point of the service that runs lost checks for closed Envoy connections.
use anyhow::Result;
use rivet_engine_context::EngineContext;

/// Runs the process's lost checks until the caller drops the future.
pub async fn start(ctx: EngineContext, service: crate::Service) -> Result<()> {
	service.lost_checks.run(&ctx).await
}
