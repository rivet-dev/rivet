//! Adapts the shared backoff algorithm to legacy Tokio Instant-based callers.

use rivet_env::Clock;
use std::sync::Arc;
use tokio::time::{Duration, Instant};

pub struct Backoff {
	inner: super::Backoff,
	boot_origin: u64,
	instant_origin: Instant,
}

impl Backoff {
	pub fn new(
		max_exponent: usize,
		max_retries: Option<usize>,
		wait: usize,
		randomness: usize,
	) -> Self {
		Self::new_at(max_exponent, max_retries, wait, randomness, 0)
	}

	pub fn new_at(
		max_exponent: usize,
		max_retries: Option<usize>,
		wait: usize,
		randomness: usize,
		index: usize,
	) -> Self {
		let clock = Arc::new(rivet_env::TokioClock::new().expect("legacy production clock"));
		let instant_origin = Instant::now();
		let boot_origin = clock.observe().expect("legacy production clock").boot_ms;
		let mut seed = [0; 8];
		rivet_env::backends::production::secure_random(&mut seed).expect("legacy jitter entropy");
		let rng = rivet_env::RngStream::seeded(
			u64::from_le_bytes(seed),
			&rivet_env::NodeId(Default::default()),
			"legacy.backoff",
		);
		Self {
			inner: super::Backoff::new(clock, rng, max_exponent, max_retries, wait, randomness)
				.expect("legacy production clock")
				.at(index),
			boot_origin,
			instant_origin,
		}
	}

	pub fn tick_index(&self) -> usize {
		self.inner.tick_index()
	}

	pub fn current_duration(&self) -> usize {
		self.inner
			.current_duration()
			.expect("legacy backoff overflow")
			.try_into()
			.expect("legacy duration fits usize")
	}

	pub async fn tick(&mut self) -> bool {
		self.inner.tick().await.expect("legacy backoff clock")
	}

	pub fn step(&mut self) -> Option<Instant> {
		self.inner
			.step()
			.expect("legacy backoff clock")
			.map(|deadline| {
				self.instant_origin
					+ Duration::from_millis(deadline.saturating_sub(self.boot_origin))
			})
	}

	pub fn default_infinite() -> Self {
		Self::new(8, None, 1000, 1000)
	}
}

impl Default for Backoff {
	fn default() -> Self {
		Self::new(5, Some(16), 1000, 1000)
	}
}
