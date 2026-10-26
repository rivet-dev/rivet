//! Pure transitions for one log replica, embedded in the owning shard state machine.
//!
//! The shard resolves the log key and admits the request against current topology,
//! tablet version, and authority before calling these client operations. This core
//! enforces client claims and log matching; the shard admits grant-authorized
//! backfill. A successful transition is not a disk acknowledgment.
//!
//! Every rejection leaves state and entries unchanged. Successful replies capture
//! accepted state, including for duplicate requests; the shard must delay delivery
//! until that state is on disk. Repair incarnation randomness is supplied by the
//! caller, never drawn by the core.

use crate::log_shard_machine::{
	Effects,
	state::{
		LogShardState, fail,
		memory_batches::{
			ForgetResult, GrantBatch, PendingReply, ReplicaRejection, WaitingReply, reject,
		},
		payloads::ReplicaTotals,
	},
	types::{
		Authority, InitialEntriesDrop, LoadedVolumeReplicaCommand, LogShardCommand,
		LogShardRequest, RequestId,
	},
};
use depot_types::{VolumeLogKey, payload_codec::PayloadDigest};
use rivet_data::generated::depot_log_shard_v1::{EntryFrame, ForgetFrame, Frame, StateFrame};
use rivet_data::generated::{
	depot_log_shard_v1::{
		PayloadCodec, PayloadWrite, VolumeLogEntry, VolumeLogEntryBody, VolumeLogReplicaState,
	},
	depot_storage_worker_protocol_v1::{self as protocol, RejectResponse},
	shared_depot_types_v1::{ClientIdentity, VolumeLogPosition, VolumeLogReplicaStatus},
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use vbare::Encode;

/// Pure substate of the log shard state machine for a volume log.
/// It emits no effects and sends no replies; the owning shard handles disk writes
/// and delivery. Only shard transitions change its status.
///
/// A loaded volume has three log replicas because a client opened it. An unloaded
/// volume has no replicas, only its row and blocks in the cold store. The shard's
/// startup recovery pass restores replicas for volumes that were loaded before restart.
///
/// Health is independent of tablet membership, which the coordinator keeps in the manifest.
/// `InSync` holds the complete history through appended and its acknowledgements count.
/// `Repairing` is known to lack history above cold and cannot count; corruption recovery
/// or a repairing truncate sets it, and MarkInSync restores it after copying the history.
/// `Deleted` means the maintainer is deleting the volume. A synced forget removes the
/// replica from the shard entirely.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoadedVolumeReplica {
	pub volume_log_key: VolumeLogKey,

	/// Last synced state, written only by the shard, never read by replica rules.
	pub disk_state: Option<VolumeLogReplicaState>,

	/// Accepted state, on disk only once the shard batch holding the change is written to disk.
	memory_state: VolumeLogReplicaState,

	/// Every entry above `cold`, whether still in memory or already written to disk. Writing to disk does not
	/// remove entries; a trim removes those it passes. Truncate can also
	/// discard entries. The shard's filling batch is the only write buffer.
	pub(crate) entries: VecDeque<ReplicaEntry>,

	/// How many leading entries no longer hold their payload in memory. They are all on
	/// disk, and no entry after them is released, so a read below this index goes to disk
	/// and every other index rule is unchanged.
	pub(crate) released: usize,

	/// The synced position the last status notice of this replica covered, in memory only.
	/// The next notice carries entries only above it. It is a hint for avoiding resending
	/// entries, never a correctness input: the maintainer applies carried entries only when
	/// they start right after its applied position and a quorum proves them committed.
	pub(crate) noticed_volume_seq: u64,
}

/// One retained log entry: its header, its payload and its place in the byte and batch
/// order. Payloads are held by reference, so copying a replica copies no payload bytes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplicaEntry {
	/// Prefix byte total makes status reads constant time; rebuilt from entries at recovery.
	/// `end_bytes - start_bytes` is the entry's exact encoded size. A u64 total saturates
	/// only after 16 EiB of one replica's entries, so it is stored narrow to keep each retained
	/// entry small.
	pub end_bytes: u64,
	pub start_bytes: u64,
	/// Assigned by the shard when the containing batch is dispatched.
	pub batch_log_seq: Option<u64>,
	pub written_at_unix_ms: i64,
	pub volume_seq: u64,
	pub client_epoch: u64,
	pub body: EntryBody,
}

/// The body of a retained entry. A payload write keeps its identity and digest beside the
/// payload, so entries compare without touching payload bytes where the digest decides.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntryBody {
	ClaimEntry,
	PayloadWrite {
		/// The payload's version family and version; family 0 is untracked. Shards never
		/// decode the payload, so these are what their FORMAT marks record.
		payload_family: u16,
		payload_version: u16,
		client_seq: u64,
		request_id: [u8; 16],
		payload_digest: PayloadDigest,
		/// How `payload` is encoded. Shards never decode it.
		codec: PayloadCodec,
		payload_len: u64,
		payload: Payload,
	},
}

/// Where a payload write's bytes are.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Payload {
	/// Held in memory, shared with every copy of the replica.
	Retained(bytes::Bytes),
	/// Only on disk. The entry keeps its header, digest and length.
	Released,
}

impl ReplicaEntry {
	/// Takes ownership of a log entry's payload without copying it. `start_bytes` is the
	/// replica's byte total before this entry.
	pub fn from_log_entry(
		entry: VolumeLogEntry,
		start_bytes: u64,
		batch_log_seq: Option<u64>,
	) -> Self {
		let end_bytes = start_bytes.saturating_add(depot_types::log_shard::entry_bytes(&entry));
		let body = match entry.body {
			VolumeLogEntryBody::ClaimEntry => EntryBody::ClaimEntry,
			VolumeLogEntryBody::PayloadWrite(write) => EntryBody::PayloadWrite {
				payload_family: write.payload_family,
				payload_version: write.payload_version,
				client_seq: write.client_seq,
				request_id: write.request_id,
				payload_digest: write.payload_digest,
				codec: write.codec,
				payload_len: write.payload.len() as u64,
				payload: Payload::Retained(bytes::Bytes::from(write.payload)),
			},
		};
		Self {
			end_bytes,
			start_bytes,
			batch_log_seq,
			written_at_unix_ms: entry.written_at_unix_ms,
			volume_seq: entry.volume_seq,
			client_epoch: entry.client_epoch,
			body,
		}
	}

	/// The exact encoded size of the entry, as `depot_types::log_shard::entry_bytes` reports.
	pub fn encoded_bytes(&self) -> u64 {
		self.end_bytes - self.start_bytes
	}

	pub fn position(&self) -> VolumeLogPosition {
		VolumeLogPosition {
			volume_seq: self.volume_seq,
			client_epoch: self.client_epoch,
		}
	}

	/// The wire form of the entry, sharing its payload, or `None` once the payload was
	/// released and only disk holds it.
	pub fn log_entry(&self) -> Option<VolumeLogEntry> {
		let body = match &self.body {
			EntryBody::ClaimEntry => VolumeLogEntryBody::ClaimEntry,
			EntryBody::PayloadWrite {
				payload_family,
				payload_version,
				client_seq,
				request_id,
				payload_digest,
				codec,
				payload_len: _,
				payload: Payload::Retained(payload),
			} => VolumeLogEntryBody::PayloadWrite(PayloadWrite {
				payload_family: *payload_family,
				payload_version: *payload_version,
				client_seq: *client_seq,
				request_id: *request_id,
				payload_digest: *payload_digest,
				codec: codec.clone(),
				payload: payload.clone(),
			}),
			EntryBody::PayloadWrite {
				payload: Payload::Released,
				..
			} => return None,
		};
		Some(VolumeLogEntry {
			written_at_unix_ms: self.written_at_unix_ms,
			volume_seq: self.volume_seq,
			client_epoch: self.client_epoch,
			body,
		})
	}

	/// Whether the entry's payload has left memory.
	pub fn is_released(&self) -> bool {
		matches!(
			self.body,
			EntryBody::PayloadWrite {
				payload: Payload::Released,
				..
			}
		)
	}

	/// Drops the payload from memory, keeping the header. Only an entry already on disk
	/// may release it.
	pub(crate) fn release_payload(&mut self) {
		if let EntryBody::PayloadWrite { payload, .. } = &mut self.body {
			*payload = Payload::Released;
		}
	}

	/// Whether `entry` has this entry's header: time, position, kind, and for a payload
	/// write its identity, digest and length. One writer holds a client epoch and writes one
	/// entry per position, so an equal header names the same write.
	pub fn same_header(&self, entry: &VolumeLogEntry) -> bool {
		if self.written_at_unix_ms != entry.written_at_unix_ms
			|| self.volume_seq != entry.volume_seq
			|| self.client_epoch != entry.client_epoch
		{
			return false;
		}
		match (&self.body, &entry.body) {
			(EntryBody::ClaimEntry, VolumeLogEntryBody::ClaimEntry) => true,
			(
				EntryBody::PayloadWrite {
					payload_family,
					payload_version,
					client_seq,
					request_id,
					payload_digest,
					codec,
					payload_len,
					payload: _,
				},
				VolumeLogEntryBody::PayloadWrite(write),
			) => {
				*payload_family == write.payload_family
					&& *payload_version == write.payload_version
					&& *client_seq == write.client_seq
					&& *request_id == write.request_id
					&& *payload_digest == write.payload_digest
					&& *codec == write.codec
					&& *payload_len == write.payload.len() as u64
			}
			(EntryBody::ClaimEntry, VolumeLogEntryBody::PayloadWrite(_))
			| (EntryBody::PayloadWrite { .. }, VolumeLogEntryBody::ClaimEntry) => false,
		}
	}

	/// Whether `entry` is exactly this entry, comparing payload bytes while they are held.
	/// A released payload is compared by its digest and length. One writer holds a client
	/// epoch and sends one entry per position, so an equal header at the same position is a
	/// resend of the same entry; the digest only catches a writer that broke that rule.
	pub fn same_entry(&self, entry: &VolumeLogEntry) -> bool {
		if !self.same_header(entry) {
			return false;
		}
		match (&self.body, &entry.body) {
			(
				EntryBody::PayloadWrite {
					payload: Payload::Retained(payload),
					..
				},
				VolumeLogEntryBody::PayloadWrite(write),
			) => payload[..] == write.payload[..],
			(
				EntryBody::PayloadWrite {
					payload: Payload::Released,
					..
				},
				VolumeLogEntryBody::PayloadWrite(_),
			)
			| (EntryBody::ClaimEntry, VolumeLogEntryBody::ClaimEntry)
			| (EntryBody::ClaimEntry, VolumeLogEntryBody::PayloadWrite(_))
			| (EntryBody::PayloadWrite { .. }, VolumeLogEntryBody::ClaimEntry) => true,
		}
	}
}

