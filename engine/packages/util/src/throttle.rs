use tokio::time::{Duration, Instant};

/// Compatibility adapters for dormant Engine workflows.
pub mod legacy;

/// Backoff driven by caller-owned time and operational randomness.
/// The first tick is immediate; subsequent ticks retain the scheduled cadence.
pub struct Backoff {
	clock: std::sync::Arc<dyn rivet_env::Clock>,
	rng: rivet_env::RngStream,
	max_exponent: usize,
	max_retries: Option<usize>,
	wait: usize,
	randomness: usize,
	i: usize,
	next: u64,
}

impl Backoff {
	pub fn new(
		clock: std::sync::Arc<dyn rivet_env::Clock>,
		rng: rivet_env::RngStream,
		max_exponent: usize,
		max_retries: Option<usize>,
		wait: usize,
		randomness: usize,
	) -> Result<Backoff, rivet_env::ClockError> {
		let next = clock.observe()?.boot_ms;
		Ok(Backoff {
			clock,
			rng,
			max_exponent,
			max_retries,
			wait,
			randomness,
			i: 0,
			next,
		})
	}
}

impl Backoff {
	pub fn at(mut self, index: usize) -> Self {
		self.i = index;
		self
	}

	pub fn step(&mut self) -> Result<Option<u64>, rivet_env::ClockError> {
		if self.max_retries.is_some_and(|max| self.i > max) {
			return Ok(None);
		}
		let base = self.current_duration()?;
		let jitter = if self.randomness == 0 {
			0
		} else {
			self.rng.gen_range(0..self.randomness) as u64
		};
		self.next = self
			.next
			.checked_add(base)
			.and_then(|next| next.checked_add(jitter))
			.ok_or(rivet_env::ClockError::Overflow)?;
		self.i = self
			.i
			.checked_add(1)
			.ok_or(rivet_env::ClockError::Overflow)?;
		Ok(Some(self.next))
	}

	pub fn tick_index(&self) -> usize {
		self.i
	}

	pub fn current_duration(&self) -> Result<u64, rivet_env::ClockError> {
		let exponent = u32::try_from(self.i.min(self.max_exponent))
			.map_err(|_| rivet_env::ClockError::Overflow)?;
		let base = 1u64
			.checked_shl(exponent)
			.and_then(|factor| factor.checked_mul(self.wait as u64))
			.ok_or(rivet_env::ClockError::Overflow)?;
		Ok(base)
	}

	pub async fn tick(&mut self) -> Result<bool, rivet_env::ClockError> {
		if self.max_retries.is_some_and(|max| self.i > max) {
			return Ok(false);
		}
		self.clock.sleep_until(self.next).await?;
		self.step()?;
		Ok(true)
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
					// Wait for the current window to reset rather than a full period from now,
					// then debit this caller's token from the refilled window.
					tokio::time::sleep_until(*reset_time).await;

					*requests_remaining = requests_limit.saturating_sub(1);
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

					let now = Instant::now();
					// Account for the actual wake time, then debit this acquire's token.
					*accum_drip += (now - *last_acquire).div_duration_f32(*drip_rate);
					*last_acquire = now;
					*requests_remaining =
						(*accum_drip as u64).min(*requests_limit).saturating_sub(1);
					*accum_drip = accum_drip.fract();
				}
			}
		}
	}
}
