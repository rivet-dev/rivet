use std::time::{Duration, Instant};

/// Utility for rate limiting logs.
///
/// Logs are rate limited within this code instead of Vector because the earlier we drop logs, the
/// more resources it saves.
pub struct Throttle {
	threshold: usize,
	window: Duration,

	window_start: Instant,
	count: usize,
}

impl Throttle {
	pub fn new(threshold: usize, window: Duration) -> Self {
		Throttle {
			threshold,
			window,
			window_start: Instant::now(),
			count: 0,
		}
	}

	pub fn tick(&mut self) -> Result<(), TickError> {
		// Reset window
		if self.window_start.elapsed() > self.window {
			self.window_start = Instant::now();
			self.count = 0;
		}

		// Count
		//
		// Do this before error in order to determine if first throttle
		self.count += 1;

		// Throttle
		if self.count > self.threshold {
			return Err(TickError {
				time_remaining: self.window - self.window_start.elapsed(),
				first_throttle_in_window: self.count == self.threshold + 1,
			});
		}

		Ok(())
	}
}

pub struct TickError {
	/// How much time is remaining in this window
	pub time_remaining: Duration,

	/// The first throttle in this time window
	pub first_throttle_in_window: bool,
}
