//! Stall thresholds of one canary request. Each threshold is counted once, when the request
//! has been outstanding that long, whether it later answers, fails or times out.

use std::time::Duration;

/// A request outstanding at least `after` counts as a stall at this threshold.
#[derive(Debug, PartialEq, Eq)]
pub struct StallThreshold {
	pub after: Duration,
	pub label: &'static str,
}

/// Ascending, so a tracker passes them in order.
pub const STALL_THRESHOLDS: [StallThreshold; 3] = [
	StallThreshold {
		after: Duration::from_secs(1),
		label: "1s",
	},
	StallThreshold {
		after: Duration::from_secs(5),
		label: "5s",
	},
	StallThreshold {
		after: Duration::from_secs(10),
		label: "10s",
	},
];

/// Which thresholds one request has passed, so a timer firing and the final catch-up at
/// completion never count the same threshold twice.
#[derive(Debug, Default)]
pub struct StallTracker {
	passed: usize,
}

impl StallTracker {
	/// The lowest threshold this request has not passed yet.
	pub fn next(&self) -> Option<&'static StallThreshold> {
		STALL_THRESHOLDS.get(self.passed)
	}

	/// Marks every threshold at or below `elapsed` as passed and returns the ones newly
	/// passed by this call.
	pub fn pass(&mut self, elapsed: Duration) -> &'static [StallThreshold] {
		let first = self.passed;
		while let Some(threshold) = STALL_THRESHOLDS.get(self.passed) {
			if threshold.after > elapsed {
				break;
			}
			self.passed += 1;
		}
		&STALL_THRESHOLDS[first..self.passed]
	}
}
