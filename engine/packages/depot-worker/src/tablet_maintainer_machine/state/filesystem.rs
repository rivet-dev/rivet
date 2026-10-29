//! Continuous filesystem apply within the tablet maintainer state machine.
//! Replica commitment authorizes reads; completed store work publishes applied;
//! only a flushed record authorizes log trim. All work uses the parent's queues.

use super::{
	MaintainerPhase, MaintainerState, Operation, fence, store::StoreWork, volume_keys::KeyId,
};
use crate::tablet_maintainer_machine::types::{
	COLD_BATCH_DELAY_MS, MAX_COLD_BYTES, OPEN_APPLY_LAG_BYTES, OPEN_APPLY_LAG_ENTRIES,
	OPEN_APPLY_STALL_MS,
};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::MaintainerEffect,
	types::{FilesystemApplyError, MaintainerTimer, OpId},
};
use depot_fs_core::{FsVolumeRecord, FsVolumeStatus};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_log_shard_v1::{VolumeLogEntry, VolumeLogEntryBody, VolumeLogReplicaState},
	depot_storage_worker_protocol_v1::{self as wire, StorageWorkerRequest, StorageWorkerResponse},
	shared_depot_types_v1::{ClientIdentity, VolumeLogPosition, VolumeLogReplicaStatus},
};
use rivet_state_machine::IndexedDeadlineQueue;
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ops::Bound;

/// The authority a record lookup may use when the exact incarnation has no stored row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoadPurpose {
	/// Normal apply registration, with the existing quorum/creation recovery rules.
	Register,
	/// Stop recovery accepts a persisted record only; absence grants no creation authority.
	StopProof,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Volume {
	pub record: super::record::SharedRecord,
	pub record_persisted: bool,
	pub applied: VolumeLogPosition,
	/// The highest applied position with no filesystem transaction half applied, at or below
	/// `applied`. A latest session open serves it, so an opening can replay the log from just
	/// above it without rebuilding a transaction it cannot see the start of.
	pub settled: VolumeLogPosition,
	pub resolver: depot_fs_core::txn::Resolver,
	/// The first applied position of each client epoch, by volume seq, for every applied entry
	/// above the record's cold. The last boundary at or below cold is kept, so the entry just
	/// above cold has one. An open answers from it with the epochs of entries the opener then
	/// need not read; see [`applied_history`]. Sorted `(volume_seq, client_epoch)` pairs in an
	/// exact-size slice: a volume usually holds one boundary, which costs 16 bytes here
	/// against a whole tree node in a map.
	pub epochs: Box<[(u64, u64)]>,
	/// Changed only through [`Filesystem::set_unflushed_bytes`], which keeps the
	/// piggyback index in step.
	pub unflushed_bytes: u64,
	/// Set by the first unapersisted entry; later entries cannot postpone it.
	pub due_boot_ms: Option<u64>,
	/// Changed only through [`Filesystem::request_flush`] and
	/// [`Filesystem::clear_flush_request`], which keep `flush_candidates` in step.
	pub flush_requested: bool,
	pub retention_checked_boot_ms: u64,
	/// The volume seq of the newest retention checkpoint the store holds, as the last
	/// retention op's apply reported it, or `None` until one has. A check that finds the
	/// volume still applied at this seq logs nothing and leaves the retention queue: an idle
	/// volume costs no quorum operation and no timer per interval, and keeps its older
	/// distinct restore points. The apply that moves `applied` queues the volume again an
	/// interval later; see [`requeue_retention`].
	pub retention_checkpoint_seq: Option<u64>,
	pub retry_wait: bool,
	/// Failed read/status sources remain excluded until a fresh status succeeds.
	pub unavailable_sources: BTreeSet<WorkerId>,
	/// Mean log bytes of the entries the last apply took, or 0 before one. It estimates what
	/// an opener would read per unapplied entry.
	pub entry_bytes: u32,
}
/// Progress of one in-flight filesystem flush.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FlushStage {
	/// The executor is writing the records into the store after every earlier
	/// mutation of their volumes.
	Staging,
	/// The records are written. The executor is making them and everything before
	/// them durable; later store work proceeds and lands in a newer memtable.
	Uploading,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Page {
	key: VolumeLogKey,
	previous: VolumeLogPosition,
	through: VolumeLogPosition,
	entries: Vec<VolumeLogEntry>,
	bytes: u64,
}
/// Entries a member's status notice carried that the apply could not take yet: the volume's
/// previous page was still in flight, or no quorum had proven them committed. `replica` is
/// the member's state in that notice, the snapshot the entries were read under. The apply
/// judges them by it alone, never by a later observation of the member: the snapshot's log
/// through its tail is the claiming writer's history, so a quorum proving that writer's
/// position commits the snapshot's entries below it, whatever the member holds now.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Carried {
	worker: WorkerId,
	replica: VolumeLogReplicaState,
	entries: Vec<VolumeLogEntry>,
	bytes: u64,
	/// Order of keeping, so a full budget drops the oldest set first.
	inserted: u64,
}
/// Progress of a volume whose opens are refused while its apply catches up.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct OpenWait {
	/// The volume's applied seq when an open last checked it.
	applied_seq: u64,
	/// When an open last saw the applied seq advance, or the first refusal.
	advanced_boot_ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Filesystem {
	pub volumes: BTreeMap<VolumeLogKey, Volume>,
	/// Incarnations whose record a recovered op of their name may still write: ones without a
	/// stored record whose replicas start above zero, as a restore's or fork's publication
	/// creates them. They load again once their name's replay ends.
	pub(crate) awaiting_replay: BTreeSet<VolumeLogKey>,
	/// Loaded incarnations whose own-record lookup found nothing while a member reports a cold
	/// position above zero and no recovered op of their name can still write it. The record
	/// was written and lost, which no maintainer can repair: the replicas' creation evidence
	/// names the base chain and retention but not the rows the record covered. Such an
	/// incarnation stays loaded and unregistered for this lifetime, is never unloaded, trimmed
	/// or written to the store from memory, and its name refuses opens and reads with
	/// `VolumeRecordMissing` until the incarnation is deleted, which proceeds as any delete
	/// does, or until an operator rebuilds its record (`rebuild_record`). Written only by
	/// `mark_record_missing`, cleared only by `rebuild_record` and when the incarnation leaves
	/// memory. A successor maintainer finds the same store and marks again.
	pub(crate) record_missing: BTreeSet<VolumeLogKey>,
	pages: VecDeque<Page>,
	/// Carried entries waiting for their volume's page in flight or for a quorum to prove
	/// them, at most one set per volume. They count toward the cold byte budget like
	/// buffered pages; a full budget drops the oldest sets, and a volume's forget or quiesce
	/// drops its own.
	carried: BTreeMap<VolumeLogKey, Carried>,
	/// `carried` by order of keeping, for dropping the oldest when the budget is full.
	carried_order: BTreeSet<(u64, VolumeLogKey)>,
	/// The `inserted` of the next kept set.
	carried_next: u64,
	/// Sum of `carried` bytes.
	carried_bytes: u64,
	/// Volumes with carried entries that something may have made applicable: a member
	/// observation, an apply or a retry. `drive` offers only these, so the carried sets of
	/// quiet volumes cost nothing per event.
	carried_ready: BTreeSet<VolumeLogKey>,
	/// When `FilesystemFlushBatch` is armed for, while it is armed.
	flush_scheduled: Option<u64>,
	/// A throttled flush was requested since `drive` last ran, so `drive` schedules a
	/// flush within `memtable_flush_throttle_ms`.
	throttled_flush_wanted: bool,
	/// A prompt flush was requested since the last flush dispatched, so a flush that
	/// finishes with requests left runs the next one at once. Without one, the remaining
	/// throttled requests wait for their window.
	prompt_flush_wanted: bool,
	read_after: Option<VolumeLogKey>,
	/// Volumes whose read sources or applied position changed since `drive` last
	/// found them unreadable. `drive` evaluates only these, so its cost per event
	/// does not grow with the number of loaded volumes.
	candidates: BTreeSet<VolumeLogKey>,
	/// Loaded volumes that a change may have made eligible for registration. `drive`
	/// offers each to `load` after the lifecycle drives of the same event settled who
	/// owns the volume key.
	offered: BTreeSet<VolumeLogKey>,
	/// When each registered volume with an active retention policy is next checked, by
	/// interned key id. Entries are check times; `retention_checked_boot_ms` and the policy
	/// are the authority.
	pub(crate) retention: IndexedDeadlineQueue<KeyId>,
	/// The time `MaintainerTimer::RetentionDue` is armed for.
	pub(crate) retention_armed_at: Option<u64>,
	/// Volumes a flush skipped because work of theirs was in flight or queued ahead of
	/// it. The next flush selects them first, and the queued flush holds their later
	/// work behind it, so each is deferred at most once. Changes only in
	/// `dispatch_flush`, `forget` and `quiesce`.
	pub(crate) owed: BTreeSet<VolumeLogKey>,
	/// Volumes with `flush_requested` set that may have something to flush. Flush
	/// selection and completion visit only these, so their cost grows with the volumes
	/// that asked for a flush, not with every registered volume. Selection drops a
	/// visited volume that has nothing to flush and no reason to wait; its flag stays
	/// set, and the apply that next gives it data re-adds it through `request_flush`.
	flush_candidates: BTreeSet<VolumeLogKey>,
	/// Registered volumes with unflushed applied bytes, largest first and then in key
	/// order. Piggyback selection walks it from the front and stops below the threshold.
	unflushed: BTreeSet<(Reverse<u64>, VolumeLogKey)>,
	/// Sum of every registered volume's unflushed bytes. Kept in step by
	/// `set_unflushed_bytes`.
	unflushed_total: u64,
	/// Bytes applied since the last pass was dispatched, an estimate of the memtable. A pass
	/// uploads the whole memtable, so dispatching one resets it; at
	/// `tablet_flush_above_bytes` the tablet runs a pass of its largest volumes. Bytes of
	/// volumes no pass carries, such as one being deleted, do not keep it raised.
	memtable_bytes: u64,
	/// The highest position a loaded volume's members proved committed under the claim of
	/// the writer that appended it, kept while it is above the volume's applied position. A
	/// commit never becomes uncommitted, but that evidence lives only in the members' current
	/// state: an opener that claims them at a newer epoch without writing hides it, and apply
	/// would stop below the old writer's tail until some writer commits again. Kept here, the
	/// position keeps counting as commit evidence on members that still hold it.
	pub(crate) committed_floors: BTreeMap<VolumeLogKey, VolumeLogPosition>,
	/// Volumes whose opens are refused while their apply catches up. `drive` evaluates them
	/// before the other candidates, so an open waits for its own volume's apply, not for the
	/// whole tablet's.
	pub(crate) waiting_opens: BTreeMap<VolumeLogKey, OpenWait>,
}

impl Filesystem {
	/// Registered volumes with applied bytes not yet written to cold, largest first.
	pub(crate) fn unflushed_keys(&self) -> Vec<VolumeLogKey> {
		self.unflushed.iter().map(|(_, key)| key.clone()).collect()
	}

	/// Sets a registered volume's flush request and offers it to the next selection.
	pub fn request_flush(&mut self, key: &VolumeLogKey) {
		if let Some(volume) = self.volumes.get_mut(key) {
			volume.flush_requested = true;
			self.flush_candidates.insert(key.clone());
		}
	}

	/// Requests a flush that nobody urgently waits on. The next `drive` schedules one
	/// within `memtable_flush_throttle_ms` unless an earlier flush is already scheduled.
	pub(crate) fn request_throttled_flush(&mut self, key: &VolumeLogKey) {
		if self.volumes.contains_key(key) {
			self.request_flush(key);
			self.throttled_flush_wanted = true;
		}
	}

	/// Clears a volume's flush request after a flush carried it or deletion cancelled it.
	pub(crate) fn clear_flush_request(&mut self, key: &VolumeLogKey) {
		if let Some(volume) = self.volumes.get_mut(key) {
			volume.flush_requested = false;
		}
		self.flush_candidates.remove(key);
	}

	/// Sets a registered volume's unflushed byte count and its place in the piggyback
	/// index.
	pub fn set_unflushed_bytes(&mut self, key: &VolumeLogKey, bytes: u64) {
		let Some(volume) = self.volumes.get_mut(key) else {
			return;
		};
		let old = std::mem::replace(&mut volume.unflushed_bytes, bytes);
		self.unflushed_total = self
			.unflushed_total
			.saturating_sub(old)
			.saturating_add(bytes);
		if old > 0 {
			self.unflushed.remove(&(Reverse(old), key.clone()));
		}
		if bytes > 0 {
			self.unflushed.insert((Reverse(bytes), key.clone()));
		}
	}

	/// Bytes of carried entries waiting for their volume's page or a commit proof.
	pub(crate) fn carried_bytes(&self) -> u64 {
		self.carried_bytes
	}

	/// Unflushed applied bytes of every registered volume together.
	pub fn unflushed_total(&self) -> u64 {
		self.unflushed_total
	}

	/// Volumes flush selection and completion visit, for cost assertions in tests.
	#[doc(hidden)]
	pub fn flush_candidate_count(&self) -> usize {
		self.flush_candidates.len()
	}
}



/// Counts one volume visited by flush selection or completion. Test builds only.
fn count_flush_visit() {
	
}

