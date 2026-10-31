//! Periodically returns glibc's freed heap to the kernel.
//!
//! glibc malloc keeps freed memory in per-thread arenas and only unmaps the top of each arena
//! chain, so a process whose live allocations are interleaved with large transient buffers (page
//! reads served to runners, blob reads for compaction) keeps its resident size at the high-water
//! mark for as long as it runs. `malloc_trim(0)` walks every arena and gives freed pages back with
//! `madvise`, which is what brings RSS back down after a burst. This is a stopgap until the engine
//! moves to an allocator that purges on its own.

use std::time::Duration;

use tokio::time::MissedTickBehavior;

/// Environment variable naming the trim interval in milliseconds. `0` disables the task.
const TRIM_INTERVAL_ENV: &str = "RIVET_MALLOC_TRIM_INTERVAL_MS";
const DEFAULT_TRIM_INTERVAL: Duration = Duration::from_secs(60);

pub(crate) fn trim_interval() -> Option<Duration> {
	match std::env::var(TRIM_INTERVAL_ENV) {
		Ok(value) => match value.trim().parse::<u64>() {
			Ok(0) => None,
			Ok(ms) => Some(Duration::from_millis(ms)),
			Err(err) => {
				tracing::warn!(
					?err,
					value,
					"invalid {TRIM_INTERVAL_ENV}, using the default malloc trim interval"
				);
				Some(DEFAULT_TRIM_INTERVAL)
			}
		},
		Err(_) => Some(DEFAULT_TRIM_INTERVAL),
	}
}

/// Runs `malloc_trim(0)` on a fixed schedule. The trim walks every arena and can take tens of
/// milliseconds on a large heap, so it runs on a blocking thread rather than a runtime worker.
pub(crate) async fn trim_malloc_periodically(interval: Duration) {
	let mut ticker = tokio::time::interval(interval);
	ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

	loop {
		ticker.tick().await;

		let started = std::time::Instant::now();
		let released = tokio::task::spawn_blocking(|| {
			// SAFETY: `malloc_trim` takes a padding size and touches only allocator-internal
			// state; it is safe to call from any thread at any time.
			unsafe { libc::malloc_trim(0) }
		})
		.await;
		match released {
			Ok(released) => tracing::debug!(
				released_memory = released != 0,
				elapsed_ms = started.elapsed().as_millis() as u64,
				"trimmed malloc heap"
			),
			Err(err) => tracing::warn!(?err, "malloc trim task failed"),
		}
	}
}
