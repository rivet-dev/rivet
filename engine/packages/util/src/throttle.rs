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

		let next_wait = self.current_duration() + rand::thread_rng().gen_range(0..self.randomness);
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

pub enum RateLimitMethod {
	FixedWindow {
		requests: u64,
		period: Duration,
	},
	LeakyBucket {
		requests: u64,
		/// How quickly to regain requests. 1 / drip_rate
		drip_rate: Duration,
	},
}

enum RateLimitState {
	FixedWindow {
		requests_remaining: u64,
		requests_limit: u64,
		reset_time: Instant,
		period: Duration,
	},
	LeakyBucket {
		requests_remaining: u64,
		requests_limit: u64,
		last_acquire: Instant,
		drip_rate: Duration,
		accum_drip: f32,
	},
}

pub struct RateLimiter {
	state: RateLimitState,
}

impl RateLimiter {
	pub fn new(method: RateLimitMethod) -> Self {
		Self {
			state: match method {
				RateLimitMethod::FixedWindow { requests, period } => RateLimitState::FixedWindow {
					requests_remaining: requests,
					requests_limit: requests,
					reset_time: Instant::now() + period,
					period,
				},
				RateLimitMethod::LeakyBucket {
					requests,
					drip_rate,
				} => RateLimitState::LeakyBucket {
					requests_remaining: requests,
					requests_limit: requests,
					last_acquire: Instant::now(),
					drip_rate: drip_rate,
					accum_drip: 0.0,
				},
			},
		}
	}

	pub fn try_acquire(&mut self) -> bool {
		match &mut self.state {
			RateLimitState::FixedWindow {
				requests_remaining,
				requests_limit,
				reset_time,
				period,
			} => {
				let now = Instant::now();
				// Check if we need to reset the counter
				if now >= *reset_time {
					*requests_remaining = *requests_limit;
					*reset_time = now + *period;
				}

				// Try to consume a request
				if *requests_remaining > 0 {
					*requests_remaining -= 1;
					true
				} else {
					false
				}
			}
			RateLimitState::LeakyBucket {
				requests_remaining,
				requests_limit,
				last_acquire,
				drip_rate,
				accum_drip,
			} => {
				let now = Instant::now();
				let dt = now - *last_acquire;
				*last_acquire = now;

				// Drip bucket
				if requests_remaining < requests_limit {
					*accum_drip += dt.div_duration_f32(*drip_rate);

					*requests_remaining +=
						(*accum_drip as u64).min(*requests_limit - *requests_remaining);

					if *accum_drip >= 1.0 {
						*accum_drip = accum_drip.fract();
					}
				}

				if *requests_remaining > 0 {
					*requests_remaining -= 1;
					true
				} else {
					false
				}
			}
		}
	}

