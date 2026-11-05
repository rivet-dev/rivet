//! Errors of the V2 migration that cross the engine edge or reach the V2 migrator.
use rivet_error::RivetError;
use serde::{Deserialize, Serialize};

/// Retry hint of `actor.migrating_from_v2`. A held actor's refusal window is about the
/// migrator's confirm delay, so a client retrying at this pace lands soon after the actor
/// activates.
pub const MIGRATING_FROM_V2_RETRY_AFTER_MS: u64 = 1_000;

/// The actor is moving into this cluster from V2. Nothing ran, so the request may be retried.
#[derive(Debug, Serialize, Deserialize, RivetError)]
#[error(
	"actor",
	"migrating_from_v2",
	"The actor is moving to this cluster from V2. Retry after the hinted delay.",
	"The actor is moving to this cluster from V2. Retry after {retry_after_ms} ms."
)]
pub struct ActorMigratingFromV2 {
	pub retry_after_ms: u64,
}

impl ActorMigratingFromV2 {
	pub fn error() -> anyhow::Error {
		Self {
			retry_after_ms: MIGRATING_FROM_V2_RETRY_AFTER_MS,
		}
		.build()
	}
}

/// A create named an actor version in `x-rivet-actor-version` that this cluster cannot create.
#[derive(Debug, Serialize, Deserialize, RivetError)]
#[error(
	"actor",
	"invalid_version",
	"The requested actor version cannot be created here.",
	"The requested actor version '{version}' cannot be created here; the only accepted value is 3."
)]
pub struct ActorInvalidVersion {
	pub version: String,
}

/// A V2 migration event, or a keyed create in a namespace migrating in from V2, arrived while
/// some live storage worker still reads only actor state older than `required_version`, so the
/// held or delegated actor could not be stored. The caller retries on its own deadline.
#[derive(Debug, Serialize, Deserialize, RivetError)]
#[error(
	"actor",
	"migrate_from_v2_unsupported",
	"The cluster cannot hold actors for a V2 migration yet.",
	"The cluster cannot hold actors for a V2 migration until every storage worker reads actor state version {required_version}; it reads version {write_version}."
)]
pub struct MigrateFromV2Unsupported {
	pub write_version: u16,
	pub required_version: u16,
}
