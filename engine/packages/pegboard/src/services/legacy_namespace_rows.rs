//! Legacy: entry point of the service that moves every legacy namespace row of the catalog into
//! its namespace volume once the fleet reads namespace volumes; see
//! `crate::legacy_namespace_rows`. The legacy behavior moved a row only when a request read it.
//! Started on every storage worker; only the elected one walks.
use anyhow::Result;
use rivet_engine_context::EngineContext;

/// Legacy: runs the pass for this process's lifetime, until the caller drops the future.
pub async fn legacy_start(ctx: EngineContext) -> Result<()> {
	crate::legacy_namespace_rows::legacy_run(ctx).await
}
