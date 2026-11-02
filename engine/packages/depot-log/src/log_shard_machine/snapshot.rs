//! Immutable views show accepted and disk positions side by side.

use super::state::{LogShardPhase, disk_segments::CheckpointCycle};
use depot_types::VolumeLogKey;
use rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoadedVolumeReplicaView {
	pub volume_log_key: VolumeLogKey,
	pub memory_state: VolumeLogReplicaState,
	pub disk_state: Option<VolumeLogReplicaState>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WriteToDiskCycle {
	Idle,
	WritingToDisk {
		first_log_seq: u64,
		last_log_seq: u64,
	},
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogShardSnapshot {
	/// Disk lifetime from FORMAT; zero only before recovery finishes.
	pub disk_incarnation: [u8; 16],
	pub phase: LogShardPhase,
	pub write_to_disk: WriteToDiskCycle,
	pub filling_requests: u32,
	pub checkpoint: CheckpointCycle,
	/// Empty when `LogShardConfig::publish_replica_views` is off.
	pub loaded_volumes: Vec<LoadedVolumeReplicaView>,
	pub resources: Vec<ResourceUsage>,
	/// Retained log entries across replicas; each keeps its header in memory.
	pub retained_entries: u64,
	/// Encoded bytes of the retained entries: the log tails above cold, which disk also
	/// holds. Memory held for payloads is the `RetainedPayloadBytes` resource.
	pub retained_entry_bytes: u64,
	/// Replica notices not yet delivered to a maintainer: marked, waiting out an append
	/// delay, or in flight. A replica marked again while its notice is in flight counts
	/// twice. Zero means every change so far was delivered.
	pub undelivered_status_notices: u64,
	/// The newest batch sequence synced to disk.
	pub synced_log_seq: u64,
	/// The lowest batch sequence a retained entry still lives in: the batch that keeps the
	/// oldest segment from being collected. `None` when no retained entry has a disk batch.
	pub oldest_pinned_batch_log_seq: Option<u64>,
	/// Segment files the shard holds, active segment included.
	pub segments: u32,
}

impl rivet_state_machine::Snapshot for LogShardSnapshot {}

/// Bounded resource names are safe metric labels and contain no log identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Resource {
	PendingRequests,
	PendingBytes,
	DiskBytes,
	Replicas,
	QuarantinedBytes,
	Inbox,
	/// Encoded bytes of synced and unsynced entries whose payloads are held in memory,
	/// against `max_retained_payload_bytes`.
	RetainedPayloadBytes,
	/// Reads dispatched to disk and not yet answered, against `max_disk_reads`.
	DiskReads,
	/// Milliseconds the latest submission waited in the inbox, against
	/// `max_admission_wait_ms`; submissions past it are refused.
	AdmissionWaitMs,
}

impl Resource {
	pub fn label(self) -> &'static str {
		match self {
			Self::PendingRequests => "pending_requests",
			Self::PendingBytes => "pending_bytes",
			Self::DiskBytes => "disk_bytes",
			Self::Replicas => "replicas",
			Self::QuarantinedBytes => "quarantined_bytes",
			Self::Inbox => "inbox",
			Self::RetainedPayloadBytes => "retained_payload_bytes",
			Self::DiskReads => "disk_reads",
			Self::AdmissionWaitMs => "admission_wait_ms",
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceUsage {
	pub resource: Resource,
	pub used: u64,
	pub capacity: u64,
}
