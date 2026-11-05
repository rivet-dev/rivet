//! Near-capacity warning state shared by bounded resources that log when they fill.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Shortest time between two near-capacity warnings for one resource. A resource that sits
/// at its cap is visible on its usage and capacity gauges; the log only needs to show that
/// the condition recurs, and one line a minute does that without crowding out other logs.
pub const NEAR_CAPACITY_WARNING_INTERVAL_MS: u64 = 60_000;

/// Decides when a bounded resource logs that it is near its cap.
///
/// A resource enters the near band at 90% of its capacity and leaves it only below 80%, so
/// a value hovering at the threshold does not re-enter on every sample. On top of that, at
/// most one entry per [`NEAR_CAPACITY_WARNING_INTERVAL_MS`] warns; entries inside the
/// interval are counted and reported with the next warning. The state is atomic so a
/// shared admission path can observe it through `&self`; concurrent observers may race on a
/// transition, which only affects whether one diagnostic line is logged.
#[derive(Debug, Default)]
pub struct NearCapacityWarning {
	near: AtomicBool,
	/// Earliest clock reading at which the next entry may warn.
	next_warning_ms: AtomicU64,
	/// Entries the interval suppressed since the last warning.
	suppressed_episodes: AtomicU64,
}

/// An entry into the near band that the caller should log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NearCapacityEpisode {
	/// Earlier entries the rate limit suppressed since the previous warning.
	pub suppressed_episodes: u64,
}

impl NearCapacityWarning {
	pub const fn new() -> Self {
		Self {
			near: AtomicBool::new(false),
			next_warning_ms: AtomicU64::new(0),
			suppressed_episodes: AtomicU64::new(0),
		}
	}

	/// Records one usage sample. Returns an episode when the resource just entered the near
	/// band and the rate limit allows a warning. `now_ms` is read only on entry, so callers on
	/// hot paths pay for a clock read only when usage crosses into the band. A missing clock
	/// reading warns without advancing the rate limit.
	pub fn observe(
		&self,
		used: u64,
		capacity: u64,
		now_ms: impl FnOnce() -> Option<u64>,
	) -> Option<NearCapacityEpisode> {
		if !is_near(used, capacity) {
			if is_clear(used, capacity) && self.near.load(Ordering::Relaxed) {
				self.near.store(false, Ordering::Relaxed);
			}
			return None;
		}
		if self.near.load(Ordering::Relaxed) || self.near.swap(true, Ordering::Relaxed) {
			return None;
		}

		// The resource entered the band; the interval decides whether this entry warns.
		let Some(now_ms) = now_ms() else {
			return Some(NearCapacityEpisode {
				suppressed_episodes: self.suppressed_episodes.swap(0, Ordering::Relaxed),
			});
		};
		if now_ms < self.next_warning_ms.load(Ordering::Relaxed) {
			self.suppressed_episodes.fetch_add(1, Ordering::Relaxed);
			return None;
		}
		self.next_warning_ms.store(
			now_ms.saturating_add(NEAR_CAPACITY_WARNING_INTERVAL_MS),
			Ordering::Relaxed,
		);
		Some(NearCapacityEpisode {
			suppressed_episodes: self.suppressed_episodes.swap(0, Ordering::Relaxed),
		})
	}
}

/// Whether usage is at 90% of capacity or more. Empty resources are never near, so a zero
/// capacity with nothing used stays quiet.
fn is_near(used: u64, capacity: u64) -> bool {
	used > 0 && u128::from(used) * 10 >= u128::from(capacity) * 9
}

/// Whether usage fell below 80% of capacity, which ends a near-capacity episode.
fn is_clear(used: u64, capacity: u64) -> bool {
	u128::from(used) * 10 < u128::from(capacity) * 8
}
