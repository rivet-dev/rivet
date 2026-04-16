//! Native SQLite library for RivetKit.
//!
//! Provides a custom SQLite VFS backed by a transport-agnostic KV trait.
//! Consumers supply a `SqliteKv` implementation and this crate handles
//! VFS registration, database open/close, and chunk-level I/O.
//!
//! This is a pure Rust library. N-API bindings and transport clients
//! live in separate crates that compose this one.
//!
//! The KV-backed SQLite implementation used by `rivetkit-native` is defined in
//! this crate. Keep its storage layout and behavior in sync with the internal
//! SQLite data-channel spec.
//!
//! Key invariants:
//! - KV key layout and encoding (see `kv.rs`)
//! - Chunk size (4 KiB)
//! - PRAGMA settings
//! - VFS callback-to-KV-operation mapping
//! - Delete and truncate behavior
//! - Journal and BATCH_ATOMIC behavior

/// KV key layout for the native SQLite VFS.
pub mod kv;

/// Transport-agnostic KV trait for the SQLite VFS.
pub mod sqlite_kv;

/// Custom SQLite VFS that maps VFS callbacks to KV operations via the trait.
pub mod vfs;

/// Building blocks for the upcoming SQLite v2 actor-side VFS.
pub mod v2;
