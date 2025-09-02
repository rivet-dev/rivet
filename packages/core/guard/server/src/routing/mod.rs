use std::sync::Arc;

use anyhow::*;
use gas::prelude::*;
use hyper::header::HeaderName;
use rivet_guard_core::RoutingFn;

use crate::errors;

//pub(crate) mod actor;
mod api_peer;
mod api_public;
pub mod pegboard_gateway;
mod pegboard_tunnel;
mod runner_ws;

pub(crate) const X_RIVET_TARGET: HeaderName = HeaderName::from_static("x-rivet-target");

/// Creates the main routing function that handles all incoming requests
#[tracing::instrument(skip_all)]
pub fn create_routing_function(ctx: StandaloneCtx) -> RoutingFn {
	Arc::new(
		move |hostname: &str,
		      path: &str,
		      port_type: rivet_guard_core::proxy_service::PortType,
		      headers: &hyper::HeaderMap| {
			let ctx = ctx.clone();

			Box::pin(
				async move {
					// Extract just the host, stripping the port if present
					let host = hostname.split(':').next().unwrap_or(hostname);

					tracing::debug!("Routing request for hostname: {host}, path: {path}");

					// Read target
					if let Some(target) = headers.get(X_RIVET_TARGET).and_then(|x| x.to_str().ok())
					{
						// if let Some(routing_output) =
						// 	actor::route_request(&ctx, target, host, path, headers).await?
						// {
						// 	return Ok(routing_output);
						// }

						if let Some(routing_output) =
							runner_ws::route_request(&ctx, target, host, path).await?
						{
							return Ok(routing_output);
						}

						if let Some(routing_output) =
							pegboard_gateway::route_request(&ctx, target, host, path, headers)
								.await?
						{
							return Ok(routing_output);
						}

						if let Some(routing_output) =
							pegboard_tunnel::route_request(&ctx, target, host, path).await?
						{
							return Ok(routing_output);
						}

						if let Some(routing_output) =
							api_public::route_request(&ctx, target, host, path).await?
						{
							return Ok(routing_output);
						}

						if let Some(routing_output) =
							api_peer::route_request(&ctx, target, host, path).await?
						{
							return Ok(routing_output);
						}
					} else {
						// No x-rivet-target header, try routing to api-public by default
						if let Some(routing_output) =
							api_public::route_request(&ctx, "api-public", host, path).await?
						{
							return Ok(routing_output);
						}
					}

					// No matching route found
					tracing::debug!("No route found for: {host} {path}");
					Err(errors::NoRoute {
						host: host.to_string(),
						path: path.to_string(),
					}
					.build())
				}
				.instrument(tracing::info_span!("routing_fn", %hostname, %path, ?port_type)),
			)
		},
	)
}
