use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
};

use anyhow::Result;
use gas::prelude::*;
use rivet_guard_core::request_context::RequestContext;

use crate::routing::{
	ActorPathInfo, SEC_WEBSOCKET_PROTOCOL, WS_PROTOCOL_ACTOR, pegboard_gateway::X_RIVET_ACTOR,
};

/// Build cache key for path-based actor routing
#[tracing::instrument(skip_all)]
pub fn build_cache_key_path_based(
	req_ctx: &RequestContext,
	actor_path_info: &ActorPathInfo,
) -> Result<u64> {
	let target = "actor";

	// Parse actor ID from path
	let actor_id = Id::parse(&actor_path_info.actor_id).context("invalid actor id in path")?;

	// Create a hash using actor_id, stripped path, and method
	let mut hasher = DefaultHasher::new();
	target.hash(&mut hasher);
	actor_id.hash(&mut hasher);
	// TODO: Should this exclude query for cache key?
	actor_path_info.stripped_path.hash(&mut hasher);
	req_ctx.method().as_str().hash(&mut hasher);
	let hash = hasher.finish();

	Ok(hash)
}

/// Build cache key for target-based actor routing (header or WebSocket protocol)
#[tracing::instrument(skip_all)]
pub fn build_cache_key_target_based(req_ctx: &RequestContext, target: &str) -> Result<u64> {
	// Check target
	ensure!(target == "actor", "wrong target");

	// Extract actor ID from WebSocket protocol or HTTP headers
	let actor_id_str = if req_ctx.is_websocket() {
		// For WebSocket, parse the sec-websocket-protocol header
		let protocols_header = req_ctx
			.headers()
			.get(SEC_WEBSOCKET_PROTOCOL)
			.and_then(|protocols| protocols.to_str().ok())
			.ok_or_else(|| {
				crate::errors::MissingHeader {
					header: "sec-websocket-protocol".to_string(),
				}
				.build()
			})?;

		let protocols: Vec<&str> = protocols_header.split(',').map(|p| p.trim()).collect();

		let actor_id_raw = protocols
			.iter()
			.find_map(|p| p.strip_prefix(WS_PROTOCOL_ACTOR))
			.ok_or_else(|| {
				crate::errors::MissingHeader {
					header: "`rivet_actor.*` protocol in sec-websocket-protocol".to_string(),
				}
				.build()
			})?;

		urlencoding::decode(actor_id_raw)
			.context("invalid url encoding in actor id")?
			.to_string()
	} else {
		// For HTTP, use the x-rivet-actor header
		req_ctx
			.headers()
			.get(X_RIVET_ACTOR)
			.ok_or_else(|| {
				crate::errors::MissingHeader {
					header: X_RIVET_ACTOR.to_string(),
				}
				.build()
			})?
			.to_str()
			.context("invalid x-rivet-actor header")?
			.to_string()
	};

	let actor_id = Id::parse(&actor_id_str).context("invalid actor id")?;

	// Create a hash using target, actor_id, path, and method
	let mut hasher = DefaultHasher::new();
	target.hash(&mut hasher);
	actor_id.hash(&mut hasher);
	// TODO: Should this exclude query for cache key?
	req_ctx.path().hash(&mut hasher);
	req_ctx.method().as_str().hash(&mut hasher);
	let hash = hasher.finish();

	Ok(hash)
}
