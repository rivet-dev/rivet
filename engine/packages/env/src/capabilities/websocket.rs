//! Re-exports Axum's server WebSocket upgrade and framed socket API for injected HTTP listeners.
pub use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
