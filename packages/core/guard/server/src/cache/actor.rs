use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
};

use anyhow::Result;
use gas::prelude::*;

use crate::routing::pegboard_gateway::{X_RIVET_ACTOR, X_RIVET_PORT};
use hyper::header::HeaderName;

#[tracing::instrument(skip_all)]
pub fn build_cache_key(target: &str, path: &str, headers: &hyper::HeaderMap) -> Result<u64> {
	// Check target
	ensure!(target == "actor", "wrong target");

	// Find actor to route to
	let actor_id_str = headers.get(X_RIVET_ACTOR).ok_or_else(|| {
		crate::errors::MissingHeader {
			header: X_RIVET_ACTOR.to_string(),
		}
		.build()
	})?;
	let actor_id = Id::parse(actor_id_str.to_str()?)?;

	let port_name = headers.get(X_RIVET_PORT).ok_or_else(|| {
		crate::errors::MissingHeader {
			header: X_RIVET_PORT.to_string(),
		}
		.build()
	})?;
	let port_name = port_name.to_str()?;

	// Create a hash using target, actor_id and port_name
	let mut hasher = DefaultHasher::new();
	target.hash(&mut hasher);
	actor_id.hash(&mut hasher);
	port_name.hash(&mut hasher);
	path.hash(&mut hasher);
	let hash = hasher.finish();

	Ok(hash)
}
