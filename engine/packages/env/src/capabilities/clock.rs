//! Injected clock interface replacing direct `std::time` clock reads and
//! `tokio::time::sleep_until` calls.

use crate::Clocks;
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClockError {
	Unhealthy,
	MonotonicRegression { previous: u64, current: u64 },
	Overflow,
	Source(String),
}

impl std::fmt::Display for ClockError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}

impl std::error::Error for ClockError {}

/// Permanently poisoned by regression or source failure.
pub trait Clock: Send + Sync {
	fn observe(&self) -> Result<Clocks, ClockError>;

	/// Reads only the boot clock, for callers that never use wall time. It applies the same
	/// monotonicity check and poisoning as `observe`.
	fn boot_ms(&self) -> Result<u64, ClockError> {
		self.observe().map(|clocks| clocks.boot_ms)
	}

	/// Deadlines use this clock's boot-time domain, never another host's clock.
	fn sleep_until(
		&self,
		boot_ms: u64,
	) -> Pin<Box<dyn Future<Output = Result<(), ClockError>> + Send>>;
}