/// Whether two filesystem states are equal apart from which volumes are marked for the
/// next `drive` and which are offered for registration.
#[cfg(debug_assertions)]
pub(crate) fn same_ignoring_marks(a: &Filesystem, b: &Filesystem) -> bool {
	a.volumes == b.volumes
		&& a.pages == b.pages
		&& a.carried == b.carried
		&& a.carried_order == b.carried_order
		&& a.carried_next == b.carried_next
		&& a.carried_bytes == b.carried_bytes
		&& a.flush_scheduled == b.flush_scheduled
		&& a.throttled_flush_wanted == b.throttled_flush_wanted
		&& a.prompt_flush_wanted == b.prompt_flush_wanted
		&& a.read_after == b.read_after
		&& a.retention == b.retention
		&& a.retention_armed_at == b.retention_armed_at
		&& a.owed == b.owed
		&& a.flush_candidates == b.flush_candidates
		&& a.unflushed == b.unflushed
		&& a.unflushed_total == b.unflushed_total
		&& a.memtable_bytes == b.memtable_bytes
		&& a.committed_floors == b.committed_floors
		&& a.waiting_opens == b.waiting_opens
}

/// Asks the next `drive` to register a loaded volume if it has become eligible.
pub fn offer(state: &mut MaintainerState, key: &VolumeLogKey) {
	if state.loaded_volumes.contains_key(key) && !state.filesystem.volumes.contains_key(key) {
		state.filesystem.offered.insert(key.clone());
	}
}

/// Registers every eligible loaded volume and marks every registered one, so the next
/// `drive` evaluates all of them. A new assignment changes quorum eligibility even
/// without another member observation; the debug oracle also uses this for its full pass.
pub(crate) fn mark_all(state: &mut MaintainerState) {
	let keys = state.loaded_volumes.keys().cloned().collect::<Vec<_>>();
	for key in keys {
		offer(state, &key);
		mark_candidate(state, &key);
	}
}

/// Marks a volume for the next `drive` after anything that may make it readable, or may let
/// its carried entries be applied.
pub fn mark_candidate(state: &mut MaintainerState, key: &VolumeLogKey) {
	if state.filesystem.volumes.contains_key(key) {
		state.filesystem.candidates.insert(key.clone());
	}
	if state.filesystem.carried.contains_key(key) {
		state.filesystem.carried_ready.insert(key.clone());
	}
}

/// Registers a loaded volume for filesystem apply once it is eligible, or queues the
/// lookup of its record. Every change that can make a loaded volume eligible calls this;
/// a registered volume returns after one map lookup.
pub(crate) fn load(state: &mut MaintainerState, key: VolumeLogKey) {
	load_for(state, key, LoadPurpose::Register);
}

/// Reads the durable record of a recovered volume that lacks an in-sync quorum. The
/// result supplies metadata only; later status barriers still own every stop proof.
pub(crate) fn load_stop_record(state: &mut MaintainerState, key: VolumeLogKey) {
	load_for(state, key, LoadPurpose::StopProof);
}

fn load_for(state: &mut MaintainerState, key: VolumeLogKey, purpose: LoadPurpose) {
	if state.filesystem.volumes.contains_key(&key) {
		return;
	}
	if state.phase != MaintainerPhase::Serving
		|| retiring(state, &key)
		|| !state.loaded_volumes.contains_key(&key)
		|| state.filesystem.volumes.contains_key(&key)
		|| state.filesystem.awaiting_replay.contains(&key)
		|| state.filesystem.record_missing.contains(&key)
	{
		return;
	}
	let loaded = &state.loaded_volumes[&key];
	// A deleted incarnation is registered only while its delete applies the tail it left.
	let draining = super::drain::applying(state, &key);
	if (loaded.is_deleted() && !draining)
		|| loaded.creation.as_ref().is_some_and(|v| !v.acknowledged)
	{
		return;
	}
	// A delete applying the tail its tombstones kept reads it from one tombstone holding it:
	// the delete fixed that tail from a quorum, and other members may already be forgotten.
	if loaded.creation.is_none()
		&& loaded.stored_row.is_none()
		&& purpose == LoadPurpose::Register
		&& !draining
		&& !super::membership::quorum(&state.tablet.replica_set, |worker| {
			loaded
				.members
				.get(worker)
				.is_some_and(|r| r.status == VolumeLogReplicaStatus::InSync)
		}) {
		return;
	}
	if let Some(record) = state
		.loaded_volumes
		.get(&key)
		.and_then(|v| v.creation.as_ref())
		.filter(|v| v.acknowledged)
		.filter(|_| purpose == LoadPurpose::Register)
		.map(|v| v.row.clone())
	{
		register(state, key, record, false);
		return;
	}
	if state.store_queue.contains(&StoreWork::FilesystemLoad {
		key: key.clone(),
		purpose: LoadPurpose::Register,
	}) || state.operations.index().filesystem_loads.contains_key(&key)
	{
		return;
	}
	state
		.store_queue
		.push_back(StoreWork::FilesystemLoad { key, purpose });
}
/// Whether `key`'s registration for filesystem apply is running: its record lookup is queued
/// or in flight. It offers the volume for registration first, so an eligible volume starts
/// one now and a volume that registers at once returns false.
pub(crate) fn registering(state: &mut MaintainerState, key: &VolumeLogKey) -> bool {
	load(state, key.clone());
	state.store_queue.contains(&StoreWork::FilesystemLoad {
		key: key.clone(),
		purpose: LoadPurpose::Register,
	}) || state.operations.index().filesystem_loads.contains_key(key)
}

/// Re-evaluates a volume after a member observation was recorded outside
/// `cold::observe`: it may now be eligible for registration or readable.
pub(crate) fn member_observed(state: &mut MaintainerState, key: &VolumeLogKey) {
	raise_committed_floor(state, key);
	super::catch_up::mark(state, key);
	super::holders::claim_observed(state, key);
	offer(state, key);
	mark_candidate(state, key);
}

/// Keeps the position `key`'s members now prove committed under their writer's own claim,
/// when it is above what is applied and above any position kept before. Called after each
/// member observation, so a later claim that hides that evidence does not lose it.
pub(crate) fn raise_committed_floor(state: &mut MaintainerState, key: &VolumeLogKey) {
	let Some((proven, _)) = claim_quorum_commit(state, key) else {
		return;
	};
	let applied = state
		.filesystem
		.volumes
		.get(key)
		.map_or(0, |volume| volume.applied.volume_seq);
	let kept = state
		.filesystem
		.committed_floors
		.get(key)
		.map_or(0, |position| position.volume_seq);
	if proven.volume_seq > applied && proven.volume_seq > kept {
		state
			.filesystem
			.committed_floors
			.insert(key.clone(), proven);
	}
}

/// Whether `key`'s apply lags what its members prove committed by more than an opener
/// should replay itself: `OPEN_APPLY_LAG_ENTRIES` entries, or `OPEN_APPLY_LAG_BYTES` at the
/// mean entry size of its last apply. Either one is about a page or more of the opener's
/// replay for every round trip it can afford before its open deadline.
fn far_behind(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	let Some(volume) = state.filesystem.volumes.get(key) else {
		return false;
	};
	let lag = known_committed(state, key).saturating_sub(volume.applied.volume_seq);
	lag > OPEN_APPLY_LAG_ENTRIES
		|| lag.saturating_mul(u64::from(volume.entry_bytes)) > OPEN_APPLY_LAG_BYTES
}

/// Whether an open of `volume_key` must wait for the maintainer's apply. It must while the
/// volume's apply is far behind what its members prove committed (see `far_behind`) and
/// has advanced within `OPEN_APPLY_STALL_MS`: the opener would otherwise replay all of it
/// within its open deadline, and its claim, which never commits if that deadline passes,
/// gains nothing. The volume's apply then goes first. An apply that stopped advancing lets
/// opens through, so they replay the entries themselves as before. The incarnation is the
/// one an open selects: the newest by incarnation epoch.
pub(crate) fn open_waits_for_apply(
	state: &mut MaintainerState,
	volume_key: &depot_types::VolumeKey,
) -> bool {
	let key = state
		.loaded_volumes
		.range(super::incarnations(volume_key))
		.filter(|(key, _)| !super::deletion::is_replaced(state, key))
		.max_by_key(|(_, volume)| super::replica_creation::known_epoch(volume))
		.map(|(key, _)| key.clone());
	let Some(key) = key else {
		return false;
	};
	let Some(applied_seq) = state
		.filesystem
		.volumes
		.get(&key)
		.map(|volume| volume.applied.volume_seq)
	else {
		return false;
	};
	if !far_behind(state, &key) {
		state.filesystem.waiting_opens.remove(&key);
		return false;
	}

	let now_boot_ms = state.now_boot_ms;
	let wait = state
		.filesystem
		.waiting_opens
		.entry(key.clone())
		.or_insert(OpenWait {
			applied_seq,
			advanced_boot_ms: now_boot_ms,
		});
	if wait.applied_seq != applied_seq {
		wait.applied_seq = applied_seq;
		wait.advanced_boot_ms = now_boot_ms;
	}
	if now_boot_ms.saturating_sub(wait.advanced_boot_ms) >= OPEN_APPLY_STALL_MS {
		return false;
	}
	mark_candidate(state, &key);
	true
}

/// Offers every loaded incarnation of `volume_key` for registration, after a lifecycle
/// change of one incarnation, such as a resolved deletion, can make the others eligible.
pub(crate) fn load_incarnations(state: &mut MaintainerState, volume_key: &depot_types::VolumeKey) {
	let keys = state
		.loaded_volumes
		.range(super::incarnations(volume_key))
		.map(|(key, _)| key.clone())
		.collect::<Vec<_>>();
	for key in keys {
		offer(state, &key);
	}
}

/// Whether a lookup of `key`'s own record that found nothing means the record is lost: the
/// incarnation is loaded and unregistered, carries no metadata of its own (no stored row,
/// creation or cold open), a member reports a cold position above zero, so a record covering
/// it was written, and no recovered op of its name can still write one. The caller supplies
/// the lookup's answer; this judges only what memory shows.
pub(crate) fn record_lost(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	let Some(volume) = state.loaded_volumes.get(key) else {
		return false;
	};
	let metadata = volume.stored_row.is_some()
		|| volume.creation.is_some()
		|| volume.unacknowledged_cold_open.is_some();
	let above_cold = volume
		.members
		.values()
		.any(|replica| replica.cold.volume_seq > 0);
	!state.filesystem.volumes.contains_key(key)
		&& !metadata
		&& above_cold
		&& !super::maintainer_log::replay::pending_for(state, &key.0.volume_key)
}

/// Quarantines `key`, whose stored record is lost (see `Filesystem::record_missing`). Once per
/// incarnation and lifetime it warns, counts, cancels an unload that began before the lookup
/// answered (its row write would fabricate a record from the replicas' creation evidence and
/// forget them), drops the incarnation's queued live record and its queued trims; a deleting
/// incarnation keeps its tombstone queued.
pub(crate) fn mark_record_missing(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	fx: &mut Effects,
) {
	if !state.filesystem.record_missing.insert(key.clone()) {
		return;
	}
	let observed = state
		.loaded_volumes
		.get(key)
		.and_then(|volume| volume.row(key))
		.map(|row| row.cold.clone());
	tracing::warn!(
		?key,
		?observed,
		"a volume has no stored record but a replica reports a cold position above zero; the record is lost, so the incarnation stays unregistered and its name is refused until the incarnation is deleted"
	);
	crate::metrics::TABLET_FS_VOLUME_RECORD_MISSING.inc();
	if state.unloading.contains_key(key) {
		super::unload::cancel(state, key, fx);
	}
	if !state.deleting.contains_key(key) {
		state.pending_rows.remove(key);
	}
	// Trims queued from the replicas' own cold positions before the lookup answered would
	// drop log entries below a position the lost record was to cover.
	state.pending_trims.remove(key);
	state.trim_ready.retain(|(queued, _)| queued != key);
	// A trim already sent before the lookup answered would raise a lagging member the same
	// way; its batch's other volumes go back to the ready set.
	let trims = state
		.operations
		.index()
		.trims
		.range(super::operations::workers_of(key))
		.flat_map(|(_, ids)| ids)
		.copied()
		.collect::<Vec<_>>();
	for id in trims {
		if let Some(Operation::TrimBatch { worker, trims }) = state.finish(id, fx) {
			for (other, _) in trims {
				if &other != key {
					state.trim_ready.insert((other, worker.clone()));
				}
			}
		}
		fx.execute(MaintainerEffect::CancelReplicaRequest { op_id: id });
	}
	// Reads parked for an apply that will never come are answered now: the typed refusal, or
	// what a read of a deleting incarnation gets.
	if let Some(waits) = state.lagging_opens.remove(key) {
		let deleting = state.deleting.contains_key(key);
		for (_, request_id) in waits {
			if deleting {
				super::filesystem_reads::reply(
					state,
					request_id,
					rivet_data::generated::shared_depot_fs_read_v1::FsReadResponse::FsReadError(
						rivet_data::generated::shared_depot_fs_read_v1::FsReadError::NotFound,
					),
					fx,
				);
			} else {
				super::filesystem_reads::reject(
					state,
					request_id,
					depot_types::client::record_missing(key.0.volume_incarnation),
					fx,
				);
			}
		}
	}
}

/// Ends the quarantine of `key` with the record an operator chose, the one its replicas'
/// creation evidence and cold positions describe (see `operator_repair::quarantine`): the
/// incarnation registers with it as a record the store does not hold yet, which the
/// tablet's ordinary pass writes, and then serves, unloads and trims like any other volume.
/// The caller checked that `key` is quarantined, loaded and not being deleted.
pub(crate) fn rebuild_record(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	record: FsVolumeRecord,
	fx: &mut Effects,
) {
	if !state.filesystem.record_missing.remove(&key) {
		return;
	}
	super::catch_up::mark(state, &key);
	register(state, key.clone(), record, false);
	if state.filesystem.volumes.contains_key(&key) {
		if !state.pending_rows.contains(&key) {
			unrecorded(state, key.clone(), fx);
		}
		mark_candidate(state, &key);
	}
	super::unload::track_idle(state, &key);
}

/// Whether `key` is quarantined for a lost record, deleting or not. Every internal gate that
/// keeps such an incarnation from registering, unloading, trimming or writing uses this.
pub(crate) fn is_record_missing(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	state.filesystem.record_missing.contains(key)
}

