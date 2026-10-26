//! Replicated volume writes driven by a deterministic core and injected transport.

#![warn(unreachable_pub)]



pub mod volume_client_machine;

#[cfg(not(target_arch = "wasm32"))]
pub mod ups;
