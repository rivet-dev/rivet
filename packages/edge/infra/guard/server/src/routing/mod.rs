use chirp_workflow::prelude::*;
use global_error::GlobalResult;
use rivet_guard_core::proxy_service::{RoutingOutput, StructuredResponse};
use rivet_guard_core::status::StatusCode;
use std::{borrow::Cow, sync::Arc};

pub mod actor;
pub mod actor_routes;
pub mod api;

/// Creates the main routing function that handles all incoming requests
pub fn create_routing_function(
	ctx: StandaloneCtx,
) -> Arc<
	dyn for<'a> Fn(
			&'a str,
			&'a str,
			rivet_guard_core::proxy_service::PortType,
		) -> futures::future::BoxFuture<'a, GlobalResult<RoutingOutput>>
		+ Send
		+ Sync,
> {
	Arc::new(
		move |hostname: &str, path: &str, port_type: rivet_guard_core::proxy_service::PortType| {
			let ctx = ctx.clone();

			Box::pin(
				async move {
					// Extract just the host, stripping the port if present
					let host = hostname.split(':').next().unwrap_or(hostname);

					tracing::info!("Routing request for hostname: {host}, path: {path}");

					// Get DC information
					let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;
					let dc_res = ctx
						.op(cluster::ops::datacenter::get::Input {
							datacenter_ids: vec![dc_id],
						})
						.await?;

					let dc = unwrap!(dc_res.datacenters.first());

					// Try to route using configured routes first
					tracing::info!("Attempting route-based routing for {host} {path}");
					match actor_routes::route_via_route_config(&ctx, host, path, dc_id).await {
						Ok(Some(RoutingOutput::Route(routing_result))) => {
							tracing::info!(
								"Successfully routed via route config for {host} {path}"
							);
							return Ok(RoutingOutput::Route(routing_result));
						}
						Ok(Some(RoutingOutput::Response(response))) => {
							return Ok(RoutingOutput::Response(response));
						}
						Ok(None) => {
							// Continue to next routing method
						}
						Err(err) => {
							tracing::error!("Error in route_via_route_config: {err}");

							return Ok(RoutingOutput::Response(StructuredResponse {
								status: StatusCode::INTERNAL_SERVER_ERROR,
								message: Cow::Borrowed("Failed while attempting to route request."),
								docs: None,
							}));
						}
					}

					// Try to route to actor directly (legacy method)
					match actor::route_actor_request(&ctx, host, path, dc_id).await {
						Ok(Some(RoutingOutput::Route(routing_result))) => {
							return Ok(RoutingOutput::Route(routing_result));
						}
						Ok(Some(RoutingOutput::Response(response))) => {
							return Ok(RoutingOutput::Response(response));
						}
						Ok(None) => {
							// Continue to next routing method
						}
						Err(err) => {
							tracing::error!("Error in actor_routes::route_actor_request: {err}");

							return Ok(RoutingOutput::Response(StructuredResponse {
								status: StatusCode::INTERNAL_SERVER_ERROR,
								message: Cow::Borrowed("Failed while attempting to route request."),
								docs: None,
							}));
						}
					}

					// Try to route to API
					//
					// IMPORTANT: Route this last, since in dev this will match all hostnames
					match api::route_api_request(&ctx, host, path, &dc.name_id, dc_id).await {
						Ok(Some(RoutingOutput::Route(routing_result))) => {
							return Ok(RoutingOutput::Route(routing_result));
						}
						Ok(Some(RoutingOutput::Response(response))) => {
							return Ok(RoutingOutput::Response(response));
						}
						Ok(None) => {
							// Continue
						}
						Err(err) => {
							tracing::error!("Error in api::route_api_request: {err}");

							return Ok(RoutingOutput::Response(StructuredResponse {
								status: StatusCode::INTERNAL_SERVER_ERROR,
								message: Cow::Borrowed("Failed while attempting to route request."),
								docs: None,
							}));
						}
					}

					// No matching route found
					tracing::warn!("No route found for: {host} {path}");
					Ok(RoutingOutput::Response(StructuredResponse {
						status: StatusCode::NOT_FOUND,
						message: Cow::Owned(format!(
							"No route found for hostname: {host}, path: {path}"
						)),
						docs: None,
					}))
				}
				.instrument(tracing::info_span!("routing_fn", %hostname, %path, ?port_type)),
			)
		},
	)
}
