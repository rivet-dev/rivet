use anyhow::*;
use gas::prelude::*;
use rivet_guard_core::{RoutingOutput, request_context::RequestContext};
use std::sync::Arc;
use subtle::ConstantTimeEq;

use super::validate_regional_host;

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

	validate_regional_host(ctx, req_ctx)?;

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
				crate::errors::MissingQueryParameter {
					parameter: "token".to_string(),
				}
				.build()
			})?;

		if token
			.as_bytes()
			.ct_ne(auth.admin_token.read().as_bytes())
			.into()
		{
			return Err(rivet_api_builder::ApiForbidden.build());
		}

		tracing::debug!("authenticated kv channel connection");
	}

	Ok(Some(RoutingOutput::CustomServe(handler.clone())))
}
