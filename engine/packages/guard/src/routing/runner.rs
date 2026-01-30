use anyhow::*;
use gas::prelude::*;
use rivet_guard_core::{RoutingOutput, request_context::RequestContext};
use std::sync::Arc;

use super::{SEC_WEBSOCKET_PROTOCOL, X_RIVET_TOKEN};
pub(crate) const WS_PROTOCOL_TOKEN: &str = "rivet_token.";

/// Route requests to the runner service using header-based routing
#[tracing::instrument(skip_all)]
pub async fn route_request(
	ctx: &StandaloneCtx,
	req_ctx: &RequestContext,
	target: &str,
) -> Result<Option<RoutingOutput>> {
	if target != "runner" {
		return Ok(None);
	}

	tracing::debug!(hostname=%req_ctx.hostname(), path=%req_ctx.path(), "routing to runner via header");

	route_runner_internal(ctx, req_ctx).await.map(Some)
}

/// Route requests to the runner service using path-based routing
/// Matches path: /runners/connect
#[tracing::instrument(skip_all)]
pub async fn route_request_path_based(
	ctx: &StandaloneCtx,
	req_ctx: &RequestContext,
) -> Result<Option<RoutingOutput>> {
	// Check if path matches /runners/connect
	let path_without_query = req_ctx.path().split('?').next().unwrap_or(req_ctx.path());
	if path_without_query != "/runners/connect" {
		return Ok(None);
	}

	tracing::debug!(hostname=%req_ctx.hostname(), path=%req_ctx.path(), "routing to runner via path");

	route_runner_internal(ctx, req_ctx).await.map(Some)
}

/// Internal runner routing logic shared by both header-based and path-based routing
#[tracing::instrument(skip_all)]
async fn route_runner_internal(
	ctx: &StandaloneCtx,
	req_ctx: &RequestContext,
) -> Result<RoutingOutput> {
	// Validate that the host is valid for the current datacenter
	let current_dc = ctx.config().topology().current_dc()?;
	if !current_dc.is_valid_regional_host(req_ctx.hostname()) {
		tracing::warn!(hostname=%req_ctx.hostname(), datacenter=?current_dc.name, "invalid host for current datacenter");

		// Determine valid hosts for error message
		let valid_hosts = if let Some(hosts) = &current_dc.valid_hosts {
			hosts.join(", ")
		} else {
			current_dc
				.public_url
				.host_str()
				.map(|h| h.to_string())
				.unwrap_or_else(|| "unknown".to_string())
		};

		return Err(crate::errors::MustUseRegionalHost {
			host: req_ctx.hostname().to_string(),
			datacenter: current_dc.name.clone(),
			valid_hosts,
		}
		.build());
	}

	tracing::debug!(datacenter = ?current_dc.name, "validated host for datacenter");

	// Check auth (if enabled)
	if let Some(auth) = &ctx.config().auth {
		// Extract token from protocol or header
		let token = if req_ctx.is_websocket() {
			req_ctx
				.headers()
				.get(SEC_WEBSOCKET_PROTOCOL)
				.and_then(|protocols| protocols.to_str().ok())
				.and_then(|protocols| {
					protocols
						.split(',')
						.map(|p| p.trim())
						.find_map(|p| p.strip_prefix(WS_PROTOCOL_TOKEN))
				})
				.ok_or_else(|| {
					crate::errors::MissingHeader {
						header: "`rivet_token.*` protocol in sec-websocket-protocol".to_string(),
					}
					.build()
				})?
		} else {
			req_ctx
				.headers()
				.get(X_RIVET_TOKEN)
				.and_then(|x| x.to_str().ok())
				.ok_or_else(|| {
					crate::errors::MissingHeader {
						header: X_RIVET_TOKEN.to_string(),
					}
					.build()
				})?
		};

		// Validate token
		if token != auth.admin_token.read() {
			return Err(rivet_api_builder::ApiForbidden.build());
		}

		tracing::debug!("authenticated runner connection");
	}

	let tunnel = pegboard_runner::PegboardRunnerWsCustomServe::new(ctx.clone());
	Ok(RoutingOutput::CustomServe(Arc::new(tunnel)))
}