/// The exact disk frames required by a successful replica transition.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplicaChange {
	/// No new frames; the shard still waits for previously accepted work to sync.
	Unchanged,

	/// The shard writes these entries with the current committed position.
	AppendedEntries(Vec<VolumeLogEntry>),

	/// The shard writes the complete resulting state.
	StateChanged,

	/// The shard writes a forget frame and removes the replica when that write completes.
	Removed,
}

impl LoadedVolumeReplica {
	/// Restores disk state only when the retained entries form a complete,
	/// internally ordered suffix ending at the recorded tail.
	pub(crate) fn restore(
		volume_log_key: VolumeLogKey,
		state: VolumeLogReplicaState,
		mut entries: Vec<ReplicaEntry>,
	) -> anyhow::Result<Self> {
		anyhow::ensure!(
			state.cold.volume_seq <= state.known_committed_volume_seq
				&& state.known_committed_volume_seq <= state.appended.volume_seq,
			"invalid committed position"
		);

		let mut previous = state.cold.clone();

		// An authoritative trim can pass a divergent, uncommitted local entry.
		// The retained suffix may begin below the floor's epoch until repair,
		// but epochs within that suffix must still be nondecreasing.
		for (index, entry) in entries.iter().enumerate() {
			anyhow::ensure!(
				previous.volume_seq.checked_add(1) == Some(entry.volume_seq)
					&& (index == 0 || entry.client_epoch >= previous.client_epoch),
				"invalid recovered entry sequence"
			);
			previous = VolumeLogPosition {
				volume_seq: entry.volume_seq,
				client_epoch: entry.client_epoch,
			};
		}

		anyhow::ensure!(
			previous == state.appended
				|| (state.status == VolumeLogReplicaStatus::Deleted && entries.is_empty()),
			"recovered tail differs from state"
		);

		let mut end_bytes = 0;
		for retained in &mut entries {
			let length = retained.encoded_bytes();
			retained.start_bytes = end_bytes;
			end_bytes = end_bytes.saturating_add(length);
			retained.end_bytes = end_bytes;
		}
		// Recovery may release the oldest payloads; they must form a prefix.
		let released = entries
			.iter()
			.position(|entry| {
				matches!(
					entry.body,
					EntryBody::PayloadWrite {
						payload: Payload::Retained(_),
						..
					}
				)
			})
			.unwrap_or(entries.len());
		anyhow::ensure!(
			entries[released..].iter().all(|entry| !entry.is_released()),
			"recovered released payloads are not a prefix"
		);
		Ok(Self {
			volume_log_key,
			disk_state: Some(state.clone()),
			// A recovered replica's maintainer reads anything it has not applied, so the
			// notices that follow recovery carry no old entries.
			noticed_volume_seq: state.appended.volume_seq,
			memory_state: state,
			entries: entries.into(),
			released,
		})
	}

	/// Starts an empty replica at a caller-established committed position.
	/// Use `(0, 0)` for a new log, or the cold position for a learner. The
	/// supplied incarnation identifies this replica lifetime, independent of clients.
	pub fn new(
		volume_log_key: VolumeLogKey,
		replica_incarnation: [u8; 16],
		cold: VolumeLogPosition,
		status: VolumeLogReplicaStatus,
	) -> Self {
		Self {
			volume_log_key,
			disk_state: None,
			memory_state: VolumeLogReplicaState {
				creation: None,
				learner_replica_set_version: 0,
				status,
				replica_incarnation,
				claimed_client: None,
				claimed_holder: None,
				appended: cold.clone(),
				known_committed_volume_seq: cold.volume_seq,
				cold,
				lost_entries: false,
			},
			entries: VecDeque::new(),
			released: 0,
			noticed_volume_seq: 0,
		}
	}

	/// Accepted state, on disk only once the shard batch holding the change is written to disk.
	/// Frees the entries buffer once nothing sits above cold, so a quiet replica holds only
	/// its states. A volume that writes again allocates a fresh buffer on its next append.
	fn release_empty_tail(&mut self) {
		if self.entries.is_empty() {
			self.entries = VecDeque::new();
		}
	}

	pub fn state(&self) -> &VolumeLogReplicaState {
		&self.memory_state
	}

	/// Every entry above `cold`, in consecutive volume sequence order. Entries
	/// remain after reaching disk; a trim removes those it passes. Truncate and
	/// truncate can also discard entries. The shard's filling batch is the only write buffer.
	pub fn entries(&self) -> &VecDeque<ReplicaEntry> {
		&self.entries
	}

	/// Encoded bytes of every retained entry, whether or not its payload is held. This is
	/// the log tail above cold that the replica tracks, not its memory use.
	pub fn entry_bytes(&self) -> u64 {
		match (self.entries.front(), self.entries.back()) {
			(Some(front), Some(back)) => back.end_bytes - front.start_bytes,
			(None, _) | (_, None) => 0,
		}
	}

	/// How many leading entries have released their payloads.
	pub fn released(&self) -> usize {
		self.released
	}

	/// Encoded bytes of the entries after the released prefix, whose payloads are held in
	/// memory. Constant time from the prefix byte totals.
	pub fn retained_payload_bytes(&self) -> u64 {
		let Some(back) = self.entries.back() else {
			return 0;
		};
		let start = match self.released.checked_sub(1) {
			Some(last_released) => self.entries[last_released].end_bytes,
			None => self.entries.front().map_or(0, |front| front.start_bytes),
		};
		back.end_bytes - start
	}

	/// Releases payloads from the oldest held entry forward while more than `keep_bytes`
	/// stay held. It stops at the first entry not on disk through `synced_through`, so only
	/// synced payloads leave memory. Returns the bytes released.
	pub(crate) fn release_synced(&mut self, synced_through: u64, keep_bytes: u64) -> u64 {
		let before = self.retained_payload_bytes();
		while self.released < self.entries.len() && self.retained_payload_bytes() > keep_bytes {
			let entry = &mut self.entries[self.released];
			if !entry
				.batch_log_seq
				.is_some_and(|batch_log_seq| batch_log_seq <= synced_through)
			{
				break;
			}
			entry.release_payload();
			self.released += 1;
		}
		before - self.retained_payload_bytes()
	}

	/// Checks that released payloads form a synced prefix and every later payload is held.
	pub(crate) fn validate_payloads(&self, synced_through: u64) -> anyhow::Result<()> {
		anyhow::ensure!(
			self.released <= self.entries.len(),
			"released past the tail"
		);
		for (index, entry) in self.entries.iter().enumerate() {
			if index < self.released {
				anyhow::ensure!(
					entry
						.batch_log_seq
						.is_some_and(|batch_log_seq| batch_log_seq <= synced_through),
					"released payload is not on disk"
				);
			} else {
				anyhow::ensure!(!entry.is_released(), "released payload after a held one");
			}
		}
		Ok(())
	}

	/// Reports only the disk-committed suffix. Speculative tails never make a volume due.
	/// Prefix totals and sequence indexing avoid walking retained payloads on every poll.
	pub(crate) fn cold_write_status(
		&self,
		disk: &VolumeLogReplicaState,
		disk_pressure: bool,
	) -> Option<protocol::ColdWriteStatus> {
		if disk.status != VolumeLogReplicaStatus::InSync
			|| disk.known_committed_volume_seq <= disk.cold.volume_seq
		{
			return None;
		}
		let base = self.entries.front()?.volume_seq;
		let first_index =
			usize::try_from(disk.cold.volume_seq.checked_add(1)?.checked_sub(base)?).ok()?;
		let last_index =
			usize::try_from(disk.known_committed_volume_seq.checked_sub(base)?).ok()?;
		let first = self.entries.get(first_index)?;
		let last = self.entries.get(last_index)?;
		let start_bytes = first.start_bytes;
		Some(protocol::ColdWriteStatus {
			first_write_unix_ms: first.written_at_unix_ms,
			committed_bytes: last.end_bytes - start_bytes,
			disk_pressure,
		})
	}

	/// Fences older clients. An exact repeated claim succeeds without changing state.
	/// A fresh epoch exceeds the claimed client, every retained entry epoch and cold.clientEpoch.
	/// The cold epoch fences pre-unload clients when a reloaded replica has no entries.
	/// Different random incarnations distinguish clients choosing the same epoch.
	///
	pub fn claim(
		&mut self,
		client: &ClientIdentity,
		holder: Option<&rivet_data::generated::shared_depot_types_v1::VolumeHolder>,
	) -> Result<ReplicaChange, RejectResponse> {
		self.claim_with(client, holder, false)
	}

	/// [`Self::claim`] for a request whose tablet grant this shard already checked: a tablet
	/// maintainer claiming its own log, or a maintainer-granted open claim.
	///
	/// A replica emptied by corruption recovery keeps only the last claim a checksum-valid frame
	/// recorded, and may have lost a newer one, so it cannot tell the writer that reached its
	/// cold position from another client at that epoch. Under a
	/// checked grant only the current maintainer can send the claim, so the replica takes it at
	/// the cold epoch; a refusal would read to the current writer as a newer owner and make it
	/// give up the log instead of repairing the replica. A plain client claim gets no such
	/// exception: the lost claim may have fenced that client, and claiming it again would let a stale
	/// writer mark the replica in sync and commit beside its successor. The claim grants
	/// nothing on its own: the replica stays `Repairing`, which no quorum counts, until a
	/// client marks it in sync.
	pub fn claim_granted(
		&mut self,
		client: &ClientIdentity,
		holder: Option<&rivet_data::generated::shared_depot_types_v1::VolumeHolder>,
	) -> Result<ReplicaChange, RejectResponse> {
		self.claim_with(client, holder, true)
	}

