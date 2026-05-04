//! Depot-backed SQLite client.
//!
//! Provides the native SQLite VFS used by Rivet actors.
//!
//! This is a pure Rust library. N-API bindings and transport clients
//! live in separate crates that compose this one.
//!
//! The SQLite implementation used by `rivetkit-napi` is defined in this crate.
//! Keep its storage layout and behavior in sync with the internal SQLite
//! data-channel spec.
//!
//! Key invariants:
//! - PRAGMA settings
//! - Delete and truncate behavior
//! - Journal and BATCH_ATOMIC behavior

/// Unified native database handles and open helpers.
pub mod database;

/// SQLite optimization feature flags.
pub mod optimization_flags;

/// SQLite query execution helpers.
pub mod query;

/// SQLite transport adapters for same-process Depot usage.
pub mod transport;

pub use depot_client_types as types;

/// Custom SQLite VFS for actor-side depot transport.
pub mod vfs;

/// Single-threaded native SQLite command worker.
pub mod worker;
