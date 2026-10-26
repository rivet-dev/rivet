//! Read-only progress for observers; payload bytes never enter the snapshot.

use super::{
	state::{BlockedAppend, MatchStep, VolumeClientPhase},
	types::{OpenPurpose, VolumeClientConfig},
};
use rivet_data::generated::shared_depot_types_v1::{ClientIdentity, TabletReplicaSet};
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoadedVolumeReplicaView {
	pub worker_id: WorkerId,
	pub acked: u64,
	pub claimed: bool,
	pub counted: bool,
	pub matched: bool,
	pub replica_incarnation: [u8; 16],
	pub status: Option<rivet_data::generated::shared_depot_types_v1::VolumeLogReplicaStatus>,
	pub repair: Option<MatchStep>,
	pub in_flight: u32,
	/// The entry this replica's shard permanently refuses, if any; nothing at or above it is
	/// sent to the replica again on this handle.
	pub blocked: Option<BlockedAppend>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VolumeClientSnapshot {
	/// Zero until open resolves the volume row; fixed for the resolved incarnation.
	/// Fixed encoded-entry budget, including its position. Available memory can
	/// still make admission refuse a request below this limit.
	pub max_mutation_bytes: u32,
	/// Absent until the first live row resolves the logical volume key.
	pub volume_log_key: Option<Arc<depot_types::VolumeLogKey>>,
	pub purpose: OpenPurpose,
	pub cold: rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
	/// Outstanding open attempts tracked until their reply or control deadline.
	pub pending_opens: u32,
	pub phase: VolumeClientPhase,
	pub client: ClientIdentity,
	/// Shared with the state, so publishing never copies it and comparing usually stops at
	/// the pointer.
	pub replica_set: Arc<TabletReplicaSet>,
	pub next_volume_seq: u64,
	pub committed: u64,
	pub claim_entry_volume_seq: Option<u64>,
	/// When this opening's claim entry was written, with `claim_entry_volume_seq`.
	pub claim_entry_written_at_unix_ms: Option<i64>,
	/// The volume row names a fork base, so its rows are not only in this volume's log.
	pub has_base_chain: bool,
	/// The current open adopted a claim the maintainer granted, so the maintainer knows it.
	pub granted_claim: bool,
	/// The maintainer acknowledged this client's close, so its owner sends no second one.
	pub close_acknowledged: bool,
	/// The open found nothing to recover, so writes ride behind the in-flight claim entry.
	pub opened_empty: bool,
	/// The open created the volume with the opener's initial writes, committed through here.
	pub created: Option<rivet_data::generated::shared_depot_types_v1::VolumeLogPosition>,
	/// The values the creating open read back from an existing volume; see `VolumeOpenResponse.read`.
	pub read_back: Option<Arc<Vec<Option<vbare::Bytes>>>>,
	pub last_client_seq: u64,
	/// The lowest entry no quorum can append any more, with the shard refusal that blocks it.
	/// Nothing at or behind it commits on this client, so an owner fences and reopens.
	pub history_blocked: Option<BlockedAppend>,
	pub pending_writes: u32,
	pub memory_bytes: u64,
	/// Log bytes above cold, including payloads already released from RAM.
	pub dirty_bytes: u64,
	/// Shared with the state; see `replica_set`.
	pub config: Arc<VolumeClientConfig>,
	pub replicas: Vec<LoadedVolumeReplicaView>,
}

impl rivet_state_machine::Snapshot for VolumeClientSnapshot {}
