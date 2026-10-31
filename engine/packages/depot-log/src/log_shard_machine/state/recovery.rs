//! Recovery outcomes are data; the shard decides which replicas must be marked repairing.

use super::{
	LogShardPhase, LogShardState, RecoveryStep, disk_segments::DiskSegmentUsage, fail,
	loaded_volume_replica::ReplicaEntry,
};
use crate::log_shard_machine::state::loaded_volume_replica::LoadedVolumeReplica;
use crate::log_shard_machine::{Effects, effect::LogShardEffect, types::LogShardTimer};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_log_shard_v1::{StateFrame, VolumeLogReplicaState},
	shared_depot_types_v1::{ClientIdentity, VolumeLogPosition, VolumeLogReplicaStatus},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use vbare::Encode;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecoveredReplica {
	pub volume_log_key: VolumeLogKey,
	pub state: VolumeLogReplicaState,
	pub entries: Vec<ReplicaEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecoveredShard {
	pub checkpoint_bytes: u64,
	pub retired_volumes: super::retired_volumes::RetiredVolumes,
	pub disk_incarnation: [u8; 16],
	pub replicas: Vec<RecoveredReplica>,
	pub tablet_grants: BTreeMap<[u8; 16], u64>,
	pub tablet_replica_sets:
		BTreeMap<[u8; 16], rivet_data::generated::shared_depot_types_v1::TabletReplicaSet>,
	pub disk_segments: BTreeMap<u64, DiskSegmentUsage>,
	pub last_log_seq: u64,
	pub checkpoint_last_included_log_seq: u64,
	pub bytes_since_checkpoint: u64,
	/// Volume logs whose replica state or retirement changed in batches replayed above
	/// the checkpoint chain. The next checkpoint delta must cover them.
	pub changed_since_checkpoint: std::collections::BTreeSet<VolumeLogKey>,
	pub quarantined_bytes: u64,
	/// The batch limit FORMAT records, the largest any writer of this folder used. Reads size
	/// their pages and scan budgets by the larger of it and the live limit, so a stored entry
	/// above a lowered limit stays readable. Zero means no stored batch exceeds the live limit.
	pub stored_max_batch_bytes: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecoveryOutcome {
	Clean(RecoveredShard),
	Corrupt(CorruptionFound),
}

/// Checkpoint floors are the only trusted history after corruption. Newly discovered
/// keys without a checkpoint floor are included at zero so they cannot resume in sync.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorruptionFound {
	pub retired_volumes: super::retired_volumes::RetiredVolumes,
	pub disk_incarnation: [u8; 16],
	pub kind: CorruptionKind,
	/// False when undecodable or missing batches may have erased a tablet fence.
	/// Replica repair cannot reconstruct grant epochs or retired-parent sentinels.
	pub grant_fences_complete: bool,
	pub replica_floors: Vec<StateFrame>,
	pub tablet_grants: BTreeMap<[u8; 16], u64>,
	pub tablet_replica_sets:
		BTreeMap<[u8; 16], rivet_data::generated::shared_depot_types_v1::TabletReplicaSet>,
	pub quarantine_from_log_seq: u64,
	pub last_valid_log_seq: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CorruptionKind {
	DamagedBatch {
		log_seq: u64,
	},
	SegmentNameMismatch {
		file_log_seq: u64,
		first_batch_log_seq: u64,
	},
	/// A preallocated segment's batches end before the length its header records as
	/// synced by earlier writes, so acknowledged batches were lost.
	LostSyncedTail {
		first_log_seq: u64,
		synced_bytes: u64,
		found_bytes: u64,
	},
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepairMark {
	pub learner_replica_set_version: u64,
	pub volume_log_key: VolumeLogKey,
	/// Deletion survives recovery; all other damaged replicas become repairing.
	pub status: VolumeLogReplicaStatus,
	pub cold: VolumeLogPosition,
	/// The newest claim a checksum-valid frame recorded for the damaged replica. The emptied
	/// replica keeps it so a client the kept claim fenced cannot claim the replica again and commit
	/// beside the client that fenced it.
	pub claimed_client: Option<ClientIdentity>,
	pub new_replica_incarnation: [u8; 16],
}

/// Why recovery refused to start. Every variant fails the shard before anything on disk is
/// cut, marked or quarantined, so an operator can fix the cause and restart on the same bytes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecoveryError {
	ForeignFolder,
	UnsupportedFormatVersion {
		found: u16,
		supported: u16,
	},
	/// FORMAT names another record schema, or none, so no record can be interpreted.
	IncompatibleFormat {
		reason: String,
	},
	/// A batch passed its checksum, so it holds exactly what a writer produced, but it does not
	/// decode. Another schema or a bug wrote it. Every member of a tablet runs the same binary,
	/// so treating it as disk damage would empty the same replicas on every member at once.
	UndecodableBatch {
		segment_first_log_seq: u64,
		offset: u64,
		reason: String,
	},
	/// A checksum-valid batch decoded but broke a replay invariant. Like an undecodable batch it
	/// points at a version or code mismatch, never at disk damage.
	InvalidBatch {
		segment_first_log_seq: u64,
		offset: u64,
		reason: String,
	},
	/// The checksum-valid log replayed, but the recovered image failed validation.
	InvalidImage {
		reason: String,
	},
	/// The checkpoint chain cannot be read, so the tablet grant fences it holds are unknown.
	/// Rebuilding an empty grant table could revive retired owners.
	UnreadableCheckpoint {
		reason: String,
	},
	Io(String),
}

impl std::fmt::Display for RecoveryError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::ForeignFolder => write!(
				f,
				"log shard folder belongs to another worker; check the configured log directory"
			),
			Self::UnsupportedFormatVersion { found, supported } => write!(
				f,
				"log shard FORMAT version {found} is not supported by this binary, which reads \
				 version {supported}; run a binary that supports it"
			),
			Self::IncompatibleFormat { reason } => write!(f, "{reason}"),
			Self::UndecodableBatch {
				segment_first_log_seq,
				offset,
				reason,
			} => write!(
				f,
				"log segment seg-{segment_first_log_seq} holds a checksum-valid batch at offset \
				 {offset} that this binary cannot decode ({reason}); refusing to recover and \
				 leaving its segments untouched. This is a schema or binary mismatch, not disk damage: \
				 run the binary that wrote this disk"
			),
			Self::InvalidBatch {
				segment_first_log_seq,
				offset,
				reason,
			} => write!(
				f,
				"log segment seg-{segment_first_log_seq} holds a checksum-valid batch at offset \
				 {offset} that breaks a replay invariant ({reason}); refusing to recover and \
				 leaving its segments untouched. Run the binary that wrote this disk"
			),
			Self::InvalidImage { reason } => write!(
				f,
				"checksum-valid log shard image failed validation ({reason}); refusing to \
				 recover without marking or quarantining anything. Run the binary that wrote \
				 this disk"
			),
			Self::UnreadableCheckpoint { reason } => write!(
				f,
				"log shard checkpoint chain is unreadable ({reason}), so its tablet grant fences \
				 cannot be recovered; leaving its segments untouched. Run the binary that wrote it, or \
				 reinitialize the disk after its tablets are replaced"
			),
			Self::Io(reason) => write!(f, "log shard recovery I/O failed: {reason}"),
		}
	}
}

