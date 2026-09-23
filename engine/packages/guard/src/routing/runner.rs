use crate::shared_state::SharedState;
use anyhow::Result;
use gas::prelude::*;
use rivet_guard_core::{RoutingOutput, request_context::RequestContext};
use std::sync::Arc;

use super::{check_connection_auth, validate_regional_host};

/// Route requests to the runner service using header-based routing
#[tracing::instrument(skip_all)]
pub async fn route_request(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	req_ctx: &RequestContext,
	target: &str,
) -> Result<Option<RoutingOutput>> {
	if target != "runner" {
		return Ok(None);
	}

	tracing::debug!(hostname=%req_ctx.hostname(), path=%req_ctx.path_for_logs(), "routing to runner via header");

	route_request_inner(ctx, shared_state, req_ctx)
		.await
		.map(Some)
}

/// Route requests to the runner service using path-based routing
/// Matches path: /runners/connect
#[tracing::instrument(skip_all)]
pub async fn route_request_path_based(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	req_ctx: &RequestContext,
) -> Result<Option<RoutingOutput>> {
	// Check if path matches /runners/connect
	let path_without_query = req_ctx.path().split('?').next().unwrap_or(req_ctx.path());
	if path_without_query != "/runners/connect" && path_without_query != "/runners/connect/" {
		return Ok(None);
	}

	tracing::debug!(hostname=%req_ctx.hostname(), path=%req_ctx.path_for_logs(), "routing to runner via path");

	route_request_inner(ctx, shared_state, req_ctx)
		.await
		.map(Some)
}

/// Internal runner routing logic shared by both header-based and path-based routing
#[tracing::instrument(skip_all)]
async fn route_request_inner(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	req_ctx: &RequestContext,
) -> Result<RoutingOutput> {
	validate_regional_host(ctx, req_ctx)?;

	check_connection_auth(ctx, shared_state, req_ctx, "runner").await?;

	let tunnel = pegboard_runner::PegboardRunnerWsCustomServe::new(ctx.clone());
	Ok(RoutingOutput::CustomServe(Arc::new(tunnel)))
}
