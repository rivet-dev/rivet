use anyhow::*;
use gas::prelude::*;
use rivet_guard_core::{RoutingOutput, request_context::RequestContext};
use std::sync::Arc;

/// Route requests to the KV channel service using path-based routing.
/// Matches path: /kv/connect
#[tracing::instrument(skip_all)]
pub async fn route_request_path_based(
	ctx: &StandaloneCtx,
	req_ctx: &RequestContext,
	handler: &Arc<pegboard_kv_channel::PegboardKvChannelCustomServe>,
) -> Result<Option<RoutingOutput>> {
	let path_without_query = req_ctx.path().split('?').next().unwrap_or(req_ctx.path());
	if path_without_query != "/kv/connect" && path_without_query != "/kv/connect/" {
		return Ok(None);
	}

	tracing::debug!(
		hostname = %req_ctx.hostname(),
		path = %req_ctx.path(),
		"routing to kv channel via path"
	);

	// Validate that the host is valid for the current datacenter.
	let current_dc = ctx.config().topology().current_dc()?;
	if !current_dc.is_valid_regional_host(req_ctx.hostname()) {
		tracing::warn!(
			hostname = %req_ctx.hostname(),
			datacenter = ?current_dc.name,
			"invalid host for current datacenter"
		);

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

	// Check auth (if enabled).
	if let Some(auth) = &ctx.config().auth {
		// Extract token from query params.
		let url = url::Url::parse(&format!("ws://placeholder{}", req_ctx.path()))
			.context("failed to parse URL for auth")?;
		let token = url
			.query_pairs()
			.find(|(k, _)| k == "token")
			.map(|(_, v)| v.to_string())
			.ok_or_else(|| {
				crate::errors::MissingHeader {
					header: "token query parameter".to_string(),
				}
				.build()
			})?;

		if token != *auth.admin_token.read() {
			return Err(rivet_api_builder::ApiForbidden.build());
		}

		tracing::debug!("authenticated kv channel connection");
	}

	Ok(Some(RoutingOutput::CustomServe(handler.clone())))
}
