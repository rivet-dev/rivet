use anyhow::*;
use gas::prelude::*;
use rivet_guard_core::proxy_service::{RouteConfig, RouteTarget, RoutingOutput, RoutingTimeout};
use std::sync::Arc;

/// Route requests to the API service
#[tracing::instrument(skip_all)]
pub async fn route_request(
	ctx: &StandaloneCtx,
	target: &str,
	_host: &str,
	path: &str,
) -> Result<Option<RoutingOutput>> {
	if target != "runner" {
		return Ok(None);
	}

	let tunnel = pegboard_runner::PegboardRunnerWsCustomServe::new(ctx.clone());
	Ok(Some(RoutingOutput::CustomServe(Arc::new(tunnel))))
}
