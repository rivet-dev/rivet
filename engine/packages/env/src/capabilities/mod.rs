//! Capabilities used by callers, independent of the selected environment backend.

pub mod clock;

pub mod guard;
pub mod object_store;
pub mod rand;
pub mod tokio;

pub mod fs;
pub mod host;
pub mod http;

pub mod shutdown;

#[cfg(feature = "slatedb")]
pub mod slatedb;
pub mod slatedb_result;

#[cfg(not(target_arch = "wasm32"))]
pub mod websocket;
