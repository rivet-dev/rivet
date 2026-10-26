//! Authoritative client history, per-replica windows and pending callers.

use super::types::*;
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_log_shard_v1::VolumeLogReplicaState,
	shared_depot_types_v1::{
		ClientIdentity, TabletPhase, TabletReplicaSet, VolumeLogPosition, VolumeLogReplicaStatus,
	},
};
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use vbare::Encode;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VolumeClientPhase {
	/// Admission is closed while the maintainer supplies the volume row.
	Opening { op_id: Option<OpId> },

	/// A claim, matching history, and committed claim entry must precede writes.
	Recovering(RecoveryPhase),

	/// New writes are closed while accepted writes reach a quorum. The maintainer derives the
	/// final committed position from the replica tails, so nothing is announced before close.
	Draining { request_id: Option<RequestId> },

	/// The maintainer release is outstanding; only its correlated result may finish close.
	Closing {
		op_id: OpId,
		request_id: Option<RequestId>,
	},

	/// The close attempt has ended and this handle cannot accept writes.
	Stopped,

	/// A committed claim entry authorizes new writes and independent replica pipelines.
	Appending,

	/// Authority is lost; pending callers are failed and old evidence cannot restore it.
	Fenced { error: WriteError },
}

/// Where an open's replay starts: just above cold, or at the applied position the
/// maintainer reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplayStart {
	Cold,
	Applied,
}

/// A claim round is retried as a whole after contention; prior operation IDs are retired.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecoveryPhase {
	/// Claim replies are being collected under the claim epoch the maintainer granted.
	Claiming,

	/// One validated claimed tail defines history above cold; another voter may
	/// supply pages only when it proves that exact tail at this client claim. Unless the
	/// claim is settled, the client's claim entry is already retained on that tail and on its
	/// way to the members matched there, and may commit while the pages are read.
	Replaying {
		source: WorkerId,
		proof: VolumeLogReplicaState,
		op_id: OpId,
		next: u64,
		/// The position the first page must start with: the maintainer's applied position
		/// when the replay starts there instead of above cold. A page that disagrees with it
		/// is evidence of another history, so the open fails.
		verify: Option<VolumeLogPosition>,
		/// Where the replay started, for the open replay metrics only.
		start: ReplayStart,
	},

	/// Recovered history, with the claim entry on it, is being matched onto a quorum. The
	/// members matched at the recovered tail already hold or are receiving the claim entry;
	/// the others reach it through repair.
	Matching,

	/// Serving starts only after this entry has a disk in-sync quorum. The entry was retained
	/// when recovery selected its history, so it may already be committed when matching ends,
	/// and this phase then ends at once. A volume opened empty already admits writes in this
	/// phase; they follow the claim entry and commit after it.
	CommittingClaimEntry { volume_seq: u64 },

	/// A recovery-only handle repairs remaining members onto its committed claim entry, then drains.
	Finishing { volume_seq: u64 },
}

/// One conditional operation owns an unmatched replica until its history agrees.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MatchStep {
	/// A matched peer supplies bytes missing from retained history.
	Fetching {
		op_id: OpId,
		source: WorkerId,
		source_incarnation: [u8; 16],
		target: VolumeLogReplicaState,
	},

	/// Repair has backed off and consumes no active repair slot.
	Waiting { op_id: OpId },

	/// InSync promotion is outstanding for the repaired incarnation and exact tail.
	Promoting {
		op_id: OpId,
		expected: VolumeLogReplicaState,
	},

	/// A bounded read tests a candidate divergence position without modifying the replica.
	Probing {
		op_id: OpId,
		at: u64,
		search: ProbeSearch,
		observed: VolumeLogReplicaState,
	},

	/// A conditional truncate is outstanding at the matched prefix boundary.
	Truncating {
		op_id: OpId,
		keep: VolumeLogPosition,
		mark_repairing: bool,
	},

	/// An append is outstanding for the selected retained suffix. `bytes` is the request's
	/// estimated size as the copy bounded it, kept here because retained entries may be
	/// released while the copy is in flight; a size refusal lowers the limit below it.
	Copying {
		op_id: OpId,
		end: VolumeLogPosition,
		bytes: u64,
	},
}

