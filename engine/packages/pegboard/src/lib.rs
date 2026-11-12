//! Partition-backed actor lifecycle and storage. External I/O lives in the executor.
pub mod actor_key;
pub mod actor_machine;
pub mod catalog;
pub mod commands;
pub mod envoy_check;
pub mod envoy_protocol;
pub mod epoch;
pub mod errors;
pub mod index;
pub use catalog::NamespaceStanding;
pub use rivet_data::generated::pegboard_namespace_v1 as namespace_wire;
pub use rivet_data::pegboard_actor as wire;
pub mod keys;
mod legacy_backfill;
mod legacy_namespace_rows;
pub mod legacy_volumes;
mod metrics;
pub mod migrate_from_v2;
pub mod namespace_machine;
pub mod namespace_waits;
pub mod sealed;
pub mod services;
pub mod validation;
pub use metrics::{
	ACTOR_FORKS, ENVOY_INDEX_UPSERTS, ENVOY_VERSION_EVICTIONS, STARTUP_PHASE_DURATION,
	STORAGE_OPERATION_DURATION,
};

pub const DEFAULT_NAMESPACE: &str = "default";
pub const DEFAULT_POOL: &str = "default";
pub const ENVOY_HEARTBEAT_TIMEOUT_MS: i64 = 15_000;
