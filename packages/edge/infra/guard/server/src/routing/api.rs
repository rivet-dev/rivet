use std::borrow::Cow;

use chirp_workflow::prelude::*;
use global_error::GlobalResult;
use rivet_guard_core::proxy_service::{
	RouteConfig, RouteTarget, RoutingOutput, RoutingTimeout, StructuredResponse,
};
use rivet_guard_core::status::StatusCode;
use service_discovery::ServiceDiscovery;
use url::Url;
use uuid::Uuid;

// TODO: Copied from cluster/src/workflows/server/install/install_scripts/components/rivet/mod.rs
const TUNNEL_API_EDGE_PORT: u16 = 5010;

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

	// NOTE: We use service discovery instead of server::list or datacenter::server_discovery because cache is not
	// shared between edge-edge or edge-core. SD requests the core which has a single cache.
	let url = Url::parse(&format!("http://127.0.0.1:{TUNNEL_API_EDGE_PORT}/provision/datacenters/{dc_id}/servers?pools=worker"))?;
	let sd = ServiceDiscovery::new(url);
	let servers = sd.fetch().await?;

	tracing::debug!(?servers, "api servers");

	let port = ctx.config().server()?.rivet.api_public.port();
	let targets = servers
		.iter()
		.map(|server| {
			// For each server, create a target
			Ok(RouteTarget {
				actor_id: None,
				server_id: Some(server.server_id),
				host: unwrap_ref!(server.lan_ip).to_string(),
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
