//! Partition-backed actor lifecycle and storage. External I/O lives in the executor.
pub mod actor_key;
pub mod actor_machine;
pub mod catalog;
pub mod commands;
pub mod envoy_check;
pub mod epoch;
pub mod errors;
pub mod index;
pub use rivet_data::pegboard_actor as wire;
pub mod keys;
mod metrics;
pub mod migrate_from_v2;
pub mod validation;
pub use metrics::{
	ENVOY_INDEX_UPSERTS, ENVOY_VERSION_EVICTIONS, STARTUP_PHASE_DURATION,
	STORAGE_OPERATION_DURATION,
};

pub const DEFAULT_NAMESPACE: &str = "default";
pub const DEFAULT_POOL: &str = "default";
pub const ENVOY_HEARTBEAT_TIMEOUT_MS: i64 = 15_000;
