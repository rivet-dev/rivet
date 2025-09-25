use anyhow::*;
use gas::prelude::*;
use rivet_guard_core::proxy_service::RoutingOutput;
use std::sync::Arc;

use super::{SEC_WEBSOCKET_PROTOCOL, X_RIVET_TOKEN};
pub(crate) const WS_PROTOCOL_TOKEN: &str = "rivet_token.";

/// Route requests to the API service
#[tracing::instrument(skip_all)]
pub async fn route_request(
	ctx: &StandaloneCtx,
	target: &str,
	_host: &str,
	_path: &str,
	headers: &hyper::HeaderMap,
) -> Result<Option<RoutingOutput>> {
	if target != "runner" {
		return Ok(None);
	}

	let is_websocket = headers
		.get("upgrade")
		.and_then(|v| v.to_str().ok())
		.map(|v| v.eq_ignore_ascii_case("websocket"))
		.unwrap_or(false);

	// Check auth (if enabled)
	if let Some(auth) = &ctx.config().auth {
		// Extract token
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
	}

	let tunnel = pegboard_runner::PegboardRunnerWsCustomServe::new(ctx.clone());
	Ok(Some(RoutingOutput::CustomServe(Arc::new(tunnel))))
}