	fn claim_with(
		&mut self,
		client: &ClientIdentity,
		holder: Option<&rivet_data::generated::shared_depot_types_v1::VolumeHolder>,
		grant_checked: bool,
	) -> Result<ReplicaChange, RejectResponse> {
		self.check_live()?;
		let floor = self
			.memory_state
			.cold
			.client_epoch
			.max(self.memory_state.appended.client_epoch);
		// Repair marking keeps the last claim a valid frame recorded. A kept claim below the cold
		// epoch cannot belong to the writer that reached cold, so it does not stop that
		// writer's grant-checked repeat claim; a kept claim at or above it fences as usual.
		let lost_claim = grant_checked
			&& self.memory_state.lost_entries
			&& self
				.memory_state
				.claimed_client
				.as_ref()
				.is_none_or(|claimed| claimed.client_epoch < floor)
			&& self.memory_state.appended == self.memory_state.cold;
		if client.client_epoch < floor
			|| (client.client_epoch == floor
				&& !lost_claim
				&& self.memory_state.claimed_client.as_ref() != Some(client))
		{
			return Err(RejectResponse::StaleClientEpoch(
				protocol::StaleClientEpoch {
					state: self.memory_state.clone(),
				},
			));
		}

		// An exact retry is harmless, but a different identity cannot reuse its epoch.
		if let Some(claimed) = &self.memory_state.claimed_client {
			if client == claimed {
				return Ok(ReplicaChange::Unchanged);
			}

			if client.client_epoch == claimed.client_epoch {
				return Err(RejectResponse::ClientConflict(protocol::ClientConflict {
					state: self.memory_state.clone(),
				}));
			}
		}

		if self
			.memory_state
			.claimed_client
			.as_ref()
			.is_some_and(|claimed| client.client_epoch < claimed.client_epoch)
		{
			return Err(RejectResponse::StaleClientEpoch(
				protocol::StaleClientEpoch {
					state: self.memory_state.clone(),
				},
			));
		}

		// Publish the claim only after every fence check succeeds. A claim names its holder; one
		// that does not keeps the holder only for the same client incarnation, which is the
		// same process, and otherwise leaves it unknown.
		let same_process = self
			.memory_state
			.claimed_client
			.as_ref()
			.is_some_and(|claimed| claimed.client_incarnation == client.client_incarnation);
		match holder {
			Some(holder) => self.memory_state.claimed_holder = Some(holder.clone()),
			None if same_process => {}
			None => self.memory_state.claimed_holder = None,
		}
		self.memory_state.claimed_client = Some(client.clone());
		Ok(ReplicaChange::StateChanged)
	}

	/// Extends a matched suffix, or acknowledges a duplicate already present in full.
	/// previous must match appended to extend the log; this orders entries without timing
	/// assumptions. A position ahead leaves a VolumeSeqGap (the missing entries come first); the same
	/// sequence with another epoch is Diverged (truncate first). A duplicate already-held
	/// suffix may start below appended, but partial overlaps cannot overwrite entries.
	pub fn append(
		&mut self,
		client: &ClientIdentity,
		previous: VolumeLogPosition,
		entries: Vec<VolumeLogEntry>,
		committed_volume_seq: u64,
	) -> Result<ReplicaChange, RejectResponse> {
		self.check_client(client)?;
		self.append_entries(
			Some(client.client_epoch),
			previous,
			entries,
			committed_volume_seq,
		)
	}

	/// Copies only entries the maintainer has already established as committed.
	/// Grant fencing belongs to the shard; matching still protects the local prefix.
	pub fn append_committed(
		&mut self,
		previous: VolumeLogPosition,
		entries: Vec<VolumeLogEntry>,
		committed_volume_seq: u64,
	) -> Result<ReplicaChange, RejectResponse> {
		self.check_live()?;
		if entries
			.last()
			.is_some_and(|entry| entry.volume_seq > committed_volume_seq)
		{
			return Err(self.volume_seq_gap());
		}
		self.append_entries(None, previous, entries, committed_volume_seq)
	}

	/// Validates a complete suffix before applying it under the admitted authority.
	fn append_entries(
		&mut self,
		client_epoch: Option<u64>,
		previous: VolumeLogPosition,
		entries: Vec<VolumeLogEntry>,
		committed_volume_seq: u64,
	) -> Result<ReplicaChange, RejectResponse> {
		if entries.is_empty() {
			return Err(self.volume_seq_gap());
		}
		if previous.volume_seq < self.memory_state.cold.volume_seq {
			return Err(self.below_cold());
		}

		let Some(local_previous) = self.position(previous.volume_seq) else {
			return Err(self.volume_seq_gap());
		};

		if local_previous != previous {
			return Err(RejectResponse::Diverged(protocol::Diverged {
				state: self.memory_state.clone(),
			}));
		}

		// Validate the whole batch before changing anything, including commit progress.
		let mut last = previous.clone();
		for entry in &entries {
			if last.volume_seq.checked_add(1) != Some(entry.volume_seq)
				|| entry.client_epoch < last.client_epoch
				|| client_epoch.is_some_and(|epoch| entry.client_epoch > epoch)
			{
				return Err(self.volume_seq_gap());
			}
			last = volume_log_position(entry);
		}

		// Existing suffixes must match in full; only a new tail extends memory state.
		let appended_entries =
			previous.volume_seq == self.memory_state.appended.volume_seq && !entries.is_empty();
		if previous.volume_seq < self.memory_state.appended.volume_seq {
			if entries.iter().any(|entry| {
				self.position(entry.volume_seq)
					.is_some_and(|replica_entry| replica_entry.client_epoch != entry.client_epoch)
			}) {
				return Err(RejectResponse::Diverged(protocol::Diverged {
					state: self.memory_state.clone(),
				}));
			}

			let offset = usize::try_from(previous.volume_seq - self.memory_state.cold.volume_seq)
				.map_err(|_| self.volume_seq_gap())?;
			let retained = self.entries.iter().skip(offset).take(entries.len());
			let matched = retained.len() == entries.len()
				&& retained
					.zip(entries.iter())
					.all(|(replica_entry, entry)| replica_entry.same_entry(entry));
			if !matched {
				return Err(self.volume_seq_gap());
			}
		} else {
			let mut end_bytes = self.entries.back().map_or(0, |entry| entry.end_bytes);
			self.entries.extend(entries.iter().cloned().map(|entry| {
				let retained = ReplicaEntry::from_log_entry(entry, end_bytes, None);
				end_bytes = retained.end_bytes;
				retained
			}));
			self.memory_state.appended = last;
		}

		// Commit knowledge advances monotonically and never passes the local tail.
		let committed = self
			.memory_state
			.known_committed_volume_seq
			.max(committed_volume_seq.min(self.memory_state.appended.volume_seq));
		let commit_changed = committed != self.memory_state.known_committed_volume_seq;
		self.memory_state.known_committed_volume_seq = committed;
		Ok(if appended_entries {
			ReplicaChange::AppendedEntries(entries)
		} else if commit_changed {
			ReplicaChange::StateChanged
		} else {
			ReplicaChange::Unchanged
		})
	}

	/// Drops the suffix after an existing position, conditional on the observed tail.
	/// Recovery callers establish which suffix is safe to remove. Commit knowledge
	/// is clamped to the kept position so it never points beyond the retained log.
	/// Neither form moves cold. A repairing truncate empties everything above cold and
	/// replaces the incarnation, invalidating old proofs and MarkInSync requests. The
	/// caller supplies a random 16-byte incarnation, assumed never to repeat.
	/// Compares an operator's observed incarnation in the same transition as truncation.
	/// Recreating or resyncing a replica invalidates a delayed request even when its
	/// client and appended position happen to be unchanged.
	pub fn truncate_observed(
		&mut self,
		client: &ClientIdentity,
		expected_replica_incarnation: Option<[u8; 16]>,
		expected_appended: VolumeLogPosition,
		keep_through_volume_seq: u64,
		mark_repairing: bool,
		new_replica_incarnation: Option<[u8; 16]>,
	) -> Result<ReplicaChange, RejectResponse> {
		self.check_client(client)?;
		if expected_replica_incarnation
			.is_some_and(|expected| expected != self.memory_state.replica_incarnation)
		{
			return Err(RejectResponse::VolumeLogReplicaIncarnationMismatch(
				protocol::VolumeLogReplicaIncarnationMismatch {
					state: self.memory_state.clone(),
				},
			));
		}
		self.truncate(
			client,
			expected_appended,
			keep_through_volume_seq,
			mark_repairing,
			new_replica_incarnation,
		)
	}

	pub fn truncate(
		&mut self,
		client: &ClientIdentity,
		expected_appended: VolumeLogPosition,
		keep_through_volume_seq: u64,
		mark_repairing: bool,
		new_replica_incarnation: Option<[u8; 16]>,
	) -> Result<ReplicaChange, RejectResponse> {
		self.check_client(client)?;
		self.check_appended(&expected_appended)?;

		if keep_through_volume_seq < self.memory_state.cold.volume_seq {
			return Err(self.below_cold());
		}

		// Repair discards only the suffix above the existing cold record, never moving it.
		if mark_repairing {
			if keep_through_volume_seq != self.memory_state.cold.volume_seq {
				return Err(self.volume_seq_gap());
			}
			let Some(incarnation) =
				new_replica_incarnation.filter(|i| *i != self.memory_state.replica_incarnation)
			else {
				return Err(RejectResponse::VolumeLogReplicaIncarnationMismatch(
					protocol::VolumeLogReplicaIncarnationMismatch {
						state: self.memory_state.clone(),
					},
				));
			};
			self.entries = VecDeque::new();
			self.released = 0;
			self.memory_state.appended = self.memory_state.cold.clone();
			self.memory_state.known_committed_volume_seq = self.memory_state.cold.volume_seq;
			self.memory_state.replica_incarnation = incarnation;
			self.memory_state.status = VolumeLogReplicaStatus::Repairing;
			// The entries written above cold from now on are new to the maintainer.
			self.noticed_volume_seq = self
				.noticed_volume_seq
				.min(self.memory_state.cold.volume_seq);
			return Ok(ReplicaChange::StateChanged);
		}

		let kept = self
			.position(keep_through_volume_seq)
			.ok_or_else(|| self.volume_seq_gap())?;
		let count = usize::try_from(keep_through_volume_seq - self.memory_state.cold.volume_seq)
			.map_err(|_| self.volume_seq_gap())?;
		if kept == self.memory_state.appended {
			return Ok(ReplicaChange::Unchanged);
		}

		// Discard the suffix only after the observed and retained positions are validated.
		self.entries.truncate(count);
		self.release_empty_tail();
		self.released = self.released.min(self.entries.len());
		self.memory_state.appended = kept;
		self.memory_state.known_committed_volume_seq = self
			.memory_state
			.known_committed_volume_seq
			.min(keep_through_volume_seq);
		// Entries written over the cut suffix are new to the maintainer, so the next notice
		// carries them again from the kept position.
		self.noticed_volume_seq = self.noticed_volume_seq.min(keep_through_volume_seq);
		Ok(ReplicaChange::StateChanged)
	}

