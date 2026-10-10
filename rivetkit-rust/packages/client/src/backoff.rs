use std::time::Duration;

/// Configuration for reconnection exponential backoff and retry policies.
#[derive(Debug, Clone, PartialEq)]
pub struct BackoffConfig {
	/// Initial delay before the first reconnect attempt. Default: 1 second.
	pub initial_delay: Duration,
	/// Maximum delay between reconnect attempts. Default: 30 seconds.
	pub max_delay: Duration,
	/// Multiplier applied to the delay after each attempt. Default: 2.0.
	pub multiplier: f64,
	/// Maximum number of retries after the initial connection attempt.
	/// For example, `Some(3)` means: 1 initial attempt + up to 3 retries = 4 total attempts.
	/// `None` indicates infinite retries (default).
	pub max_retries: Option<usize>,
	/// Jitter factor between 0.0 and 1.0 to randomize delays and prevent thundering herds.
	/// Default: 0.0 (disabled).
	pub jitter_factor: f64,
}

impl Default for BackoffConfig {
	fn default() -> Self {
		Self {
			initial_delay: Duration::from_secs(1),
			max_delay: Duration::from_secs(30),
			multiplier: 2.0,
			max_retries: None,
			jitter_factor: 0.0,
		}
	}
}

impl BackoffConfig {
	/// Normalizes the configuration to ensure invariants:
	/// - Ensures `max_delay >= initial_delay`
	/// - Clamps `multiplier` to at least `1.0` (or `1.0` if `NaN`)
	/// - Clamps `jitter_factor` to `0.0..=1.0` (or `0.0` if `NaN`)
	pub fn normalize(&mut self) {
		if self.max_delay < self.initial_delay {
			self.max_delay = self.initial_delay;
		}
		if self.multiplier.is_nan() || self.multiplier < 1.0 {
			self.multiplier = 1.0;
		}
		if self.jitter_factor.is_nan() {
			self.jitter_factor = 0.0;
		} else {
			self.jitter_factor = self.jitter_factor.clamp(0.0, 1.0);
		}
	}

	/// Creates a new `BackoffConfig` with the given initial and max delays.
	///
	/// If `max_delay < initial_delay`, `max_delay` is raised to `initial_delay`.
	pub fn new(initial_delay: Duration, max_delay: Duration) -> Self {
		let max_delay = max_delay.max(initial_delay);
		Self {
			initial_delay,
			max_delay,
			..Default::default()
		}
	}

	/// Sets the initial delay before the first retry attempt.
	///
	/// If `max_delay < delay`, `max_delay` is automatically raised to match `delay`.
	pub fn initial_delay(mut self, delay: Duration) -> Self {
		self.initial_delay = delay;
		if self.max_delay < delay {
			self.max_delay = delay;
		}
		self
	}

	/// Sets the maximum ceiling delay between retry attempts.
	///
	/// Clamped to be at least `initial_delay`.
	pub fn max_delay(mut self, delay: Duration) -> Self {
		self.max_delay = delay.max(self.initial_delay);
		self
	}

	/// Sets the exponential multiplier.
	///
	/// Values less than `1.0` are silently clamped to `1.0` (ensuring delays never shrink across retries).
	pub fn multiplier(mut self, multiplier: f64) -> Self {
		self.multiplier = if multiplier.is_nan() {
			1.0
		} else {
			multiplier.max(1.0)
		};
		self
	}

	/// Sets the maximum number of retries after the initial connection attempt.
	/// For example, `Some(3)` allows 1 initial attempt + 3 retries = 4 total.
	/// Pass `None` for indefinite retries.
	pub fn max_retries(mut self, max_retries: Option<usize>) -> Self {
		self.max_retries = max_retries;
		self
	}

	/// Enables or disables standard jitter (uses 20% jitter if enabled).
	pub fn jitter(mut self, enabled: bool) -> Self {
		self.jitter_factor = if enabled { 0.2 } else { 0.0 };
		self
	}

	/// Sets a custom jitter factor.
	///
	/// Values outside `[0.0, 1.0]` are silently clamped into `0.0..=1.0` (e.g. values `< 0.0`
	/// become `0.0`, and values `> 1.0` become `1.0`).
	pub fn jitter_factor(mut self, factor: f64) -> Self {
		self.jitter_factor = if factor.is_nan() {
			0.0
		} else {
			factor.clamp(0.0, 1.0)
		};
		self
	}
}

/// Exponential backoff calculator with optional jitter and attempt limits.
#[derive(Debug, Clone)]
pub struct Backoff {
	config: BackoffConfig,
	delay: Duration,
	attempt: usize,
}

impl Backoff {
	/// Creates a backoff with the given initial and max delay using default settings.
	pub fn new(initial: Duration, max_delay: Duration) -> Self {
		Self::from_config(BackoffConfig::new(initial, max_delay))
	}

	/// Creates a backoff from a `BackoffConfig`, normalizing any invalid fields.
	pub fn from_config(mut config: BackoffConfig) -> Self {
		config.normalize();
		let delay = config.initial_delay;
		Self {
			config,
			delay,
			attempt: 0,
		}
	}

	/// Returns a reference to the active `BackoffConfig`.
	pub fn config(&self) -> &BackoffConfig {
		&self.config
	}

	/// Returns the number of retry attempts made so far.
	pub fn attempt(&self) -> usize {
		self.attempt
	}

	/// Returns whether additional retries are permitted under the configured `max_retries`.
	pub fn can_retry(&self) -> bool {
		match self.config.max_retries {
			Some(max) => self.attempt < max,
			None => true,
		}
	}

	/// Returns the current base delay for this attempt before stepping.
	pub fn delay(&self) -> Duration {
		self.delay
	}

	/// Advances the backoff state by one attempt and returns the computed sleep duration.
	/// Returns `None` if `can_retry()` is false.
	///
	/// **Note:** The internal attempt counter and delay are advanced immediately,
	/// before the caller performs the actual sleep. If using `step()` directly
	/// (rather than `tick()`), be aware that cancellation after `step()` but
	/// before sleeping will still have advanced the backoff state.
	pub fn step(&mut self) -> Option<Duration> {
		if !self.can_retry() {
			return None;
		}

		let base = self.delay;
		self.attempt += 1;

		let next_secs =
			(base.as_secs_f64() * self.config.multiplier).min(self.config.max_delay.as_secs_f64());
		self.delay = Duration::from_secs_f64(next_secs);

		let sleep_duration = if self.config.jitter_factor > 0.0 {
			let jitter_offset = (rand::random::<f64>() * 2.0 - 1.0) * self.config.jitter_factor;
			let factor = (1.0 + jitter_offset).max(0.0);
			let max_secs = self.config.max_delay.as_secs_f64();
			let jittered = (base.as_secs_f64() * factor).clamp(0.0, max_secs);
			Duration::from_secs_f64(jittered)
		} else {
			base
		};

		Some(sleep_duration)
	}

	/// Waits for the backoff delay. Returns `true` if waited, or `false` if max retries exceeded.
	pub async fn tick(&mut self) -> bool {
		let Some(duration) = self.step() else {
			return false;
		};
		tokio::time::sleep(duration).await;
		true
	}

	/// Resets the backoff state back to the initial delay and attempt 0.
	pub fn reset(&mut self) {
		self.delay = self.config.initial_delay;
		self.attempt = 0;
	}
}
