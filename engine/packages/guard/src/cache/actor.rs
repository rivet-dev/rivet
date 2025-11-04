use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
};

use anyhow::Result;
use gas::prelude::*;

use crate::routing::pegboard_gateway::X_RIVET_ACTOR;

#[tracing::instrument(skip_all)]
pub fn build_cache_key(
	target: &str,
	path: &str,
	method: &hyper::Method,
	headers: &hyper::HeaderMap,
) -> Result<u64> {
	// Check target
	ensure!(target == "actor", "wrong target");

	// Find actor to route to
	let actor_id_str = headers
		.get(X_RIVET_ACTOR)
		.ok_or_else(|| {
			crate::errors::MissingHeader {
				header: X_RIVET_ACTOR.to_string(),
			}
			.build()
		})?
		.to_str()
		.context("invalid x-rivet-actor header")?;
	let actor_id = Id::parse(actor_id_str).context("invalid x-rivet-actor header")?;

	// Create a hash using target, actor_id, path, and method
	let mut hasher = DefaultHasher::new();
	target.hash(&mut hasher);
	actor_id.hash(&mut hasher);
	path.hash(&mut hasher);
	method.as_str().hash(&mut hasher);
	let hash = hasher.finish();

	Ok(hash)
}