	/// Records the quorum of acknowledgements only the client can observe. The same hint
	/// rides every append; the maintainer derives commitment from the replica tails instead
	/// of trusting it, so a serving client sends this only as a membership probe, a cold
	/// refresh or a confirm barrier. The recorded position cannot pass appended.
	///
	/// Progress applies only while the tail was written by the announcing writer's epoch.
	/// Log matching then proves the whole retained log is a prefix of that writer's history.
	/// A writer that claimed this replica without matching it would otherwise mark a deposed
	/// writer's divergent suffix committed; such an announcement leaves the replica unchanged
	/// until the writer's own entries reach it.
	pub fn advance_committed(
		&mut self,
		client: &ClientIdentity,
		committed_volume_seq: u64,
	) -> Result<ReplicaChange, RejectResponse> {
		self.check_client(client)?;
		if self.memory_state.appended.client_epoch != client.client_epoch {
			return Ok(ReplicaChange::Unchanged);
		}
		let committed = committed_volume_seq.min(self.memory_state.appended.volume_seq);
		if committed <= self.memory_state.known_committed_volume_seq {
			return Ok(ReplicaChange::Unchanged);
		}
		self.memory_state.known_committed_volume_seq = committed;
		Ok(ReplicaChange::StateChanged)
	}

	/// Promotes the confirmed incarnation and tail, or its strictly newer same-epoch cold image.
	/// A racing grant-authorized trim may replace that prefix, but no unverified hot suffix counts.
	pub fn mark_in_sync(
		&mut self,
		client: &ClientIdentity,
		replica_incarnation: [u8; 16],
		expected_appended: VolumeLogPosition,
	) -> Result<ReplicaChange, RejectResponse> {
		self.check_client(client)?;
		if replica_incarnation != self.memory_state.replica_incarnation {
			return Err(RejectResponse::VolumeLogReplicaIncarnationMismatch(
				protocol::VolumeLogReplicaIncarnationMismatch {
					state: self.memory_state.clone(),
				},
			));
		}

		let newer_cold = self.memory_state.appended == self.memory_state.cold
			&& !self.memory_state.lost_entries
			&& self.memory_state.cold.volume_seq > expected_appended.volume_seq
			&& self.memory_state.cold.client_epoch == expected_appended.client_epoch
			&& self.memory_state.cold.client_epoch <= client.client_epoch;
		if !newer_cold {
			self.check_appended(&expected_appended)?;
		}

		if self.memory_state.status == VolumeLogReplicaStatus::InSync {
			return Ok(ReplicaChange::Unchanged);
		}

		// Exact-tail repair retains its corruption recovery behavior. Extending that proof
		// requires the same epoch and an intact, fully cold image under the unchanged claim.
		self.memory_state.status = VolumeLogReplicaStatus::InSync;
		self.memory_state.lost_entries = false;
		Ok(ReplicaChange::StateChanged)
	}

	/// Records grant-authorized cold progress and raises the claim copied from a member.
	/// Late trims may raise the claim, but never move the cold position backward.
	/// A trim beyond the local tail establishes a new empty starting position.
	/// A deleted replica keeps its fence; a trim only releases its entries.
	pub fn trim(
		&mut self,
		cold: VolumeLogPosition,
		claimed_client: Option<ClientIdentity>,
	) -> Result<ReplicaChange, RejectResponse> {
		let deleted = self.memory_state.status == VolumeLogReplicaStatus::Deleted;

		// A copied claim lets a lagging member rejoin without weakening claim admission.
		// Equal epochs retain the existing incarnation, including conflicting copies.
		let mut change = ReplicaChange::Unchanged;
		if let Some(client) = claimed_client.filter(|_| !deleted)
			&& self
				.memory_state
				.claimed_client
				.as_ref()
				.is_none_or(|claimed| client.client_epoch > claimed.client_epoch)
		{
			// A copied claim does not say which process holds it.
			self.memory_state.claimed_client = Some(client);
			self.memory_state.claimed_holder = None;
			change = ReplicaChange::StateChanged;
		}

		// Cold progress and the copied claim share one state frame and disk barrier.
		if cold.volume_seq <= self.memory_state.cold.volume_seq {
			return Ok(change);
		}

		if cold.volume_seq > self.memory_state.cold.volume_seq {
			let before = self.entries.len();
			self.entries.retain(|e| e.volume_seq > cold.volume_seq);
			self.release_empty_tail();
			// A trim removes a prefix, so the released prefix shrinks by as much.
			self.released = self.released.saturating_sub(before - self.entries.len());
			self.memory_state.cold = cold.clone();
			if self.memory_state.appended.volume_seq <= cold.volume_seq {
				self.memory_state.appended = cold.clone();
			}

			self.memory_state.known_committed_volume_seq = self
				.memory_state
				.known_committed_volume_seq
				.max(cold.volume_seq);
		}

		Ok(ReplicaChange::StateChanged)
	}

	/// Checks retirement preconditions; the shard owns removal after the forget frame reaches disk.
	pub fn forget(
		&mut self,
		expected_appended: Option<VolumeLogPosition>,
	) -> Result<ReplicaChange, RejectResponse> {
		if let Some(expected) = expected_appended {
			if self.memory_state.status == VolumeLogReplicaStatus::Deleted {
				return Err(RejectResponse::VolumeLogReplicaDeleted(
					protocol::VolumeLogReplicaDeleted {
						state: self.memory_state.clone(),
					},
				));
			}
			self.check_appended(&expected)?;
			// A replica emptied by corruption recovery may lack entries it acknowledged, so
			// its position proves nothing. Forgetting it would erase that mark, and a later
			// load could not tell it from a replica that never held those entries.
			if expected != self.memory_state.cold || self.memory_state.lost_entries {
				return Err(RejectResponse::AppendedMismatch(
					protocol::AppendedMismatch {
						state: self.memory_state.clone(),
					},
				));
			}
		}

		// The shard writes a forget frame and removes this replica only after sync.
		Ok(ReplicaChange::Removed)
	}

	/// Retains the deletion fact and fences every client. Entries above cold stay, readable
	/// by the maintainer, until its trims release them: they may hold the deleted volume's
	/// final tail, which the maintainer applies and writes to cold before it trims.
	pub fn mark_volume_deleted(&mut self) -> Result<ReplicaChange, RejectResponse> {
		if self.memory_state.status == VolumeLogReplicaStatus::Deleted {
			return Ok(ReplicaChange::Unchanged);
		}
		self.memory_state.status = VolumeLogReplicaStatus::Deleted;
		Ok(ReplicaChange::StateChanged)
	}

	/// Deleted replicas keep their fence until the maintainer completes the sweep.
	fn check_live(&self) -> Result<(), RejectResponse> {
		if self.memory_state.status == VolumeLogReplicaStatus::Deleted {
			Err(RejectResponse::VolumeLogReplicaDeleted(
				protocol::VolumeLogReplicaDeleted {
					state: self.memory_state.clone(),
				},
			))
		} else {
			Ok(())
		}
	}

	/// Requires the exact claimed identity and distinguishes stale epochs from conflicts.
	fn check_client(&self, client: &ClientIdentity) -> Result<(), RejectResponse> {
		self.check_live()?;
		match self.memory_state.claimed_client.as_ref() {
			Some(claimed) if client == claimed => Ok(()),
			Some(claimed) if client.client_epoch < claimed.client_epoch => Err(
				RejectResponse::StaleClientEpoch(protocol::StaleClientEpoch {
					state: self.memory_state.clone(),
				}),
			),
			Some(claimed) if client.client_epoch == claimed.client_epoch => {
				Err(RejectResponse::ClientConflict(protocol::ClientConflict {
					state: self.memory_state.clone(),
				}))
			}
			_ => Err(RejectResponse::NotClaimed(protocol::NotClaimed {
				state: self.memory_state.clone(),
			})),
		}
	}

	/// Rejects conditional mutations if the caller observed a different tail.
	fn check_appended(&self, expected: &VolumeLogPosition) -> Result<(), RejectResponse> {
		if expected == &self.memory_state.appended {
			Ok(())
		} else {
			Err(RejectResponse::AppendedMismatch(
				protocol::AppendedMismatch {
					state: self.memory_state.clone(),
				},
			))
		}
	}

	/// Resolves the cold boundary or a retained entry; unavailable positions return None.
	fn position(&self, volume_seq: u64) -> Option<VolumeLogPosition> {
		if volume_seq == self.memory_state.cold.volume_seq {
			return Some(self.memory_state.cold.clone());
		}

		let offset = volume_seq
			.checked_sub(self.memory_state.cold.volume_seq)?
			.checked_sub(1)?;
		self.entries
			.get(usize::try_from(offset).ok()?)
			.map(ReplicaEntry::position)
	}

	fn volume_seq_gap(&self) -> RejectResponse {
		RejectResponse::VolumeSeqGap(protocol::VolumeSeqGap {
			state: self.memory_state.clone(),
		})
	}

	fn below_cold(&self) -> RejectResponse {
		RejectResponse::BelowCold(protocol::BelowCold {
			cold_write: None,
			state: self.memory_state.clone(),
		})
	}
}

fn volume_log_position(entry: &VolumeLogEntry) -> VolumeLogPosition {
	VolumeLogPosition {
		volume_seq: entry.volume_seq,
		client_epoch: entry.client_epoch,
	}
}