impl MatchStep {
	/// Returns the single outstanding repair operation to cancel.
	pub fn operation_ids(&self) -> Vec<OpId> {
		vec![self.op_id()]
	}

	/// Returns the outstanding repair operation.
	pub fn op_id(&self) -> OpId {
		match self {
			Self::Fetching { op_id, .. }
			| Self::Waiting { op_id }
			| Self::Promoting { op_id, .. }
			| Self::Probing { op_id, .. }
			| Self::Truncating { op_id, .. }
			| Self::Copying { op_id, .. } => *op_id,
		}
	}
}

/// Probe exponentially until a match brackets the divergence, then bisect the bracket.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProbeSearch {
	Backward { distance: u64, upper: u64 },
	Binary { low: u64, high: u64 },
}

/// Only responses to these operations can advance this replica's disk position.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PendingAppend {
	pub end: u64,
	pub committed: u64,
	pub confirmation: Option<RequestId>,
	/// The request's estimated size, as `send` bounded it. A `RequestTooLarge` refusal
	/// lowers the client's append limit below it, so the next batch is strictly smaller even
	/// when the shard reports a limit at or above it.
	pub bytes: u64,
}

/// An entry a replica's shard refuses for a reason retries cannot change. Nothing at or above
/// the block is sent to the replica again on this handle, and the refusal itself starts no
/// repair, new claim or fence, because it says nothing about the replica's log. Entries below
/// the block still reach it: from memory while it is matched, where they stay retained for
/// it within the memory pin that also holds entries for refused replicas, and through
/// ordinary repair otherwise, which copies up to the block and matches it at that tail. A
/// matched replica holding everything below its block stays matched, so its
/// acknowledgements keep voting. The block clears when cold passes it or the history it
/// names is replaced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockedAppend {
	pub volume_seq: u64,
	pub refusal: PermanentRefusal,
}

/// A replica's refusal delay: the `Resume` timer that ends it and when it fires.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RefusalDelay {
	pub op_id: OpId,
	/// When the replica may be sent to again, so a write refused meanwhile can tell its caller.
	pub until_boot_ms: u64,
	/// Whether the shard refused for a full disk rather than a full queue or memory budget.
	pub disk_full: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Replica {
	pub repair: Option<MatchStep>,
	pub repair_attempt: u32,
	/// The source whose last repair fetch for this replica timed out; the next fetch prefers
	/// another matched peer.
	pub repair_avoid: Option<WorkerId>,
	pub matched: bool,
	pub claim: Option<VolumeLogReplicaState>,
	pub pending_claim: Option<OpId>,
	pub incarnation: [u8; 16],
	pub acked: u64,
	pub sent: u64,
	pub in_flight: BTreeMap<OpId, PendingAppend>,
	/// The retry delay of an `Overloaded` or `DiskFull` refusal. Nothing is sent to this
	/// replica and `in_flight` stays empty. A refusal leaves the replica matched, because it
	/// says nothing about its log; repair and fencing end the delay.
	pub refused: Option<RefusalDelay>,
	/// The appended position this replica's repair last observed while it is not matched.
	/// The client keeps committed entries above it, within half its memory budget, so the copy
	/// can send them from memory instead of fetching them from a peer that may already have
	/// moved them into cold.
	pub repair_floor: Option<u64>,
	/// All history assigned before this replica stopped counting, or the exact history
	/// selected by initial recovery. Matching this finite prefix permits ordinary pipelining;
	/// later writes already require a quorum that excludes the unmatched replica. Backoff
	/// retains it, while a new client, configuration or replica lifetime rebinds it.
	pub repair_goal: Option<u64>,
	/// The first entry this replica's shard permanently refuses; see [`BlockedAppend`].
	/// Cleared when cold passes it, after which repair catches the replica up from cold,
	/// when the shard's own tail proves it holds the entry after all, or when the history
	/// the block names is replaced by an open or recovery.
	pub blocked: Option<BlockedAppend>,
	/// The largest append request this replica's shard accepts, including the request header.
	/// It starts at `config.max_append_bytes` and only decreases: the shard's `RequestTooLarge`
	/// lowers it to the limit the shard reported, so the next batch is rebuilt to fit instead
	/// of resent as it was. The quorum's limit bounds admission; see
	/// [`VolumeClientState::max_mutation_bytes`].
	pub append_limit_bytes: u32,
}