/// The quarantined incarnation of `volume_key` that a request on the name would reach, if
/// any: one no newer acknowledged incarnation replaced and no delete holds. Opens, reads,
/// checkpoints and forks of the name are refused with it. Once its delete is admitted the name
/// is free again, so a create goes past the acknowledged delete as it does for any other.
pub(crate) fn record_missing(
	state: &MaintainerState,
	volume_key: &depot_types::VolumeKey,
) -> Option<[u8; 16]> {
	record_missing_incarnations(state, volume_key)
		.find(|key| !super::deletion::is_replaced(state, key) && !state.deleting.contains_key(key))
		.map(|key| key.0.volume_incarnation)
}

/// Every quarantined incarnation of `volume_key`, deleting or not.
fn record_missing_incarnations<'a>(
	state: &'a MaintainerState,
	volume_key: &depot_types::VolumeKey,
) -> impl Iterator<Item = &'a VolumeLogKey> + 'a {
	state
		.filesystem
		.record_missing
		.range(super::incarnations(volume_key))
}

pub(crate) fn loaded(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<Option<FsVolumeRecord>, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(Operation::FilesystemLoad { .. })
	) {
		return;
	}
	let Some(Operation::FilesystemLoad { key, purpose }) = state.finish(id, fx) else {
		unreachable!()
	};
	if retiring(state, &key) || !state.loaded_volumes.contains_key(&key) {
		return;
	}
	// A lookup queued before the incarnation was quarantined answers late; the quarantine
	// stands, since nothing it decided has changed.
	if is_record_missing(state, &key) {
		return;
	}
	if purpose == LoadPurpose::StopProof && !matches!(&result, Ok(Some(_))) {
		// A stop lookup cannot synthesize a cold-zero row from creation evidence. A lost
		// durable record retains the usual quarantine; transient read failure only blocks
		// this stop and leaves the next fresh barrier to retry, without fencing the grant.
		if matches!(&result, Ok(None)) && record_lost(state, &key) {
			mark_record_missing(state, &key, fx);
		}
		return;
	}
	// The lookup reads this incarnation's own record, so it never names another one.
	let persisted = matches!(&result, Ok(Some(_)));
	let record = match result {
		Ok(Some(record)) => record,
		Ok(None) => match state
			.loaded_volumes
			.get(&key)
			.and_then(|v| v.row(&key))
			.filter(|r| r.cold.volume_seq == 0)
		{
			Some(record) => record,
			None => {
				// A recovered maintainer log op of this name may still write the record: the
				// publication of a restore or fork creates replicas at its image position,
				// and replays once the incarnation it names applied that far. The volume
				// is offered again when that replay ends.
				if super::maintainer_log::replay::pending_for(state, &key.0.volume_key) {
					tracing::debug!(
						?key,
						"a volume without a stored record waits for its name's recovered maintainer log ops"
					);
					state.filesystem.awaiting_replay.insert(key);
					return;
				}
				// Otherwise the replicas must hold the whole log: a replica cold position
				// above zero says a record covering it was written and lost, which no
				// maintainer can repair. A fence would hand the same store to a successor, so
				// the incarnation is quarantined instead; see `Filesystem::record_missing`.
				if record_lost(state, &key) {
					mark_record_missing(state, &key, fx);
					return;
				}
				// The lookup found nothing although the volume carries metadata that says a
				// record exists, such as the row a confirmed unload read: the store and this
				// maintainer's memory disagree, so its lifetime ends.
				tracing::warn!(
					?key,
					"a volume's stored record lookup found nothing although its loaded metadata names one; fencing"
				);
				fence(state, fx);
				return;
			}
		},
		Err(_) => {
			fence(state, fx);
			return;
		}
	};
	if record.partition_key != key.0.volume_key.partition_key
		|| record.name != key.0.volume_key.name
	{
		fence(state, fx);
		return;
	}
	if record.volume_incarnation != key.0.volume_incarnation {
		fence(state, fx);
		return;
	}
	// The fallback row of a volume without a stored row reports its tombstones; a delete
	// applying their tail registers it as it was before the delete.
	let mut record = record;
	if !persisted && super::drain::applying(state, &key) {
		record.status = FsVolumeStatus::Live;
	}
	if record.status == FsVolumeStatus::Deleted {
		if let Some(volume) = state.loaded_volumes.get_mut(&key) {
			volume.stored_row = Some(record.into());
		}
		super::deletion::observe_loaded(state, &key);
		return;
	}
	if persisted {
		if let Some(volume) = state.loaded_volumes.get_mut(&key) {
			volume.stored_row = Some(record.clone().into());
		}
	}
	super::catch_up::mark(state, &key);
	// A deleting incarnation's rows ride its delete's pass, never a first record of its own.
	let queue =
		!persisted && !state.pending_rows.contains(&key) && !state.deleting.contains_key(&key);
	register(state, key.clone(), record, persisted);
	if queue && state.filesystem.volumes.contains_key(&key) {
		unrecorded(state, key, fx);
	}
}

/// Queues a registered volume whose record the store does not hold yet for the tablet's
/// ordinary pass. The record is durable in the replicas' creation evidence and the logs, so
/// it waits for the queue's age timer like any other first record.
pub(crate) fn unrecorded(state: &mut MaintainerState, key: VolumeLogKey, fx: &mut Effects) {
	if state.pending_rows.push_back(key) {
		super::cold_pass::row_queued(state, fx);
	}
}

fn register(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	record: FsVolumeRecord,
	persisted: bool,
) {
	// A load can finish after its volume left memory. Registering it then would break the
	// invariant that filesystem volumes are a subset of loaded volumes, which `drive` uses.
	if state.filesystem.volumes.contains_key(&key) || !state.loaded_volumes.contains_key(&key) {
		return;
	}
	// A record the store does not hold yet stays queued for the tablet's ordinary pass,
	// which the queue's age timer starts. A stored record needs no pass of its own.
	if persisted {
		state.pending_rows.remove(&key);
	}
	let cold_seq = record.cold.volume_seq;
	state.filesystem.volumes.insert(
		key.clone(),
		Volume {
			applied: record.cold.clone(),
			settled: record.cold.clone(),
			resolver: depot_fs_core::txn::Resolver::new(record.cold.clone()),
			epochs: Default::default(),
			record: record.into(),
			record_persisted: persisted,
			unflushed_bytes: 0,
			due_boot_ms: None,
			flush_requested: false,
			// The first check comes an interval after registration: the boot clock counts
			// from host boot, so a check from zero would be due at once for every load.
			retention_checked_boot_ms: state.now_boot_ms,
			retention_checkpoint_seq: None,
			retry_wait: false,
			unavailable_sources: BTreeSet::new(),
			entry_bytes: 0,
		},
	);
	wake_lagging_opens(state, &key, cold_seq);
	schedule_retention(state, &key);
	mark_candidate(state, &key);
}

/// A lifecycle lookup can recover a newer durable floor than replica observations.
/// Discard older in-flight reads before publishing that floor to the apply driver.
pub(crate) fn stored(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	record: &FsVolumeRecord,
	fx: &mut Effects,
) {
	let Some(volume) = state.filesystem.volumes.get(key) else {
		return;
	};
	if record.cold.volume_seq < volume.record.cold.volume_seq {
		return;
	}
	let advanced = record.cold.volume_seq > volume.applied.volume_seq;
	if advanced {
		let reads = filesystem_reads_of(state, key);
		for id in reads {
			state.finish(id, fx);
			fx.execute(MaintainerEffect::CancelReplicaRequest { op_id: id });
		}
		state.filesystem.pages.retain(|page| &page.key != key);
		state
			.store_queue
			.remove_all(&[StoreWork::FilesystemApply { key: key.clone() }]);
	}
	let volume = state
		.filesystem
		.volumes
		.get_mut(key)
		.expect("filesystem owner");
	if record.cold.volume_seq >= volume.applied.volume_seq {
		let cold_seq = record.cold.volume_seq;
		volume.applied = record.cold.clone();
		volume.settled = record.cold.clone();
		volume.resolver = depot_fs_core::txn::Resolver::new(record.cold.clone());
		volume.epochs = Default::default();
		volume.due_boot_ms = None;
		fx.cancel_timer(MaintainerTimer::FilesystemDue(key.clone()));
		state.filesystem.set_unflushed_bytes(key, 0);
		if state
			.filesystem
			.committed_floors
			.get(key)
			.is_some_and(|floor| floor.volume_seq <= cold_seq)
		{
			state.filesystem.committed_floors.remove(key);
		}
		if !far_behind(state, key) {
			state.filesystem.waiting_opens.remove(key);
		}
		wake_lagging_opens(state, key, cold_seq);
	}
	let volume = state
		.filesystem
		.volumes
		.get_mut(key)
		.expect("filesystem owner");
	let unapersisted = volume
		.record
		.recent_writes
		.iter()
		.filter(|r| r.volume_seq > record.cold.volume_seq)
		.cloned()
		.collect::<Vec<_>>();
	// A record with a different policy was written with that policy applied to the retention
	// rows, which can release the checkpoints the maintainer knew; the next check learns again.
	let policy_changed = volume.record.retention != record.retention;
	if policy_changed {
		volume.retention_checkpoint_seq = None;
	}
	volume.record = record.clone().into();
	merge_receipts(&mut volume.record, unapersisted);
	volume.record_persisted = true;
	mark_candidate(state, key);
	if policy_changed {
		schedule_retention(state, key);
	} else if advanced {
		requeue_retention(state, key);
	}
}

pub(crate) fn owns(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	state.filesystem.volumes.contains_key(key)
		|| state.store_queue.contains(&StoreWork::FilesystemLoad {
			key: key.clone(),
			purpose: LoadPurpose::Register,
		}) || state.operations.index().filesystem_loads.contains_key(key)
}
/// A hint asks for fresh status; the hinted seq itself never supplies a commitment proof.
/// A parked claim read's floor is proof, but it counts through `committed_on` while that read
/// waits, not through this hint.
pub(crate) fn committed(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	seq: u64,
	fx: &mut Effects,
) {
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	// A loaded volume still registering for apply has nothing applied yet, so a hint names
	// a position it lacks; polling its replicas now lets the apply start as soon as the
	// registration lands instead of at the next scheduled poll.
	let behind = match state.filesystem.volumes.get(&key) {
		Some(volume) => volume.applied.volume_seq < seq,
		None => owns(state, &key),
	};
	if behind {
		super::poll::request(state, key, fx);
	}
}
pub(crate) fn queued_bytes(state: &MaintainerState) -> u64 {
	state
		.filesystem
		.pages
		.iter()
		.map(|p| p.bytes * 2)
		.sum::<u64>()
		.saturating_add(state.filesystem.carried_bytes)
}

/// Takes `key`'s carried entries, if any.
fn take_carried(state: &mut MaintainerState, key: &VolumeLogKey) -> Option<Carried> {
	let carried = state.filesystem.carried.remove(key)?;
	state.filesystem.carried_bytes = state.filesystem.carried_bytes.saturating_sub(carried.bytes);
	state
		.filesystem
		.carried_order
		.remove(&(carried.inserted, key.clone()));
	state.filesystem.carried_ready.remove(key);
	Some(carried)
}

/// Drops `key`'s carried entries, if any.
fn drop_carried(state: &mut MaintainerState, key: &VolumeLogKey) {
	take_carried(state, key);
}

/// Keeps `carried` for a later offer, replacing `key`'s kept set, within the cold byte
/// budget: other volumes' sets make room, oldest first, and a set larger than one read is
/// not kept at all. A replaced set keeps its place in the order, so offering a kept set
/// again changes nothing. Returns whether it was kept.
fn keep_carried(state: &mut MaintainerState, key: &VolumeLogKey, mut carried: Carried) -> bool {
	let replaced = take_carried(state, key);
	if carried.bytes > u64::from(state.config.tuning().max_cold_read_bytes) {
		return false;
	}
	while super::cold::bytes(state).saturating_add(carried.bytes) > MAX_COLD_BYTES {
		let Some((_, oldest)) = state.filesystem.carried_order.first().cloned() else {
			break;
		};
		drop_carried(state, &oldest);
	}
	if super::cold::bytes(state).saturating_add(carried.bytes) > MAX_COLD_BYTES {
		return false;
	}
	carried.inserted = match replaced {
		Some(replaced) => replaced.inserted,
		None => {
			let inserted = state.filesystem.carried_next;
			state.filesystem.carried_next = state.filesystem.carried_next.wrapping_add(1);
			inserted
		}
	};
	state
		.filesystem
		.carried_order
		.insert((carried.inserted, key.clone()));
	state.filesystem.carried_bytes = state.filesystem.carried_bytes.saturating_add(carried.bytes);
	state.filesystem.carried.insert(key.clone(), carried);
	true
}

/// Offers the carried entries of volumes marked since they were kept: a page of theirs
/// finished, an observation may have proven them committed, or the budget that held their
/// page back may have room. A volume whose page is still in flight stays marked for the
/// event that finishes it.
fn apply_carried(state: &mut MaintainerState, fx: &mut Effects) {
	if state.filesystem.carried_ready.is_empty() {
		return;
	}
	for key in std::mem::take(&mut state.filesystem.carried_ready) {
		if !state.filesystem.carried.contains_key(&key) {
			continue;
		}
		if apply_in_progress(state, &key) {
			state.filesystem.carried_ready.insert(key);
			continue;
		}
		offer_carried(state, &key, fx);
	}
}

/// Offers `key`'s carried entries to `pushed` again under the snapshot they were read with.
/// The kept set stays in place meanwhile, so `pushed` merges the offer into it and an
/// offer that can take nothing leaves the state as it was.
fn offer_carried(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	let Some(carried) = state.filesystem.carried.get(key).cloned() else {
		return;
	};
	pushed(
		state,
		key,
		&carried.worker,
		&carried.replica,
		carried.entries,
		fx,
	);
}

