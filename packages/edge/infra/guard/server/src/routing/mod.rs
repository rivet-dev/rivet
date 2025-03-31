use chirp_workflow::prelude::*;
use global_error::GlobalResult;
use rivet_guard_core::proxy_service::RoutingResponse;
use std::sync::Arc;

pub mod actor;
pub mod api;

/// Creates the main routing function that handles all incoming requests
pub fn create_routing_function(
	ctx: StandaloneCtx,
) -> Arc<
	dyn for<'a> Fn(
			&'a str,
			&'a str,
			rivet_guard_core::proxy_service::PortType,
		) -> futures::future::BoxFuture<'a, GlobalResult<RoutingResponse>>
		+ Send
		+ Sync,
> {
	Arc::new(
		move |hostname: &str, path: &str, port_type: rivet_guard_core::proxy_service::PortType| {
			let ctx = ctx.clone();

			Box::pin(async move {
				// Extract just the host, stripping the port if present
				let host = hostname.split(':').next().unwrap_or(hostname);

				// Get DC information
				let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;
				let dc_res = ctx
					.op(cluster::ops::datacenter::get::Input {
						datacenter_ids: vec![dc_id],
					})
					.await?;
				let dc = unwrap!(dc_res.datacenters.first());

				// Try to route to actor
				if let Ok(Some(routing_result)) =
					actor::route_actor_request(&ctx, host, path, dc_id).await
				{
					return Ok(RoutingResponse::Ok(routing_result));
				}

				// Try to route to API
				//
				// IMPORTANT: Route this last, since in dev this will match all hostnames
				if let Ok(Some(routing_result)) =
					api::route_api_request(&ctx, host, path, &dc.name_id, dc_id).await
				{
					return Ok(RoutingResponse::Ok(routing_result));
				}

				// No matching route found
				tracing::warn!("No route found for: {host} {path}");
				Ok(RoutingResponse::NotFound)
			})
		},
	)
}
