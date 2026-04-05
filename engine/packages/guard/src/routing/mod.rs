use std::sync::Arc;

use anyhow::Result;
use gas::prelude::*;
use hyper::header::HeaderName;
use rivet_guard_core::{RoutingFn, request_context::RequestContext};

use crate::{errors, metrics, shared_state::SharedState};

pub mod actor_path;
mod api_public;
mod envoy;
mod kv_channel;
pub mod pegboard_gateway;
mod runner;
mod ws_health;

pub(crate) const X_RIVET_TARGET: HeaderName = HeaderName::from_static("x-rivet-target");
pub(crate) const X_RIVET_TOKEN: HeaderName = HeaderName::from_static("x-rivet-token");
pub(crate) const SEC_WEBSOCKET_PROTOCOL: HeaderName =
	HeaderName::from_static("sec-websocket-protocol");
pub(crate) const WS_PROTOCOL_TARGET: &str = "rivet_target.";
pub(crate) const WS_PROTOCOL_ACTOR: &str = "rivet_actor.";
pub(crate) const WS_PROTOCOL_TOKEN: &str = "rivet_token.";

/// Creates the main routing function that handles all incoming requests
#[tracing::instrument(skip_all)]
pub fn create_routing_function(ctx: &StandaloneCtx, shared_state: SharedState) -> RoutingFn {
	let ctx = ctx.clone();
	let kv_channel_handler = Arc::new(pegboard_kv_channel::PegboardKvChannelCustomServe::new(
		ctx.clone(),
	));
	Arc::new(move |req_ctx| {
		let ctx = ctx.with_ray(req_ctx.ray_id(), req_ctx.req_id()).unwrap();
		let shared_state = shared_state.clone();
		let kv_channel_handler = kv_channel_handler.clone();
		let hostname = req_ctx.hostname().to_string();
		let path = req_ctx.path().to_string();

		Box::pin(
			async move {
				tracing::debug!(hostname=%req_ctx.hostname(), path=%req_ctx.path(), "Routing request");

				if ws_health::matches_path(req_ctx.path()) {
					if ctx.config().guard().enable_websocket_health_route() {
						metrics::ROUTE_TOTAL.with_label_values(&["ws_health"]).inc();
						return Ok(ws_health::route_request());
					}

					metrics::ROUTE_TOTAL.with_label_values(&["none"]).inc();

					return Err(errors::NoRoute {
						host: req_ctx.hostname().to_string(),
						path: req_ctx.path().to_string(),
					}
					.build());
				}

				// MARK: Path-based routing
				// Route actor
				if let Some(routing_output) =
					pegboard_gateway::route_request_path_based(&ctx, &shared_state, req_ctx).await?
				{
					metrics::ROUTE_TOTAL.with_label_values(&["gateway"]).inc();

					return Ok(routing_output);
				}

				// Route runner
				if let Some(routing_output) =
					runner::route_request_path_based(&ctx, req_ctx).await?
				{
					metrics::ROUTE_TOTAL.with_label_values(&["runner"]).inc();

					return Ok(routing_output);
				}

				// Route envoy
				if let Some(routing_output) = envoy::route_request_path_based(&ctx, req_ctx).await?
				{
					metrics::ROUTE_TOTAL.with_label_values(&["envoy"]).inc();

					return Ok(routing_output);
				}

				// Route KV channel
				if let Some(routing_output) =
					kv_channel::route_request_path_based(&ctx, req_ctx, &kv_channel_handler).await?
				{
					metrics::ROUTE_TOTAL
						.with_label_values(&["kv_channel"])
						.inc();

					return Ok(routing_output);
				}

				// MARK: Header- & protocol-based routing (X-Rivet-Target)
				// Determine target
				let target = if req_ctx.is_websocket() {
					// For WebSocket, parse the sec-websocket-protocol header
					req_ctx
						.headers()
						.get(SEC_WEBSOCKET_PROTOCOL)
						.and_then(|protocols| protocols.to_str().ok())
						.and_then(|protocols| {
							// Parse protocols to find target.{value}
							protocols
								.split(',')
								.map(|p| p.trim())
								.find_map(|p| p.strip_prefix(WS_PROTOCOL_TARGET))
						})
				} else {
					// For HTTP, use the x-rivet-target header
					req_ctx
						.headers()
						.get(X_RIVET_TARGET)
						.and_then(|x| x.to_str().ok())
				};

				// Read target
				if let Some(target) = target {
					if let Some(routing_output) =
						pegboard_gateway::route_request(&ctx, &shared_state, req_ctx, target)
							.await?
					{
						metrics::ROUTE_TOTAL.with_label_values(&["gateway"]).inc();

						return Ok(routing_output);
					}

					if let Some(routing_output) =
						runner::route_request(&ctx, req_ctx, target).await?
					{
						metrics::ROUTE_TOTAL.with_label_values(&["runner"]).inc();

						return Ok(routing_output);
					}

					if let Some(routing_output) =
						envoy::route_request(&ctx, req_ctx, target).await?
					{
						metrics::ROUTE_TOTAL.with_label_values(&["envoy"]).inc();

						return Ok(routing_output);
					}

					if let Some(routing_output) = api_public::route_request(&ctx, target).await? {
						metrics::ROUTE_TOTAL.with_label_values(&["api"]).inc();

						return Ok(routing_output);
					}
				} else {
					// No x-rivet-target header, try routing to api-public by default
					if let Some(routing_output) =
						api_public::route_request(&ctx, "api-public").await?
					{
						metrics::ROUTE_TOTAL.with_label_values(&["api"]).inc();

						return Ok(routing_output);
					}
				}

				metrics::ROUTE_TOTAL.with_label_values(&["none"]).inc();

				tracing::debug!(hostname=%req_ctx.hostname(), path=%req_ctx.path(), "No route found");
				Err(errors::NoRoute {
					host: req_ctx.hostname().to_string(),
					path: req_ctx.path().to_string(),
				}
				.build())
			}
			.instrument(tracing::info_span!("routing_fn", %hostname, %path)),
		)
	})
}

/// Validates that the request hostname is valid for the current datacenter.
/// Returns an error if the host does not match a valid regional host.
pub(crate) fn validate_regional_host(ctx: &StandaloneCtx, req_ctx: &RequestContext) -> Result<()> {
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

		return Err(errors::MustUseRegionalHost {
			host: req_ctx.hostname().to_string(),
			datacenter: current_dc.name.clone(),
			valid_hosts,
		}
		.build());
	}

	Ok(())
}
