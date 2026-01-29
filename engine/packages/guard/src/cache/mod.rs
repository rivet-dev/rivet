use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
	sync::Arc,
};

use anyhow::Result;
use gas::prelude::*;
use rivet_guard_core::{CacheKeyFn, request_context::RequestContext};

pub mod actor;

use crate::routing::X_RIVET_TARGET;

/// Creates the main cache key function that handles all incoming requests
#[tracing::instrument(skip_all)]
pub fn create_cache_key_function() -> CacheKeyFn {
	Arc::new(move |req_ctx| {
		tracing::debug!("building cache key");

		let target = match read_target(req_ctx.headers()) {
			Ok(target) => target,
			Err(err) => {
				tracing::debug!(?err, "failed parsing target for cache key");

				return Ok(host_path_method_cache_key(req_ctx));
			}
		};

		let cache_key = match actor::build_cache_key(req_ctx, target) {
			Ok(key) => Some(key),
			Err(err) => {
				tracing::debug!(?err, "failed to create actor cache key");

				None
			}
		};

		// Fallback to hostname + path + method hash if actor did not work
		if let Some(cache_key) = cache_key {
			Ok(cache_key)
		} else {
			Ok(host_path_method_cache_key(req_ctx))
		}
	})
}

fn read_target(headers: &hyper::HeaderMap) -> Result<&str> {
	// Read target
	let target = headers.get(X_RIVET_TARGET).ok_or_else(|| {
		crate::errors::MissingHeader {
			header: X_RIVET_TARGET.to_string(),
		}
		.build()
	})?;

	Ok(target.to_str()?)
}

fn host_path_method_cache_key(req_ctx: &RequestContext) -> u64 {
	let mut hasher = DefaultHasher::new();
	req_ctx.hostname().hash(&mut hasher);
	req_ctx.path().hash(&mut hasher);
	req_ctx.method().as_str().hash(&mut hasher);
	hasher.finish()
}
