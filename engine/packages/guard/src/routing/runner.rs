use anyhow::*;
use gas::prelude::*;
use rivet_guard_core::proxy_service::RoutingOutput;
use std::sync::Arc;

use super::{SEC_WEBSOCKET_PROTOCOL, X_RIVET_TOKEN};
pub(crate) const WS_PROTOCOL_TOKEN: &str = "rivet_token.";

/// Route requests to the runner service using header-based routing
#[tracing::instrument(skip_all)]
pub async fn route_request(
	ctx: &StandaloneCtx,
	target: &str,
	host: &str,
	path: &str,
	headers: &hyper::HeaderMap,
) -> Result<Option<RoutingOutput>> {
	if target != "runner" {
		return Ok(None);
	}

	tracing::debug!(?host, path, "routing to runner via header");

	route_runner_internal(ctx, host, headers).await.map(Some)
}

/// Route requests to the runner service using path-based routing
/// Matches path: /runners/connect
#[tracing::instrument(skip_all)]
pub async fn route_request_path_based(
	ctx: &StandaloneCtx,
	host: &str,
	path: &str,
	headers: &hyper::HeaderMap,
) -> Result<Option<RoutingOutput>> {
	// Check if path matches /runners/connect
	let path_without_query = path.split('?').next().unwrap_or(path);
	if path_without_query != "/runners/connect" {
		return Ok(None);
	}

	tracing::debug!(?host, path, "routing to runner via path");

	route_runner_internal(ctx, host, headers).await.map(Some)
}

/// Internal runner routing logic shared by both header-based and path-based routing
#[tracing::instrument(skip_all)]
async fn route_runner_internal(
	ctx: &StandaloneCtx,
	host: &str,
	headers: &hyper::HeaderMap,
) -> Result<RoutingOutput> {
	// Validate that the host is valid for the current datacenter
	let current_dc = ctx.config().topology().current_dc()?;
	if !current_dc.is_valid_regional_host(host) {
		tracing::warn!(?host, datacenter = ?current_dc.name, "invalid host for current datacenter");

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
			host: host.to_string(),
			datacenter: current_dc.name.clone(),
			valid_hosts,
		}
		.build());
	}

	tracing::debug!(datacenter = ?current_dc.name, "validated host for datacenter");

	let is_websocket = headers
		.get("upgrade")
		.and_then(|v| v.to_str().ok())
		.map(|v| v.eq_ignore_ascii_case("websocket"))
		.unwrap_or(false);

	tracing::debug!(is_websocket, "connection type");

	// Check auth (if enabled)
	if let Some(auth) = &ctx.config().auth {
		// Extract token from protocol or header
		let token = if is_websocket {
			headers
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
			headers
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