/// Whether a read, buffered page or apply of `key` is in flight.
fn apply_in_progress(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	let index = state.operations.index();
	state.filesystem.pages.iter().any(|page| &page.key == key)
		|| index.filesystem_reads.contains_key(key)
		|| index.filesystem_applies.contains_key(key)
}
pub(crate) fn drive(state: &mut MaintainerState, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	// Every change that can make a loaded volume eligible offers it. The debug oracle
	// checks that no change is missed.
	for key in std::mem::take(&mut state.filesystem.offered) {
		load(state, key);
	}
	if std::mem::take(&mut state.filesystem.throttled_flush_wanted) {
		schedule_flush_within(state, state.config.memtable_flush_throttle_ms, fx);
	}
	apply_carried(state, fx);
	if state.filesystem.candidates.is_empty() {
		return;
	}
	// This runs after every maintainer event, so it must not scan operations or loaded
	// volumes. Reads and applies in flight come from the operation index. Buffered pages
	// stay a scan: the cold byte budget bounds them to a handful.
	let reads = state.operations.index().counts.filesystem_read as usize;
	let mut busy = BTreeSet::new();
	for page in &state.filesystem.pages {
		busy.insert(&page.key);
	}
	let max_reads = state.config.max_background_cold_reads as usize;
	let read_budget = super::cold::read_budget(state);
	// Each started read reserves `read_budget`, exactly as `cold::bytes` counts it.
	let mut cold_bytes = super::cold::bytes(state);
	let round_robin: Box<dyn Iterator<Item = &VolumeLogKey>> = match &state.filesystem.read_after {
		Some(after) => Box::new(
			state
				.filesystem
				.candidates
				.range((Bound::Excluded(after), Bound::Unbounded))
				.chain(
					state
						.filesystem
						.candidates
						.range((Bound::Unbounded, Bound::Included(after))),
				),
		),
		None => Box::new(state.filesystem.candidates.iter()),
	};
	// Volumes with refused opens go first, in at most half the read slots so the others'
	// apply and trim never stop; every candidate, including further waiting ones, then
	// follows in round-robin order.
	let priority: Vec<&VolumeLogKey> = state
		.filesystem
		.waiting_opens
		.keys()
		.filter(|key| state.filesystem.candidates.contains(*key))
		.take((max_reads / 2).max(1))
		.collect();
	let ordered = priority
		.iter()
		.copied()
		.chain(round_robin.filter(|key| !priority.contains(key)));
	let mut selected = Vec::new();
	// Candidates found unreadable leave the set. Whatever later makes them readable
	// marks them again: a member observation, an apply, or a retry timer.
	let mut evaluated = Vec::new();
	for key in ordered {
		if reads + selected.len() >= max_reads || cold_bytes + read_budget > MAX_COLD_BYTES {
			break;
		}
		evaluated.push(key.clone());
		let Some(volume) = state.filesystem.volumes.get(key) else {
			continue;
		};
		// A recovered maintainer log op holds the volume's apply at its position.
		let held = super::maintainer_log::replay::apply_limit(state, key)
			.is_some_and(|limit| volume.applied.volume_seq >= limit);
		if volume.retry_wait
			|| held || busy.contains(key)
			|| state.operations.index().filesystem_reads.contains_key(key)
			|| state
				.operations
				.index()
				.filesystem_applies
				.contains_key(key)
			|| retiring(state, key)
		{
			continue;
		}
		let Some((worker, proof)) = state
			.loaded_volumes
			.get(key)
			.and_then(|v| {
				v.members.read_source(
					state.config.local_worker.as_ref(),
					|w, r| {
						!volume.unavailable_sources.contains(w)
							&& (r.status == VolumeLogReplicaStatus::InSync
								|| (r.status == VolumeLogReplicaStatus::Deleted
									&& super::drain::applying(state, key)))
							&& r.cold.volume_seq <= volume.applied.volume_seq
							&& committed_on(state, key, r) > volume.applied.volume_seq
					},
					// Among the members that hold the most, read the lowest worker ID: the
					// entry carrier when this worker is not a member, whose shard most
					// likely still holds the entries in memory.
					|candidates| {
						candidates.max_by_key(|(w, r)| (committed_on(state, key, r), Reverse(*w)))
					},
				)
			})
			.map(|(w, r)| (w.clone(), r.clone()))
		else {
			continue;
		};
		cold_bytes += read_budget;
		selected.push((key.clone(), worker, proof, volume.applied.clone()));
	}

	for key in evaluated {
		state.filesystem.candidates.remove(&key);
	}
	for (key, worker, proof, previous) in selected {
		if !state.filesystem.waiting_opens.contains_key(&key) {
			state.filesystem.read_after = Some(key.clone());
		}
		let id = state.begin(
			Operation::FilesystemRead {
				key: key.clone(),
				worker: worker.clone(),
				proof,
				previous: previous.clone(),
			},
			fx,
		);
		fx.execute(MaintainerEffect::ReplicaRequest {
			op_id: id,
			worker_id: worker,
			request: StorageWorkerRequest::ReadVolumeLogEntriesRequest(
				wire::ReadVolumeLogEntriesRequest {
					// A delete's drain reads the tail its tombstones keep.
					include_deleted: super::drain::applying(state, &key),
					volume_log_key: key.0,
					first_volume_seq: previous.volume_seq + 1,
					max_bytes: state.config.tuning().max_cold_read_bytes,
				},
			),
		});
	}
}
pub(crate) fn read(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	worker: WorkerId,
	proof: VolumeLogReplicaState,
	previous: VolumeLogPosition,
	result: Result<StorageWorkerResponse, String>,
	fx: &mut Effects,
) {
	let Ok(StorageWorkerResponse::ReadVolumeLogEntriesResponse(mut page)) = result else {
		source_failed(state, &key, worker);
		retry(state, key, fx);
		return;
	};
	// A delete's drain reads its tail from a tombstone.
	let readable = page.state.status == VolumeLogReplicaStatus::InSync
		|| (page.state.status == VolumeLogReplicaStatus::Deleted
			&& super::drain::applying(state, &key));
	if !readable || page.state.replica_incarnation != proof.replica_incarnation {
		super::cold::observe(state, &key, worker, page.state);
		retry(state, key, fx);
		return;
	}
	let committed = committed_on(state, &key, &proof).min(committed_on(state, &key, &page.state));
	let bytes = page
		.entries
		.iter()
		.map(depot_types::log_shard::entry_bytes)
		.sum::<u64>();
	if bytes > u64::from(state.config.tuning().max_cold_read_bytes) {
		fence(state, fx);
		return;
	}
	// Apply stops at the position of the volume's next recovered maintainer log op.
	let limit = super::maintainer_log::replay::apply_limit(state, &key).unwrap_or(u64::MAX);
	page.entries
		.retain(|e| e.volume_seq <= committed && e.volume_seq <= limit);
	super::cold::observe(state, &key, worker, page.state);
	if page.entries.is_empty() {
		retry(state, key, fx);
		return;
	}
	let bytes = page
		.entries
		.iter()
		.map(depot_types::log_shard::entry_bytes)
		.sum::<u64>();
	accept_page(state, key, previous, page.entries, bytes, fx);
}

/// Queues committed `entries` that continue `previous` for apply. Fences, and returns false,
/// when they do not continue it or the volume's applied position moved meanwhile: either
/// means the maintainer's view of the log is wrong.
fn accept_page(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	previous: VolumeLogPosition,
	entries: Vec<VolumeLogEntry>,
	bytes: u64,
	fx: &mut Effects,
) -> bool {
	let mut through = previous.clone();
	for entry in &entries {
		if through.volume_seq.checked_add(1) != Some(entry.volume_seq)
			|| entry.client_epoch < through.client_epoch
			|| !matches!(
				entry.body,
				VolumeLogEntryBody::ClaimEntry | VolumeLogEntryBody::PayloadWrite(_)
			) {
			fence(state, fx);
			return false;
		}
		through = VolumeLogPosition {
			volume_seq: entry.volume_seq,
			client_epoch: entry.client_epoch,
		};
	}
	if state
		.filesystem
		.volumes
		.get(&key)
		.is_none_or(|v| v.applied != previous)
	{
		fence(state, fx);
		return false;
	}
	state.filesystem.pages.push_back(Page {
		key: key.clone(),
		previous,
		through,
		entries,
		bytes,
	});
	state
		.store_queue
		.push_back(StoreWork::FilesystemApply { key });
	true
}

/// Whether two carried sets come from one member under one writer, so entries at equal
/// positions are the same entry and the sets may be merged.
fn same_lineage(kept: &VolumeLogReplicaState, replica: &VolumeLogReplicaState) -> bool {
	kept.replica_incarnation == replica.replica_incarnation
		&& kept.claimed_client == replica.claimed_client
		&& kept.appended.client_epoch == replica.appended.client_epoch
}

/// The union of two runs of consecutive entries from one lineage that overlap or touch.
fn merge_runs(kept: Vec<VolumeLogEntry>, new: Vec<VolumeLogEntry>) -> Vec<VolumeLogEntry> {
	let kept_first = kept.first().map(|entry| entry.volume_seq);
	let new_first = new.first().map(|entry| entry.volume_seq);
	let (mut low, high) = if kept_first <= new_first {
		(kept, new)
	} else {
		(new, kept)
	};
	let low_last = low.last().map_or(0, |entry| entry.volume_seq);
	low.extend(high.into_iter().filter(|entry| entry.volume_seq > low_last));
	low
}

fn entries_bytes(entries: &[VolumeLogEntry]) -> u64 {
	entries
		.iter()
		.map(depot_types::log_shard::entry_bytes)
		.sum::<u64>()
}

