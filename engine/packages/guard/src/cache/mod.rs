use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
	sync::Arc,
};

use gas::prelude::*;
use rivet_guard_core::{CacheKeyFn, request_context::RequestContext};

pub mod pegboard_gateway;

use crate::routing::{
	SEC_WEBSOCKET_PROTOCOL, WS_PROTOCOL_TARGET, X_RIVET_TARGET, parse_actor_path,
};

/// Creates the main cache key function that handles all incoming requests
#[tracing::instrument(skip_all)]
pub fn create_cache_key_function() -> CacheKeyFn {
	Arc::new(move |req_ctx| {
		tracing::debug!("building cache key");

		// MARK: Path-based cache key
		// Check for path-based actor routing
		if let Some(actor_path_info) = parse_actor_path(req_ctx.path()) {
			tracing::debug!("using path-based cache key for actor");

			if let Ok(cache_key) =
				pegboard_gateway::build_cache_key_path_based(req_ctx, &actor_path_info)
			{
				return Ok(cache_key);
			}
		}

		// MARK: Header- & protocol-based cache key (X-Rivet-Target)
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

		// Check target-based cache functions
		if let Some(target) = target {
			if let Ok(cache_key) = pegboard_gateway::build_cache_key_target_based(req_ctx, target) {
				return Ok(cache_key);
			}
		}

		// MARK: Fallback
		tracing::debug!("using fallback cache key");
		Ok(host_path_method_cache_key(req_ctx))
	})
}

fn host_path_method_cache_key(req_ctx: &RequestContext) -> u64 {
	let mut hasher = DefaultHasher::new();
	req_ctx.hostname().hash(&mut hasher);
	req_ctx.path().hash(&mut hasher);
	req_ctx.method().as_str().hash(&mut hasher);
	hasher.finish()
}
