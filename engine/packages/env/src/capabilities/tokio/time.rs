//! Reimplements `tokio::time::timeout_at` and `interval` using an injected `Clock`
//! and millisecond deadlines. Interval ticks skip missed periods.

use crate::{Clock, ClockError};
use std::{future::Future, sync::Arc, time::Duration};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimeoutError {
	Elapsed,
	Clock(ClockError),
}

impl std::fmt::Display for TimeoutError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Elapsed => f.write_str("deadline elapsed"),
			Self::Clock(error) => write!(f, "clock failed: {error}"),
		}
	}
}

impl std::error::Error for TimeoutError {}

pub async fn timeout_at<F: Future>(
	clock: &dyn Clock,
	deadline_boot_ms: u64,
	fut: F,
) -> Result<F::Output, TimeoutError> {
	crate::select_biased!("expired deadlines win over ready work",
		result = clock.sleep_until(deadline_boot_ms) => Err(match result {
			Ok(()) => TimeoutError::Elapsed,
			Err(error) => TimeoutError::Clock(error),
		}),
		result = fut => Ok(result),
	)
}

pub struct Interval {
	clock: Arc<dyn Clock>,
	period: u64,
	next: Result<u64, ClockError>,
}

pub fn interval(clock: Arc<dyn Clock>, period_ms: u64) -> Interval {
	assert!(period_ms > 0, "interval period must be positive");
	let next = clock.observe().map(|c| c.boot_ms);
	Interval {
		clock,
		period: period_ms,
		next,
	}
}

impl Interval {
	/// Skips missed ticks. Faults are explicit rather than becoming a busy loop.
	pub async fn tick(&mut self) -> Result<u64, ClockError> {
		let next = self.next.clone()?;
		self.clock.sleep_until(next).await?;
		let now = self.clock.observe()?.boot_ms;
		let periods = now.saturating_sub(next) / self.period;
		self.next = periods
			.checked_add(1)
			.and_then(|count| count.checked_mul(self.period))
			.and_then(|advance| next.checked_add(advance))
			.ok_or(ClockError::Overflow);

		Ok(now)
	}
}

/// Convert a relative budget once into this clock's absolute boot-time domain.
pub fn deadline_after(clock: &dyn Clock, duration: Duration) -> Result<u64, ClockError> {
	let millis = u64::try_from(duration.as_millis()).map_err(|_| ClockError::Overflow)?;
	clock
		.observe()?
		.boot_ms
		.checked_add(millis)
		.ok_or(ClockError::Overflow)
}

pub async fn sleep(clock: &dyn Clock, duration: Duration) -> Result<(), ClockError> {
	clock.sleep_until(deadline_after(clock, duration)?).await
}

pub async fn timeout<F: Future>(
	clock: &dyn Clock,
	duration: Duration,
	future: F,
) -> Result<F::Output, TimeoutError> {
	let deadline = deadline_after(clock, duration).map_err(TimeoutError::Clock)?;
	timeout_at(clock, deadline, future).await
}
