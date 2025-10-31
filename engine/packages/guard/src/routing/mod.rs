use std::sync::Arc;

use anyhow::*;
use gas::prelude::*;
use hyper::header::HeaderName;
use rivet_guard_core::RoutingFn;

use crate::{errors, shared_state::SharedState};

mod api_public;
pub mod pegboard_gateway;
mod runner;

pub(crate) const X_RIVET_TARGET: HeaderName = HeaderName::from_static("x-rivet-target");
pub(crate) const X_RIVET_TOKEN: HeaderName = HeaderName::from_static("x-rivet-token");
pub(crate) const SEC_WEBSOCKET_PROTOCOL: HeaderName =
	HeaderName::from_static("sec-websocket-protocol");
pub(crate) const WS_PROTOCOL_TARGET: &str = "rivet_target.";
pub(crate) const WS_PROTOCOL_ACTOR: &str = "rivet_actor.";
pub(crate) const WS_PROTOCOL_TOKEN: &str = "rivet_token.";

#[derive(Debug, Clone)]
pub struct ActorPathInfo {
	pub actor_id: String,
	pub token: Option<String>,
	pub remaining_path: String,
}

/// Creates the main routing function that handles all incoming requests
#[tracing::instrument(skip_all)]
pub fn create_routing_function(ctx: StandaloneCtx, shared_state: SharedState) -> RoutingFn {
	Arc::new(
		move |hostname: &str,
		      path: &str,
		      port_type: rivet_guard_core::proxy_service::PortType,
		      headers: &hyper::HeaderMap| {
			let ctx = ctx.clone();
			let shared_state = shared_state.clone();

			Box::pin(
				async move {
					// Extract just the host, stripping the port if present
					let host = hostname.split(':').next().unwrap_or(hostname);

					tracing::debug!("Routing request for hostname: {host}, path: {path}");

					// Check if this is a WebSocket upgrade request
					let is_websocket = headers
						.get("upgrade")
						.and_then(|v| v.to_str().ok())
						.map(|v| v.eq_ignore_ascii_case("websocket"))
						.unwrap_or(false);

					// First, check if this is an actor path-based route
					if let Some(actor_path_info) = parse_actor_path(path) {
						tracing::debug!(?actor_path_info, "routing using path-based actor routing");

						// Route to pegboard gateway with the extracted information
						if let Some(routing_output) = pegboard_gateway::route_request_path_based(
							&ctx,
							&shared_state,
							&actor_path_info.actor_id,
							actor_path_info.token.as_deref(),
							&actor_path_info.remaining_path,
							headers,
							is_websocket,
						)
						.await?
						{
							return Ok(routing_output);
						}
					}

					// Fallback to header-based routing
					// Extract target from WebSocket protocol or HTTP header
					let target = if is_websocket {
						// For WebSocket, parse the sec-websocket-protocol header
						headers
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
						headers.get(X_RIVET_TARGET).and_then(|x| x.to_str().ok())
					};

					// Read target
					if let Some(target) = target {
						if let Some(routing_output) =
							runner::route_request(&ctx, target, host, path, headers).await?
						{
							return Ok(routing_output);
						}

						if let Some(routing_output) = pegboard_gateway::route_request(
							&ctx,
							&shared_state,
							target,
							host,
							path,
							headers,
							is_websocket,
						)
						.await?
						{
							return Ok(routing_output);
						}

						if let Some(routing_output) =
							api_public::route_request(&ctx, target, host, path).await?
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

/// Parse actor routing information from path
/// Matches patterns:
/// - /gateway/actors/{actor_id}/tokens/{token}/route/{...path}
/// - /gateway/actors/{actor_id}/route/{...path}
pub fn parse_actor_path(path: &str) -> Option<ActorPathInfo> {
	// Find query string position (everything from ? onwards, but before fragment)
	let query_pos = path.find('?');
	let fragment_pos = path.find('#');

	// Extract query string (excluding fragment)
	let query_string = match (query_pos, fragment_pos) {
		(Some(q), Some(f)) if q < f => &path[q..f],
		(Some(q), None) => &path[q..],
		_ => "",
	};

	// Extract base path (before query and fragment)
	let base_path = match query_pos {
		Some(pos) => &path[..pos],
		None => match fragment_pos {
			Some(pos) => &path[..pos],
			None => path,
		},
	};

	// Check for double slashes (invalid path)
	if base_path.contains("//") {
		return None;
	}

	// Split the path into segments
	let segments: Vec<&str> = base_path.split('/').filter(|s| !s.is_empty()).collect();

	// Check minimum required segments: gateway, actors, {actor_id}, route
	if segments.len() < 4 {
		return None;
	}

	// Verify the fixed segments
	if segments[0] != "gateway" || segments[1] != "actors" {
		return None;
	}

	// Check for empty actor_id
	if segments[2].is_empty() {
		return None;
	}

	let actor_id = segments[2].to_string();

	// Check for token or direct route
	let (token, remaining_path_start_idx) =
		if segments.len() >= 6 && segments[3] == "tokens" && segments[5] == "route" {
			// Pattern with token: /gateway/actors/{actor_id}/tokens/{token}/route/{...path}
			// Check for empty token
			if segments[4].is_empty() {
				return None;
			}
			(Some(segments[4].to_string()), 6)
		} else if segments.len() >= 4 && segments[3] == "route" {
			// Pattern without token: /gateway/actors/{actor_id}/route/{...path}
			(None, 4)
		} else {
			return None;
		};

	// Calculate the position in the original path where remaining path starts
	let mut prefix_len = 0;
	for (i, segment) in segments.iter().enumerate() {
		if i >= remaining_path_start_idx {
			break;
		}
		prefix_len += 1 + segment.len(); // +1 for the slash
	}

	// Extract the remaining path preserving trailing slashes
	let remaining_base = if prefix_len < base_path.len() {
		&base_path[prefix_len..]
	} else {
		"/"
	};

	// Ensure remaining path starts with /
	let remaining_path = if remaining_base.is_empty() || !remaining_base.starts_with('/') {
		format!("/{}{}", remaining_base, query_string)
	} else {
		format!("{}{}", remaining_base, query_string)
	};

	Some(ActorPathInfo {
		actor_id,
		token,
		remaining_path,
	})
}
