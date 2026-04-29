//! SQLite library for RivetKit.
//!
//! Provides the native SQLite VFS used by RivetKit actors.
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

/// Custom SQLite VFS for actor-side sqlite-storage transport.
pub mod vfs;
