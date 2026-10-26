//! Production and simulation implementations of the environment capabilities.

#[cfg(not(target_arch = "wasm32"))]
pub mod production;