impl Replica {
	/// Stop counting a replica without chasing writes admitted during the same repair.
	pub(super) fn unmatch(&mut self, through: Option<u64>) {
		if self.matched || self.repair_goal.is_none() {
			self.repair_goal = through;
		}
		self.matched = false;
	}

	/// Whether appends and announcements may go to this replica now.
	pub fn sendable(&self, max_in_flight: u32) -> bool {
		self.matched && self.refused.is_none() && self.in_flight.len() < max_in_flight as usize
	}

	/// Whether this replica can still append `volume_seq`: it is not blocked at or below it.
	pub fn can_append(&self, volume_seq: u64) -> bool {
		self.blocked
			.is_none_or(|blocked| blocked.volume_seq > volume_seq)
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PendingWrite {
	pub token: WriteToken,
	pub submitted: bool,

	/// A retry can join the original operation; each caller has its own deadline.
	pub waiters: BTreeMap<RequestId, bool>,
}

/// Quorum progress as only the client sees it. Appends carry it to the replicas as a free
/// hint; the maintainer derives commitment from the replica tails it observes, so the client
/// never announces it separately.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitPublishing {
	/// Highest volume sequence acknowledged by an in-sync quorum; never decreases.
	pub committed: u64,
}

/// A logical key is resolved once; subsequent opens stay pinned to that volume lifetime.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VolumeIdentity {
	/// No open has resolved the incarnation yet. `create_if_absent` asks the maintainer to
	/// create a missing volume inside the open; a resolved identity never creates.
	Unresolved {
		key: depot_types::VolumeKey,
		create_if_absent: bool,
	},
	/// Shared with every published snapshot, which never copies the key.
	Known(std::sync::Arc<VolumeLogKey>),
}

impl From<depot_types::VolumeKey> for VolumeIdentity {
	fn from(key: depot_types::VolumeKey) -> Self {
		Self::Unresolved {
			key,
			create_if_absent: false,
		}
	}
}

impl From<std::sync::Arc<VolumeLogKey>> for VolumeIdentity {
	fn from(key: std::sync::Arc<VolumeLogKey>) -> Self {
		Self::Known(key)
	}
}

impl From<VolumeLogKey> for VolumeIdentity {
	fn from(key: VolumeLogKey) -> Self {
		Self::Known(std::sync::Arc::new(key))
	}
}

impl VolumeIdentity {
	pub(crate) fn logical_key(&self) -> &depot_types::VolumeKey {
		match self {
			Self::Unresolved { key, .. } => key,
			Self::Known(key) => &key.0.volume_key,
		}
	}

	/// Whether the next open may create a missing volume. Only an unresolved identity does.
	pub(crate) fn create_if_absent(&self) -> bool {
		match self {
			Self::Unresolved {
				create_if_absent, ..
			} => *create_if_absent,
			Self::Known(_) => false,
		}
	}

