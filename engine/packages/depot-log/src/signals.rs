//! Signal events the log shard emits under the `depot::signal` tracing target. Each one
//! reports a value an operator or a simulation oracle judges the shard by, such as a refusal
//! and its retry hint or how long the oldest disk segment has been held. Messages and field
//! names are stable, because traces are matched on them.

use crate::log_shard_machine::snapshot::LogShardSnapshot;
use rivet_data::generated::depot_storage_worker_protocol_v1::RejectResponse;
use std::collections::VecDeque;

/// The tracing target of every signal event.
pub(crate) const TARGET: &str = "depot::signal";

/// Most sync samples kept. Past it every other sample is dropped, so the samples still span
/// the whole pinned range at a coarser resolution.
const MAX_SYNC_SAMPLES: usize = 4096;

/// Least time between two sync samples.
const SYNC_SAMPLE_SPACING_MS: u64 = 1_000;

/// Least time between two pin age events while only the age or the oldest pin moved.
/// A change in the number of segments is reported at once.
const PIN_AGE_EVENT_SPACING_MS: u64 = 10_000;

/// Reports one refusal the shard answered, with its retry hint. Only `Overloaded` carries
/// one; every other refusal reports `has_retry_after = false`.
pub(crate) fn request_refused(reason: &RejectResponse, label: &'static str) {
	let retry_after_ms = if let RejectResponse::Overloaded(overloaded) = reason {
		Some(overloaded.retry_after_ms)
	} else {
		None
	};
	tracing::debug!(
		target: TARGET,
		layer = "log_shard",
		reason = label,
		retry_after_ms = retry_after_ms.unwrap_or(0),
		has_retry_after = retry_after_ms.is_some(),
		"request refused"
	);
}

/// Tracks how long the oldest pinned disk segment has been held. The shard does not
/// timestamp batches, so this samples the synced batch sequence against the boot clock as
/// snapshots change and dates a batch by the first sample at or above it. A batch written
/// before this process started is dated by the first sample, so its age is a lower bound.
#[derive(Default)]
pub(crate) struct SegmentPinAge {
	/// `(synced batch sequence, boot ms)` in rising order of both.
	samples: VecDeque<(u64, u64)>,
	last_event: Option<PinEvent>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PinEvent {
	oldest_pinned_batch_log_seq: Option<u64>,
	segments: u32,
	at_boot_ms: u64,
}

impl SegmentPinAge {
	/// Folds in one published snapshot and emits `segment pin age` when it moved.
	pub(crate) fn observe(&mut self, snapshot: &LogShardSnapshot, now_boot_ms: u64) {
		let age_ms = self.observe_positions(
			snapshot.synced_log_seq,
			snapshot.oldest_pinned_batch_log_seq,
			snapshot.segments,
			now_boot_ms,
		);
		crate::metrics::observe_segment_pin_age(age_ms);
	}

	/// Folds in the shard's positions and returns the oldest pin's age in milliseconds.
	fn observe_positions(
		&mut self,
		synced_log_seq: u64,
		oldest: Option<u64>,
		segments: u32,
		now_boot_ms: u64,
	) -> u64 {
		self.sample(synced_log_seq, now_boot_ms);
		if let Some(oldest) = oldest {
			// Keep one sample at or below the oldest pin so it can still be dated.
			while self.samples.len() >= 2 && self.samples[1].0 <= oldest {
				self.samples.pop_front();
			}
		}
		let age_ms = oldest.map_or(0, |oldest| {
			let written_at = self
				.samples
				.iter()
				.find(|(log_seq, _)| *log_seq >= oldest)
				.or(self.samples.back())
				.map_or(now_boot_ms, |(_, at)| *at);
			now_boot_ms.saturating_sub(written_at)
		});

		let due = match self.last_event {
			None => true,
			Some(last) => {
				last.segments != segments
					|| ((last.oldest_pinned_batch_log_seq != oldest || age_ms > 0)
						&& now_boot_ms.saturating_sub(last.at_boot_ms) >= PIN_AGE_EVENT_SPACING_MS)
			}
		};
		if !due {
			return age_ms;
		}
		self.last_event = Some(PinEvent {
			oldest_pinned_batch_log_seq: oldest,
			segments,
			at_boot_ms: now_boot_ms,
		});
		tracing::debug!(
			target: TARGET,
			age_ms,
			oldest_pinned_batch_log_seq = oldest.unwrap_or(0),
			pinned = oldest.is_some(),
			segments = segments,
			"segment pin age"
		);
		age_ms
	}

	fn sample(&mut self, synced_log_seq: u64, now_boot_ms: u64) {
		let due = self.samples.back().is_none_or(|(log_seq, at)| {
			synced_log_seq > *log_seq && now_boot_ms.saturating_sub(*at) >= SYNC_SAMPLE_SPACING_MS
		});
		if !due {
			return;
		}
		if self.samples.len() >= MAX_SYNC_SAMPLES {
			let mut index = 0;
			self.samples.retain(|_| {
				index += 1;
				index % 2 == 1
			});
		}
		self.samples.push_back((synced_log_seq, now_boot_ms));
	}
}