	pub async fn acquire(&mut self) {
		match &mut self.state {
			RateLimitState::FixedWindow {
				requests_remaining,
				requests_limit,
				reset_time,
				period,
			} => {
				let now = Instant::now();
				// Check if we need to reset the counter
				if now >= *reset_time {
					*requests_remaining = *requests_limit;
					*reset_time = now + *period;
				}

				// Try to consume a request
				if *requests_remaining > 0 {
					*requests_remaining -= 1;
				} else {
					tokio::time::sleep(*period).await;

					*requests_remaining = *requests_limit;
					*reset_time = Instant::now() + *period;
				}
			}
			RateLimitState::LeakyBucket {
				requests_remaining,
				requests_limit,
				last_acquire,
				drip_rate,
				accum_drip,
			} => {
				let now = Instant::now();
				let dt = now - *last_acquire;
				*last_acquire = now;

				// Drip bucket
				if requests_remaining < requests_limit {
					*accum_drip += dt.div_duration_f32(*drip_rate);

					*requests_remaining +=
						(*accum_drip as u64).min(*requests_limit - *requests_remaining);

					if *accum_drip >= 1.0 {
						*accum_drip = accum_drip.fract();
					}
				}

				if *requests_remaining > 0 {
					*requests_remaining -= 1;
				} else {
					let deficit = 1.0 - *accum_drip;
					tokio::time::sleep(drip_rate.mul_f32(deficit)).await;

					*last_acquire = Instant::now();
					*accum_drip = 0.0;
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{RateLimitMethod, RateLimiter};
	use tokio::time::{Duration, Instant};

	// MARK: FixedWindow / try_acquire

	#[tokio::test(start_paused = true)]
	async fn fixed_window_allows_full_burst_then_blocks() {
		let mut rl = RateLimiter::new(RateLimitMethod::FixedWindow {
			requests: 3,
			period: Duration::from_millis(100),
		});

		assert!(rl.try_acquire());
		assert!(rl.try_acquire());
		assert!(rl.try_acquire());
		// Limit reached within the window.
		assert!(!rl.try_acquire());
	}

	#[tokio::test(start_paused = true)]
	async fn fixed_window_does_not_refill_before_period() {
		let mut rl = RateLimiter::new(RateLimitMethod::FixedWindow {
			requests: 2,
			period: Duration::from_millis(100),
		});

		assert!(rl.try_acquire());
		assert!(rl.try_acquire());
		assert!(!rl.try_acquire());

		// Just shy of a full period: still no refill. The window is
		// all-or-nothing, it does not drip partial credit.
		tokio::time::advance(Duration::from_millis(99)).await;
		assert!(!rl.try_acquire());
	}

	#[tokio::test(start_paused = true)]
	async fn fixed_window_resets_to_full_after_period() {
		let mut rl = RateLimiter::new(RateLimitMethod::FixedWindow {
			requests: 2,
			period: Duration::from_millis(100),
		});

		assert!(rl.try_acquire());
		assert!(rl.try_acquire());
		assert!(!rl.try_acquire());

		// After a full period the window resets to its full allowance.
		tokio::time::advance(Duration::from_millis(100)).await;
		assert!(rl.try_acquire());
		assert!(rl.try_acquire());
		assert!(!rl.try_acquire());
	}

	// MARK: LeakyBucket / try_acquire

	#[tokio::test(start_paused = true)]
	async fn leaky_bucket_allows_full_burst_then_blocks() {
		let mut rl = RateLimiter::new(RateLimitMethod::LeakyBucket {
			requests: 3,
			drip_rate: Duration::from_millis(10),
		});

		assert!(rl.try_acquire());
		assert!(rl.try_acquire());
		assert!(rl.try_acquire());
		assert!(!rl.try_acquire());
	}

	#[tokio::test(start_paused = true)]
	async fn leaky_bucket_drips_exactly_one_token_per_rate() {
		let mut rl = RateLimiter::new(RateLimitMethod::LeakyBucket {
			requests: 3,
			drip_rate: Duration::from_millis(10),
		});

		// Drain the bucket.
		for _ in 0..3 {
			assert!(rl.try_acquire());
		}
		assert!(!rl.try_acquire());

		// Exactly one drip period yields exactly one token, no more.
		tokio::time::advance(Duration::from_millis(10)).await;
		assert!(rl.try_acquire());
		assert!(!rl.try_acquire());
	}

	#[tokio::test(start_paused = true)]
	async fn leaky_bucket_refill_is_capped_at_capacity() {
		let mut rl = RateLimiter::new(RateLimitMethod::LeakyBucket {
			requests: 3,
			drip_rate: Duration::from_millis(10),
		});

		for _ in 0..3 {
			assert!(rl.try_acquire());
		}
		assert!(!rl.try_acquire());

		// Idle far longer than it takes to refill the whole bucket. Credit must
		// not accumulate past capacity, so only `requests` tokens are available.
		tokio::time::advance(Duration::from_millis(1_000)).await;
		assert!(rl.try_acquire());
		assert!(rl.try_acquire());
		assert!(rl.try_acquire());
		assert!(!rl.try_acquire());
	}

	#[tokio::test(start_paused = true)]
	async fn leaky_bucket_accumulates_fractional_drip_across_calls() {
		let mut rl = RateLimiter::new(RateLimitMethod::LeakyBucket {
			requests: 1,
			drip_rate: Duration::from_millis(10),
		});

		// Consume the only token.
		assert!(rl.try_acquire());
		assert!(!rl.try_acquire());

		// Half a drip period: less than one whole token, still blocked.
		tokio::time::advance(Duration::from_millis(5)).await;
		assert!(!rl.try_acquire());

		// Another half period: the fractional credit from the previous interval
		// must carry over and complete one whole token.
		tokio::time::advance(Duration::from_millis(5)).await;
		assert!(rl.try_acquire());
		assert!(!rl.try_acquire());
	}

	// MARK: acquire (blocking)

	#[tokio::test(start_paused = true)]
	async fn acquire_returns_immediately_while_tokens_remain() {
		let mut rl = RateLimiter::new(RateLimitMethod::LeakyBucket {
			requests: 3,
			drip_rate: Duration::from_millis(10),
		});

		let start = Instant::now();
		rl.acquire().await;
		rl.acquire().await;
		rl.acquire().await;
		// Burst is served without waiting.
		assert_eq!(start.elapsed(), Duration::ZERO);
	}

	#[tokio::test(start_paused = true)]
	async fn acquire_blocks_until_a_token_is_available() {
		let mut rl = RateLimiter::new(RateLimitMethod::LeakyBucket {
			requests: 1,
			drip_rate: Duration::from_millis(10),
		});

		// Drain the single token.
		rl.acquire().await;

		// The next acquire must wait one full drip period for a token.
		let start = Instant::now();
		rl.acquire().await;
		assert!(start.elapsed() >= Duration::from_millis(10));
	}

	#[tokio::test(start_paused = true)]
	async fn acquire_sustains_the_drip_rate_without_doubling() {
		let mut rl = RateLimiter::new(RateLimitMethod::LeakyBucket {
			requests: 1,
			drip_rate: Duration::from_millis(10),
		});

		// Drain the initial burst token so every subsequent acquire starts empty.
		rl.acquire().await;

		let start = Instant::now();
		// Five acquires, each starting from an empty bucket, must each cost one
		// drip period, so the total is at least 5 * drip_rate. A limiter that
		// admits the post-sleep request without debiting a token finishes in
		// ~3 periods, effectively doubling the sustained rate.
		for _ in 0..5 {
			rl.acquire().await;
		}
		assert!(start.elapsed() >= Duration::from_millis(50));
	}

	#[tokio::test(start_paused = true)]
	async fn fixed_window_acquire_blocks_until_window_resets() {
		let mut rl = RateLimiter::new(RateLimitMethod::FixedWindow {
			requests: 2,
			period: Duration::from_millis(100),
		});

		rl.acquire().await;
		rl.acquire().await;

		// The window is exhausted, so the next acquire must wait for the reset.
		let start = Instant::now();
		rl.acquire().await;
		assert!(start.elapsed() >= Duration::from_millis(100));
	}
}