/// Takes the entries a member's status notice carried, as a read of that member would
/// return them, when they continue the applied position and a quorum proves them committed;
/// the volume then needs no read. Entries the apply cannot take yet, because no quorum has
/// proven them, the volume's previous page is in flight, the volume is waiting out a retry
/// or a hold, or the budget has no room for their page, are kept for a later offer within
/// the cold byte budget. A kept set that another offer of the same lineage overlaps or
/// extends is merged with it, so offering a kept set again changes nothing. Carried entries
/// only ever save a read: whatever is neither taken nor kept is left to the ordinary read.
///
/// `replica` is the member's state in the notice: the entries were read under it, so it
/// alone decides what a quorum proves about them; see [`Carried`]. Returns whether an apply
/// started.
pub(crate) fn pushed(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	worker: &WorkerId,
	replica: &VolumeLogReplicaState,
	mut entries: Vec<VolumeLogEntry>,
	fx: &mut Effects,
) -> bool {
	if state.phase != MaintainerPhase::Serving || entries.is_empty() {
		return false;
	}
	let Some(volume) = state.filesystem.volumes.get(key) else {
		return false;
	};
	let previous = volume.applied.clone();
	let held = super::maintainer_log::replay::apply_limit(state, key)
		.is_some_and(|limit| previous.volume_seq >= limit);
	let blocked = volume.retry_wait
		|| held
		|| volume.unavailable_sources.contains(worker)
		|| retiring(state, key)
		|| apply_in_progress(state, key);

	// A kept set of the same lineage that these entries overlap or extend is merged into
	// them. Otherwise the set under the newer writer stays, and under one writer the set
	// that starts earlier, as the more likely to continue the applied position.
	let mut merged = false;
	if let Some(kept) = state.filesystem.carried.get(key) {
		let kept_first = kept.entries.first().map_or(0, |entry| entry.volume_seq);
		let kept_last = kept.entries.last().map_or(0, |entry| entry.volume_seq);
		let new_first = entries.first().map_or(0, |entry| entry.volume_seq);
		let new_last = entries.last().map_or(0, |entry| entry.volume_seq);
		let touches =
			new_first <= kept_last.saturating_add(1) && kept_first <= new_last.saturating_add(1);
		if kept.worker == *worker && same_lineage(&kept.replica, replica) && touches {
			entries = merge_runs(kept.entries.clone(), entries);
			merged = true;
		} else {
			let kept_rank = (kept.replica.appended.client_epoch, Reverse(kept_first));
			let new_rank = (replica.appended.client_epoch, Reverse(new_first));
			if new_rank <= kept_rank {
				return false;
			}
		}
	}
	let keep = |state: &mut MaintainerState, entries: Vec<VolumeLogEntry>| {
		let bytes = entries_bytes(&entries);
		keep_carried(
			state,
			key,
			Carried {
				worker: worker.clone(),
				replica: replica.clone(),
				entries,
				bytes,
				inserted: 0,
			},
		)
	};

	// The volume cannot take a page now: keep the entries for the offer that follows the
	// page in flight, the retry or the next observation.
	if blocked {
		if keep(state, entries) && apply_in_progress(state, key) {
			state.filesystem.carried_ready.insert(key.clone());
		}
		return false;
	}

	// Entries the maintainer already applied are dropped; the rest must continue it. A set
	// that cannot is useless to the apply, and so is one from a source the apply cannot read.
	entries.retain(|entry| entry.volume_seq > previous.volume_seq);
	let continues = entries
		.first()
		.is_some_and(|entry| Some(entry.volume_seq) == previous.volume_seq.checked_add(1));
	let readable = replica.status == VolumeLogReplicaStatus::InSync
		&& replica.cold.volume_seq <= previous.volume_seq;
	if !continues || !readable {
		if merged {
			drop_carried(state, key);
		}
		return false;
	}

	// Apply stops at the proven commit and at the position of the volume's next recovered
	// maintainer log op; what lies above waits for the next observation of the volume.
	let committed = committed_on(state, key, replica);
	let limit = super::maintainer_log::replay::apply_limit(state, key).unwrap_or(u64::MAX);
	let ready_through = committed.min(limit);
	let ready_count = entries.partition_point(|entry| entry.volume_seq <= ready_through);
	let rest = entries.split_off(ready_count);
	let ready = entries;
	if ready.is_empty() {
		keep(state, rest);
		return false;
	}
	let ready_bytes = entries_bytes(&ready);
	if ready_bytes > u64::from(state.config.tuning().max_cold_read_bytes) {
		drop_carried(state, key);
		return false;
	}
	if super::cold::bytes(state).saturating_add(ready_bytes.saturating_mul(2)) > MAX_COLD_BYTES {
		// No room for the page now: offer again on every event until the budget has some,
		// as the ordinary read does for its candidates.
		if keep(state, merge_runs(ready, rest)) {
			state.filesystem.carried_ready.insert(key.clone());
		}
		return false;
	}
	if rest.is_empty() {
		drop_carried(state, key);
	} else {
		keep(state, rest);
	}
	crate::metrics::record_notice_apply();
	accept_page(state, key.clone(), previous, ready, ready_bytes, fx)
}
fn retry(state: &mut MaintainerState, key: VolumeLogKey, fx: &mut Effects) {
	if let Some(v) = state.filesystem.volumes.get_mut(&key) {
		v.retry_wait = true;
		fx.arm_timer_after(
			MaintainerTimer::FilesystemRetry(key),
			state.config.tuning().retry_ms,
		);
	}
}
/// Excludes a read source until a fresh observation of it, and brings the volume's next
/// status poll forward so that observation does not wait for the lost-notice fallback.
pub(crate) fn source_failed(state: &mut MaintainerState, key: &VolumeLogKey, worker: WorkerId) {
	if let Some(volume) = state.filesystem.volumes.get_mut(key) {
		volume.unavailable_sources.insert(worker);
		super::poll::reset(state, key);
	}
}
pub(crate) fn source_observed(state: &mut MaintainerState, key: &VolumeLogKey, worker: &WorkerId) {
	if let Some(volume) = state.filesystem.volumes.get_mut(key) {
		volume.unavailable_sources.remove(worker);
	}
	mark_candidate(state, key);
}
pub(crate) fn retry_ready(state: &mut MaintainerState, key: &VolumeLogKey) {
	if let Some(v) = state.filesystem.volumes.get_mut(key) {
		v.retry_wait = false;
	}
	mark_candidate(state, key);
}
pub(crate) fn dispatch_apply(state: &mut MaintainerState, key: VolumeLogKey, fx: &mut Effects) {
	let Some(index) = state.filesystem.pages.iter().position(|p| p.key == key) else {
		return;
	};
	let page = state.filesystem.pages.remove(index).expect("queued page");
	let before = state.filesystem.volumes[&key].resolver.clone();
	let mut resolved = before.clone();
	let mut applied_bytes = 0;
	let mut first_pending = None::<u64>;
	let mut retention = None;
	let mut releases_holds = false;
	for entry in &page.entries {
		match resolved.push(entry) {
			Ok(depot_fs_core::txn::Resolution::Standalone(decoded)) => {
				for op in &decoded.ops {
					if let depot_fs_core::FsOp::FsRetentionSet(set) = op {
						retention = Some(set.policy.clone());
					}
					if matches!(
						op,
						depot_fs_core::FsOp::FsCheckpointRelease(_)
							| depot_fs_core::FsOp::FsForkHoldRelease(_)
					) {
						releases_holds = true;
					}
				}
				applied_bytes += depot_types::log_shard::entry_bytes(entry)
			}
			Ok(depot_fs_core::txn::Resolution::Commit { open, .. }) => {
				first_pending = Some(first_pending.map_or(open.first.volume_seq, |first| {
					first.min(open.first.volume_seq)
				}));
				applied_bytes += open.bytes + depot_types::log_shard::entry_bytes(entry)
			}
			Ok(_) => {}
			Err(_) => {
				fence(state, fx);
				return;
			}
		}
	}
	let source = state
		.loaded_volumes
		.get(&key)
		.and_then(|v| {
			v.members.read_source(
				state.config.local_worker.as_ref(),
				|w, r| {
					!state.filesystem.volumes[&key]
						.unavailable_sources
						.contains(w) && first_pending.is_none_or(|first| r.cold.volume_seq < first)
						// A delete's drain reads its tail from a tombstone.
						&& (r.status == VolumeLogReplicaStatus::InSync
							|| (r.status == VolumeLogReplicaStatus::Deleted
								&& super::drain::applying(state, &key)))
						&& committed_on(state, &key, r) >= page.through.volume_seq
				},
				|candidates| candidates.next(),
			)
		})
		.map(|(worker, proof)| (worker.clone(), proof.clone()));

	if first_pending.is_some() && source.is_none() {
		// No writes have been dispatched. Re-fetch this bounded page after fresh
		// replica observations; resolver and applied/cold positions stay unchanged.
		retry(state, key, fx);
		return;
	}

	// A logged policy prunes the retention rows of the loaded record when no pass has
	// stored it yet; every other page leaves the record alone.
	let loaded_record = if retention.is_some() {
		Some(state.filesystem.volumes[&key].record.to_record())
	} else {
		None
	};
	let id = state.begin(
		Operation::FilesystemApply {
			resolved,
			applied_bytes,
			releases_holds,
			epochs: epoch_boundaries(&page.entries),
			receipts: page
				.entries
				.iter()
				.filter_map(|entry| match &entry.body {
					VolumeLogEntryBody::PayloadWrite(write) => {
						Some(rivet_data::generated::shared_depot_fs_v1::FsWriteReceipt {
							client_seq: write.client_seq,
							request_id: write.request_id,
							payload_digest: write.payload_digest,
							volume_seq: entry.volume_seq,
						})
					}
					_ => None,
				})
				.collect(),
			retention,
			key: key.clone(),
			through: page.through,
			bytes: page.bytes,
			first_write_unix_ms: page.entries[0].written_at_unix_ms,
		},
		fx,
	);
	// A recovered delete applies the tail its tombstones kept under the incarnation's own
	// keys, whatever its record says by then.
	let authority = if super::drain::applying_from_tombstones(state, &key) {
		crate::tablet_maintainer_machine::types::ApplyAuthority::RetiredTail
	} else {
		crate::tablet_maintainer_machine::types::ApplyAuthority::Current
	};
	let drain_through = super::drain::through(state, &key).cloned();
	fx.execute(MaintainerEffect::FilesystemApplyRequest {
		authority,
		drain_through,
		loaded_record,
		read_timeout_ms: state.config.tuning().operation_timeout_ms,
		read_deadline_boot_ms: fx.now_boot_ms().saturating_add(
			state.config.tuning().cold_io_timeout_ms.saturating_sub(
				state
					.config
					.tuning()
					.operation_timeout_ms
					.min(state.config.tuning().cold_io_timeout_ms / 2),
			),
		),
		max_read_bytes: state.config.tuning().max_cold_read_bytes,
		op_id: id,
		key,
		previous: page.previous,
		resolver: before,
		source,
		entries: page.entries,
	});
}
pub(crate) fn applied(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<VolumeLogPosition, FilesystemApplyError>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(Operation::FilesystemApply { .. })
	) {
		return;
	}
	let Some(Operation::FilesystemApply {
		resolved,
		applied_bytes,
		receipts,
		epochs,
		retention,
		releases_holds: _,
		key,
		through,
		bytes: _,
		first_write_unix_ms,
	}) = state.finish(id, fx)
	else {
		unreachable!()
	};
	if let Err(FilesystemApplyError::ReplicaRead { worker, .. }) = result {
		// Every staged row still uses its immutable commit sequence. Replaying
		// from the unchanged resolver is idempotent; none of this suffix is cold.
		source_failed(state, &key, worker);
		retry(state, key, fx);
		return;
	}
	if result.as_ref() != Ok(&through) {
		match &result {
			Err(error) => tracing::warn!(%error, "filesystem apply failed in the store; fencing"),
			Ok(position) => tracing::warn!(
				?position,
				?through,
				"filesystem apply ended at an unexpected position; fencing"
			),
		}
		fence(state, fx);
		return;
	}
	let Some(volume) = state.filesystem.volumes.get_mut(&key) else {
		fence(state, fx);
		return;
	};
	merge_receipts(&mut volume.record, receipts);
	merge_epochs(volume, epochs);
	if let Some(policy) = retention {
		// The next pass writes the policy with the record and applies it to the retention
		// rows in the same batch, as a direct policy change does. That batch can release
		// the retention checkpoints the maintainer knew, so the next check learns again.
		volume.record.make_mut().retention = policy;
		volume.retention_checkpoint_seq = None;
	}
	let applied_seq = through.volume_seq;
	let entries = applied_seq.saturating_sub(volume.applied.volume_seq);
	if entries > 0 {
		volume.entry_bytes = u32::try_from(applied_bytes / entries).unwrap_or(u32::MAX);
	}
	if resolved.open().is_empty() {
		volume.settled = through.clone();
	}
	volume.applied = through;
	volume.resolver = resolved;
	if state
		.filesystem
		.committed_floors
		.get(&key)
		.is_some_and(|floor| floor.volume_seq <= applied_seq)
	{
		state.filesystem.committed_floors.remove(&key);
	}
	if !far_behind(state, &key) {
		state.filesystem.waiting_opens.remove(&key);
	}
	wake_lagging_opens(state, &key, applied_seq);
	mark_candidate(state, &key);
	if entries > 0 {
		requeue_retention(state, &key);
	}
	let unflushed_bytes = state.filesystem.volumes[&key]
		.unflushed_bytes
		.saturating_add(applied_bytes);
	state.filesystem.set_unflushed_bytes(&key, unflushed_bytes);
	state.filesystem.memtable_bytes = state
		.filesystem
		.memtable_bytes
		.saturating_add(applied_bytes);
	// New data may give a requested volume something to flush, so it is offered to
	// selection again even while it retires, when `request_flush` below declines.
	if state.filesystem.volumes[&key].flush_requested {
		state.filesystem.request_flush(&key);
	}
	// A flush already scheduled carries a requested volume, so its new data does not
	// move that flush earlier.
	let flush_scheduled = state.filesystem.flush_scheduled.is_some();
	let pressure = tablet_pressure(state);
	let volume = state
		.filesystem
		.volumes
		.get_mut(&key)
		.expect("applied filesystem volume");
	if volume.due_boot_ms.is_none() {
		let age = fx
			.clocks()
			.unix_ms
			.saturating_sub(first_write_unix_ms)
			.max(0) as u64;
		let jitter = u64::from_be_bytes(
			key.0.volume_incarnation[..8]
				.try_into()
				.expect("UUID prefix"),
		) % (state.config.write_to_cold_after_ms / 10 + 1);
		let delay = state
			.config
			.write_to_cold_after_ms
			.saturating_add(jitter)
			.saturating_sub(age);
		volume.due_boot_ms = Some(fx.now_boot_ms().saturating_add(delay));
		fx.arm_timer_after(MaintainerTimer::FilesystemDue(key.clone()), delay);
	}
	if (volume.flush_requested && !flush_scheduled)
		|| volume.unflushed_bytes >= state.config.write_to_cold_above_bytes
		|| pressure
	{
		request_flush(state, &key, fx);
	} else if volume
		.due_boot_ms
		.is_some_and(|due| due <= fx.clocks().boot_ms)
	{
		// A volume past its batching age owes a pass nobody waits on, as when its due timer
		// fires. With steady writers some due volume writes again every few seconds, so a
		// prompt pass here would flush the tablet that often instead of once per window.
		request_due_flush(state, &key, fx);
	}
}

/// Whether the bytes applied since the last pass reached `tablet_flush_above_bytes`. A pass
/// then runs at once and carries the volumes with the most unflushed bytes, so it uploads
/// the memtable and records their cold positions before SlateDB uploads it on its own,
/// which would record none.
fn tablet_pressure(state: &MaintainerState) -> bool {
	state.filesystem.memtable_bytes >= state.config.tablet_flush_above_bytes
}

/// The entries of `entries` whose client epoch differs from the entry before them, starting
/// with the first.
fn epoch_boundaries(entries: &[VolumeLogEntry]) -> Vec<VolumeLogPosition> {
	let mut boundaries: Vec<VolumeLogPosition> = Vec::new();
	for entry in entries {
		let changed = boundaries
			.last()
			.is_none_or(|last| last.client_epoch != entry.client_epoch);
		if changed {
			boundaries.push(VolumeLogPosition {
				volume_seq: entry.volume_seq,
				client_epoch: entry.client_epoch,
			});
		}
	}
	boundaries
}

/// Adds the boundaries of an applied page to the volume's epochs, then drops every boundary
/// below the last one at or below the record's cold, which no entry above cold needs.
fn merge_epochs(volume: &mut Volume, page: Vec<VolumeLogPosition>) {
	let mut epochs = std::mem::take(&mut volume.epochs).into_vec();
	for boundary in page {
		let current = epochs
			.iter()
			.rev()
			.find(|(seq, _)| *seq < boundary.volume_seq)
			.map(|(_, epoch)| *epoch);
		if current != Some(boundary.client_epoch) {
			match epochs.binary_search_by_key(&boundary.volume_seq, |(seq, _)| *seq) {
				Ok(index) => epochs[index].1 = boundary.client_epoch,
				Err(index) => epochs.insert(index, (boundary.volume_seq, boundary.client_epoch)),
			}
		}
	}
	let cold_seq = volume.record.cold.volume_seq;
	if let Some(floor) = epochs.iter().rposition(|(seq, _)| *seq <= cold_seq) {
		epochs.drain(..floor);
	}
	volume.epochs = epochs.into_boxed_slice();
}