/// Evaluates a command on a private replica copy, then admits its frames and reply together.
/// Replica changes become visible only after all capacity checks pass. Replies, including
/// replica-level rejections, wait for the batch's disk write barrier.
///
/// `VolumeNotLoaded` means no replica is loaded here: the volume is unloaded, this worker
/// is not in its tablet replica set, or the caller's key is stale. The shard cannot
/// create one itself: existence, the cold position and the fence epoch live in
/// the volume row, which only the tablet maintainer holds. The client reopens
/// through `VolumeOpen`, which creates replicas at that row's cold position.
/// Creation and repairing truncation draw 16-byte random incarnations, assumed never to repeat.
pub(crate) fn admit(
	state: &mut LogShardState,
	request: LogShardRequest,
	rng: &mut rivet_env::rand::LocalRng,
	effects: &mut Effects,
) {
	// Retain a future-version request before any replica transition or disk reservation.
	if super::manifest_wait::needed(state, &request) {
		super::manifest_wait::hold(state, request, effects);
		return;
	}
	let request_id = request.request_id;
	let deadline_boot_ms = request.deadline_boot_ms;
	// Recovery must establish the disk identity before it can be observed or checked.
	if !matches!(state.phase, super::LogShardPhase::Serving) {
		reject(request_id, RejectResponse::NotServing, effects);
		return;
	}
	let (volume_log_key, authority, command) = match request.command {
		LogShardCommand::GetDiskIncarnation => {
			effects.execute(crate::log_shard_machine::effect::LogShardEffect::Reply {
				request_id,
				response: protocol::StorageWorkerResponse::GetDiskIncarnationResponse(
					protocol::GetDiskIncarnationResponse {
						disk_incarnation: state.disk_incarnation,
					},
				),
			});
			return;
		}
		LogShardCommand::ListTabletReplicas(request) => {
			super::tablet_grants::list(state, request_id, request, rng, effects);
			return;
		}
		LogShardCommand::ReadStatus { volume_log_keys } => {
			super::reads::admit_status(state, request_id, volume_log_keys, effects);
			return;
		}
		LogShardCommand::ForgetVolumes { grant, volumes } => {
			let commands = volumes
				.into_iter()
				.map(|(key, expected_appended, expected_claim)| {
					(
						key,
						LoadedVolumeReplicaCommand::Forget {
							observation: None,
							expected_appended: Some(expected_appended),
							expected_claim,
						},
					)
				})
				.collect();
			admit_grant_batch(
				state,
				request_id,
				grant,
				GrantBatch::Forget,
				commands,
				rng,
				effects,
			);
			return;
		}
		LogShardCommand::TrimVolumes { grant, volumes } => {
			let commands = volumes
				.into_iter()
				.map(|(key, cold, claimed_client)| {
					(
						key,
						LoadedVolumeReplicaCommand::Trim {
							cold,
							claimed_client,
						},
					)
				})
				.collect();
			admit_grant_batch(
				state,
				request_id,
				grant,
				GrantBatch::Trim,
				commands,
				rng,
				effects,
			);
			return;
		}
		LogShardCommand::Replica {
			volume_log_key,
			authority,
			command,
		} => (volume_log_key, authority, command),
	};
	// A removed assignment cannot recreate replicas while retirement is pending or complete.
	// An operator forget only removes a replica, under its own set check below, so it is
	// admitted even when this shard's manifest view already excludes the replica.
	if super::retirement::excluded(state, &volume_log_key)
		&& !matches!(
			&command,
			LoadedVolumeReplicaCommand::Forget {
				observation: Some(_),
				..
			}
		) {
		// A cached writer needs the new set before interpreting retirement as lost
		// replicas. This is only an advisory refusal; no authority frames are admitted.
		if let Some(authority @ Authority::Client { .. }) = &authority
			&& let Err(reason @ RejectResponse::StaleTabletReplicaSet(_)) =
				super::tablet_grants::check_authority(state, &volume_log_key, authority)
		{
			reject(request_id, reason, effects);
			return;
		}
		let reason = if state.loaded_volume_replicas.contains_key(&volume_log_key) {
			RejectResponse::NotServing
		} else {
			RejectResponse::VolumeNotLoaded
		};
		reject(request_id, reason, effects);
		return;
	}
	if let LoadedVolumeReplicaCommand::LoadVolume {
		disk_incarnation, ..
	} = &command
		&& *disk_incarnation != state.disk_incarnation
	{
		reject(
			request_id,
			RejectResponse::DiskIncarnationMismatch(protocol::DiskIncarnationMismatch {
				disk_incarnation: state.disk_incarnation,
			}),
			effects,
		);
		return;
	}
	if let LoadedVolumeReplicaCommand::Read {
		first_volume_seq,
		max_bytes,
		include_deleted,
	} = command
	{
		super::reads::admit(
			state,
			request_id,
			deadline_boot_ms,
			volume_log_key,
			first_volume_seq,
			max_bytes,
			include_deleted,
			effects,
		);
		return;
	}
	let Some(authority) = authority else {
		reject(request_id, RejectResponse::NotServing, effects);
		return;
	};

	// Reject full reply queues before doing replica work.
	if replies_full(state) {
		reject(request_id, super::admission::overloaded(state), effects);
		return;
	}

	match admit_mutation(state, volume_log_key, authority, command, rng, effects) {
		Admission::Rejected(reason) => reject(
			request_id,
			super::admission::jittered(state, reason, rng),
			effects,
		),
		Admission::Waiting(reply) => state
			.memory_batches
			.filling
			.push_reply(WaitingReply { request_id, reply }),
		Admission::Failed => {}
	}
}

/// Whether the shard already holds as many waiting replies as it may.
fn replies_full(state: &LogShardState) -> bool {
	state
		.memory_batches
		.pending()
		.map(|p| p.waiting_replies().len())
		.sum::<usize>()
		+ state.reads.len()
		+ state.manifest_waits.len()
		>= state.config.max_memory_requests as usize
}

/// Admits a batch of grant-authorized replica commands, conditional forgets or trims, in
/// one transition. One grant check covers the batch, and the batch takes one reply slot.
/// Each volume then takes its single-request path, so its frames join the filling batch
/// and one disk write covers every volume; the combined reply waits for that write. A
/// volume rejected at admission fails alone.
fn admit_grant_batch(
	state: &mut LogShardState,
	request_id: RequestId,
	grant: protocol::TabletGrant,
	kind: GrantBatch,
	volumes: Vec<(VolumeLogKey, LoadedVolumeReplicaCommand)>,
	rng: &mut rivet_env::rand::LocalRng,
	effects: &mut Effects,
) {
	if volumes.len() > state.config.max_forget_batch_volumes as usize {
		reject(
			request_id,
			RejectResponse::RequestTooLarge(protocol::RequestTooLarge {
				max_bytes: state.config.max_forget_batch_volumes,
			}),
			effects,
		);
		return;
	}
	if replies_full(state) {
		reject(request_id, super::admission::overloaded(state), effects);
		return;
	}
	if let Err(reason) = super::tablet_grants::check_grant(state, &grant) {
		reject(request_id, reason, effects);
		return;
	}

	let authority = Authority::Grant(grant);
	let mut results = Vec::with_capacity(volumes.len());
	for (volume_log_key, command) in volumes {
		// A removed assignment answers exactly as a single forget would.
		if super::retirement::excluded(state, &volume_log_key) {
			let reason = if state.loaded_volume_replicas.contains_key(&volume_log_key) {
				RejectResponse::NotServing
			} else {
				RejectResponse::VolumeNotLoaded
			};
			results.push(ForgetResult::Rejected(reason));
			continue;
		}
		let result = match admit_mutation(
			state,
			volume_log_key,
			authority.clone(),
			command,
			rng,
			effects,
		) {
			Admission::Rejected(reason) => {
				ForgetResult::Rejected(super::admission::jittered(state, reason, rng))
			}
			Admission::Waiting(PendingReply::ReplicaState {
				volume_log_key,
				rejection,
				authority: _,
			}) => ForgetResult::Replica {
				volume_log_key,
				rejection,
			},
			Admission::Waiting(PendingReply::RejectedReplica {
				volume_log_key,
				authority: _,
				reason,
			}) => ForgetResult::RejectedAfterGrant {
				volume_log_key,
				reason,
			},
			Admission::Waiting(
				PendingReply::Read { .. }
				| PendingReply::LoadedVolumeReplicas { .. }
				| PendingReply::Status { .. }
				| PendingReply::Forgets { .. },
			) => {
				fail(
					state,
					"batched forget or trim admission produced a non-replica reply".into(),
					effects,
				);
				return;
			}
			Admission::Failed => return,
		};
		results.push(result);
	}
	state.memory_batches.filling.push_reply(WaitingReply {
		request_id,
		reply: PendingReply::Forgets {
			kind,
			authority,
			results,
		},
	});
}

/// How one admitted mutation is answered.
pub(crate) enum Admission {
	/// Answer now; nothing was queued for disk.
	Rejected(RejectResponse),
	/// The command's frames are in the filling batch, and this reply waits for its write.
	Waiting(PendingReply),
	/// The shard failed while admitting; it replies to nobody.
	Failed,
}

/// What an in-place command changed on the installed replica, so a rejection can undo it.
struct ReplicaUndo {
	/// The key the replica was installed under, which owns its own buffer.
	key: VolumeLogKey,
	memory_state: VolumeLogReplicaState,
	entries_len: usize,
	totals: ReplicaTotals,
}

/// Reinstalls a replica that ran a rejected command in place, as it was before the command.
/// A replica worked on as a private copy is dropped.
fn restore(state: &mut LogShardState, undo: Option<ReplicaUndo>, mut replica: LoadedVolumeReplica) {
	if let Some(undo) = undo {
		replica.memory_state = undo.memory_state;
		replica.entries.truncate(undo.entries_len);
		state.loaded_volume_replicas.insert(undo.key, replica);
	}
}

/// Checks authority, applies `command` to the replica and, once every budget admits it,
/// queues its frames and installs the result. A command a budget rejects leaves the replica
/// as it was. The caller decides where the resulting reply goes, so a batch can collect
/// several of them into one answer.
/// The refusal for the first entry in `command` whose payload family and version this
/// worker does not advertise reading in `read_versions`, or `None`. Family 0 payloads are
/// untracked and always admitted. Integer comparisons only; payload bytes are never decoded.
fn unreadable_payload(
	command: &LoadedVolumeReplicaCommand,
	read_versions: &[rivet_data::generated::topology_worker_heartbeat_v1::FamilyVersions],
) -> Option<RejectResponse> {
	let entries = match command {
		LoadedVolumeReplicaCommand::Append { entries, .. } => entries,
		LoadedVolumeReplicaCommand::LoadVolume {
			initial_entries, ..
		} => initial_entries,
		LoadedVolumeReplicaCommand::Claim { .. }
		| LoadedVolumeReplicaCommand::MarkInSync { .. }
		| LoadedVolumeReplicaCommand::Truncate { .. }
		| LoadedVolumeReplicaCommand::AdvanceCommitted { .. }
		| LoadedVolumeReplicaCommand::Trim { .. }
		| LoadedVolumeReplicaCommand::MarkVolumeDeleted
		| LoadedVolumeReplicaCommand::Read { .. }
		| LoadedVolumeReplicaCommand::Forget { .. } => return None,
	};
	for entry in entries {
		let VolumeLogEntryBody::PayloadWrite(write) = &entry.body else {
			continue;
		};
		if write.payload_family == 0 {
			continue;
		}
		let max_read = rivet_data::families::advertised(
			read_versions,
			rivet_data::families::VersionFamily(write.payload_family),
		)
		.map(|range| range.max_read);
		if max_read.is_none_or(|max_read| write.payload_version > max_read) {
			return Some(RejectResponse::UnsupportedPayloadVersion(
				protocol::UnsupportedPayloadVersion {
					payload_family: write.payload_family,
					payload_version: write.payload_version,
					max_read: max_read.unwrap_or(0),
				},
			));
		}
	}
	None
}

