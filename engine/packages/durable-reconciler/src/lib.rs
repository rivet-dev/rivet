//! Use for provisioning and configuration that must resume after a crash.
//! The database is authoritative; intermediate desired states may be coalesced.
//! Effects must be idempotent: a crash after external success but before saving
//! progress can repeat the action.
//!
//! [`core`] supports application-owned storage; [`depot`] provides a desired/actual
//! record with conditional writes. For protocols where each input drives a
//! transition, such as lease expiry, use `rivet-state-machine-ephemeral`.
pub mod core;
pub mod depot;
mod metrics;
