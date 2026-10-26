//! Ordered disk requests and replies released only after their disk write barrier.

use super::{state::recovery::RepairMark, types::RequestId};
use rivet_data::generated::{
	depot_log_shard_v1::{DiskBatch, LogShardCheckpoint},
	depot_storage_worker_protocol_v1::StorageWorkerResponse,
};
use serde::{Deserialize, Serialize};

/// Filesystem requests return their corresponding response event to the shard.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogShardEffect {
	/// **When**
	///
	/// The first client waits for a replica-set version ahead of the shard's current manifest,
	/// or a versioned listing is refused because this shard's manifest does not hold its
	/// tablet yet or holds it at an older replica-set version, such as a split child
	/// published moments ago.
	///
	/// **Executor**
	///
	/// The runner notifies its manifest subscriber. The supervisor refreshes object storage
	/// and feeds TabletManifestUpdated; a notification never changes the view itself.
	RefreshTabletManifest,

	/// **When**
	///
	/// Dispatch cuts every accumulated frame into bounded batches while no write is in flight and installs
	/// its sequence in `memory_batches.writing_to_disk`.
	///
	/// **Executor**
	///
	/// `ShardExecutor` serializes disk work. `LogShardExecutor` encodes all batches, rolls the
	/// segment if needed, writes them after the segment's last batch in one write and syncs once,
	/// updates byte locations, and returns `WriteToDiskResponse`. A roll takes a ready zero-filled
	/// spare when preallocation is enabled; a write inside that allocation syncs only its data.
	WriteToDiskRequest {
		batches: Vec<DiskBatch>,
		/// How long the oldest frame of this write waited in the filling batch for the
		/// previous write and sync; 0 on an idle shard.
		fill_wait_ms: u64,
	},

	/// **When**
	///
	/// Segment maintenance captures a checkpoint delta when accumulated bytes or collectible
	/// segments require a checkpoint. The delta holds only the volume logs changed since the
	/// previous capture, over the chain ending at the installed boundary.
	///
	/// **Executor**
	///
	/// `LogShardExecutor::checkpoint` writes and syncs a temporary delta file, renames it,
	/// syncs the directory, and folds the chain into a new base file when the deltas have
	/// outgrown it. It returns `WriteCheckpointResponse` through the runner and does not
	/// delete segments.
	WriteCheckpointRequest {
		checkpoint: LogShardCheckpoint,
		max_checkpoint_bytes: u64,
		base_bytes: u64,
	},

	/// **When**
	///
	/// Maintenance selects inactive segments covered by the checkpoint with no retained entries
	/// or pending speculative writes.
	///
	/// **Executor**
	///
	/// `LogShardExecutor::remove_segments` unlinks exactly the selected files, syncs the
	/// directory, drops executor accounting, and returns `RemoveSegmentsResponse`.
	RemoveSegmentsRequest { first_log_seqs: Vec<u64> },

	/// **When**
	///
	/// The start hook finds the shard in `Recovering(Scanning)` and arms the recovery timeout.
	///
	/// **Executor**
	///
	/// `ShardExecutor` opens existing storage or initializes it only when the format marker is
	/// absent. The scan validates ownership, the record schema fingerprint, the checkpoint, and
	/// segments, then returns a clean or corrupt `RecoverResponse`. Only damage a checksum
	/// catches is corrupt. Another schema fingerprint, an unreadable checkpoint chain, or a
	/// checksum-valid batch that fails to decode or replay returns an error during the scan,
	/// before any segment is cut, marked or quarantined. A recovered image that fails validation
	/// returns an error too; by then only an incomplete final write may have been cut. The
	/// supplied random disk identity is written only for a new folder; intact storage always
	/// retains the identity from its format marker.
	RecoverRequest { disk_incarnation: [u8; 16] },

	/// **When**
	///
	/// A corrupt recovery outcome selects new repairing incarnations and moves the shard to
	/// `Recovering(MarkingRepairing)`.
	///
	/// **Executor**
	///
	/// `LogShardExecutor::mark_repairing` writes replacement states and a checkpoint to disk before
	/// quarantining damaged segments, syncs the directory, and returns the rebuilt image in
	/// `MarkRepairingResponse`.
	MarkRepairingRequest {
		retired_volumes: super::state::retired_volumes::RetiredVolumes,
		tablet_grants: std::collections::BTreeMap<[u8; 16], u64>,
		tablet_replica_sets: std::collections::BTreeMap<
			[u8; 16],
			rivet_data::generated::shared_depot_types_v1::TabletReplicaSet,
		>,
		repair_marks: Vec<RepairMark>,
		quarantine_from_log_seq: u64,
		first_log_seq: u64,
	},

	/// **When**
	///
	/// An unfenced read reaches its disk barrier, captures the replica's disk state, and
	/// cannot be answered from retained entries for `reason`.
	///
	/// **Executor**
	///
	/// Counts the fallback by reason. In effect order, the write queue resolves the indexed
	/// locations of the requested entries, which no later write moves or overwrites. A pool of
	/// `disk_read_concurrency` readers then reads and verifies those batches beside the write
	/// queue. The reader observes the original deadline while waiting and reading, then sends
	/// ReadResponse with that same state when it ends.
	ReadRequest {
		request_id: RequestId,
		deadline_boot_ms: u64,
		volume_log_key: depot_types::VolumeLogKey,
		state: rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState,
		first_volume_seq: u64,
		max_bytes: u32,
		/// Why memory could not answer; the executor counts it.
		reason: crate::log_shard_machine::types::DiskReadReason,
	},

	/// **When**
	///
	/// Admission rejects immediately, or batch completion releases an accepted result after its
	/// disk write barrier.
	///
	/// **Executor**
	///
	/// `ShardExecutor` records rejection metrics when applicable, removes the registered
	/// response sender, and delivers the result inline in effect order. A dropped receiver is
	/// ignored.
	Reply {
		request_id: RequestId,
		response: StorageWorkerResponse,
	},

	/// **When**
	///
	/// Draining finishes with no outstanding batch or maintenance, or startup is cancelled.
	///
	/// **Executor**
	///
	/// `ShardExecutor` performs no action for this marker. The owner observes `Stopped` in the
	/// published snapshot, closes the inbox, and joins its tasks.
	Stopped,

	/// **When**
	///
	/// Dispatch after an event finds replicas marked for a maintainer worker with no notice
	/// in flight to it and no backoff, and arms `NoticeDeadline` for it.
	///
	/// **Executor**
	///
	/// `ShardExecutor` sends the request through the installed `StatusNotifier` and returns
	/// `NotifyMaintainerResponse` with `notice_id` and the outcome. With no notifier
	/// installed it returns `Failed` at once. It never retries; the state machine resends
	/// after a backoff.
	NotifyMaintainerRequest {
		worker: rivet_topology::worker::WorkerId,
		notice_id: u64,
		request: rivet_data::generated::depot_client_protocol_v1::ReplicaStatusNoticeRequest,
	},
}

impl rivet_state_machine::Effect for LogShardEffect {
	fn name(&self) -> &'static str {
		match self {
			LogShardEffect::RefreshTabletManifest => "refresh_tablet_manifest",
			LogShardEffect::WriteToDiskRequest { .. } => "write_to_disk_request",
			LogShardEffect::WriteCheckpointRequest { .. } => "write_checkpoint_request",
			LogShardEffect::RemoveSegmentsRequest { .. } => "remove_segments_request",
			LogShardEffect::RecoverRequest { .. } => "recover_request",
			LogShardEffect::MarkRepairingRequest { .. } => "mark_repairing_request",
			LogShardEffect::ReadRequest { .. } => "read_request",
			LogShardEffect::Reply { .. } => "reply",
			LogShardEffect::Stopped => "stopped",
			LogShardEffect::NotifyMaintainerRequest { .. } => "notify_maintainer_request",
		}
	}
}
