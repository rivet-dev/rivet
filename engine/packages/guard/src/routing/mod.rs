use std::sync::Arc;

use gas::prelude::*;
use hyper::header::HeaderName;
use rivet_guard_core::RoutingFn;

use crate::{errors, metrics, shared_state::SharedState};

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
	pub stripped_path: String,
}

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
				if let Some(actor_path_info) = parse_actor_path(req_ctx.path()) {
					tracing::debug!(?actor_path_info, "routing using path-based actor routing");

					// Route to pegboard gateway with the extracted information
					if let Some(routing_output) = pegboard_gateway::route_request_path_based(
						&ctx,
						&shared_state,
						req_ctx,
						&actor_path_info.actor_id,
						actor_path_info.token.as_deref(),
						&actor_path_info.stripped_path,
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
						runner::route_request(&ctx, req_ctx, target).await?
					{
						metrics::ROUTE_TOTAL.with_label_values(&["runner"]).inc();

						return Ok(routing_output);
					}

					if let Some(routing_output) =
						pegboard_gateway::route_request(&ctx, &shared_state, req_ctx, target)
							.await?
					{
						metrics::ROUTE_TOTAL.with_label_values(&["gateway"]).inc();

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

/// Parse actor routing information from path
/// Matches patterns:
/// - /gateway/{actor_id}/{...path}
/// - /gateway/{actor_id}@{token}/{...path}
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

	// Check minimum required segments: gateway, {actor_id}
	if segments.len() < 2 {
		return None;
	}

	// Verify the fixed segment
	if segments[0] != "gateway" {
		return None;
	}

	// Check for empty actor_id segment
	if segments[1].is_empty() {
		return None;
	}

	// Parse actor_id and optional token from second segment
	// Pattern: {actor_id}@{token} or just {actor_id}
	let actor_id_segment = segments[1];
	let (actor_id, token) = if let Some(at_pos) = actor_id_segment.find('@') {
		let aid = &actor_id_segment[..at_pos];
		let tok = &actor_id_segment[at_pos + 1..];

		// Check for empty actor_id or token
		if aid.is_empty() || tok.is_empty() {
			return None;
		}

		// URL-decode both actor_id and token
		let decoded_aid = urlencoding::decode(aid).ok()?.to_string();
		let decoded_tok = urlencoding::decode(tok).ok()?.to_string();

		(decoded_aid, Some(decoded_tok))
	} else {
		// URL-decode actor_id
		let decoded_aid = urlencoding::decode(actor_id_segment).ok()?.to_string();
		(decoded_aid, None)
	};

	// Calculate the position in the original path where remaining path starts
	// We need to skip "/gateway/{actor_id_segment}"
	let prefix_len = 1 + segments[0].len() + 1 + segments[1].len(); // "/gateway/{actor_id_segment}"

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
		stripped_path: remaining_path,
	})
}
