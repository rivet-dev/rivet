use anyhow::*;
use gas::prelude::*;
use rivet_guard_core::proxy_service::RoutingOutput;
use std::sync::Arc;

use super::X_RIVET_TOKEN;

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

	// Check auth (if enabled)
	if let Some(auth) = &ctx.config().auth {
		let token = headers
			.get(X_RIVET_TOKEN)
			.and_then(|x| x.to_str().ok())
			.ok_or_else(|| {
				crate::errors::MissingHeader {
					header: X_RIVET_TOKEN.to_string(),
				}
				.build()
			})?;

		if token != auth.admin_token {
			return Err(rivet_api_builder::ApiForbidden.build());
		}
	}

	let tunnel = pegboard_runner::PegboardRunnerWsCustomServe::new(ctx.clone());
	Ok(Some(RoutingOutput::CustomServe(Arc::new(tunnel))))
}
