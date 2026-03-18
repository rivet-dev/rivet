//! WebSocket KV channel client.
//!
//! Manages a persistent WebSocket connection to the KV channel endpoint,
//! sends requests with correlation IDs, and handles reconnection with
//! exponential backoff.
//!
//! One channel per process, shared across all actors.
//! See `docs-internal/engine/NATIVE_SQLITE_DATA_CHANNEL.md` for the full spec.
