//! Authoritative shard state, grouped by the independent replica, batch and segment loops.
//!
//! Maintainer grant admission writes a higher tablet
//! epoch to disk as a TabletGrantFrame before replying, fencing older epochs
//! after restart too. A new maintainer first lists a quorum, intersecting every old
//! quorum. This per-tablet fence belongs to the shard, not an individual volume log.

use super::types::LogShardConfig;
use super::{Effects, effect::LogShardEffect, types::LogShardTimer};
use disk_segments::CheckpointCycle;
use disk_segments::DiskSegments;
use memory_batches::MemoryBatch;
use memory_batches::MemoryBatches;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod admission;
pub mod disk_segments;
pub mod loaded_volume_replica;
pub mod manifest_wait;
pub mod memory_batches;
pub mod payloads;
pub mod reads;
pub mod recovery;
pub mod replica_hashes;
pub mod replica_table;
pub mod retired_volumes;
pub mod retirement;
pub mod segment_pins;
pub mod status_notices;
pub mod tablet_grants;
pub mod tablet_replica_sets;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogShardPhase {
	/// Admission is closed until scanning and any required repair marking complete.
	Recovering(RecoveryStep),

	/// Recovered replicas may admit commands subject to the shard budgets.
	Serving,

	/// New requests are rejected while accepted batches and maintenance finish.
	Draining,

	/// Draining completed; the owner may release the runner and its tasks.
	Stopped,

	/// An uncertain disk write outcome requires restart before serving again.
	Failed { reason: String },
}

/// Startup scans first; corruption must be durably marked repairing before serving.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecoveryStep {
	/// The startup scan is outstanding and no replica state has been published.
	Scanning,

	/// Fresh repairing incarnations must reach disk before the shard can serve.
	MarkingRepairing,
}

/// Owns the loaded log replicas and their shared disk writes on one storage worker.
///
/// 1. Check each command against its loaded log replica's client fence, status and
///    entry sequence. Accepted changes advance the replica's in-memory state.
/// 2. Add the resulting frames to a shared batch. The executor writes the batch to
///    a segment and syncs it; only then does the shard advance disk replica
///    state and acknowledge. The volume client commits after a quorum acknowledges.
/// 3. Track segment lengths and which segments contain retained entries. Entries
///    remain in segments after acknowledgment; trimming releases entries, and a
///    segment stays while even one entry in it is still needed.
/// 4. The shard checkpoints disk replica state information at a fixed batch boundary.
///    Once that checkpoint covers the old state and no retained entries need an
///    inactive segment, the shard can remove it. Separately, the tablet maintainer
///    writes committed volume data to cold storage and requests trimming; it does
///    not write the shard's local checkpoint.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogShardState {
	/// Shared topology identity of the storage worker owning this shard.
	pub(crate) worker_id: rivet_topology::worker::WorkerId,
	/// FORMAT identity installed by recovery; retained across every intact restart.
	pub(crate) disk_incarnation: [u8; 16],
	pub(crate) config: LogShardConfig,
	/// The batch limit recorded in FORMAT, installed by recovery. See
	/// [`LogShardState::read_batch_limit`].
	pub(crate) stored_max_batch_bytes: u32,

	/// Ordered supervisor view used for client admission and removed-member retirement.
	pub(crate) tablet_manifest: super::types::TabletManifestView,
	/// `tablet_manifest`'s tablets by hash range, rebuilt with every installed view.
	pub(crate) tablet_ranges: depot_types::hash_range_index::HashRangeIndex<[u8; 16]>,
	/// Client requests whose replica-set version is ahead of the local routing view.
	pub(crate) manifest_waits: manifest_wait::ManifestWaits,
	/// Highest synced grant per tablet. Pending frames supply the speculative fence.
	pub(crate) tablet_grants: BTreeMap<[u8; 16], u64>,
	pub(crate) retired_volumes: retired_volumes::RetiredVolumes,
	/// Highest synced replica set per tablet; the pending frame fences before sync.
	pub(crate) tablet_replica_sets:
		BTreeMap<[u8; 16], rivet_data::generated::shared_depot_types_v1::TabletReplicaSet>,
	pub(crate) phase: LogShardPhase,

	/// Replicas loaded on this worker, until their forget frame reaches disk.
	/// Ordinary commands for an absent replica answer `VolumeNotLoaded`.
	/// The client reopens through the maintainer's `VolumeOpen`, which supplies the
	/// volume row's cold position and fence epoch when creating replicas.
	/// The shard cannot infer that state
	/// or load an unknown volume on its own. Shard startup recovery is separate.
	pub(crate) loaded_volume_replicas: replica_table::ReplicaTable,
	/// The keys of `loaded_volume_replicas` in partition-key hash order, kept in step with it.
	pub(crate) replica_hashes: replica_hashes::ReplicaHashes,
	/// Located entries of `loaded_volume_replicas` by batch, kept in step with it.
	pub(crate) segment_pins: segment_pins::SegmentPins,
	/// Which replicas hold payloads in memory and how many bytes, kept in step with
	/// `loaded_volume_replicas`.
	pub(crate) payloads: payloads::Payloads,
	/// Outstanding disk reads and the reply byte bound each one reserved, bounded by the
	/// same request budget as writes.
	pub(crate) reads: reads::OutstandingReads,
	pub(crate) memory_batches: MemoryBatches,
	pub(crate) disk_segments: DiskSegments,
	/// Whether a loaded replica may still need a forget frame for the current routing view.
	pub(crate) retirement_scan: retirement::RetirementScan,
	/// A fence was synced for a tablet the routing view already names as retired, so its
	/// permanent retirement grant must be queued even though the scan settled.
	pub(crate) parent_retirement_due: bool,
	/// Whether a manifest tablet may still need a replica-set fence queued.
	pub(crate) replica_set_scan: tablet_replica_sets::ReplicaSetScan,
	/// Replicas whose synced state change still needs a notice to their maintainer.
	pub(crate) status_notices: status_notices::StatusNotices,
	/// How long the latest submission waited in the inbox, published against
	/// `max_admission_wait_ms` as the `AdmissionWait` resource. Observation only.
	pub(crate) last_admission_wait_ms: u64,
}