	/// Replica requests only run after the maintainer supplied and validated a live row.
	pub(crate) fn loaded_key(&self) -> &VolumeLogKey {
		let Self::Known(key) = self else {
			unreachable!("replica access before volume open")
		};
		key
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VolumeClientState {
	pub(crate) purpose: OpenPurpose,
	/// A recovery owner captured these destinations in its admitted assignment. Each must
	/// match and learn the claim entry before success; other destinations may remain incomplete.
	pub(crate) required_recovery_members: BTreeSet<WorkerId>,
	/// Shared with every published snapshot; the config never changes after construction.
	pub(crate) config: std::sync::Arc<VolumeClientConfig>,
	pub(crate) entry_frame_overhead: u32,
	pub(crate) phase: VolumeClientPhase,
	pub(crate) volume: VolumeIdentity,
	pub(crate) client: ClientIdentity,
	/// A completed create supplies the initial open evidence. Consumed exactly once by start;
	/// subsequent recovery always performs a fresh open pinned to this volume incarnation.
	pub(crate) initial_open:
		Option<rivet_data::generated::depot_client_protocol_v1::VolumeOpenResponse>,
	/// Whether `initial_open` was kept from an earlier create rather than just answered, so
	/// its member states may be stale and every member is claimed again; see
	/// [`super::lifecycle::opened`].
	pub(crate) initial_open_kept: bool,
	pub(crate) cold: VolumeLogPosition,
	pub(crate) waiting: BTreeMap<RequestId, super::waiting::Caller>,
	/// Open attempts remain correlated until the UPS deadline, even after the last caller leaves.
	pub(crate) open_attempts: BTreeSet<OpId>,
	/// Destinations not loaded on their log shard: omitted from the open reply or answering
	/// `VolumeNotLoaded` since. The maintainer reloads them and repair catches them up; a
	/// successful claim removes one. Only losing a loaded quorum makes the client reopen.
	pub(crate) loading_members: BTreeSet<WorkerId>,
	pub(crate) next_volume_seq: u64,
	pub(crate) commit_publishing: CommitPublishing,
	/// When everything admitted last became committed, while nothing new is admitted. The
	/// next write's gap from it decides whether that burst frees its maps when it commits.
	pub(crate) quiet_since_boot_ms: Option<u64>,
	/// The burst in progress followed a gap of at least `compact_after_quiet_ms`, or is the
	/// first, so its maps are freed once it commits; see [`Self::free_quiet_containers`].
	pub(crate) compact_when_quiet: bool,
	pub(crate) replicas: BTreeMap<WorkerId, Replica>,
	/// The accepted manifest configuration; controls destinations, request versions, and every quorum.
	/// Shared with every published snapshot and replaced whole on reconfiguration.
	pub(crate) replica_set: std::sync::Arc<TabletReplicaSet>,
	pub(crate) retained: BTreeMap<u64, EntryRef>,
	/// Epoch boundaries remain after committed payloads are evicted.
	pub(crate) history_epochs: BTreeMap<u64, u64>,
	/// The sizes of the entries above cold, kept after committed payloads leave RAM; their
	/// total is the snapshot's `dirty_bytes`.
	pub(crate) above_cold: AboveCold,
	pub(crate) writes: BTreeMap<u64, PendingWrite>,
	pub(crate) memory_bytes: u64,
	pub(crate) next_op: u64,
	/// Appends issued before this operation ID carry no vote: a membership change retired them
	/// because uncommitted writes need fresh votes under the new set, so their late replies are
	/// not counted as acknowledgments.
	pub(crate) late_ack_floor: u64,
	pub(crate) claim_attempt: u32,
	pub(crate) observed_epoch: u64,
	pub(crate) last_client_seq: u64,
	pub(crate) cold_receipts: Vec<rivet_data::generated::shared_depot_fs_v1::FsWriteReceipt>,
	/// What the accepted open row's maintainer had applied above its cold: the epochs and
	/// receipts of those entries, so recovery replays only from the applied position. Set by
	/// every accepted open row.
	pub(crate) applied_history:
		Option<rivet_data::generated::depot_client_protocol_v1::VolumeAppliedHistory>,
	pub(crate) dedupe: super::types::DedupeWindow,
	pub(crate) barriers: BTreeMap<RequestId, Barrier>,
	/// This opening's claim entry, retained when recovery selected its history, before the
	/// replay, so that a quorum can hold it while the replay runs. Only an open whose claimed
	/// quorum did not settle the log writes one.
	pub(crate) claim_entry: Option<AppendedClaimEntry>,
	/// The claimed quorum that recovery selected history from all ended at one committed
	/// tail: the cold position, or a position a member knew was committed. No other writer
	/// can then hold a committed entry above it, so the client serves once its members match,
	/// with no claim entry. Set by every recovery start.
	pub(crate) claim_settled: bool,
	pub(crate) claim_quorum: BTreeSet<WorkerId>,
	/// The volume row names a fork base, so rows below the log may come from another volume.
	/// Set from every accepted open row.
	pub(crate) has_base_chain: bool,
	/// The current open adopted a claim the maintainer granted in its open response, so the
	/// maintainer already knows this client's claim. Set by every accepted open row.
	pub(crate) granted_claim: bool,
	/// The maintainer acknowledged this client's `VolumeCloseRequest`. A close that ended on
	/// an unreachable, moved or busy maintainer also stops the client but leaves this unset,
	/// so its owner knows the maintainer may not have the close yet.
	pub(crate) close_acknowledged: bool,
	/// The accepted open row describes a volume with nothing to recover: this open asked to
	/// create a missing volume, adopted a granted claim, and found cold at zero, no fork base and
	/// no member with an appended entry. Its claim entry is then the first entry, and writes
	/// admitted while the claim entry is in flight are sent right behind it instead of waiting for
	/// it to commit. Set by every accepted open row.
	pub(crate) opened_empty: bool,
	/// A creating opener's first writes, held in the payload store until the first open
	/// answers. The open sends them; on `created` they become the volume's first entries, and
	/// otherwise they are released and the owner writes them as usual. Empty after that open.
	pub(crate) initial_writes: Vec<InitialWrite>,
	/// The position a creating open committed the initial writes through, once accepted.
	pub(crate) created: Option<VolumeLogPosition>,
	/// Keys the creating open asks the maintainer to read back; see `VolumeCreateOptions.read`.
	pub(crate) read_keys: Option<rivet_data::generated::depot_client_protocol_v1::VolumeReadKeys>,
	/// The values the open read back, when the maintainer could answer them.
	pub(crate) read_back: Option<std::sync::Arc<Vec<Option<vbare::Bytes>>>>,
}

/// One initial write in the payload store, with its measured entry size and the retry
/// identity it is written with. The identities of an opening's initial writes carry
/// `client_seq` 1 to n in order, so writes after them continue the sequence.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InitialWrite {
	pub payload: PayloadId,
	pub encoded_bytes: u32,
	pub identity: super::types::MutationIdentity,
}

/// The claim entry this opening appended. Its client epoch is the client's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct AppendedClaimEntry {
	pub(crate) volume_seq: u64,
	pub(crate) written_at_unix_ms: i64,
}

