//! Edge validation of names that end up inside formal keys. Actor names, actor keys and pool
//! names sit inside partition keys, which Depot caps in size, so every create path and the
//! Envoy connect check them before anything is written.
use anyhow::Result;
use rivet_error::RivetError;
use serde::{Deserialize, Serialize};

/// The largest actor name, in bytes.
pub const MAX_ACTOR_NAME_BYTES: usize = 64;
/// The largest actor key, in bytes.
pub const MAX_ACTOR_KEY_BYTES: usize = 1024;
pub const MAX_NAMESPACE_NAME_BYTES: usize = 63;
/// The largest pool name, in bytes.
pub const MAX_POOL_NAME_BYTES: usize = 63;

#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"actor",
	"invalid_name",
	"Actor name is invalid.",
	"Actor name must be 1 to {max_bytes} bytes."
)]
pub struct InvalidActorName {
	pub max_bytes: usize,
}

#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"actor",
	"invalid_key",
	"Actor key is invalid.",
	"Actor key must be at most {max_bytes} bytes."
)]
pub struct InvalidActorKey {
	pub max_bytes: usize,
}

#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"envoy",
	"invalid_namespace_name",
	"Namespace name is invalid.",
	"Namespace name must be a DNS label of 1 to {max_bytes} lowercase letters, digits and hyphens."
)]
pub struct InvalidNamespaceName {
	pub max_bytes: usize,
}

#[derive(Debug, RivetError, Serialize, Deserialize)]
#[error(
	"envoy",
	"invalid_pool_name",
	"Pool name is invalid.",
	"Pool name must be a DNS label of 1 to {max_bytes} lowercase letters, digits and hyphens."
)]
pub struct InvalidPoolName {
	pub max_bytes: usize,
}

#[derive(Debug, RivetError)]
#[error("namespace", "not_found", "The namespace does not exist.")]
pub struct NamespaceNotFound;

pub fn actor_name(name: &str) -> Result<()> {
	if name.is_empty() || name.len() > MAX_ACTOR_NAME_BYTES {
		return Err(InvalidActorName {
			max_bytes: MAX_ACTOR_NAME_BYTES,
		}
		.build());
	}
	Ok(())
}

pub fn actor_key(key: Option<&str>) -> Result<()> {
	if key.is_some_and(|key| key.len() > MAX_ACTOR_KEY_BYTES) {
		return Err(InvalidActorKey {
			max_bytes: MAX_ACTOR_KEY_BYTES,
		}
		.build());
	}
	Ok(())
}

pub fn namespace_name(namespace: &str) -> Result<()> {
	let valid = !namespace.is_empty()
		&& namespace.len() <= MAX_NAMESPACE_NAME_BYTES
		&& namespace
			.bytes()
			.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
		&& !namespace.starts_with('-')
		&& !namespace.ends_with('-');
	if !valid {
		return Err(InvalidNamespaceName {
			max_bytes: MAX_NAMESPACE_NAME_BYTES,
		}
		.build());
	}
	Ok(())
}

pub fn pool_name(pool: &str) -> Result<()> {
	let valid = !pool.is_empty()
		&& pool.len() <= MAX_POOL_NAME_BYTES
		&& pool
			.bytes()
			.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
		&& !pool.starts_with('-')
		&& !pool.ends_with('-');
	if !valid {
		return Err(InvalidPoolName {
			max_bytes: MAX_POOL_NAME_BYTES,
		}
		.build());
	}
	Ok(())
}



#[derive(Debug, RivetError)]
#[error("pool", "not_found", "The node pool does not exist.")]
pub struct PoolNotFound;