impl LogShardState {
	/// Validates resource limits before an owner opens or locks the storage directory.
	pub fn validate_config(config: &LogShardConfig) -> anyhow::Result<()> {
		anyhow::ensure!(
			config.inbox_capacity > 0
				&& config.max_memory_requests > 0
				&& config.max_memory_bytes
					>= 2 * (u64::from(config.max_batch_bytes)
						+ depot_types::log_shard::BATCH_FRAMING_BYTES as u64)
				&& config.max_batch_bytes >= 16
				&& config.max_write_bytes
					>= u64::from(config.max_batch_bytes)
						+ depot_types::log_shard::BATCH_FRAMING_BYTES as u64
				&& config.max_write_bytes <= config.segment_bytes
				&& config.max_disk_bytes > 0
				&& (1..100).contains(&config.write_to_cold_at_disk_percent)
				&& config.max_loaded_volumes > 0
				&& config.write_to_disk_timeout_ms > 0
				&& config.recovery_timeout_ms > 0
				&& config.manifest_refresh_timeout_ms > 0
				&& config.checkpoint_timeout_ms > 0
				&& config.remove_segments_timeout_ms > 0
				&& config.segment_bytes > 0
				&& config.checkpoint_bytes > 0
				&& config.max_forget_batch_volumes > 0
				&& config.max_retained_payload_bytes > 0
				&& config.release_idle_payloads_after_ms > 0
				&& config.max_disk_reads > 0
				&& config.disk_read_concurrency > 0
				&& config.max_admission_wait_ms > 0
				&& config.max_append_admission_wait_ms >= config.max_admission_wait_ms
				&& config.overload_retry_after_ms > 0,
			"invalid shard limits"
		);
		Ok(())
	}