/// A commit barrier captures the current end; confirmation also needs fresh replies from the current voting majorities.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Barrier {
	pub volume_seq: u64,
	pub confirm: bool,
	pub confirmed: BTreeSet<WorkerId>,
}

/// Frees the nodes an emptied map still holds. A `BTreeMap` keeps its root leaf after its
/// last entry is removed, so a map that saw one write keeps about a node's worth of memory
/// until it is replaced.
fn free_if_empty<K, V>(map: &mut BTreeMap<K, V>) {
	if map.is_empty() {
		*map = BTreeMap::new();
	}
}

fn free_set_if_empty<K>(set: &mut BTreeSet<K>) {
	if set.is_empty() {
		*set = BTreeSet::new();
	}
}

impl VolumeClientState {
	/// Frees the working containers a quiet client no longer uses: the maps and sets its
	/// writes, callers and open attempts filled, which stay allocated while empty, and the
	/// spare capacity of its vectors. Containers that still hold anything are kept as they
	/// are, so this changes no state the machine reads. A quiet volume holds no timer for
	/// this; the next burst's gap from the quiet transition decides.
	pub(crate) fn free_quiet_containers(&mut self) {
		free_if_empty(&mut self.waiting);
		free_set_if_empty(&mut self.open_attempts);
		free_set_if_empty(&mut self.loading_members);
		free_if_empty(&mut self.retained);
		free_if_empty(&mut self.history_epochs);
		self.above_cold.shrink_to_fit();
		free_if_empty(&mut self.writes);
		free_if_empty(&mut self.barriers);
		free_set_if_empty(&mut self.claim_quorum);
		for replica in self.replicas.values_mut() {
			free_if_empty(&mut replica.in_flight);
		}
		self.cold_receipts.shrink_to_fit();
		self.initial_writes.shrink_to_fit();
	}

	

