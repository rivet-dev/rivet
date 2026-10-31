//! Disk segment lengths and checkpoint coverage determine which files may be collected.

use super::{LogShardState, fail};
use crate::log_shard_machine::{Effects, effect::LogShardEffect, types::LogShardTimer};
use depot_types::VolumeLogKey;
use rivet_data::generated::depot_log_shard_v1::{
	LogShardCheckpoint, StateFrame, TabletGrantFrame, TabletReplicaSetFrame,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CheckpointCycle {
	#[default]
	/// No checkpoint write is pending or outstanding.
	Idle,

	/// A checkpoint is needed and awaits capture of its fixed disk image.
	Wanted { reason: CheckpointReason },

	/// The selected checkpoint image is outstanding; only its matching reply installs coverage.
	Writing {
		last_included_log_seq: u64,
		/// Bytes included in this captured image; newer writes remain charged on completion.
		covered_bytes: u64,
	},
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CheckpointReason {
	BytesSinceCheckpoint,
	GarbageCollection,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiskSegmentUsage {
	pub last_log_seq: u64,
	pub len_bytes: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiskSegments {
	/// Actual installed base and delta bytes, reported after disk work and on recovery.
	pub checkpoint_bytes: u64,
	/// Running upper bound of complete checkpoint metadata, adjusted only for changed frames.
	pub checkpoint_metadata_bytes: u64,
	/// Segment files by first log sequence. Changed only through the methods below,
	/// which keep `segment_bytes` in step.
	by_first_log_seq: BTreeMap<u64, DiskSegmentUsage>,
	/// The sum of `len_bytes` over `by_first_log_seq`.
	segment_bytes: u64,
	pub checkpoint: CheckpointCycle,
	pub checkpoint_last_included_log_seq: u64,
	/// Framed batch bytes written above the installed checkpoint boundary.
	pub bytes_since_checkpoint: u64,
	/// Volume logs whose disk replica state or retirement changed since the last
	/// captured checkpoint. Capture takes the set into the delta it emits, so changes
	/// installed while that delta is written stay here for the next one. A failed
	/// checkpoint write fails the shard, so a captured set is never needed again.
	pub changed_since_checkpoint: BTreeSet<VolumeLogKey>,

	/// Files whose unlink is outstanding. They remain budgeted until completion.
	pub removing: Vec<u64>,

	/// Quarantine is retained for operator inspection and remains charged to disk.
	pub quarantined_bytes: u64,

	/// The size each preallocated segment and spare is zero-filled to, or zero when the
	/// executor does not preallocate. Fixed by the configuration at construction.
	pub preallocated_segment_bytes: u64,
}

impl DiskSegments {
	/// Disk bytes the shard's files may occupy. Sealed segments are truncated to their
	/// logical length at a roll, but the active segment may be allocated up to
	/// `preallocated_segment_bytes` and one spare of that size may exist, so both are charged
	/// whether or not the executor has allocated them yet.
	pub fn disk_bytes(&self) -> u64 {
		let base = self.checkpoint_base_bound();
		// Keep room for an installed base and concurrent checkpoint output. The executor
		// compacts directly when another delta would consume this reserve.
		self.non_checkpoint_bytes()
			.saturating_add(self.checkpoint_bytes.max(base))
			.saturating_add(base.saturating_mul(3))
	}

	/// Maximum base size, including version headers, list counts and an integrity envelope.
	pub(crate) fn checkpoint_base_bound(&self) -> u64 {
		self.checkpoint_metadata_bytes.saturating_add(128)
	}

	/// Metadata can grow by at most twice a pending frame's length. The checkpoint budget
	/// keeps up to four copies, so reserve eight frame lengths plus the segment itself.
	pub(crate) fn pending_disk_bytes(bytes: u64) -> u64 {
		bytes.saturating_mul(9)
	}

	fn non_checkpoint_bytes(&self) -> u64 {
		let preallocated = self.preallocated_segment_bytes;
		let active_len = self
			.by_first_log_seq
			.last_key_value()
			.map_or(0, |(_, usage)| usage.len_bytes);
		let unused_allocation = preallocated
			.saturating_sub(active_len)
			.saturating_add(preallocated);
		self.quarantined_bytes
			.saturating_add(self.segment_bytes)
			.saturating_add(unused_allocation)
	}

	pub fn segments(&self) -> &BTreeMap<u64, DiskSegmentUsage> {
		&self.by_first_log_seq
	}

	/// Records a segment's usage, replacing the active segment's earlier length.
	pub(crate) fn insert(&mut self, first_log_seq: u64, usage: DiskSegmentUsage) {
		self.segment_bytes = self.segment_bytes.saturating_add(usage.len_bytes);
		if let Some(old) = self.by_first_log_seq.insert(first_log_seq, usage) {
			self.segment_bytes = self.segment_bytes.saturating_sub(old.len_bytes);
		}
	}

	pub(crate) fn remove(&mut self, first_log_seq: u64) {
		if let Some(old) = self.by_first_log_seq.remove(&first_log_seq) {
			self.segment_bytes = self.segment_bytes.saturating_sub(old.len_bytes);
		}
	}

	/// Installs the segments found by recovery.
	pub(crate) fn replace(&mut self, segments: BTreeMap<u64, DiskSegmentUsage>) {
		self.segment_bytes = segments.values().map(|s| s.len_bytes).sum();
		self.by_first_log_seq = segments;
	}

	/// Checks the running total against a full sum, for debug validation.
	pub(crate) fn validate(&self) -> anyhow::Result<()> {
		anyhow::ensure!(
			self.segment_bytes
				== self
					.by_first_log_seq
					.values()
					.map(|s| s.len_bytes)
					.sum::<u64>(),
			"segment byte total drifted from its segments"
		);
		Ok(())
	}
}

/// Collects only inactive segments with no retained entries and all state, forget and
/// grant frames covered by the installed checkpoint. Pending writes cannot justify GC.
pub(crate) fn dispatch_maintenance(state: &mut LogShardState, effects: &mut Effects) {
	if !matches!(
		state.disk_segments.checkpoint,
		CheckpointCycle::Idle | CheckpointCycle::Wanted { .. }
	) || !state.disk_segments.removing.is_empty()
	{
		return;
	}

	// Retained entries pin their segments; the active file is never collected. A
	// segment holds the batches from its first log sequence up to the next segment's,
	// and pending releasing frames retain their own batch pins until the sync completes.
	let disk_segments = &state.disk_segments.by_first_log_seq;
	let active = disk_segments.last_key_value().map(|(first, _)| *first);
	let retained: std::collections::BTreeSet<u64> = state
		.memory_batches
		.pending()
		.flat_map(|batch| batch.retained_batch_log_seqs.iter().copied())
		.collect();
	let pinned = |first: u64| {
		let end = disk_segments
			.range((std::ops::Bound::Excluded(first), std::ops::Bound::Unbounded))
			.next()
			.map(|(next, _)| *next);
		let retained_here = match end {
			Some(end) => retained.range(first..end).next().is_some(),
			None => retained.range(first..).next().is_some(),
		};
		retained_here || state.segment_pins.any_in(first, end)
	};

	let dead: Vec<_> = disk_segments
		.iter()
		.filter(|(first, usage)| {
			Some(**first) != active
				&& usage.last_log_seq <= state.disk_segments.checkpoint_last_included_log_seq
				&& !pinned(**first)
		})
		.map(|(first, _)| *first)
		.collect();

	// Charge selected files to disk until the executor confirms their removal.
	if !dead.is_empty() {
		state.disk_segments.removing = dead.clone();
		effects.arm_timer_after(
			LogShardTimer::RemoveSegments,
			state.config.remove_segments_timeout_ms,
		);
		effects.execute(LogShardEffect::RemoveSegmentsRequest {
			first_log_seqs: dead,
		});
	}

	// Request a checkpoint for accumulated bytes or to cover otherwise dead segments.
	if matches!(state.disk_segments.checkpoint, CheckpointCycle::Idle) {
		let reason = if state.disk_segments.bytes_since_checkpoint >= state.config.checkpoint_bytes
		{
			Some(CheckpointReason::BytesSinceCheckpoint)
		} else if disk_segments.iter().any(|(first, s)| {
			Some(*first) != active
				&& s.last_log_seq > state.disk_segments.checkpoint_last_included_log_seq
				&& !pinned(*first)
		}) {
			Some(CheckpointReason::GarbageCollection)
		} else {
			None
		};

		if let Some(reason) = reason {
			state.disk_segments.checkpoint = CheckpointCycle::Wanted { reason };
		}
	}

	// Capture a fixed disk boundary so concurrent writes cannot change this checkpoint.
	if matches!(
		state.disk_segments.checkpoint,
		CheckpointCycle::Wanted { .. }
	) {
		let checkpoint = state.checkpoint_delta();
		state.disk_segments.checkpoint = CheckpointCycle::Writing {
			last_included_log_seq: checkpoint.last_included_log_seq,
			covered_bytes: state.disk_segments.bytes_since_checkpoint,
		};
		effects.arm_timer_after(
			LogShardTimer::WriteCheckpoint {
				last_included_log_seq: checkpoint.last_included_log_seq,
			},
			state.config.checkpoint_timeout_ms,
		);
		let pending = state
			.memory_batches
			.writing_to_disk
			.as_ref()
			.map_or(0, |writing| writing.pending.sizing.total());
		effects.execute(LogShardEffect::WriteCheckpointRequest {
			max_checkpoint_bytes: state
				.config
				.max_disk_bytes
				.saturating_sub(state.disk_segments.non_checkpoint_bytes())
				.saturating_sub(pending),
			base_bytes: state.disk_segments.checkpoint_base_bound(),
			checkpoint,
		});
	}
}

impl LogShardState {
	/// The batch boundary a checkpoint captured now covers: every batch whose frames
	/// are installed into disk state.
	fn checkpoint_boundary(&self) -> u64 {
		self.memory_batches
			.writing_to_disk
			.as_ref()
			.map_or(self.memory_batches.last_log_seq, |b| b.first_log_seq - 1)
	}

	fn checkpoint_tablet_replica_sets(&self) -> Vec<TabletReplicaSetFrame> {
		self.tablet_replica_sets
			.iter()
			.map(|(id, set)| TabletReplicaSetFrame {
				tablet_id: *id,
				replica_set: set.clone(),
			})
			.collect()
	}

	fn checkpoint_tablet_grants(&self) -> Vec<TabletGrantFrame> {
		self.tablet_grants
			.iter()
			.map(|(tablet_id, epoch)| TabletGrantFrame {
				tablet_id: *tablet_id,
				tablet_grant_epoch: *epoch,
			})
			.collect()
	}

	/// Captures a delta over the installed checkpoint chain: the synced state and
	/// retirement of every volume log changed since the last capture, the complete
	/// tablet grants and replica sets, and the completed batch boundary. Its cost
	/// grows with the changed volume logs, not the loaded ones. Takes the changed set.
	pub fn checkpoint_delta(&mut self) -> LogShardCheckpoint {
		let changed = std::mem::take(&mut self.disk_segments.changed_since_checkpoint);
		let mut replicas = Vec::new();
		let mut retired_volumes = Vec::new();
		for key in &changed {
			if let Some(state) = self
				.loaded_volume_replicas
				.get(key)
				.and_then(|r| r.disk_state.clone())
			{
				replicas.push(StateFrame {
					volume_log_key: key.clone().into(),
					state,
				});
			}
			if let Some(frame) =
				super::retired_volumes::checkpoint_frame(&self.retired_volumes, key)
			{
				retired_volumes.push(frame);
			}
		}
		LogShardCheckpoint {
			retired_volumes,
			tablet_replica_sets: self.checkpoint_tablet_replica_sets(),
			tablet_grants: self.checkpoint_tablet_grants(),
			last_included_log_seq: self.checkpoint_boundary(),
			replicas,
			previous_log_seq: Some(self.disk_segments.checkpoint_last_included_log_seq),
			changed_volume_log_keys: changed.into_iter().map(|key| key.into()).collect(),
		}
	}

	/// Captures a complete base image of synced replica states and the completed batch
	/// boundary, with no entries. It does not consume the changed set. Recovery replays
	/// later batches to bound startup work. Checkpoint coverage permits collection of
	/// old state, forget and grant frames, independently of the cold store.
	pub fn checkpoint_image(&self) -> LogShardCheckpoint {
		LogShardCheckpoint {
			retired_volumes: super::retired_volumes::checkpoint(&self.retired_volumes),
			tablet_replica_sets: self.checkpoint_tablet_replica_sets(),
			tablet_grants: self.checkpoint_tablet_grants(),
			last_included_log_seq: self.checkpoint_boundary(),
			replicas: self
				.loaded_volume_replicas
				.values()
				.filter_map(|r| {
					r.disk_state.clone().map(|state| StateFrame {
						volume_log_key: r.volume_log_key.clone().into(),
						state,
					})
				})
				.collect(),
			previous_log_seq: None,
			changed_volume_log_keys: Vec::new(),
		}
	}
}

/// Installs checkpoint coverage without removing replicas needed by pending batches.
pub(crate) fn checkpoint_written(
	state: &mut LogShardState,
	last_included_log_seq: u64,
	checkpoint_bytes: u64,
	result: Result<(), super::super::types::WriteToDiskError>,
	effects: &mut Effects,
) {
	if let CheckpointCycle::Writing {
		last_included_log_seq: expected,
		covered_bytes,
	} = state.disk_segments.checkpoint.clone()
		&& expected == last_included_log_seq
	{
		effects.cancel_timer(LogShardTimer::WriteCheckpoint {
			last_included_log_seq,
		});
		match result {
			Ok(()) => {
				state.disk_segments.checkpoint_bytes = checkpoint_bytes;
				state.disk_segments.checkpoint_last_included_log_seq = last_included_log_seq;
				state.disk_segments.bytes_since_checkpoint -= covered_bytes;

				state.disk_segments.checkpoint = CheckpointCycle::Idle;
			}

			Err(reason) => {
				tracing::error!(last_included_log_seq, error = %reason, "log shard disk write operation failed");
				fail(state, reason.to_string(), effects);
			}
		}
	}
}

/// Removes accounting only after the executor confirms the selected files were removed.
pub(crate) fn segments_removed(
	state: &mut LogShardState,
	result: Result<(), super::super::types::WriteToDiskError>,
	effects: &mut Effects,
) {
	if !state.disk_segments.removing.is_empty() {
		effects.cancel_timer(LogShardTimer::RemoveSegments);
		match result {
			Ok(()) => {
				for first in std::mem::take(&mut state.disk_segments.removing) {
					state.disk_segments.remove(first);
				}
				super::status_notices::observe_disk_pressure(state);
			}
			Err(e) => fail(state, e.to_string(), effects),
		}
	}
}