impl std::error::Error for RecoveryError {}

impl LogShardState {
	/// Publishes a validated disk image as working and disk state together.
	pub(crate) fn install_recovered(
		&mut self,
		recovered: RecoveredShard,
		now_boot_ms: u64,
	) -> anyhow::Result<()> {
		self.disk_incarnation = recovered.disk_incarnation;
		if recovered.replicas.len() > self.config.max_loaded_volumes as usize {
			tracing::warn!(
				loaded = recovered.replicas.len(),
				capacity = self.config.max_loaded_volumes,
				"loaded replicas exceed admission limit; existing logs remain available"
			);
		}

		// Validate the entire image before replacing the shard state.
		let mut replicas = super::replica_table::ReplicaTable::new();
		for r in recovered.replicas {
			let replica =
				LoadedVolumeReplica::restore(r.volume_log_key.clone(), r.state.clone(), r.entries)?;
			let key = r.volume_log_key.clone();
			anyhow::ensure!(!replicas.contains_key(&key), "duplicate recovered log");
			replicas.insert(key, replica);
		}

		anyhow::ensure!(
			!replicas
				.keys()
				.any(|key| recovered.retired_volumes.contains_key(key)),
			"retired log appears in loaded recovery image"
		);
		// Publish the replicas and their matching disk accounting together.
		self.segment_pins = super::segment_pins::SegmentPins::recompute(&replicas);
		self.payloads = super::payloads::Payloads::recovered(&replicas, now_boot_ms);
		self.replica_hashes = super::replica_hashes::ReplicaHashes::recovered(replicas.keys());
		self.loaded_volume_replicas = replicas;
		self.tablet_grants = recovered.tablet_grants;
		self.retired_volumes = recovered.retired_volumes;
		self.tablet_replica_sets = recovered.tablet_replica_sets;
		// Recovered disk fences replace the old ones, so every tablet is examined again.
		self.replica_set_scan = super::tablet_replica_sets::ReplicaSetScan::Pending;
		self.disk_segments.replace(recovered.disk_segments);
		self.memory_batches.last_log_seq = recovered.last_log_seq;
		self.disk_segments.quarantined_bytes = recovered.quarantined_bytes;
		self.disk_segments.checkpoint_bytes = recovered.checkpoint_bytes;
		// Startup already visits every replica. Thereafter only changed frames adjust this bound.
		self.disk_segments.checkpoint_metadata_bytes = self.checkpoint_image().encoded_len() as u64;
		self.stored_max_batch_bytes = recovered.stored_max_batch_bytes;
		if 2 * (u64::from(self.read_batch_limit())
			+ depot_types::log_shard::BATCH_FRAMING_BYTES as u64)
			> self.config.max_memory_bytes
		{
			tracing::warn!(
				stored_max_batch_bytes = recovered.stored_max_batch_bytes,
				max_memory_bytes = self.config.max_memory_bytes,
				"stored log batches exceed what the memory budget can read; raise storage.log_shard.max_memory_bytes or reads of older segments are refused as overloaded"
			);
		}
		self.disk_segments.bytes_since_checkpoint = recovered.bytes_since_checkpoint;
		self.disk_segments.changed_since_checkpoint = recovered.changed_since_checkpoint;
		self.disk_segments.checkpoint_last_included_log_seq =
			recovered.checkpoint_last_included_log_seq;
		// A disk recovered above its watermark is under pressure before its first write.
		super::status_notices::observe_disk_pressure(self);
		// Changes synced before a crash may never have been noticed, so every recovered
		// replica is noticed once to its maintainer.
		let recovered_keys = self
			.loaded_volume_replicas
			.keys()
			.cloned()
			.collect::<Vec<_>>();
		super::status_notices::changed(self, recovered_keys);

		Ok(())
	}
}

