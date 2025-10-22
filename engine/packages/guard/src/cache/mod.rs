use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
	sync::Arc,
};

use anyhow::Result;
use gas::prelude::*;
use rivet_guard_core::CacheKeyFn;

pub mod actor;

use crate::routing::X_RIVET_TARGET;

/// Creates the main cache key function that handles all incoming requests
#[tracing::instrument(skip_all)]
pub fn create_cache_key_function(_ctx: StandaloneCtx) -> CacheKeyFn {
	Arc::new(move |hostname, path, _port_type, headers| {
		tracing::debug!("building cache key");

		let target = match read_target(headers) {
			Ok(target) => target,
			Err(err) => {
				tracing::debug!(?err, "failed parsing target for cache key");

				return Ok(host_path_cache_key(hostname, path));
			}
		};

		let cache_key = match actor::build_cache_key(target, path, headers) {
			Ok(key) => Some(key),
			Err(err) => {
				tracing::debug!(?err, "failed to create actor cache key");

				None
			}
		};

		// Fallback to hostname + path hash if actor did not work
		if let Some(cache_key) = cache_key {
			Ok(cache_key)
		} else {
			Ok(host_path_cache_key(hostname, path))
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

fn host_path_cache_key(hostname: &str, path: &str) -> u64 {
	// Extract just the hostname, stripping the port if present
	let hostname_only = hostname.split(':').next().unwrap_or(hostname);

	let mut hasher = DefaultHasher::new();
	hostname_only.hash(&mut hasher);
	path.hash(&mut hasher);
	hasher.finish()
}