fn admit_mutation(
	state: &mut LogShardState,
	volume_log_key: VolumeLogKey,
	authority: Authority,
	mut command: LoadedVolumeReplicaCommand,
	rng: &mut rivet_env::rand::LocalRng,
	effects: &mut Effects,
) -> Admission {
	// Refuse a payload this binary cannot read before any work: persisting it would raise
	// FORMAT's mark past what this shard can open after a restart.
	if let Some(reason) = unreadable_payload(&command, &state.config.read_versions) {
		return Admission::Rejected(reason);
	}

	let grant_frames =
		match super::tablet_grants::check_authority(state, &volume_log_key, &authority) {
			Ok(frames) => frames,
			Err(reason) => return Admission::Rejected(reason),
		};
	// Operator cleanup must remain a nonmember cleanup at the target. A late request
	// cannot erase a recreated replica or a replica that a new assignment now counts.
	if let LoadedVolumeReplicaCommand::Forget {
		observation: Some(observation),
		expected_appended,
		..
	} = &command
	{
		let allowed = match &authority {
			Authority::Grant(grant) => state
				.tablet_manifest
				.tablets
				.get(&grant.tablet_id)
				.is_some_and(|tablet| {
					let set = super::tablet_replica_sets::effective(state, tablet);
					expected_appended.is_some()
						&& tablet.hash_range.contains(depot_types::partition_key_hash(
							&volume_log_key.0.volume_key,
						)) && set.tablet_replica_set_version == observation.tablet_replica_set_version
						&& matches!(
							set.phase,
							rivet_data::generated::shared_depot_types_v1::TabletPhase::Stable
						) && !set.members.contains(state.worker_id.as_bytes())
						&& state
							.loaded_volume_replicas
							.get(&volume_log_key)
							.is_none_or(|replica| {
								replica.memory_state.replica_incarnation
									== observation.replica_incarnation
							})
				}),
			_ => false,
		};
		if !allowed {
			return reject_after_grant(
				state,
				volume_log_key,
				authority,
				grant_frames,
				RejectResponse::NotServing,
				effects,
			);
		}
	}
	if let LoadedVolumeReplicaCommand::LoadVolume {
		open_claim: Some(_),
		claimed_client,
		learner_replica_set_version,
		..
	} = &command
		&& (claimed_client.is_some() || learner_replica_set_version.is_some())
	{
		return reject_after_grant(
			state,
			volume_log_key,
			authority,
			grant_frames,
			RejectResponse::NotServing,
			effects,
		);
	}

	if matches!(command, LoadedVolumeReplicaCommand::LoadVolume { .. })
		&& let Authority::Grant(grant) = &authority
		&& !super::retired_volumes::permits_load(state, &volume_log_key, grant)
	{
		return reject_after_grant(
			state,
			volume_log_key,
			authority,
			grant_frames,
			RejectResponse::VolumeNotLoaded,
			effects,
		);
	}
	// Do not admit new work behind a forget that has not reached disk yet.
	if state.memory_batches.forgetting(&volume_log_key) {
		return reject_after_grant(
			state,
			volume_log_key,
			authority,
			grant_frames,
			RejectResponse::VolumeNotLoaded,
			effects,
		);
	}

	// Ordinary opens are idempotent. A new Learning generation replaces old membership history.
	let existing = state.loaded_volume_replicas.get(&volume_log_key);
	let learner_floor = match &authority {
		Authority::Grant(grant) => {
			super::tablet_replica_sets::learner_floor(state, &grant.tablet_id)
		}
		Authority::Client { .. } | Authority::GrantedClient { .. } => 0,
	};
	// A delayed learner load from a generation this worker has already passed must not
	// create a replica either. It would carry that generation's cold position and client
	// fence, and an ordinary reopen would keep it instead of loading a current one.
	if existing.is_none()
		&& let Authority::Grant(grant) = &authority
		&& let LoadedVolumeReplicaCommand::LoadVolume {
			learner_replica_set_version: Some(version),
			..
		} = &command
		&& *version <= learner_floor
	{
		let reason = match super::tablet_replica_sets::known(state, &grant.tablet_id) {
			Some(replica_set) => {
				RejectResponse::StaleTabletReplicaSet(protocol::StaleTabletReplicaSet {
					replica_set,
				})
			}
			None => RejectResponse::NotServing,
		};
		return reject_after_grant(
			state,
			volume_log_key,
			authority,
			grant_frames,
			reason,
			effects,
		);
	}
	let replacing = matches!(authority, Authority::Grant(_))
		&& match &command {
			LoadedVolumeReplicaCommand::LoadVolume {
				learner_replica_set_version: Some(version),
				..
			} => existing.is_some_and(|r| {
				*version
					> r.memory_state
						.learner_replica_set_version
						.max(learner_floor)
			}),
			_ => false,
		};
	let creating = matches!(command, LoadedVolumeReplicaCommand::LoadVolume { .. })
		&& (existing.is_none() || replacing)
		&& matches!(authority, Authority::Grant(_));
	// A new replica keeps its key for its whole lifetime, so the key must not share the request's
	// buffer, which can be a large frame or the HTTP chunk the frame arrived in.
	let volume_log_key = if creating {
		volume_log_key.detach()
	} else {
		volume_log_key
	};

	// The opener a creating load claimed this replica for, whose initial entries it may carry.
	let mut claimed_for_opener = None;
	// Capacity rejection must leave the installed replica untouched. The frequent commands
	// change only the replica's state and extend its tail, so they run on the installed
	// replica and are undone if rejected; a copy would cost time for every retained entry.
	// The others, which can drop entries, run on a private copy.
	let in_place = !creating
		&& existing.is_some()
		&& match &command {
			LoadedVolumeReplicaCommand::LoadVolume { .. }
			| LoadedVolumeReplicaCommand::Claim { .. }
			| LoadedVolumeReplicaCommand::Append { .. }
			| LoadedVolumeReplicaCommand::AdvanceCommitted { .. }
			| LoadedVolumeReplicaCommand::MarkInSync { .. }
			| LoadedVolumeReplicaCommand::MarkVolumeDeleted => true,
			LoadedVolumeReplicaCommand::Truncate { .. }
			| LoadedVolumeReplicaCommand::Trim { .. }
			| LoadedVolumeReplicaCommand::Forget { .. }
			| LoadedVolumeReplicaCommand::Read { .. } => false,
		};
	let mut undo = None;
	let mut replica = if in_place {
		let Some((key, replica)) = state.loaded_volume_replicas.remove_entry(&volume_log_key)
		else {
			return Admission::Rejected(RejectResponse::VolumeNotLoaded);
		};
		undo = Some(ReplicaUndo {
			key,
			memory_state: replica.memory_state.clone(),
			entries_len: replica.entries.len(),
			totals: ReplicaTotals::of(&replica),
		});
		replica
	} else if creating {
		let LoadedVolumeReplicaCommand::LoadVolume {
			open_claim,
			learner_replica_set_version,
			disk_incarnation: _,
			claimed_client,
			claimed_holder,
			cold,
			status,
			initial_entries: _,
			creation,
		} = &command
		else {
			return Admission::Rejected(RejectResponse::NotServing);
		};

		if existing.is_none()
			&& state.loaded_volume_replicas.len() >= state.config.max_loaded_volumes as usize
		{
			return reject_after_grant(
				state,
				volume_log_key,
				authority,
				grant_frames,
				RejectResponse::TooManyLoadedVolumes(protocol::TooManyLoadedVolumes {
					max_loaded_volumes: state.config.max_loaded_volumes,
				}),
				effects,
			);
		}

		let mut replica = LoadedVolumeReplica::new(
			volume_log_key.clone(),
			*rng.uuid_v4().as_bytes(),
			cold.clone(),
			status.clone(),
		);
		// The creation state frame makes the copied fence part of the same disk write.
		replica.memory_state.claimed_client = claimed_client.clone();
		replica.memory_state.creation = creation.clone();
		replica.memory_state.claimed_holder = claimed_holder.clone();
		// A new learner generation replaces history, not accepted client fences.
		// Its source may not have observed a newer claim already admitted here.
		// Equal epochs also retain the existing identity and its matching holder.
		if replacing
			&& let Some(existing) = existing
			&& existing
				.memory_state
				.claimed_client
				.as_ref()
				.is_some_and(|client| {
					replica
						.memory_state
						.claimed_client
						.as_ref()
						.is_none_or(|copied| client.client_epoch >= copied.client_epoch)
				}) {
			replica.memory_state.claimed_client = existing.memory_state.claimed_client.clone();
			replica.memory_state.claimed_holder = existing.memory_state.claimed_holder.clone();
		}
		if let (Some(open), Authority::Grant(grant)) = (open_claim, &authority)
			&& open_claim_applies(state, grant, open)
		{
			// A failed claim leaves the newly loaded replica unclaimed. The opener examines
			// the returned state and claims it at the same granted epoch.
			if replica
				.claim_granted(&open.client, open.holder.as_ref())
				.is_ok()
			{
				claimed_for_opener = Some(open.client.clone());
			}
		}
		replica.memory_state.learner_replica_set_version =
			learner_replica_set_version.unwrap_or(learner_floor);
		replica.disk_state = existing.and_then(|r| r.disk_state.clone());
		replica
	} else if let Some(existing) = existing {
		existing.clone()
	} else {
		return reject_after_grant(
			state,
			volume_log_key,
			authority,
			grant_frames,
			RejectResponse::VolumeNotLoaded,
			effects,
		);
	};

	// Apply the command speculatively; its result identifies the exact change to write to disk.
	// Appended entries move into the replica, so later checks see only the command's kind.
	let change = match (&authority, &mut command) {
		(Authority::Grant(grant), LoadedVolumeReplicaCommand::LoadVolume { open_claim, .. }) => {
			if creating {
				Ok(ReplicaChange::StateChanged)
			} else {
				// An open of a loaded replica claims it for the opener at the maintainer's
				// epoch, so the claim is the open's only durable write. A claim the replica
				// refuses, such as one below a newer claim, leaves it unchanged; the reply
				// carries its state and the opener sees it was not claimed.
				match open_claim
					.as_ref()
					.filter(|open| open_claim_applies(state, grant, open))
				{
					Some(open) => replica
						.claim_granted(&open.client, open.holder.as_ref())
						.or(Ok(ReplicaChange::Unchanged)),
					None => Ok(ReplicaChange::Unchanged),
				}
			}
		}
		(Authority::Client { identity, .. }, LoadedVolumeReplicaCommand::Claim { holder }) => {
			replica.claim(identity, holder.as_ref())
		}
		(
			Authority::GrantedClient { identity, .. },
			LoadedVolumeReplicaCommand::Claim { holder },
		) => replica.claim_granted(identity, holder.as_ref()),
		(
			Authority::Client { identity, .. } | Authority::GrantedClient { identity, .. },
			LoadedVolumeReplicaCommand::Append {
				previous,
				entries,
				committed_volume_seq,
			},
		) => replica.append(
			identity,
			previous.clone(),
			std::mem::take(entries),
			*committed_volume_seq,
		),
		(
			Authority::Grant(_),
			LoadedVolumeReplicaCommand::Append {
				previous,
				entries,
				committed_volume_seq,
			},
		) => replica.append_committed(
			previous.clone(),
			std::mem::take(entries),
			*committed_volume_seq,
		),
		(
			Authority::Client { identity, .. } | Authority::GrantedClient { identity, .. },
			LoadedVolumeReplicaCommand::MarkInSync {
				replica_incarnation,
				expected_appended,
			},
		) => replica.mark_in_sync(identity, *replica_incarnation, expected_appended.clone()),
		(
			Authority::Client { identity, .. } | Authority::GrantedClient { identity, .. },
			LoadedVolumeReplicaCommand::Truncate {
				expected_replica_incarnation,
				expected_appended,
				keep_through_volume_seq,
				mark_repairing,
			},
		) => replica.truncate_observed(
			identity,
			*expected_replica_incarnation,
			expected_appended.clone(),
			*keep_through_volume_seq,
			*mark_repairing,
			mark_repairing.then(|| *rng.uuid_v4().as_bytes()),
		),
		(
			Authority::Client { identity, .. } | Authority::GrantedClient { identity, .. },
			LoadedVolumeReplicaCommand::AdvanceCommitted {
				committed_volume_seq,
			},
		) => replica.advance_committed(identity, *committed_volume_seq),
		(
			Authority::Grant(_),
			LoadedVolumeReplicaCommand::Trim {
				cold,
				claimed_client,
			},
		) => replica.trim(cold.clone(), claimed_client.clone()),
		(Authority::Grant(_), LoadedVolumeReplicaCommand::MarkVolumeDeleted) => {
			replica.mark_volume_deleted()
		}
		(
			Authority::Grant(_),
			LoadedVolumeReplicaCommand::Forget {
				observation: _,
				expected_appended,
				expected_claim,
			},
		) => {
			// A conditional forget removes only the replica the maintainer observed. A claim
			// installed since, such as one an open granted after the forget was sent, keeps the
			// replica, so a forget that arrives late never drops a member the open counted.
			// A deleted replica answers as `forget` decides.
			let claim_changed = expected_appended.is_some()
				&& replica.memory_state.status != VolumeLogReplicaStatus::Deleted
				&& replica.memory_state.claimed_client != *expected_claim;
			if claim_changed {
				Err(RejectResponse::AppendedMismatch(
					protocol::AppendedMismatch {
						state: replica.memory_state.clone(),
					},
				))
			} else {
				replica.forget(expected_appended.clone())
			}
		}
		(
			Authority::GrantedClient { identity, .. },
			LoadedVolumeReplicaCommand::Forget {
				observation: None,
				expected_appended: Some(expected),
				..
			},
		) => {
			// Cleanup cannot erase a claim installed after its observation. The grant,
			// claim and fully-cold position are checked in this same admission.
			let matching = if replica.memory_state.claimed_client.is_none() {
				replica.check_live()
			} else {
				replica.check_client(identity)
			};
			matching.and_then(|()| replica.forget(Some(expected.clone())))
		}
		(
			Authority::Client { .. } | Authority::GrantedClient { .. },
			LoadedVolumeReplicaCommand::LoadVolume { .. }
			| LoadedVolumeReplicaCommand::Trim { .. }
			| LoadedVolumeReplicaCommand::MarkVolumeDeleted
			| LoadedVolumeReplicaCommand::Forget { .. },
		)
		| (
			Authority::Grant(_),
			LoadedVolumeReplicaCommand::Claim { .. }
			| LoadedVolumeReplicaCommand::MarkInSync { .. }
			| LoadedVolumeReplicaCommand::Truncate { .. }
			| LoadedVolumeReplicaCommand::AdvanceCommitted { .. },
		) => replica.check_live().and_then(|()| {
			Err(RejectResponse::NotClaimed(protocol::NotClaimed {
				state: replica.state().clone(),
			}))
		}),
		(
			Authority::Client { .. } | Authority::Grant(_) | Authority::GrantedClient { .. },
			LoadedVolumeReplicaCommand::Read { .. },
		) => unreachable!("reads admitted separately"),
	};

	// A create claimed for its opener carries the opener's first entries in the same frames, so
	// the replica, its claim and those entries reach disk in one sync. Entries that do not extend
	// the new replica under the claim, or that would not fit this write, are dropped and the
	// opener writes them as usual. The create itself never fails because of them.
	let mut initial = None;
	if let LoadedVolumeReplicaCommand::LoadVolume {
		initial_entries, ..
	} = &mut command
		&& !initial_entries.is_empty()
	{
		let entries = std::mem::take(initial_entries);
		initial = match (&change, &claimed_for_opener) {
			(Ok(ReplicaChange::StateChanged), Some(client)) => {
				let created = replica.state().clone();
				let mut sizes: Vec<u64> = grant_frames
					.iter()
					.map(|frame| frame.encoded_len() as u64)
					.collect();
				if replacing {
					sizes.push(
						Frame::ForgetFrame(ForgetFrame {
							retirement: None,
							volume_log_key: volume_log_key.0.clone(),
						})
						.encoded_len() as u64,
					);
				}
				sizes.push(
					Frame::StateFrame(StateFrame {
						volume_log_key: volume_log_key.clone().into(),
						state: created.clone(),
					})
					.encoded_len() as u64,
				);
				sizes.extend(entries.iter().map(|entry| {
					Frame::EntryFrame(EntryFrame {
						volume_log_key: volume_log_key.clone().into(),
						entry: entry.clone(),
						known_committed_volume_seq: created.known_committed_volume_seq,
					})
					.encoded_len() as u64
				}));
				if !initial_entries_fit(state, &sizes) {
					crate::metrics::record_initial_entries_dropped(InitialEntriesDrop::TooLarge);
					None
				} else {
					let previous = created.appended.clone();
					match replica.append(client, previous, entries, 0) {
						Ok(ReplicaChange::AppendedEntries(entries)) => Some((created, entries)),
						Ok(
							ReplicaChange::Unchanged
							| ReplicaChange::StateChanged
							| ReplicaChange::Removed,
						)
						| Err(_) => {
							// The maintainer builds these entries to extend the new replica
							// under its claim, so a refusal points at a maintainer bug.
							tracing::warn!(
								?volume_log_key,
								"initial entries do not extend the created replica; the opener writes them itself"
							);
							crate::metrics::record_initial_entries_dropped(
								InitialEntriesDrop::Malformed,
							);
							None
						}
					}
				}
			}
			(Ok(ReplicaChange::StateChanged), None) => {
				crate::metrics::record_initial_entries_dropped(InitialEntriesDrop::NotClaimed);
				None
			}
			(
				Ok(
					ReplicaChange::Unchanged
					| ReplicaChange::AppendedEntries(_)
					| ReplicaChange::Removed,
				),
				_,
			) => {
				crate::metrics::record_initial_entries_dropped(InitialEntriesDrop::Existing);
				None
			}
			// The whole load is rejected, and its caller retries it with the entries.
			(Err(_), _) => None,
		};
	}

	let retirement = if matches!(change, Ok(ReplicaChange::Removed))
		&& replica.state().status == VolumeLogReplicaStatus::Deleted
	{
		let Authority::Grant(grant) = &authority else {
			unreachable!("forget requires a grant")
		};
		if !super::retired_volumes::has_capacity(state, &volume_log_key) {
			restore(state, undo, replica);
			return reject_after_grant(
				state,
				volume_log_key,
				authority,
				grant_frames,
				RejectResponse::TooManyLoadedVolumes(protocol::TooManyLoadedVolumes {
					max_loaded_volumes: state.config.max_loaded_volumes,
				}),
				effects,
			);
		}
		Some(
			rivet_data::generated::depot_log_shard_v1::TabletGrantFrame {
				tablet_id: grant.tablet_id,
				tablet_grant_epoch: grant.tablet_grant_epoch,
			},
		)
	} else {
		None
	};
	// Write to disk only the reported change. Unchanged or rejected commands need no new frames.
	let mut frames = grant_frames;
	if replacing {
		frames.push(Frame::ForgetFrame(ForgetFrame {
			retirement: None,
			volume_log_key: volume_log_key.0.clone(),
		}));
	}
	frames.extend(match &change {
		Ok(ReplicaChange::Removed) => {
			vec![Frame::ForgetFrame(ForgetFrame {
				retirement,
				volume_log_key: volume_log_key.clone().into(),
			})]
		}
		Ok(ReplicaChange::AppendedEntries(entries)) => entries
			.iter()
			.map(|entry| {
				Frame::EntryFrame(EntryFrame {
					volume_log_key: volume_log_key.clone().into(),
					entry: entry.clone(),
					known_committed_volume_seq: replica.state().known_committed_volume_seq,
				})
			})
			.collect(),
		Ok(ReplicaChange::StateChanged) => match &initial {
			// The creation state, then the entries that extend it on replay.
			Some((created, entries)) => std::iter::once(Frame::StateFrame(StateFrame {
				volume_log_key: volume_log_key.clone().into(),
				state: created.clone(),
			}))
			.chain(entries.iter().map(|entry| {
				Frame::EntryFrame(EntryFrame {
					volume_log_key: volume_log_key.clone().into(),
					entry: entry.clone(),
					known_committed_volume_seq: replica.state().known_committed_volume_seq,
				})
			}))
			.collect(),
			None => vec![Frame::StateFrame(StateFrame {
				volume_log_key: volume_log_key.clone().into(),
				state: replica.state().clone(),
			})],
		},
		Ok(ReplicaChange::Unchanged) | Err(_) => Vec::new(),
	});

	// Each frame must fit a checksummed batch, and the whole request must fit one write.
	let frame_sizes: Vec<u64> = frames
		.iter()
		.map(|frame| frame.encoded_len() as u64)
		.collect();
	let encoded_bytes = frame_sizes.iter().sum::<u64>();
	let mut limit = state
		.config
		.max_write_bytes
		.min(state.config.max_memory_bytes)
		.min(super::memory_batches::drain_limit_bytes(
			state,
			effects.now_boot_ms(),
		));
	let request_bytes =
		super::memory_batches::write_bytes_for_sizes(&frame_sizes, state.config.max_batch_bytes);
	if request_bytes.is_err() {
		limit = limit.min(u64::from(state.config.max_batch_bytes));
	}
	if !frames.is_empty()
		&& (encoded_bytes > limit
			|| request_bytes
				.as_ref()
				.map_or(true, |bytes| *bytes > state.config.max_write_bytes))
	{
		restore(state, undo, replica);
		return Admission::Rejected(RejectResponse::RequestTooLarge(protocol::RequestTooLarge {
			max_bytes: limit.min(u32::MAX as u64) as u32,
		}));
	}

	// Bound all accepted bytes waiting for disk writes, including the in-flight batch.
	let occupied = super::reads::occupied_bytes(state);
	if encoded_bytes > state.config.max_memory_bytes.saturating_sub(occupied) {
		restore(state, undo, replica);
		return Admission::Rejected(super::admission::overloaded(state));
	}

	// Filling is one bounded write, even while the previous write is still in flight.
	let filling_bytes = match state
		.memory_batches
		.filling
		.size_with(&frame_sizes, state.config.max_batch_bytes)
	{
		Ok(bytes) => bytes,
		Err(error) => {
			fail(state, error.to_string(), effects);
			restore(state, undo, replica);
			return Admission::Failed;
		}
	};
	if filling_bytes > state.config.max_write_bytes {
		restore(state, undo, replica);
		return Admission::Rejected(super::admission::overloaded(state));
	}

	// Keep the next write within the disk's drain bound, including the first batch
	// accepted before the disk has supplied any rate evidence.
	let drain_limit = super::memory_batches::drain_limit_bytes(state, effects.now_boot_ms());
	if !frames.is_empty() && filling_bytes > drain_limit {
		restore(state, undo, replica);
		crate::metrics::record_admission_shed(crate::metrics::AdmissionShed::DiskDrain, 1);
		return Admission::Rejected(super::admission::drain_refusal(
			state,
			filling_bytes,
			drain_limit,
			rng,
		));
	}

	// Reserve all framed bytes, leaving room for maintenance that enables collection.
	let maintenance = matches!(
		command,
		LoadedVolumeReplicaCommand::Trim { .. }
			| LoadedVolumeReplicaCommand::Forget { .. }
			| LoadedVolumeReplicaCommand::MarkVolumeDeleted
	);
	let pending_disk = state
		.memory_batches
		.writing_to_disk
		.as_ref()
		.map_or(0, |write| write.pending.sizing.total());
	let reservation = filling_bytes;

	let disk_limit = state.config.max_disk_bytes.saturating_sub(if maintenance {
		0
	} else {
		super::disk_segments::DiskSegments::pending_disk_bytes(
			u64::from(state.config.max_batch_bytes)
				+ depot_types::log_shard::BATCH_FRAMING_BYTES as u64,
		)
	});
	if !frames.is_empty()
		&& reservation > 0
		&& state.disk_segments.disk_bytes().saturating_add(
			super::disk_segments::DiskSegments::pending_disk_bytes(
				pending_disk.saturating_add(reservation),
			),
		) > disk_limit
	{
		restore(state, undo, replica);
		return Admission::Rejected(super::admission::disk_full());
	}

	// Transfer the locations dropped by speculative trim/truncate/deletion to the
	// pending batch. GC cannot release their files until these frames reach disk.
	// Entries are ordered by volume sequence and a version drops only a prefix and a
	// suffix, so the dropped entries are found by binary search at both ends.
	if let Some(old) = state.loaded_volume_replicas.get(&volume_log_key) {
		let (kept_start, kept_end) = match (replica.entries.front(), replica.entries.back()) {
			(Some(first), Some(last)) => (
				old.entries
					.partition_point(|entry| entry.volume_seq < first.volume_seq),
				old.entries
					.partition_point(|entry| entry.volume_seq <= last.volume_seq),
			),
			(None, _) | (_, None) => (old.entries.len(), old.entries.len()),
		};
		let dropped = old
			.entries
			.range(..kept_start)
			.chain(old.entries.range(kept_end.max(kept_start)..));
		state
			.memory_batches
			.filling
			.retained_batch_log_seqs
			.extend(dropped.filter_map(|entry| entry.batch_log_seq));
	}

	// Admit the replica and its reply atomically. Even a rejection may depend on unsynced state.
	let appended = matches!(change, Ok(ReplicaChange::AppendedEntries(_))) || initial.is_some();
	let rejection = match change {
		Ok(_) => None,
		Err(reason) => match ReplicaRejection::from_response(&reason) {
			Some(reason) => Some(reason),
			None => {
				fail(
					state,
					format!("replica rule returned a non-replica rejection: {reason:?}"),
					effects,
				);
				restore(state, undo, replica);
				return Admission::Failed;
			}
		},
	};
	if let Err(error) = state.memory_batches.filling.push_sized_frames(
		frames,
		&frame_sizes,
		state.config.max_batch_bytes,
	) {
		fail(state, error.to_string(), effects);
		restore(state, undo, replica);
		return Admission::Failed;
	}
	state.memory_batches.filling.encoded_bytes += encoded_bytes;
	let reply = PendingReply::ReplicaState {
		volume_log_key: volume_log_key.clone(),
		rejection,
		authority: Some(authority),
	};
	match undo {
		// An in-place command only extended the tail, and the new entries have no disk batch
		// yet, so the segment pins are unchanged and the replica is already indexed.
		Some(undo) => {
			debug_assert!(
				replica.entries.len() >= undo.entries_len,
				"an in-place command dropped entries"
			);
			let totals = ReplicaTotals::of(&replica);
			let (id, _) = state.loaded_volume_replicas.insert(undo.key, replica);
			state.payloads.replaced(
				id,
				Some(undo.totals),
				Some(totals),
				appended,
				effects.now_boot_ms(),
			);
		}
		None => {
			let installed = state.loaded_volume_replicas.get(&volume_log_key);
			state.segment_pins.replace(installed, Some(&replica));
			let old_totals = installed.map(ReplicaTotals::of);
			let totals = ReplicaTotals::of(&replica);
			state.replica_hashes.replaced(&volume_log_key, true);
			let (id, _) = state.loaded_volume_replicas.insert(volume_log_key, replica);
			state.payloads.replaced(
				id,
				old_totals,
				Some(totals),
				appended,
				effects.now_boot_ms(),
			);
		}
	}
	Admission::Waiting(reply)
}