	fn initialize(
		config: VolumeClientConfig,
		volume: VolumeIdentity,
		client: ClientIdentity,
		cold: VolumeLogPosition,
		replicas: Vec<(WorkerId, VolumeLogReplicaState)>,
	) -> anyhow::Result<Self> {
		anyhow::ensure!(
			config.finish_recovery_timeout_ms > 0
				&& config.max_pending_writes > 0
				&& config.max_memory_bytes > 0
				&& config.max_append_bytes > 128
				&& config.max_in_flight > 0
				&& config.append_timeout_ms > 0
				&& config.quorum_timeout_ms > 0
				&& config.open_timeout_ms > 0
				&& config.stall_timeout_ms > 0,
			"invalid volume client limits"
		);
		anyhow::ensure!(
			(1..=3).contains(&replicas.len()),
			"volume client requires one to three replicas"
		);

		// Require distinct in-sync replicas with the same claim and starting tail.
		let mut members = BTreeMap::new();
		for (worker, state) in replicas {
			anyhow::ensure!(
				state.status == VolumeLogReplicaStatus::InSync
					&& state.claimed_client.as_ref() == Some(&client)
					&& state.appended == cold,
				"replica is not claimed at the starting position"
			);
			anyhow::ensure!(
				members
					.insert(
						worker,
						Replica {
							repair: None,
							repair_attempt: 0,
							repair_avoid: None,
							matched: true,
							claim: Some(state.clone()),
							pending_claim: None,
							incarnation: state.replica_incarnation,
							acked: cold.volume_seq,
							sent: cold.volume_seq,
							in_flight: BTreeMap::new(),
							refused: None,
							repair_floor: None,
							repair_goal: None,
							blocked: None,
							append_limit_bytes: config.max_append_bytes,
						}
					)
					.is_none(),
				"duplicate replica worker"
			);
		}

		// Start accounting from the shared cold floor with no retained writes.
		let entry_frame_overhead = u32::try_from(volume.logical_key().encoded_len() + 16 + 9)?;
		Ok(Self {
			initial_open: None,
			initial_open_kept: false,
			purpose: OpenPurpose::Serve,
			required_recovery_members: BTreeSet::new(),
			entry_frame_overhead,
			config: std::sync::Arc::new(config.clone()),
			phase: VolumeClientPhase::Appending,
			volume,
			client,
			next_volume_seq: cold
				.volume_seq
				.checked_add(1)
				.ok_or_else(|| anyhow::anyhow!("volume sequence exhausted"))?,
			quiet_since_boot_ms: None,
			compact_when_quiet: false,
			commit_publishing: CommitPublishing {
				committed: cold.volume_seq,
			},
			cold,
			replica_set: std::sync::Arc::new(TabletReplicaSet {
				disks: Vec::new(),
				tablet_replica_set_version: 1,
				members: members.keys().map(|w| *w.as_bytes()).collect(),
				phase: TabletPhase::Stable,
			}),
			waiting: BTreeMap::new(),
			open_attempts: BTreeSet::new(),
			loading_members: BTreeSet::new(),
			replicas: members,
			retained: BTreeMap::new(),
			history_epochs: BTreeMap::new(),
			above_cold: AboveCold::default(),
			writes: BTreeMap::new(),
			memory_bytes: 0,
			next_op: 1,
			late_ack_floor: 0,
			claim_attempt: 0,
			observed_epoch: 0,
			last_client_seq: 0,
			dedupe: super::types::DedupeWindow::default(),
			cold_receipts: Vec::new(),
			applied_history: None,
			barriers: BTreeMap::new(),
			claim_entry: None,
			claim_settled: false,
			claim_quorum: BTreeSet::new(),
			has_base_chain: false,
			granted_claim: false,
			close_acknowledged: false,
			opened_empty: false,
			initial_writes: Vec::new(),
			created: None,
			read_keys: None,
			read_back: None,
		})
	}

	

