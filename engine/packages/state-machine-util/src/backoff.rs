use rivet_env::rand::LocalRng;
use serde::{Deserialize, Serialize};

/// Per-resource retry history; the parent stores the absolute retry gate separately.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Backoff {
	/// Consecutive unknown outcomes since the last confirmed success.
	attempts: u32,

	/// Initial delay before jitter, clamped to the cap.
	base_ms: u64,

	/// Maximum delay including jitter; arithmetic saturates at the clock domain's end.
	cap_ms: u64,
}

impl Backoff {
	/// Creates an unused sequence; a zero cap deliberately permits immediate retry.
	pub fn new(base_ms: u64, cap_ms: u64) -> Self {
		Self {
			attempts: 0,
			base_ms,
			cap_ms,
		}
	}

	/// Returns an absolute deadline using capped exponential delay plus bounded positive jitter.
	pub fn next(&mut self, now_boot_ms: u64, rng: &mut LocalRng) -> u64 {
		let factor = 1u64.checked_shl(self.attempts).unwrap_or(u64::MAX);
		let delay = self.base_ms.saturating_mul(factor).min(self.cap_ms);
		let jitter = rng.gen_range(0..(u128::from(delay.min(self.cap_ms - delay)) + 1)) as u64;
		self.attempts = self.attempts.saturating_add(1);
		now_boot_ms.saturating_add(delay).saturating_add(jitter)
	}

	/// Successful progress starts the next failure at the base delay.
	pub fn reset(&mut self) {
		self.attempts = 0;
	}
}
