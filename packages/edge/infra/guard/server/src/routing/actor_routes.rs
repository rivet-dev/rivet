use chirp_workflow::prelude::*;
use fdb_util::FormalKey; // Added for deserialize method
use global_error::GlobalResult;
use rivet_guard_core::proxy_service::{
	RouteConfig, RoutingOutput, RoutingTimeout, StructuredResponse,
};
use rivet_guard_core::status::StatusCode;
use rivet_guard_core::RouteTarget;
use std::borrow::Cow;
use std::collections::HashMap;
use uuid::Uuid;

/// Route requests to actors based on the route configuration
pub async fn route_via_route_config(
	ctx: &StandaloneCtx,
	host: &str,
	path: &str,
	dc_id: Uuid,
) -> GlobalResult<Option<RoutingOutput>> {
	// Get route directly using hostname and path
	// The operation handles priority internally and returns the best match
	let routes_res = ctx
		.op(route::ops::get_by_hostname_path::Input {
			hostname: host.to_string(),
			path: path.to_string(),
		})
		.await?;

	// If we didn't find any route, check if it's a rivet.run domain and return a custom message
	let Some(route) = routes_res.route else {
		// Check if the hostname is a rivet.run domain
		if routes_res.is_route_hostname {
			return Ok(Some(RoutingOutput::Response(StructuredResponse {
				status: StatusCode::NOT_FOUND,
				message: Cow::Borrowed("No matching routes found for this rivet.run domain"),
				docs: None,
			})));
		} else {
			return Ok(None);
		}
	};

	let namespace_id = route.namespace_id;

	tracing::debug!(
		host = host,
		path = path,
		route_id = %route.route_id,
		"Found matching route"
	);

	// Convert the selector tags from the route
	let selector_tags: HashMap<String, String> = match &route.target {
		route::types::RouteTarget::Actors { selector_tags } => selector_tags.clone(),
	};

	// Query actors with matching tags in this environment
	let actors_res = ctx
		.op(pegboard::ops::actor::list_for_env::Input {
			env_id: namespace_id,
			tags: selector_tags,
			include_destroyed: false,
			created_before: None,
			limit: 50, // Reasonable limit for load balancing
		})
		.await?;

	if actors_res.actors.is_empty() {
		tracing::warn!(
			host = host,
			path = path,
			route_id = %route.route_id,
			"Found matching route but no actors with matching tags"
		);
		return Ok(Some(RoutingOutput::Response(StructuredResponse {
			status: StatusCode::SERVICE_UNAVAILABLE,
			message: Cow::Borrowed("Found matching route but no actors with matching tags"),
			docs: None,
		})));
	}

	// Process the path once before the loop
	// First, extract the path and query parts
	let (path_part, query_part) = match path.split_once('?') {
		Some((p, q)) => (p, Some(q)),
		None => (path, None),
	};

	// Determine path to forward based only on strip_prefix
	let forwarded_path = if route.strip_prefix {
		// Strip the prefix if needed
		if path_part.len() > route.path.len() {
			&path_part[route.path.len()..]
		} else {
			// If nothing left after stripping, use root path
			"/"
		}
	} else {
		// Keep the full path
		path_part
	};

	// Reattach the query string if it exists
	let path_to_forward = match query_part {
		Some(q) => format!("{}?{}", forwarded_path, q),
		None => forwarded_path.to_string(),
	};

	tracing::debug!(
		original_path = %path,
		route_path = %route.path,
		strip_prefix = %route.strip_prefix,
		forwarded_path = %path_to_forward,
		"Path transformation for forwarding"
	);

	// Fetch each actor's details to get their connection information
	let mut targets = Vec::new();

	for actor_entry in &actors_res.actors {
		// Find actor's proxied ports
		if let Some(actor_targets) =
			find_actor_targets(ctx, &actor_entry.actor_id, dc_id, &path_to_forward).await?
		{
			targets.extend(actor_targets);
		}
	}

	if targets.is_empty() {
		tracing::warn!(
			host = host,
			path = path,
			route_id = %route.route_id,
			"Found matching actors but no valid targets"
		);
		return Ok(Some(RoutingOutput::Response(StructuredResponse {
			status: StatusCode::BAD_GATEWAY,
			message: Cow::Borrowed("Found matching route but no valid actor targets"),
			docs: None,
		})));
	}

	// Return routing result with all targets for load balancing
	Ok(Some(RoutingOutput::Route(RouteConfig {
		targets,
		timeout: RoutingTimeout {
			routing_timeout: 10, // 10 seconds timeout
		},
	})))
}

/// Find all potential targets for an actor
async fn find_actor_targets(
	ctx: &StandaloneCtx,
	actor_id: &Uuid,
	_dc_id: Uuid, // Unused but kept for API compatibility
	path_to_forward: &str,
) -> GlobalResult<Option<Vec<RouteTarget>>> {
	// Fetch the actor's ports
	let proxied_ports = ctx
		.fdb()
		.await?
		.run(|tx, _mc| async move {
			// NOTE: This is not SERIALIZABLE because we don't want to conflict with port updates
			// and its not important if its slightly stale
			let proxied_ports_key = pegboard::keys::actor::ProxiedPortsKey::new(*actor_id);
			let raw = tx
				.get(
					&pegboard::keys::subspace().pack(&proxied_ports_key),
					fdb_util::SNAPSHOT,
				)
				.await?;
			if let Some(raw) = raw {
				let proxied_ports = proxied_ports_key
					.deserialize(&raw)
					.map_err(|x| foundationdb::FdbBindingError::CustomError(x.into()))?;
				Ok(Some(proxied_ports))
			} else {
				Ok(None)
			}
		})
		.await?;

	let Some(proxied_ports) = proxied_ports else {
		// Actor exists but has no proxied ports
		return Ok(None);
	};

	// Create targets for each proxied port
	let mut targets = Vec::new();
	for pp in proxied_ports {
		targets.push(RouteTarget {
			actor_id: Some(*actor_id),
			server_id: None,
			host: pp.lan_hostname.parse()?,
			port: pp.source,
			path: path_to_forward.to_string(),
		});
	}

	Ok(Some(targets))
}