/// Installs a clean scan or marks damaged replicas repairing before serving.
/// New incarnations invalidate proofs held by clients that claimed before the loss.
/// The executor syncs existing evidence, writes and syncs replacement states in a new
/// segment, then quarantines damage. A crash before the marks rediscovers corruption;
/// after them, recovery sees repairing replicas. Renaming first would erase evidence.
pub(crate) fn recovered(
	state: &mut LogShardState,
	result: Result<RecoveryOutcome, RecoveryError>,
	rng: &mut rivet_env::rand::LocalRng,
	effects: &mut Effects,
) {
	effects.cancel_timer(LogShardTimer::Recover);
	match result {
		Ok(RecoveryOutcome::Clean(recovered)) => {
			match state.install_recovered(recovered, effects.now_boot_ms()) {
				Ok(()) => serve_recovered(state),
				Err(e) => fail(state, e.to_string(), effects),
			}
		}
		// Corrupt images must write fresh repairing identities to disk before serving.
		Ok(RecoveryOutcome::Corrupt(found)) => {
			state.disk_incarnation = found.disk_incarnation;
			// A lost checkpoint may contain retired-tablet sentinels whose segments
			// were collected. Rebuilding an empty grant table would revive old owners.
			if !found.grant_fences_complete {
				fail(
					state,
					"corrupt storage: tablet grant fences cannot be recovered".into(),
					effects,
				);
				return;
			}

			let repair_marks = found
				.replica_floors
				.iter()
				.map(|old| {
					let mut incarnation = *rng.uuid_v4().as_bytes();
					while incarnation == old.state.replica_incarnation {
						incarnation = *rng.uuid_v4().as_bytes();
					}
					RepairMark {
						learner_replica_set_version: old.state.learner_replica_set_version,
						volume_log_key: old.volume_log_key.clone().into(),
						status: if old.state.status == VolumeLogReplicaStatus::Deleted {
							VolumeLogReplicaStatus::Deleted
						} else {
							VolumeLogReplicaStatus::Repairing
						},
						cold: old.state.cold.clone(),
						claimed_client: old.state.claimed_client.clone(),
						new_replica_incarnation: incarnation,
					}
				})
				.collect();
			let Some(first_log_seq) = found.last_valid_log_seq.checked_add(1) else {
				fail(state, "log sequence exhausted".into(), effects);
				return;
			};

			// Keep admission closed until the executor confirms the replacement disk image.
			state.phase = LogShardPhase::Recovering(RecoveryStep::MarkingRepairing);
			effects.arm_timer_after(
				LogShardTimer::MarkRepairing,
				state.config.recovery_timeout_ms,
			);
			effects.execute(LogShardEffect::MarkRepairingRequest {
				tablet_grants: found.tablet_grants,
				retired_volumes: found.retired_volumes,
				tablet_replica_sets: found.tablet_replica_sets,
				repair_marks,
				quarantine_from_log_seq: found.quarantine_from_log_seq,
				first_log_seq,
			});
		}

		Err(reason) => fail(state, reason.to_string(), effects),
	}
}

/// Starts serving only after the repair marking frames and quarantine are on disk.
pub(crate) fn marked_repairing(
	state: &mut LogShardState,
	result: Result<RecoveredShard, crate::log_shard_machine::types::WriteToDiskError>,
	effects: &mut Effects,
) {
	effects.cancel_timer(LogShardTimer::MarkRepairing);
	match result {
		Ok(recovered) => match state.install_recovered(recovered, effects.now_boot_ms()) {
			Ok(()) => serve_recovered(state),
			Err(e) => fail(state, e.to_string(), effects),
		},
		Err(reason) => fail(state, reason.to_string(), effects),
	}
}

/// Starts serving a recovered shard and reports how much it recovered.
fn serve_recovered(state: &mut LogShardState) {
	state.phase = LogShardPhase::Serving;
	tracing::info!(
		replicas = state.loaded_volume_replicas.len(),
		disk_bytes = state.disk_segments.disk_bytes(),
		"log shard recovered"
	);
}
