//! Incremental `Env` facade for Tokio tasks and timers, `std::time` clocks,
//! `rand` randomness, and `object_store` storage. Exposes the APIs callers need
//! through injectable runtime and networking interfaces.

use lazy_static::lazy_static;
use std::env as process_env;

lazy_static! {
	static ref SERVICE_NAME: String =
		process_env::var("RIVET_SERVICE_NAME").unwrap_or_else(|_| "rivet".to_string());
}

/// Generic name used to differentiate pools of servers.
pub fn service_name() -> &'static str {
	&SERVICE_NAME
}

pub mod backends;
pub mod capabilities;
mod env;

pub use capabilities::guard::MutationGuard;

#[cfg(not(target_arch = "wasm32"))]
pub use backends::production::{ProductionEnv, TokioClock};
pub use capabilities::{clock, fs, host, object_store, rand, tokio};
pub use clock::{Clock, ClockError};
pub use env::{Clocks, Env, NodeId};
pub use rand::RngStream;
pub use tokio::task::{TaskHandle, TaskScope};
pub use tokio::time::{TimeoutError, deadline_after, interval, sleep, timeout, timeout_at};

#[doc(hidden)]
pub use tokio::__Choice;
