//! Worker leases, storage discovery, and endpoint lookup through one state machine.
//! Discovery lists storage nodes only; background garbage collection may list both roles.

pub mod config;
pub mod discovery;
pub mod heartbeat;
pub mod heartbeat_gc;
pub mod keys;
mod metrics;
pub mod ownership;
pub mod services;
pub mod topology;
pub mod topology_machine;
pub mod versions;
pub mod watch;
pub mod worker;
pub mod worker_lease;

pub use config::TopologyConfig;
pub use heartbeat::HeartbeatState;
pub use topology::Topology;
pub use topology_machine::snapshot::TopologySnapshot;
pub use topology_machine::types::{FenceReason, LeaseStatus, WorkerRegistration};
pub use watch::WatchHandle;
pub use worker_lease::{WorkerLease, WorkerLeaseConfig};