	/// Starts without serving authority. The start hook opens through the maintainer.
	fn unopened(
		config: VolumeClientConfig,
		volume: VolumeIdentity,
		members: Vec<WorkerId>,
	) -> anyhow::Result<Self> {
		let zero = VolumeLogPosition {
			volume_seq: 0,
			client_epoch: 0,
		};

		let client = ClientIdentity {
			client_epoch: 0,
			client_incarnation: [0; 16],
		};

		let replicas = members
			.into_iter()
			.map(|w| {
				(
					w,
					VolumeLogReplicaState {
						creation: None,
						learner_replica_set_version: 0,
						// Opening replaces this placeholder with the maintainer's volume row.
						status: VolumeLogReplicaStatus::InSync,
						replica_incarnation: [0; 16],
						claimed_client: Some(client.clone()),
						claimed_holder: None,
						cold: zero.clone(),
						appended: zero.clone(),
						known_committed_volume_seq: 0,
						lost_entries: false,
					},
				)
			})
			.collect();

		// Reuse configuration validation, then discard the provisional serving evidence.
		let mut state = Self::initialize(config, volume, client, zero, replicas)?;
		state.phase = VolumeClientPhase::Opening { op_id: None };
		for replica in state.replicas.values_mut() {
			replica.matched = false;
			replica.claim = None;
		}
		Ok(state)
	}

	/// Opens using a complete manifest view, including the two voting lists during Joint.
	pub fn with_replica_set(
		config: VolumeClientConfig,
		volume: impl Into<VolumeIdentity>,
		replica_set: TabletReplicaSet,
	) -> anyhow::Result<Self> {
		super::membership::validate(&replica_set)?;
		let mut state = Self::unopened(
			config,
			volume.into(),
			replica_set
				.members
				.iter()
				.copied()
				.map(WorkerId::from_bytes)
				.collect(),
		)?;
		let max_append_bytes = state.config.max_append_bytes;
		for worker in super::membership::destinations(&replica_set) {
			state
				.replicas
				.entry(worker)
				.or_insert_with(|| super::membership::unobserved(max_append_bytes));
		}
		state.replica_set = std::sync::Arc::new(replica_set);
		Ok(state)
	}

	/// Gives the creating open its first writes, already in the payload store. The runner sets
	/// them before the first open; see [`VolumeClientState::create_if_absent`]. Fails unless
	/// their identities carry `client_seq` 1 to n with distinct request IDs and their bytes fit
	/// `max_initial_bytes`; the caller then starts without them.
	pub fn set_initial_writes(&mut self, writes: Vec<InitialWrite>) -> anyhow::Result<()> {
		let bytes = writes
			.iter()
			.map(|write| u64::from(write.encoded_bytes))
			.sum::<u64>();
		anyhow::ensure!(
			bytes <= self.config.max_initial_bytes,
			"initial writes exceed max_initial_bytes"
		);
		let mut request_ids = std::collections::BTreeSet::new();
		for (write, client_seq) in writes.iter().zip(1..) {
			anyhow::ensure!(
				write.identity.client_seq == client_seq,
				"initial write identities must carry client_seq 1 to n"
			);
			anyhow::ensure!(
				request_ids.insert(write.identity.request_id),
				"initial write request IDs must be distinct"
			);
		}
		self.initial_writes = writes;
		Ok(())
	}

	/// Names keys the creating open asks the maintainer to read back when the volume exists.
	pub fn set_read_keys(
		&mut self,
		read: Option<rivet_data::generated::depot_client_protocol_v1::VolumeReadKeys>,
	) {
		self.read_keys = read;
	}

	/// Lets the first open create the volume when it is missing. It has no effect once an open
	/// resolved the incarnation, so a reopen never creates.
	pub fn create_if_absent(&mut self) {
		if let VolumeIdentity::Unresolved {
			create_if_absent, ..
		} = &mut self.volume
		{
			*create_if_absent = true;
		}
	}

	