/// An admitted higher grant must reach disk even when its target volume is absent.
/// Queue its rejection behind the same barrier and recheck the authority at release.
/// Whether a load may claim its replica for the opener: the tablet's installed replica set is
/// the version the maintainer granted under, and stable. Otherwise the opener claims the
/// replica itself at the same granted epoch.
fn open_claim_applies(
	state: &LogShardState,
	grant: &protocol::TabletGrant,
	open: &rivet_data::generated::shared_depot_types_v1::OpenClaim,
) -> bool {
	let Some(tablet) = state.tablet_manifest.tablets.get(&grant.tablet_id) else {
		return false;
	};
	let installed = super::tablet_replica_sets::effective(state, tablet);
	installed.tablet_replica_set_version == open.tablet_replica_set_version
		&& matches!(
			installed.phase,
			rivet_data::generated::shared_depot_types_v1::TabletPhase::Stable
		)
}

fn reject_after_grant(
	state: &mut LogShardState,
	volume_log_key: VolumeLogKey,
	authority: Authority,
	frames: Vec<Frame>,
	reason: RejectResponse,
	effects: &mut Effects,
) -> Admission {
	// State-free failures need no barrier when this authority is already on disk.
	let grant_synced = match &authority {
		Authority::Client { .. } => true,
		Authority::Grant(grant) | Authority::GrantedClient { grant, .. } => {
			state
				.tablet_grants
				.get(&grant.tablet_id)
				.copied()
				.unwrap_or(0)
				>= grant.tablet_grant_epoch
		}
	};
	let forgetting = state.memory_batches.forgetting(&volume_log_key);
	if frames.is_empty() && grant_synced && !forgetting {
		return Admission::Rejected(reason);
	}

	match super::tablet_grants::reserve(state, &frames) {
		Ok(bytes) => {
			if let Err(error) = state
				.memory_batches
				.filling
				.push_frames(frames, state.config.max_batch_bytes)
			{
				fail(state, error.to_string(), effects);
				return Admission::Failed;
			}
			state.memory_batches.filling.encoded_bytes += bytes;
			Admission::Waiting(PendingReply::RejectedReplica {
				volume_log_key,
				authority,
				reason,
			})
		}
		Err(reason) => Admission::Rejected(reason),
	}
}



