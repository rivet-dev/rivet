//! Native SQLite addon for RivetKit.
//!
//! Routes SQLite page-level KV operations over a WebSocket KV channel protocol.
//! This is the native Rust counterpart to the WASM implementation in `@rivetkit/sqlite-vfs`.
//!
//! The native VFS and WASM VFS must match 1:1 in behavior:
//! - KV key layout and encoding (see `kv.rs` and `sqlite-vfs/src/kv.ts`)
//! - Chunk size (4 KiB)
//! - PRAGMA settings (page_size=4096, busy_timeout=5000)
//! - VFS callback-to-KV-operation mapping
//! - Delete/truncate strategy (both use deleteRange)
//! - Journal mode

/// KV key layout. Mirrors `rivetkit-typescript/packages/sqlite-vfs/src/kv.ts`.
pub mod kv;

/// BARE serialization/deserialization for KV channel protocol messages.
/// Implements types from `engine/sdks/schemas/kv-channel-protocol/v1.bare`.
pub mod protocol;

/// WebSocket KV channel client with reconnection and request correlation.
pub mod channel;

/// Custom SQLite VFS that maps VFS callbacks to KV operations via the channel.
pub mod vfs;