	/// Reuses completed create/open evidence without another maintainer round trip.
	fn from_response(
		config: VolumeClientConfig,
		client_incarnation: [u8; 16],
		opened: rivet_data::generated::depot_client_protocol_v1::VolumeOpenResponse,
		replica_set: TabletReplicaSet,
	) -> anyhow::Result<Self> {
		let key = VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
			volume_key: opened.record.volume_key(),
			volume_incarnation: opened.record.volume_incarnation,
		});
		let mut state = Self::with_replica_set(config, key, replica_set)?;
		state.client.client_incarnation = client_incarnation;
		state.initial_open = Some(opened);
		Ok(state)
	}

	/// Reuses the evidence of a create response kept past the create that returned it, without
	/// another maintainer round trip. Another worker may have claimed and written the volume since, so the client claims
	/// every member again before it serves instead of trusting the kept member states.
	pub fn from_kept_open_response(
		config: VolumeClientConfig,
		client_incarnation: [u8; 16],
		opened: rivet_data::generated::depot_client_protocol_v1::VolumeOpenResponse,
		replica_set: TabletReplicaSet,
	) -> anyhow::Result<Self> {
		let mut state = Self::from_response(config, client_incarnation, opened, replica_set)?;
		state.initial_open_kept = true;
		Ok(state)
	}

	

	/// Captures required destinations in the admitted assignment. Success still needs its
	/// phase quorums and exact matching proofs; the owner retains the attempt deadline.
	pub fn recover_with_required_members(
		config: VolumeClientConfig,
		volume_log_key: VolumeLogKey,
		replica_set: TabletReplicaSet,
		required_members: BTreeSet<WorkerId>,
	) -> anyhow::Result<Self> {
		let mut state = Self::with_replica_set(config, volume_log_key, replica_set)?;
		anyhow::ensure!(
			required_members
				.iter()
				.all(|worker| state.replicas.contains_key(worker)),
			"required recovery member is outside the admitted replica set"
		);
		state.purpose = OpenPurpose::Recover;
		state.required_recovery_members = required_members;
		Ok(state)
	}

	/// The largest encoded entry a quorum of replicas can still append: the append limit a
	/// voting majority has learned, less the request header and the entry's frame. A replica
	/// with a smaller limit blocks alone on an entry above it, while an entry above this
	/// limit could never commit, so admission refuses it for good.
	pub(crate) fn quorum_entry_limit_bytes(&self) -> u32 {
		let quorum_limit = super::membership::position(&self.replica_set, |worker| {
			self.replicas
				.get(worker)
				.map_or(u64::from(self.config.max_append_bytes), |replica| {
					u64::from(replica.append_limit_bytes)
				})
		});
		(quorum_limit.min(u64::from(u32::MAX)) as u32)
			.saturating_sub(128)
			.saturating_sub(self.entry_frame_overhead)
	}

	/// The entry size the owner may write: the quorum's entry limit within the memory budget.
	/// Published as the snapshot's `max_mutation_bytes`, so the owner batches under it.
	pub(crate) fn max_mutation_bytes(&self) -> u32 {
		self.quorum_entry_limit_bytes()
			.min(self.config.max_memory_bytes.min(u64::from(u32::MAX)) as u32)
	}

	/// The append request bytes one entry of `encoded_bytes` takes on its own: the request
	/// header plus the entry and its frame. This is the size `send` compares to the append
	/// limit, so an entry whose single-entry request exceeds the limit can never be sent.
	pub(crate) fn single_entry_request_bytes(&self, encoded_bytes: u32) -> u64 {
		128 + u64::from(encoded_bytes) + u64::from(self.entry_frame_overhead)
	}

	/// Allocates an operation identity that is never reused within this volume client handle.
	pub(crate) fn operation(&mut self) -> OpId {
		let id = OpId(self.next_op);
		self.next_op = self.next_op.checked_add(1).expect("operation ID exhausted");
		id
	}

	/// Resolves the cold boundary or retained history without inventing a missing epoch.
	pub(crate) fn position(&self, seq: u64) -> Option<VolumeLogPosition> {
		if seq == self.cold.volume_seq {
			Some(self.cold.clone())
		} else {
			self.retained
				.get(&seq)
				.map(|entry| entry.id.clone())
				.or_else(|| {
					(seq < self.next_volume_seq)
						.then(|| {
							self.history_epochs
								.range(..=seq)
								.next_back()
								.map(|(_, epoch)| VolumeLogPosition {
									volume_seq: seq,
									client_epoch: *epoch,
								})
						})
						.flatten()
				})
		}
	}
}

impl rivet_state_machine::StateMachineState for VolumeClientState {}