/// Whether a create's frames, its initial entries included, fit one write, the shard's memory
/// headroom, the filling batch and the disk budget. It applies the same limits as admission, so
/// a create whose entries fail it is admitted without them instead of being rejected.
fn initial_entries_fit(state: &LogShardState, frame_sizes: &[u64]) -> bool {
	let encoded_bytes = frame_sizes.iter().sum::<u64>();
	let max_batch_bytes = state.config.max_batch_bytes;
	let fits_write = super::memory_batches::write_bytes_for_sizes(frame_sizes, max_batch_bytes)
		.is_ok_and(|bytes| bytes <= state.config.max_write_bytes);
	let occupied = super::reads::occupied_bytes(state);
	let fits_memory = encoded_bytes <= state.config.max_memory_bytes.saturating_sub(occupied);
	let filling_bytes = state
		.memory_batches
		.filling
		.size_with(frame_sizes, max_batch_bytes)
		.ok()
		.filter(|bytes| *bytes <= state.config.max_write_bytes);
	let pending_disk = state
		.memory_batches
		.writing_to_disk
		.as_ref()
		.map_or(0, |write| write.pending.sizing.total());
	let disk_limit = state.config.max_disk_bytes.saturating_sub(
		super::disk_segments::DiskSegments::pending_disk_bytes(
			u64::from(max_batch_bytes) + depot_types::log_shard::BATCH_FRAMING_BYTES as u64,
		),
	);
	let fits_disk = filling_bytes.is_some_and(|filling| {
		state.disk_segments.disk_bytes().saturating_add(
			super::disk_segments::DiskSegments::pending_disk_bytes(
				pending_disk.saturating_add(filling),
			),
		) <= disk_limit
	});
	fits_write && fits_memory && fits_disk
}
