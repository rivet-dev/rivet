use rand::Rng;
use tokio::time::{Duration, Instant};

pub struct Backoff {
	/// Maximum exponent for the backoff.
	max_exponent: usize,

	/// Maximum amount of retries.
	max_retries: Option<usize>,

	/// Base wait time in ms.
	wait: usize,

	/// Maximum randomness.
	randomness: usize,

	/// Iteration of the backoff.
	i: usize,

	/// Timestamp to sleep until in ms.
	sleep_until: Instant,
}

impl Backoff {
	pub fn new(
		max_exponent: usize,
		max_retries: Option<usize>,
		wait: usize,
		randomness: usize,
	) -> Backoff {
		Backoff {
			max_exponent,
			max_retries,
			wait,
			randomness,
			i: 0,
			sleep_until: Instant::now(),
		}
	}

	pub fn new_at(
		max_exponent: usize,
		max_retries: Option<usize>,
		wait: usize,
		randomness: usize,
		i: usize,
	) -> Backoff {
		Backoff {
			max_exponent,
			max_retries,
			wait,
			randomness,
			i,
			sleep_until: Instant::now(),
		}
	}

	pub fn tick_index(&self) -> usize {
		self.i
	}

	/// Waits for the next backoff tick.
	///
	/// Returns false if the index is greater than `max_retries`.
	pub async fn tick(&mut self) -> bool {
		if self.max_retries.map_or(false, |x| self.i > x) {
			return false;
		}

		tokio::time::sleep_until(self.sleep_until).await;

		let next_wait = self.current_duration() + rand::thread_rng().gen_range(0..self.randomness);
		self.sleep_until += Duration::from_millis(next_wait as u64);

		self.i += 1;

		true
	}

	/// Returns the instant of the next backoff tick. Does not wait.
	///
	/// Returns None if the index is greater than `max_retries`.
	pub fn step(&mut self) -> Option<Instant> {
		if self.max_retries.map_or(false, |x| self.i > x) {
			return None;
		}

		let next_wait = self.wait * 2usize.pow(self.i.min(self.max_exponent) as u32)
			+ rand::thread_rng().gen_range(0..self.randomness);
		self.sleep_until += Duration::from_millis(next_wait as u64);

		self.i += 1;

		Some(self.sleep_until)
	}

	pub fn current_duration(&self) -> usize {
		self.wait * 2usize.pow(self.i.min(self.max_exponent) as u32)
	}

	pub fn default_infinite() -> Backoff {
		Backoff::new(8, None, 1_000, 1_000)
	}
}

impl Default for Backoff {
	fn default() -> Backoff {
		Backoff::new(5, Some(16), 1_000, 1_000)
	}
}