/// What an open of `key` answers about the entries the maintainer has applied above `cold`,
/// the cold position of the record it returns: the applied position, the epoch of each
/// entry through it and the newest receipts at or below it. `None` when nothing above `cold`
/// is applied, or when the volume's epochs do not cover every entry above `cold`.
pub(crate) fn applied_history(
	state: &MaintainerState,
	key: &VolumeLogKey,
	cold: &VolumeLogPosition,
) -> Option<rivet_data::generated::depot_client_protocol_v1::VolumeAppliedHistory> {
	let volume = state.filesystem.volumes.get(key)?;
	let first = cold.volume_seq.checked_add(1)?;
	if volume.applied.volume_seq < first || cold.volume_seq < volume.record.cold.volume_seq {
		return None;
	}
	let (_, first_epoch) = volume.epochs.iter().rev().find(|(seq, _)| *seq <= first)?;
	let mut epochs = vec![VolumeLogPosition {
		volume_seq: first,
		client_epoch: *first_epoch,
	}];
	for (seq, epoch) in volume
		.epochs
		.iter()
		.filter(|(seq, _)| *seq > first && *seq <= volume.applied.volume_seq)
	{
		epochs.push(VolumeLogPosition {
			volume_seq: *seq,
			client_epoch: *epoch,
		});
	}
	if epochs.last().map(|last| last.client_epoch) != Some(volume.applied.client_epoch) {
		return None;
	}
	Some(
		rivet_data::generated::depot_client_protocol_v1::VolumeAppliedHistory {
			applied: volume.applied.clone(),
			epochs,
			recent_writes: volume
				.record
				.recent_writes
				.iter()
				.filter(|receipt| receipt.volume_seq <= volume.applied.volume_seq)
				.cloned()
				.collect(),
		},
	)
}

/// Adds `receipts` to the record, replacing any with the same request, and keeps the newest
/// 64 in client order. A record that would not change is left as it is, so a record shared
/// with the stored row stays shared.
fn merge_receipts(
	record: &mut super::record::SharedRecord,
	receipts: Vec<rivet_data::generated::shared_depot_fs_v1::FsWriteReceipt>,
) {
	let settled = receipts.is_empty()
		&& record.recent_writes.len() <= 64
		&& record.recent_writes.is_sorted_by_key(|r| r.client_seq);
	if settled {
		return;
	}
	let record = record.make_mut();
	for receipt in receipts {
		record
			.recent_writes
			.retain(|r| r.request_id != receipt.request_id);
		record.recent_writes.push(receipt);
	}
	record.recent_writes.sort_by_key(|r| r.client_seq);
	let excess = record.recent_writes.len().saturating_sub(64);
	record.recent_writes.drain(..excess);
}

pub(crate) fn request_flush(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	if retiring(state, key) {
		return;
	}
	if !state.filesystem.volumes.contains_key(key) {
		return;
	}
	state.filesystem.request_flush(key);
	state.filesystem.prompt_flush_wanted = true;
	schedule_flush_within(state, COLD_BATCH_DELAY_MS, fx);
}

/// Requests the pass a volume owes because its oldest applied entry reached the batching
/// age. Nobody waits on it, so it uses the throttle window: with a short age every
/// steady writer comes due on its own schedule, and a prompt pass per due volume would put
/// one L0 SST per volume into the cold store.
pub(crate) fn request_due_flush(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	if retiring(state, key) {
		return;
	}
	if !state.filesystem.volumes.contains_key(key) {
		return;
	}
	state.filesystem.request_throttled_flush(key);
	schedule_flush_within(state, state.config.memtable_flush_throttle_ms, fx);
}

/// Asks for a pass the maintainer log owes, promptly when the log has reached the cold-write
/// threshold, else within the throttle window like a volume that reached its batching age.
pub(crate) fn request_log_pass(state: &mut MaintainerState, prompt: bool, fx: &mut Effects) {
	if prompt {
		state.filesystem.prompt_flush_wanted = true;
		schedule_flush_within(state, COLD_BATCH_DELAY_MS, fx);
	} else {
		schedule_flush_within(state, state.config.memtable_flush_throttle_ms, fx);
	}
}

/// Arms `FilesystemFlushBatch` to fire within `delay_ms`, moving an armed one earlier
/// but never later.
fn schedule_flush_within(state: &mut MaintainerState, delay_ms: u64, fx: &mut Effects) {
	let due_boot_ms = fx.now_boot_ms().saturating_add(delay_ms);
	if state
		.filesystem
		.flush_scheduled
		.is_none_or(|scheduled| scheduled > due_boot_ms)
	{
		state.filesystem.flush_scheduled = Some(due_boot_ms);
		fx.arm_timer_after(MaintainerTimer::FilesystemFlushBatch, delay_ms);
	}
}
/// Queues the flush `FilesystemFlushBatch` scheduled.
pub(crate) fn flush_batch_due(state: &mut MaintainerState) {
	state.filesystem.flush_scheduled = None;
	queue_flush(state);
}

pub(crate) fn queue_flush(state: &mut MaintainerState) {
	if !state.store_queue.contains(&StoreWork::FilesystemFlush) {
		state.store_queue.push_back(StoreWork::FilesystemFlush);
	}
}

/// Whether a flush dispatched now would select nothing and only requeue itself: in per-volume
/// admission every requested owed volume is touched by work queued ahead of it (`ahead`), no
/// other requested volume could be flushed, and the maintainer log owes no pass. Mirrors the
/// selection in [`dispatch_flush`] without changing state. [`flush_may_wait`] holds the cheap
/// preconditions.
pub(crate) fn flush_waits_on_ahead(
	state: &MaintainerState,
	ahead: &BTreeSet<super::footprint::StoreVolume>,
) -> bool {
	use super::footprint::{StoreVolume, VolumeAccess};
	let busy = |key: &VolumeLogKey| {
		let volume = StoreVolume::from(&key.0.volume_key);
		ahead.contains(&volume)
			|| super::footprint::volume_conflict(state, &volume, VolumeAccess::Write)
	};
	let owed_waits_on_ahead = state.filesystem.owed.iter().all(|key| {
		!flush_requested(state, key) || ahead.contains(&StoreVolume::from(&key.0.volume_key))
	});
	let nothing_else = state
		.filesystem
		.owed
		.iter()
		.chain(state.filesystem.flush_candidates.iter())
		.all(|key| !flush_eligible(state, key) || busy(key));
	owed_waits_on_ahead && nothing_else
}

/// The cheap preconditions of [`flush_waits_on_ahead`]: an owed volume that is still requested, and no pass the maintainer log owes. A flush whose owed volumes
/// are no longer requested is never held: dispatching it clears the debt.
pub(crate) fn flush_may_wait(state: &MaintainerState) -> bool {
	state
		.filesystem
		.owed
		.iter()
		.any(|key| flush_requested(state, key))
		&& !super::maintainer_log::wants_pass(state)
}

/// Whether `key` asks for a flush, as [`dispatch_flush`] judges it.
fn flush_requested(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	state
		.filesystem
		.volumes
		.get(key)
		.is_some_and(|v| !state.deleting.contains_key(key) && v.flush_requested)
}

/// Whether `key` has something a flush would carry, as [`dispatch_flush`] judges it.
fn flush_eligible(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	flush_requested(state, key)
		&& state
			.filesystem
			.volumes
			.get(key)
			.is_some_and(|v| v.applied.volume_seq > v.record.cold.volume_seq || !v.record_persisted)
}

/// Selects the records of one flush. A requested volume with
/// conflicting work in flight, or with work queued ahead of the flush (`ahead`), is
/// skipped and owed instead; owed volumes are selected first next time.
pub(crate) fn dispatch_flush(
	state: &mut MaintainerState,
	ahead: &BTreeSet<super::footprint::StoreVolume>,
	fx: &mut Effects,
) {
	use super::footprint::{StoreVolume, VolumeAccess};
	state.filesystem.prompt_flush_wanted = false;
	let max = state.config.tuning().max_flush_records as usize;
	let requested = |state: &MaintainerState, key: &VolumeLogKey| {
		state
			.filesystem
			.volumes
			.get(key)
			.is_some_and(|v| !state.deleting.contains_key(key) && v.flush_requested)
	};
	let eligible = |state: &MaintainerState, key: &VolumeLogKey| {
		requested(state, key)
			&& state.filesystem.volumes.get(key).is_some_and(|v| {
				v.applied.volume_seq > v.record.cold.volume_seq || !v.record_persisted
			})
	};
	let busy = |state: &MaintainerState, key: &VolumeLogKey| {
		let volume = StoreVolume::from(&key.0.volume_key);
		ahead.contains(&volume)
			|| super::footprint::volume_conflict(state, &volume, VolumeAccess::Write)
	};

	// Owed volumes first, then every other requested volume in key order. Volumes
	// without a flush request are never selected, so only the flush candidates are
	// visited. Candidates are walked lazily so a selection stops after `max` rows.
	let owed = std::mem::take(&mut state.filesystem.owed);
	let mut keys = Vec::new();
	let mut deferred = BTreeSet::new();
	let mut visited = BTreeSet::new();
	let mut idle = Vec::new();
	let mut owed_keys = owed.iter();
	let mut after: Option<VolumeLogKey> = None;
	while keys.len() < max {
		let key = match owed_keys.next() {
			Some(key) => key.clone(),
			None => {
				let lower = match &after {
					Some(key) => Bound::Excluded(key),
					None => Bound::Unbounded,
				};
				let Some(key) = state
					.filesystem
					.flush_candidates
					.range::<VolumeLogKey, _>((lower, Bound::Unbounded))
					.find(|key| !owed.contains(*key))
					.cloned()
				else {
					break;
				};
				after = Some(key.clone());
				key
			}
		};
		count_flush_visit();
		visited.insert(key.clone());
		// A requested volume with work in flight or queued ahead is owed even before
		// that work makes it eligible: the flush must carry what that work applies.
		if requested(state, &key) && busy(state, &key) {
			deferred.insert(key);
			continue;
		}
		if !eligible(state, &key) {
			// Only an apply, a retention check or a new request can give a requested
			// volume something to flush, and each offers it again. A retiring volume
			// stays, because the end of its delete or restore offers nothing. So does a
			// deleting volume whose drain still applies its tail: the apply's end makes
			// it retiring without offering it again.
			if !retiring(state, &key) && !state.deleting.contains_key(&key) {
				idle.push(key);
			}
			continue;
		}
		keys.push(key);
	}
	for key in idle {
		state.filesystem.flush_candidates.remove(&key);
	}
	// Owed volumes past the row limit stay owed.
	for key in owed {
		if !visited.contains(&key) && requested(state, &key) {
			deferred.insert(key);
		}
	}

	// The flush makes the whole memtable durable, including every volume's applied rows,
	// so other volumes with enough unflushed data publish their cold positions in it too.
	// Without this, cold pressure flushes one volume per SST.
	let piggybacked = if keys.is_empty() {
		Vec::new()
	} else {
		piggyback_candidates(state, &keys, &deferred, max - keys.len(), &busy)
	};
	crate::metrics::TABLET_FLUSH_PIGGYBACKED.inc_by(piggybacked.len() as u64);
	keys.extend(piggybacked);
	let deferred_volumes = deferred.len() as u32;
	// A volume this flush had to defer is owed the next one at once.
	if !deferred.is_empty() {
		state.filesystem.prompt_flush_wanted = true;
	}
	state.filesystem.owed = deferred;
	if !state.filesystem.owed.is_empty() {
		// The queued flush is the ordering footprint of the owed volumes.
		queue_flush(state);
	}
	let selected = keys
		.into_iter()
		.map(|key| {
			let v = &state.filesystem.volumes[&key];
			let mut row = v.record.to_record();
			row.cold = v.resolver.cold().clone();
			row.recent_writes
				.retain(|receipt| receipt.volume_seq <= row.cold.volume_seq);
			(key, row)
		})
		.collect::<Vec<_>>();
	// The maintainer log may owe a pass of its own: to cover its applied entries and write
	// the cold position it trims to. Such a pass carries no volume records.
	if selected.is_empty() && !super::maintainer_log::wants_pass(state) {
		return;
	}

	// The flush now carries everything applied so far. Applies that land while it
	// uploads start fresh accounting, so they request their own later flush.
	let mut records = Vec::with_capacity(selected.len());
	for (key, record) in selected {
		if let Some(volume) = state.filesystem.volumes.get_mut(&key) {
			volume.due_boot_ms = None;
		}
		state.filesystem.set_unflushed_bytes(&key, 0);
		state.filesystem.clear_flush_request(&key);
		fx.cancel_timer(MaintainerTimer::FilesystemDue(key));
		records.push(record);
	}
	crate::metrics::TABLET_FLUSH_RECORDS.observe(records.len() as f64);
	state.filesystem.memtable_bytes = 0;
	let (dispatch, maintainer_log) = super::maintainer_log::pass_dispatched(state);
	let id = state.begin(
		Operation::FilesystemFlush {
			records: records.clone(),
			stage: FlushStage::Staging,
			dispatch,
		},
		fx,
	);
	fx.execute(MaintainerEffect::FilesystemFlushRequest {
		op_id: id,
		records,
		deferred_volumes,
		maintainer_log,
	});
}
/// Volumes that did not request this flush but have at least `flush_piggyback_min_bytes`
/// of applied data above their published cold position, largest first, at most `room`.
/// Volumes already selected or owed, being deleted or quiesced, and volumes with
/// conflicting store work (`busy`) are skipped and keep their own schedule.
fn piggyback_candidates(
	state: &MaintainerState,
	selected: &[VolumeLogKey],
	deferred: &BTreeSet<VolumeLogKey>,
	room: usize,
	busy: &impl Fn(&MaintainerState, &VolumeLogKey) -> bool,
) -> Vec<VolumeLogKey> {
	if room == 0 {
		return Vec::new();
	}
	let selected = selected.iter().collect::<BTreeSet<_>>();
	// Under tablet pressure every volume with unflushed bytes may join, largest first, so
	// the pass records cold for as much of the memtable as its record limit allows.
	let min_bytes = if tablet_pressure(state) {
		1
	} else {
		state.config.flush_piggyback_min_bytes
	};
	// The index is ordered largest first, so the walk ends at the first volume below the
	// threshold and visits only volumes holding at least that much unflushed data.
	let mut piggybacked = Vec::new();
	for (Reverse(bytes), key) in &state.filesystem.unflushed {
		if piggybacked.len() >= room || *bytes < min_bytes {
			break;
		}
		count_flush_visit();
		let Some(volume) = state.filesystem.volumes.get(key) else {
			continue;
		};
		if volume.applied.volume_seq > volume.record.cold.volume_seq
			&& !selected.contains(key)
			&& !deferred.contains(key)
			&& !state.deleting.contains_key(key)
			&& !busy(state, key)
		{
			piggybacked.push(key.clone());
		}
	}
	piggybacked
}

