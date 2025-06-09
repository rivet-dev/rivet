use chirp_workflow::prelude::*;
use cluster::types::{Filter, PoolType};
use global_error::GlobalResult;
use rivet_guard_core::proxy_service::{
	RouteConfig, RouteTarget, RoutingOutput, RoutingTimeout, StructuredResponse,
};
use rivet_guard_core::status::StatusCode;
use std::borrow::Cow;
use uuid::Uuid;

/// Route requests to the API service
#[tracing::instrument(skip_all)]
pub async fn route_api_request(
	ctx: &StandaloneCtx,
	host: &str,
	path: &str,
	dc_name_id: &str,
	dc_id: Uuid,
) -> GlobalResult<Option<RoutingOutput>> {
	// Match host
	if let Some(api_host) = ctx
		.config()
		.server()?
		.rivet
		.edge_api_routing_host(dc_name_id)?
	{
		if host != api_host {
			// Not an API host
			return Ok(None);
		}
	}

	// Handle ping endpoint
	if path == "/ping" {
		return Ok(Some(RoutingOutput::Response(StructuredResponse {
			status: StatusCode::OK,
			message: Cow::Borrowed("ok"),
			docs: None,
		})));
	}

	// Get API server from the cluster
	let servers_res = ctx
		.op(cluster::ops::server::list::Input {
			filter: Filter {
				datacenter_ids: Some(vec![dc_id]),
				pool_types: Some(vec![PoolType::Worker]),
				..Default::default()
			},
			include_destroyed: false,
			exclude_draining: true,
			exclude_no_vlan: true,
		})
		.await?;
	tracing::info!(?servers_res, "servers");

	let port = ctx.config().server()?.rivet.api_public.port();
	let targets = servers_res
		.servers
		.iter()
		// Only include servers that are installed
		.filter(|server| server.install_complete_ts.is_some())
		.map(|server| {
			// For each server, create a target
			Ok(RouteTarget {
				actor_id: None,
				server_id: Some(server.server_id),
				host: unwrap!(server.lan_ip).to_string(),
				port,
				path: path.to_owned(),
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	let targets = if targets.is_empty() {
		if let Some((host, port)) = ctx.config().server()?.rivet.edge_api_fallback_addr_lan() {
			vec![RouteTarget {
				actor_id: None,
				server_id: None,
				host,
				port,
				path: path.to_owned(),
			}]
		} else {
			// No API servers to route to
			return Ok(None);
		}
	} else {
		targets
	};

	return Ok(Some(RoutingOutput::Route(RouteConfig {
		targets,
		timeout: RoutingTimeout {
			routing_timeout: 10, // 10 seconds for API routing timeout
		},
	})));
}
