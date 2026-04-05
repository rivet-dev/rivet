use std::sync::Arc;

use gas::prelude::*;
use hyper::header::HeaderName;
use rivet_guard_core::RoutingFn;

use crate::{errors, metrics, shared_state::SharedState};

mod api_public;
pub mod actor_path;
mod envoy;
pub(crate) mod matrix_param_deserializer;
pub mod pegboard_gateway;
mod runner;

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
	Arc::new(move |req_ctx| {
		let ctx = ctx.with_ray(req_ctx.ray_id(), req_ctx.req_id()).unwrap();
		let shared_state = shared_state.clone();
		let hostname = req_ctx.hostname().to_string();
		let path = req_ctx.path().to_string();

		Box::pin(
			async move {
				tracing::debug!(hostname=%req_ctx.hostname(), path=%req_ctx.path(), "Routing request");

				// MARK: Path-based routing
				// Route actor
				if let Some(actor_path_info) = actor_path::parse_actor_path(req_ctx.path())? {
					tracing::debug!(?actor_path_info, "routing using path-based actor routing");

					if let Some(routing_output) = pegboard_gateway::route_request_path_based(
						&ctx,
						&shared_state,
						req_ctx,
						&actor_path_info,
					)
					.await?
					{
						metrics::ROUTE_TOTAL.with_label_values(&["gateway"]).inc();

						return Ok(routing_output);
					}
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