/// The executor wrote the flush records after every earlier mutation. Release the
/// store to later work while the upload continues; cold positions stay unpublished.
pub(crate) fn staged(state: &mut MaintainerState, id: OpId, fx: &mut Effects) {
	match state.operations.get(&id) {
		Some(Operation::FilesystemFlush {
			stage: FlushStage::Staging,
			..
		}) => {
			state.operations.update(&id, |operation| {
				if let Operation::FilesystemFlush { stage, .. } = operation {
					*stage = FlushStage::Uploading;
				}
			});
		}
		Some(Operation::FilesystemFlush { .. }) => fence(state, fx),
		_ => {}
	}
}

pub(crate) fn flushed(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<(), String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(Operation::FilesystemFlush { .. })
	) {
		return;
	}
	let Some(Operation::FilesystemFlush {
		records,
		stage,
		dispatch,
	}) = state.finish(id, fx)
	else {
		unreachable!()
	};
	if stage != FlushStage::Uploading && result.is_ok() {
		// Durability without a staged report means the executor skipped a step.
		super::maintainer_log::pass_failed(state, dispatch);
		fence(state, fx);
		return;
	}
	if let Err(error) = &result {
		tracing::warn!(%error, "filesystem flush failed in the store; fencing");
		super::maintainer_log::pass_failed(state, dispatch);
		fence(state, fx);
		return;
	}
	super::maintainer_log::pass_completed(state, dispatch, fx);
	super::materialize::passes_completed(state);
	for record in records {
		if record.status == FsVolumeStatus::Deleted {
			super::alarms::published(state, &record, fx);
		}
		let key = VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
			volume_key: depot_types::VolumeKey {
				partition_key: record.partition_key.clone(),
				name: record.name.clone(),
			},
			volume_incarnation: record.volume_incarnation,
		});
		let Some(volume) = state.filesystem.volumes.get_mut(&key) else {
			fence(state, fx);
			return;
		};
		// Applies may have advanced the resolver while this flush uploaded, but its
		// cold floor never moves backwards past what was staged.
		let resolver_cold = volume.resolver.cold();
		if resolver_cold.volume_seq < record.cold.volume_seq
			|| resolver_cold.client_epoch < record.cold.client_epoch
		{
			fence(state, fx);
			return;
		}
		// A logged policy applied after staging is newer than the staged record's, because
		// applies only move forward. It must survive, or the next pass writes the old policy
		// past the entry that changed it and the change is lost.
		let applied_retention = volume.record.retention.clone();
		// Keep receipts for writes applied after staging; the next flush persists them.
		let unflushed_receipts = volume
			.record
			.recent_writes
			.iter()
			.filter(|receipt| receipt.volume_seq > record.cold.volume_seq)
			.cloned()
			.collect::<Vec<_>>();
		// The stored row and the filesystem record share this flush's record until an apply
		// adds receipts to the filesystem's.
		let flushed = super::record::SharedRecord::new(record.clone());
		volume.record = flushed.clone();
		merge_receipts(&mut volume.record, unflushed_receipts);
		let retention_changed = applied_retention != record.retention;
		if retention_changed {
			volume.record.make_mut().retention = applied_retention;
			volume.retention_checkpoint_seq = None;
		}
		volume.record_persisted = !retention_changed;
		if let Some(loaded) = state.loaded_volumes.get_mut(&key) {
			super::release_volume_initial_entries(loaded, &mut state.initial_bytes_held);
			loaded.creation = None;
			loaded.stored_row = Some(flushed);
			state
				.rows_written
				.insert(key.clone(), (record.cold.clone(), record.status));
		}

		volume.retention_checked_boot_ms = fx.now_boot_ms();
		schedule_retention(state, &key);
		if retention_changed {
			unrecorded(state, key.clone(), fx);
		} else {
			state.pending_rows.remove(&key);
		}
		super::deletion::row_written(state, &key, &record);
		if retiring(state, &key) {
			continue;
		}
		state.pending_trims.insert(key.clone(), record.cold);
		for worker in state.members() {
			super::cold::trim(state, key.clone(), worker, fx);
		}
	}
	let mut more = false;
	for key in &state.filesystem.flush_candidates {
		count_flush_visit();
		let Some(v) = state.filesystem.volumes.get(key) else {
			continue;
		};
		let leftover = v.flush_requested
			&& (v.applied.volume_seq > v.record.cold.volume_seq || !v.record_persisted);
		if leftover {
			more = true;
		}
	}
	if more {
		// A leftover from a prompt request goes out at once; everything else waits for the
		// throttle window.
		if state.filesystem.prompt_flush_wanted {
			queue_flush(state);
		} else {
			schedule_flush_within(state, state.config.memtable_flush_throttle_ms, fx);
		}
	}
	super::operator_repair::calls::check_persisted(state, fx);
}

/// Whether a delete or restore has stopped apply work for `key`. A drain still applying the
/// incarnation's final tail keeps it running until the store holds that tail.
pub(crate) fn retiring(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	(state.deleting.contains_key(key) || super::forks::quiescing(&state.forks, &key.0.volume_key))
		&& !super::drain::applying(state, key)
}

/// The highest volume seq of `replica`'s log that is proven committed, judged by the
/// replica's state alone and by what the volume's members currently prove: its cold
/// position; a drain's claimed final tail, when the replica holds that tail at its epoch;
/// the position the volume's next op to replay follows, when the replica holds it; the floor
/// of a claim read waiting for this volume's apply, when the replica holds it; and the
/// position `claim_quorum_commit` proves on a quorum, bounded by the replica's own tail,
/// when the replica holds the proving writer's tail under that writer's claim. The replica's
/// `known_committed_volume_seq` is a hint the writer carried and is never read here.
///
/// `replica` may be an older observation or a notice's snapshot: the entries below its tail
/// are the claiming writer's history, so a quorum proving that writer's position commits them
/// whatever the member holds now.
pub(crate) fn committed_on(
	state: &MaintainerState,
	key: &VolumeLogKey,
	replica: &VolumeLogReplicaState,
) -> u64 {
	let mut committed = replica.cold.volume_seq;
	if let Some(through) = super::drain::through(state, key)
		&& super::drain::holds_through(&replica.appended, through)
	{
		committed = committed.max(through.volume_seq);
	}
	if let Some((proven, client)) = claim_quorum_commit(state, key)
		&& tail_written_by_claim_holder(replica, &client)
	{
		committed = committed.max(proven.volume_seq.min(replica.appended.volume_seq));
	}
	if let Some(through) = super::maintainer_log::replay::committed_through(state, key)
		&& reaches_committed(replica, through)
	{
		committed = committed.max(through.volume_seq);
	}
	if let Some(floor) = super::filesystem_reads::waiting_claim_floor(state, key)
		&& reaches_committed(replica, floor)
	{
		committed = committed.max(floor.volume_seq);
	}
	if let Some(committed_floor) = state.filesystem.committed_floors.get(key)
		&& reaches_committed(replica, committed_floor)
	{
		committed = committed.max(committed_floor.volume_seq);
	}
	committed
}

/// Whether `replica` holds the committed entry at `through`. An in-sync replica's log is a
/// prefix of the history of the writer that appended its tail. That writer is the one that
/// appended `through` when the epochs match, and otherwise a later writer, whose claim kept
/// every committed entry; either way the replica holds `through` when its tail reaches it.
fn reaches_committed(replica: &VolumeLogReplicaState, through: &VolumeLogPosition) -> bool {
	replica.status == VolumeLogReplicaStatus::InSync
		&& replica.appended.client_epoch >= through.client_epoch
		&& replica.appended.volume_seq >= through.volume_seq
}

/// Whether `replica` is in sync, claimed by `client`, and its tail was appended by that
/// client, so its whole log is a prefix of that client's history.
fn tail_written_by_claim_holder(replica: &VolumeLogReplicaState, client: &ClientIdentity) -> bool {
	replica.status == VolumeLogReplicaStatus::InSync
		&& replica.appended.client_epoch == client.client_epoch
		&& replica.claimed_client.as_ref() == Some(client)
}

/// The highest position a quorum of voters holds under the claim of the writer that appended
/// it, as last observed, with that writer's identity. Only the writer that claimed a replica at
/// epoch E appends entries tagged E to it, so an E entry on such a quorum is committed exactly
/// as when that writer acknowledged it, and everything beneath it is committed with it. A
/// writer never announces its commits: the appended positions its appends leave on a quorum
/// prove them, which the shards' notices report. `committed_on` adds the other evidence the
/// maintainer counts, such as cold, a drain's final tail and a waiting claim read's floor. In `Joint` a position counts only when both majorities hold it. A replica holding
/// an older epoch's tail under a newer claim does not count: the newer writer may still
/// replace that tail with its own history.
pub(crate) fn claim_quorum_commit(
	state: &MaintainerState,
	key: &VolumeLogKey,
) -> Option<(VolumeLogPosition, ClientIdentity)> {
	let volume = state.loaded_volumes.get(key)?;
	let set = &state.tablet.replica_set;
	let mut best: Option<(VolumeLogPosition, ClientIdentity)> = None;
	for candidate in volume.members.values() {
		let Some(client) = candidate.claimed_client.as_ref() else {
			continue;
		};
		let position = &candidate.appended;
		if !tail_written_by_claim_holder(candidate, client)
			|| best.as_ref().is_some_and(|(best, _)| {
				(best.client_epoch, best.volume_seq) >= (position.client_epoch, position.volume_seq)
			}) {
			continue;
		}
		let proven = super::membership::quorum(set, |worker| {
			volume.members.get(worker).is_some_and(|replica| {
				tail_written_by_claim_holder(replica, client)
					&& replica.appended.volume_seq >= position.volume_seq
			})
		});
		if proven {
			best = Some((position.clone(), client.clone()));
		}
	}
	best
}

/// The highest volume seq the loaded volume's members prove committed, and at least what the
/// maintainer applied, since apply takes only proven entries. Zero when nothing is loaded. A
/// waiting claim read's floor can raise it only while that read waits; once the read is
/// answered or expires, the floor no longer counts unless apply has reached it.
pub(crate) fn known_committed(state: &MaintainerState, key: &VolumeLogKey) -> u64 {
	let members = state.loaded_volumes.get(key).map_or(0, |volume| {
		volume
			.members
			.values()
			.map(|replica| committed_on(state, key, replica))
			.max()
			.unwrap_or(0)
	});
	let applied = state
		.filesystem
		.volumes
		.get(key)
		.map_or(0, |volume| volume.applied.volume_seq);
	members.max(applied)
}

/// The committed seq a non-holder's `Latest` fork or checkpoint waits for apply to reach: the
/// highest commit the members prove, and at least what is applied. A tail an abandoned
/// claim covered before its own entry reached a quorum is not proven here, so such a fork
/// can still miss that tail until the volume is reopened or its idle unload recovers it.
pub(crate) fn latest_committed(state: &MaintainerState, key: &VolumeLogKey, applied: u64) -> u64 {
	known_committed(state, key).max(applied)
}

/// Deletion cancels unsent data work; the single active store operation finishes in order.
pub(crate) fn quiesce(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	drop_carried(state, key);
	state.filesystem.clear_flush_request(key);
	release_owed(state, key);
	state.filesystem.pages.retain(|p| &p.key != key);
	state.store_queue.remove_all(&[
		StoreWork::FilesystemLoad {
			key: key.clone(),
			purpose: LoadPurpose::StopProof,
		},
		StoreWork::FilesystemLoad {
			key: key.clone(),
			purpose: LoadPurpose::Register,
		},
		StoreWork::FilesystemApply { key: key.clone() },
	]);
	fx.cancel_timer(MaintainerTimer::FilesystemDue(key.clone()));
	fx.cancel_timer(MaintainerTimer::FilesystemRetry(key.clone()));
}

/// Drops a volume's flush debt. The queued flush's footprint changes with it, so store
/// admission must run again.
fn release_owed(state: &mut MaintainerState, key: &VolumeLogKey) {
	if state.filesystem.owed.remove(key) {
		state.store_queue.touch();
	}
}

/// In-flight filesystem reads of one volume.
fn filesystem_reads_of(state: &MaintainerState, key: &VolumeLogKey) -> Vec<OpId> {
	state
		.operations
		.index()
		.filesystem_reads
		.get(key)
		.map_or_else(Vec::new, |ids| ids.iter().copied().collect())
}

/// Lifecycle cleanup only releases a completed opening; queued tail reads cannot revive it.
pub(crate) fn forget(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	let reads = filesystem_reads_of(state, key);
	for id in reads {
		state.finish(id, fx);
		fx.execute(MaintainerEffect::CancelReplicaRequest { op_id: id });
	}
	release_owed(state, key);
	drop_carried(state, key);
	state.filesystem.clear_flush_request(key);
	state.filesystem.set_unflushed_bytes(key, 0);
	state.filesystem.volumes.remove(key);
	state.filesystem.committed_floors.remove(key);
	state.filesystem.waiting_opens.remove(key);
	if let Some(id) = state.volume_keys.find(key) {
		state.filesystem.retention.cancel(&id);
	}
	state.filesystem.candidates.remove(key);
	state.filesystem.pages.retain(|p| &p.key != key);
	state.store_queue.remove_all(&[
		StoreWork::FilesystemLoad {
			key: key.clone(),
			purpose: LoadPurpose::StopProof,
		},
		StoreWork::FilesystemLoad {
			key: key.clone(),
			purpose: LoadPurpose::Register,
		},
		StoreWork::FilesystemApply { key: key.clone() },
	]);
	fx.cancel_timer(MaintainerTimer::FilesystemDue(key.clone()));
	fx.cancel_timer(MaintainerTimer::FilesystemRetry(key.clone()));
}