	/// Starts scanning before any command may be admitted.
	pub fn new(
		config: LogShardConfig,
		worker_id: rivet_topology::worker::WorkerId,
	) -> anyhow::Result<Self> {
		Self::validate_config(&config)?;
		let mut disk_segments = DiskSegments::default();
		if config.preallocate_segments {
			disk_segments.preallocated_segment_bytes = config.segment_bytes;
		}
		Ok(Self {
			worker_id,
			disk_incarnation: [0; 16],
			config,
			stored_max_batch_bytes: 0,
			status_notices: Default::default(),
			tablet_manifest: super::types::TabletManifestView::default(),
			tablet_ranges: Default::default(),
			manifest_waits: Default::default(),
			tablet_grants: BTreeMap::new(),
			retired_volumes: Default::default(),
			tablet_replica_sets: BTreeMap::new(),
			phase: LogShardPhase::Recovering(RecoveryStep::Scanning),
			loaded_volume_replicas: replica_table::ReplicaTable::new(),
			replica_hashes: Default::default(),
			segment_pins: Default::default(),
			payloads: Default::default(),
			reads: Default::default(),
			memory_batches: MemoryBatches::default(),
			disk_segments,
			retirement_scan: retirement::RetirementScan::Pending { after: None },
			parent_retirement_due: false,
			replica_set_scan: tablet_replica_sets::ReplicaSetScan::Pending,
			last_admission_wait_ms: 0,
		})
	}
}

impl LogShardState {
	/// The largest batch a disk read may meet: the live limit, or the larger limit an earlier
	/// writer of this folder used. Read pages and scan budgets use it so a stored entry above a
	/// lowered limit stays readable.
	pub(crate) fn read_batch_limit(&self) -> u32 {
		self.config.max_batch_bytes.max(self.stored_max_batch_bytes)
	}

	/// Checks the incrementally kept segment pins and batch sizes against a full
	/// recomputation. Debug builds only: the recomputation scans every retained entry
	/// and re-encodes every pending frame.
	#[cfg(debug_assertions)]
	pub(crate) fn validate_incremental(&self) -> anyhow::Result<()> {
		anyhow::ensure!(
			self.segment_pins == segment_pins::SegmentPins::recompute(&self.loaded_volume_replicas),
			"segment pins drifted from the loaded replicas"
		);
		anyhow::ensure!(
			self.replica_hashes
				== replica_hashes::ReplicaHashes::recovered(self.loaded_volume_replicas.keys()),
			"replica hash index drifted from the loaded replicas"
		);
		for batch in self.memory_batches.pending() {
			let encoded = memory_batches::write_bytes(batch.frames(), self.config.max_batch_bytes)?;
			anyhow::ensure!(
				batch.sizing.total() == encoded,
				"batch size drifted from its frames"
			);
			anyhow::ensure!(
				*batch.index() == memory_batches::FrameIndex::recompute(batch.frames()),
				"batch frame index drifted from its frames"
			);
			anyhow::ensure!(
				*batch.read_bounds()
					== memory_batches::ReadBounds::recompute(
						batch
							.waiting_replies()
							.iter()
							.filter_map(|reply| reply.reply.data_read_bound())
					),
				"batch read reservations drifted from its replies"
			);
		}
		self.reads.validate()?;
		self.disk_segments.validate()?;
		if matches!(self.phase, LogShardPhase::Serving | LogShardPhase::Draining)
			&& self.replica_set_scan == tablet_replica_sets::ReplicaSetScan::Settled
		{
			for tablet in self.tablet_manifest.tablets.values() {
				anyhow::ensure!(
					tablet_replica_sets::frames(self, tablet).is_empty(),
					"settled replica-set scan left a fence missing"
				);
			}
		}
		Ok(())
	}
}

impl rivet_state_machine::StateMachineState for LogShardState {}

/// Ends serving without acknowledging uncertain work and clears all maintenance activity.
pub(crate) fn fail(state: &mut LogShardState, reason: String, effects: &mut Effects) {
	tracing::error!(%reason, "log shard failed; restart required");
	state.phase = LogShardPhase::Failed { reason };
	state.reads.clear();
	state.manifest_waits.clear();
	state.memory_batches.filling = MemoryBatch::default();
	state.memory_batches.writing_to_disk = None;
	state.disk_segments.checkpoint = CheckpointCycle::Idle;
	state.disk_segments.removing.clear();
	effects.exit();
}

/// Cancels startup or lets already admitted work drain before stopping.
pub(crate) fn drain(state: &mut LogShardState, effects: &mut Effects) {
	manifest_wait::drain(state, effects);
	if matches!(state.phase, LogShardPhase::Recovering(_)) {
		state.phase = LogShardPhase::Stopped;
		effects.cancel_timer(LogShardTimer::Recover);
		effects.cancel_timer(LogShardTimer::MarkRepairing);
		effects.execute(LogShardEffect::Stopped);
	} else {
		state.phase = LogShardPhase::Draining
	}
}
