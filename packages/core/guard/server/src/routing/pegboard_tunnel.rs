use std::sync::Arc;

use anyhow::*;
use gas::prelude::*;
use rivet_guard_core::proxy_service::RoutingOutput;

/// Route requests to the pegboard-tunnel service
#[tracing::instrument(skip_all)]
pub async fn route_request(
	ctx: &StandaloneCtx,
	target: &str,
	_host: &str,
	_path: &str,
) -> Result<Option<RoutingOutput>> {
	// Check target
	if target != "tunnel" {
		return Ok(None);
	}

	// Create pegboard-tunnel service instance
	let tunnel = pegboard_tunnel::PegboardTunnelCustomServe::new(ctx.clone()).await?;

	Ok(Some(RoutingOutput::CustomServe(Arc::new(tunnel))))
}
