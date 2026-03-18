//! BARE serialization/deserialization for KV channel protocol messages.
//!
//! Implements all types from `engine/sdks/schemas/kv-channel-protocol/v1.bare`.
//! Uses `serde_bare` for encoding/decoding.
//!
//! The protocol defines ToServer (client -> server) and ToClient (server -> client)
//! union types for WebSocket binary frames.
