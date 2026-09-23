use crate::shared_state::SharedState;
use anyhow::Result;
use gas::prelude::*;
use rivet_guard_core::{RoutingOutput, request_context::RequestContext};
use std::sync::Arc;

use super::{check_connection_auth, validate_regional_host};

/// Route requests to the envoy service using header-based routing
#[tracing::instrument(skip_all)]
pub async fn route_request(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	req_ctx: &RequestContext,
	target: &str,
) -> Result<Option<RoutingOutput>> {
	if target != "envoy" {
		return Ok(None);
	}

	tracing::debug!(hostname=%req_ctx.hostname(), path=%req_ctx.path_for_logs(), "routing to envoy via header");

	route_envoy_internal(ctx, shared_state, req_ctx)
		.await
		.map(Some)
}

/// Route requests to the envoy service using path-based routing
/// Matches path: /envoys/connect
#[tracing::instrument(skip_all)]
pub async fn route_request_path_based(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	req_ctx: &RequestContext,
) -> Result<Option<RoutingOutput>> {
	// Check if path matches /envoys/connect
	let path_without_query = req_ctx.path().split('?').next().unwrap_or(req_ctx.path());
	if path_without_query != "/envoys/connect" && path_without_query != "/envoys/connect/" {
		return Ok(None);
	}

	tracing::debug!(hostname=%req_ctx.hostname(), path=%req_ctx.path_for_logs(), "routing to envoy via path");

	route_envoy_internal(ctx, shared_state, req_ctx)
		.await
		.map(Some)
}

/// Internal envoy routing logic shared by both header-based and path-based routing
#[tracing::instrument(skip_all)]
async fn route_envoy_internal(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	req_ctx: &RequestContext,
) -> Result<RoutingOutput> {
	validate_regional_host(ctx, req_ctx)?;

	check_connection_auth(ctx, shared_state, req_ctx, "envoy").await?;

	let tunnel = pegboard_envoy::PegboardEnvoyWs::new(&ctx);
	Ok(RoutingOutput::CustomServe(Arc::new(tunnel)))
}