/// Shared maintainer validation also checks the nested apply/flush invariants.
pub(crate) fn validate(state: &MaintainerState) -> anyhow::Result<()> {
	state.filesystem.retention.debug_validate()?;
	for key in &state.filesystem.record_missing {
		anyhow::ensure!(
			state.loaded_volumes.contains_key(key),
			"a record-missing mark outlived its loaded volume"
		);
		anyhow::ensure!(
			!state.filesystem.volumes.contains_key(key),
			"a record-missing incarnation is registered for apply"
		);
		anyhow::ensure!(
			!state.unloading.contains_key(key),
			"a record-missing incarnation is unloading"
		);
	}
	for (_, id) in state.filesystem.retention.iter() {
		anyhow::ensure!(
			state
				.volume_keys
				.key(*id)
				.is_some_and(|key| state.filesystem.volumes.contains_key(key)),
			"retention check queued for an unregistered volume"
		);
	}
	for key in &state.filesystem.flush_candidates {
		anyhow::ensure!(
			state
				.filesystem
				.volumes
				.get(key)
				.is_some_and(|volume| volume.flush_requested),
			"flush candidate without a flush request"
		);
	}
	for (bytes, key) in &state.filesystem.unflushed {
		anyhow::ensure!(
			state
				.filesystem
				.volumes
				.get(key)
				.is_some_and(|volume| volume.unflushed_bytes == bytes.0),
			"piggyback index disagrees with unflushed bytes"
		);
	}
	anyhow::ensure!(
		state.filesystem.unflushed_total
			== state
				.filesystem
				.volumes
				.values()
				.map(|volume| volume.unflushed_bytes)
				.sum::<u64>(),
		"tablet unflushed total disagrees with its volumes"
	);
	for (key, volume) in &state.filesystem.volumes {
		anyhow::ensure!(
			state.loaded_volumes.contains_key(key),
			"filesystem volume lost lifecycle owner"
		);
		anyhow::ensure!(
			volume.unflushed_bytes == 0
				|| state
					.filesystem
					.unflushed
					.contains(&(Reverse(volume.unflushed_bytes), key.clone())),
			"volume with unflushed bytes missing from the piggyback index"
		);
		// A requested volume with something to flush must be visible to selection.
		let has_work =
			volume.applied.volume_seq > volume.record.cold.volume_seq || !volume.record_persisted;
		anyhow::ensure!(
			!(volume.flush_requested && has_work)
				|| state.filesystem.flush_candidates.contains(key),
			"requested volume with work missing from the flush candidates"
		);
		anyhow::ensure!(
			volume.applied.volume_seq >= volume.record.cold.volume_seq
				&& volume.applied.client_epoch >= volume.record.cold.client_epoch,
			"filesystem cold exceeds applied"
		);
		anyhow::ensure!(
			state
				.filesystem
				.pages
				.iter()
				.filter(|p| &p.key == key)
				.count() <= 1,
			"duplicate filesystem apply page"
		);
	}
	anyhow::ensure!(
		state
			.operations
			.values()
			.filter(|op| matches!(op, Operation::FilesystemRead { .. }))
			.count() <= state.config.max_background_cold_reads as usize,
		"filesystem read window exceeded"
	);
	let mut applying = BTreeSet::new();
	for op in state.operations.values() {
		if let Operation::FilesystemApply { key, .. } = op {
			anyhow::ensure!(
				applying.insert(key),
				"filesystem applies of one volume overlap"
			);
		}
	}
	// At most one flush stages and two are in flight, and they never share a record. A
	// flush that staged beside an upload uploads beside it until either completes.
	let mut staging = 0;
	let mut uploading = 0;
	let mut flushing = BTreeSet::new();
	for op in state.operations.values() {
		let Operation::FilesystemFlush { records, stage, .. } = op else {
			continue;
		};
		match stage {
			FlushStage::Staging => staging += 1,
			FlushStage::Uploading => uploading += 1,
		}
		for record in records {
			anyhow::ensure!(
				flushing.insert(record.volume_key()),
				"filesystem flushes share a record"
			);
		}
	}
	anyhow::ensure!(
		staging <= 1 && staging + uploading <= 2,
		"filesystem flushes overlap"
	);
	for op in state.operations.values() {
		let Operation::FilesystemFlush { records, .. } = op else {
			continue;
		};
		for record in records {
			let key =
				VolumeLogKey::from(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
					volume_key: record.volume_key(),
					volume_incarnation: record.volume_incarnation,
				});
			if let Some(volume) = state.filesystem.volumes.get(&key) {
				anyhow::ensure!(
					volume.record.cold.volume_seq <= record.cold.volume_seq
						&& record.cold.volume_seq <= volume.applied.volume_seq,
					"in-flight flush is outside the published cold floor and applied position"
				);
			}
		}
	}
	Ok(())
}

/// Retention checks visited per timer delivery. Due checks past it stay queued and the
/// timer fires again at once.
const RETENTION_CHECKS_PER_TIMER: usize = 512;

/// When a registered volume's retention policy next elapses, or `None` without an active
/// policy. The volume's own policy wins over the configured default.
fn retention_due_at(state: &MaintainerState, key: &VolumeLogKey) -> Option<u64> {
	let volume = state.filesystem.volumes.get(key)?;
	let policy = volume.record.retention.as_ref().or(state
		.config
		.checkpoints
		.retention
		.as_ref())?;
	if policy.count == 0 {
		return None;
	}
	Some(
		volume
			.retention_checked_boot_ms
			.saturating_add(policy.interval_ms),
	)
}

/// Schedules a volume's next retention check from its record and check time, or drops
/// it when no active policy applies. Registration, record changes, each flush and the apply
/// that moves a parked volume call this.
pub(crate) fn schedule_retention(state: &mut MaintainerState, key: &VolumeLogKey) {
	match retention_due_at(state, key) {
		Some(at) => {
			let id = state.volume_keys.intern(key);
			state.filesystem.retention.schedule(id, at);
		}
		None => {
			if let Some(id) = state.volume_keys.find(key) {
				state.filesystem.retention.cancel(&id);
			}
		}
	}
}

/// The volume at `key` moved. A parked retention check, one that left the queue because the
/// volume was unchanged, is queued again an interval from now, so it never lands on the first
/// write of a burst and its restore point captures the burst. A volume still in the queue
/// keeps its check.
pub(crate) fn requeue_retention(state: &mut MaintainerState, key: &VolumeLogKey) {
	if let Some(id) = state.volume_keys.find(key)
		&& state.filesystem.retention.scheduled_at(&id).is_some()
	{
		return;
	}
	let now = state.now_boot_ms;
	let Some(volume) = state.filesystem.volumes.get_mut(key) else {
		return;
	};
	volume.retention_checked_boot_ms = now;
	schedule_retention(state, key);
}

/// Runs the due retention checks. Durable rows decide whether wall-clock time really
/// crossed an interval; each check rederives its deadline from the current record.
pub(crate) fn retention_due(state: &mut MaintainerState, fx: &mut Effects) {
	state.filesystem.retention_armed_at = None;
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	let now = state.now_boot_ms;
	let due = state
		.filesystem
		.retention
		.pop_due(now, RETENTION_CHECKS_PER_TIMER);
	crate::metrics::record_idle_visits("retention_tick", due.len());
	for (_, id) in due {
		let Some(key) = state.volume_keys.key(id).cloned() else {
			continue;
		};
		let Some(at) = retention_due_at(state, &key) else {
			continue;
		};
		if retiring(state, &key) {
			let retry_at = now + state.config.tuning().retry_ms;
			state.filesystem.retention.schedule(id, retry_at);
			continue;
		}
		if at > now {
			state.filesystem.retention.schedule(id, at);
			continue;
		}
		// A volume still applied where its newest retention checkpoint stands has nothing
		// new to keep: the check logs no op and leaves the queue, so the volume costs no
		// timer while it stays unchanged, and it keeps the older distinct restore points a
		// copy would push out of the policy's count. The apply that moves it queues it
		// again; see `applied`.
		let volume = state
			.filesystem
			.volumes
			.get_mut(&key)
			.expect("retention due for a registered volume");
		if volume.retention_checkpoint_seq == Some(volume.applied.volume_seq) {
			volume.retention_checked_boot_ms = now;
			crate::metrics::record_retention_check("parked");
			continue;
		}
		// One retention op of a volume at a time: each holds the volume's apply at its
		// position until applied, so a stream of them would keep apply from moving on. An
		// apply already in flight would pass the op's position: it could apply a later
		// policy change to the store before the op, which then still uses the earlier policy.
		// A held drain's applied tail is not a committed position for the op to record; see
		// `checkpoints::waits_for_drain`.
		if !super::maintainer_log::admits(state)
			|| super::maintainer_log::replay::pending_for(state, &key.0.volume_key)
			|| super::maintainer_log::replay::has_ops(state, &key)
			|| super::drain::held(state, &key)
			|| state
				.operations
				.index()
				.filesystem_applies
				.contains_key(&key)
		{
			let retry_at = now + state.config.tuning().retry_ms;
			state.filesystem.retention.schedule(id, retry_at);
			crate::metrics::record_retention_check("deferred");
			continue;
		}
		// The checkpoint is a maintainer log op at the volume's applied position, applied
		// once it commits. It needs no pass of its own.
		let volume = state
			.filesystem
			.volumes
			.get_mut(&key)
			.expect("retention due for a registered volume");
		volume.retention_checked_boot_ms = now;
		let after = volume.applied.clone();
		crate::metrics::record_retention_check("logged");
		let op =
			rivet_data::generated::depot_maintainer_log_v1::MaintainerOp::RetentionCheckpointOp(
				rivet_data::generated::depot_maintainer_log_v1::RetentionCheckpointOp {
					target: rivet_data::generated::depot_maintainer_log_v1::MaintainerOpTarget {
						volume_log_key: key.0.clone(),
						after,
					},
					created_at_unix_ms: fx.clocks().unix_ms,
				},
			);
		super::maintainer_log::replay::submit_live(state, op, key.clone());
		schedule_retention(state, &key);
	}
}

/// A retention op of `key` applied: the store's newest retention checkpoint stands at
/// `checkpoint_seq`. The next check of a volume still applied there logs nothing.
pub(crate) fn retention_checked(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	checkpoint_seq: Option<u64>,
) {
	if let Some(volume) = state.filesystem.volumes.get_mut(key) {
		volume.retention_checkpoint_seq = checkpoint_seq;
	}
}

/// Keeps `MaintainerTimer::RetentionDue` armed at the head of the retention queue.
pub(crate) fn arm_retention(state: &mut MaintainerState, fx: &mut Effects) {
	let next_at = state.filesystem.retention.next_at();
	super::arm_queue_head(
		&mut state.filesystem.retention_armed_at,
		next_at,
		&state.phase,
		state.now_boot_ms,
		MaintainerTimer::RetentionDue,
		fx,
	);
}

/// An administrative mutation flushed this record and its applied data. Consume
/// its proof before later store work, including when the original caller expired.
/// A materialization released `key`'s base chain in the store: the registered record drops
/// it too, and the next ordinary pass writes the record.
pub(crate) fn chain_released(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	let Some(volume) = state.filesystem.volumes.get_mut(key) else {
		return;
	};
	if volume.record.base_chain.is_empty() {
		return;
	}
	volume.record.make_mut().base_chain.clear();
	volume.record_persisted = false;
	unrecorded(state, key.clone(), fx);
}

pub(crate) fn record_flushed(
	state: &mut MaintainerState,
	record: &FsVolumeRecord,
	fx: &mut Effects,
) {
	let key = VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
		volume_key: depot_types::VolumeKey {
			partition_key: record.partition_key.clone(),
			name: record.name.clone(),
		},
		volume_incarnation: record.volume_incarnation,
	});
	stored(state, &key, record, fx);
	if let Some(volume) = state.loaded_volumes.get_mut(&key) {
		volume.stored_row = Some(record.clone().into());
	}
	// The stored row can make an unregistered volume eligible for filesystem apply.
	offer(state, &key);
	super::deletion::observe_loaded(state, &key);
	if !retiring(state, &key) {
		state.pending_trims.insert(key.clone(), record.cold.clone());
		for worker in state.members() {
			super::cold::trim(state, key.clone(), worker, fx);
		}
	}
}

/// Queues again the session opens and claim reads of `key` waiting for a position at or below
/// `applied`, in arrival order behind work already queued in their lanes.
pub fn wake_lagging_opens(state: &mut MaintainerState, key: &VolumeLogKey, applied: u64) {
	let Some(waits) = state.lagging_opens.get_mut(key) else {
		return;
	};
	let mut ready = Vec::new();
	waits.retain(|(seq, id)| {
		if *seq <= applied {
			ready.push(*id);
			false
		} else {
			true
		}
	});
	if waits.is_empty() {
		state.lagging_opens.remove(key);
	}
	for id in ready {
		let Some(request) = state.pending_fs_reads.get(&id) else {
			continue;
		};
		match super::filesystem_reads::query_order(request) {
			super::filesystem_reads::QueryOrder::Claim => state.session_reads.push_back(id),
			super::filesystem_reads::QueryOrder::Ordered
			| super::filesystem_reads::QueryOrder::Session
			| super::filesystem_reads::QueryOrder::HeldImage => state
				.store_queue
				.push_back(super::store::StoreWork::FilesystemQuery { request_id: id }),
		}
	}
}
