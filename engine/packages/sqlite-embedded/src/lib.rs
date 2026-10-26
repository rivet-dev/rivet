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
#[cfg(feature = "transport")]
pub mod database;

/// SQLite optimization feature flags.
#[cfg(feature = "transport")]
pub mod optimization_flags;

#[cfg(feature = "transport")]
mod page_buffer;

/// Guards for connections that run SQL sent by an untrusted peer.
#[cfg(feature = "transport")]
pub mod guard;

/// SQLite query execution helpers.
#[cfg(feature = "transport")]
pub mod query;

/// Lightweight SQLite page classification for prefetch planning.
#[cfg(feature = "transport")]
pub mod sqlite_page;

pub use sqlite_types as types;

/// Custom SQLite VFS for actor-side depot transport.
pub mod vfs;

/// Single-threaded native SQLite command worker.
#[cfg(feature = "transport")]
pub mod worker;

/// Process-wide SQLite page cache shared by every connection.
#[cfg(feature = "transport")]
pub mod pager_cache;


