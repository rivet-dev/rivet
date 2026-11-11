//! The group cache: what a maintainer keeps in memory about the volume groups its loaded
//! volumes belong to, and the fork pin it logs before a grouped volume's apply.
//!
//! An entry exists while a volume of the group is registered and holds the group's record,
//! its latest fork number and the fork child rows the maintainer has read. It is fed by one
//! read of the group volume on the first load of a group's volume, by the events the process
//! polls, and by the fork children a pin asks for. Before a grouped volume's entries apply, the
//! maintainer compares the group's latest fork with the volume's `fork_pinned_through`; when
//! the group was forked since, it logs a `ForkPinOp` at the volume's applied cut and the apply
//! waits for the op, so the children of those forks read the state from before the write.
//!
//! A record read that asks for no fork child rows is answered by the process-wide group record
//! cache (`group_events::records`) when the process has read the group before, so a tablet with
//! no loaded volume of the group pays no group volume read for a create's lineage walk and fork
//! stamp, or for a due alarm. The entry here still holds the per-tablet bookkeeping (loaded
//! volumes, held applies, pin waits and wanted forks); only the record comes from the cache.

use super::{MaintainerState, Operation, filesystem::LatestBarrier};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::MaintainerEffect,
	types::{
		CLOSE_NOTICE_WAIT_MS, FORK_PIN_ROW_VERSION, GROUP_READ_MAX_CHILDREN, GroupChildren,
		MaintainerTimer, OpId,
	},
};
use depot_fs_core::{FsForkGroup, FsForkSelector, FsForkSpec, keys::slatedb::Scope};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_group_v1::{Event, GroupReadResponse, GroupRecord},
	depot_maintainer_log_v2::{ForkPinOp, MaintainerOp},
	shared_depot_fs_v2::{FsClaimLogPosition, FsCut},
};
use rivet_keys::keys::codec::GroupId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The longest a failed group read waits before it is tried again. Each failure in a row
/// doubles the wait from the tuning's `retry_ms` up to this.
const GROUP_READ_MAX_BACKOFF_MS: u64 = 30_000;

/// How many reads `drive` starts from the first lane before it starts one from the background
/// lane, while both lanes hold entries. Background reads go after the reads that hold applies
/// and opens, and this share keeps a steady stream of those from delaying them without bound.
const READS_BEFORE_BACKGROUND_READ: u32 = 3;

/// The first `depot.maintainer_log` version with a fork pin op. A grant writing an older
/// version cannot log a pin, so it holds the apply of a grouped volume whose group was forked
/// until a later grant can; group forks are gated on the fleet's versions, so this only
/// happens to a grant that started before the fleet's floor rose.
pub(crate) const FORK_PIN_LOG_VERSION: u16 = 2;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Groups {
	pub(crate) entries: BTreeMap<GroupId, GroupEntry>,
	/// The sum of every entry's `wanted`, kept with each change so the snapshot reads it
	/// without a scan. Capped by `max_group_wanted_forks`.
	pub(crate) wanted_forks: usize,
	/// The sum of the children every entry's `pin_waits` holds. Capped by
	/// `max_group_pin_wait_children`.
	pub(crate) pin_wait_children: usize,
	/// How many volumes' pin waits name each child group, kept with each change. A child's
	/// entry with no loaded volume leaves when its count reaches zero, so entries made only to
	/// learn a waited child's status do not outlive the wait.
	pub(crate) wait_refs: BTreeMap<GroupId, u32>,
	/// Entries whose `want` stopped at `max_group_wanted_forks`. A held volume of such an entry
	/// may have no read coming that would release it, so once the tablet wants fewer forks than
	/// the cap their held volumes are offered for apply again, and each holds anew for what now
	/// fits. Bounded by the entries.
	pub(crate) cut_short: BTreeSet<GroupId>,
	/// Group read operations in flight, kept with each change. Capped by
	/// `max_group_reads_in_flight`. A slot is held from the read's start until its response or
	/// its operation timeout, whether or not its entry is still cached. A timeout also aborts
	/// the read's local task, so the reads this maintainer waits on stay within the cap; the cap
	/// is a local bound, and a read already sent to another worker may still finish there.
	pub(crate) reads_in_flight: usize,
	/// Entries waiting for a slot to read, so `drive` starts the next read without scanning
	/// every entry.
	pub(crate) read_queue: ReadQueue,
	/// Registered volumes of archived groups whose writer is still to be fenced by a recovery
	/// claim, which waits for a recovery slot and for lifecycle work on the volume. Each leaves
	/// once its recovery starts or the volume unregisters; bounded by the registered volumes.
	pub(crate) archive_fences: BTreeSet<VolumeLogKey>,
	/// Volumes that registered into a group the cache already knew archived, whose slot and
	/// read sessions are still to be closed. A registration has no effect sink of its own, so
	/// [`drive_archive_fences`] closes them at the end of the same event; each is also in
	/// `archive_fences`. Leaves with the close or when the volume unregisters.
	pub(crate) archive_closes: BTreeSet<VolumeLogKey>,
	/// Group fork acquires in `pending_fork_references`, by the parent group they name, kept
	/// with each admitted and answered acquire. An entry a pending acquire names stays though
	/// no loaded volume names it, so the read an idle parent's acquire waits on is never
	/// dropped by another group's unregistration. The acquire's deadline timer answers it, so
	/// a caller that never retries keeps the entry at most until then.
	pub(crate) acquire_refs: BTreeMap<GroupId, u32>,
	/// Each cached fork child by its child group, with the entry and fork number that hold
	/// it, so an archive event finds the one entry that caches the child without a scan.
	/// Bounded by the cached fork children.
	pub(crate) child_forks: BTreeMap<GroupId, (GroupId, u64)>,
	/// The cached groups whose read records name each group as their base. An entry with a
	/// cached child stays for that child's lineage, and leaves once its last child did.
	pub(crate) based_on: BTreeMap<GroupId, BTreeSet<GroupId>>,
	/// Entries that may have lost their last use since the last unregistration of a group's
	/// last volume: entries created since, and entries whose use by a loaded volume, a pin
	/// wait, an acquire, a waiting open or an alarm ended. That unregistration checks only
	/// these, and the ancestors of the ones it drops, never every entry. Bounded by the
	/// entries.
	pub(crate) unswept: BTreeSet<GroupId>,
	/// Passes over a volume's fork pins in flight that carry archived children (a pin op, a
	/// pin pass or a registering load), by operation. A completed one records how far the
	/// volume's pins have had the cache's archived children applied; see
	/// `GroupEntry::passed`. Bounded by the operations in flight.
	pub(crate) archive_passes: BTreeMap<OpId, ArchivePass>,
	/// The pin floor of each registered grouped log whose members held entries above its
	/// applied position when it registered on this grant; see [`PinFloor`]. Those entries were
	/// written under an earlier grant, which may have applied them before the fork in memory
	/// only, so a fork pin waits until the log applies through its floor and the pin's cut
	/// holds them. Without it a grant that learns of a fork before it applies its
	/// predecessor's entries pins below acknowledged writes the fork must keep. A floor leaves
	/// once its log applies through it, the log's pin is logged, or the log unregisters.
	/// Bounded by the registered logs that registered behind their members' tails.
	pub(crate) pin_floors: BTreeMap<VolumeLogKey, PinFloor>,
	/// Deleted incarnations whose pending sweep only fork pins hold, by scope, with the child
	/// groups those pins wait on. A deleted incarnation has no next pin or load to drop an
	/// archived child from its pins, so the cache does it: each waited child is counted in
	/// `pin_wait_children` and `wait_refs`, which keeps its entry read, and leaves the wait once
	/// the cache learns it archived; with the last one the sweep is queued again with every
	/// archived child, which drops them from the pins and sweeps. In memory from the held sweep
	/// until its children are archived, within `max_group_pin_wait_children`; a wait lost to a
	/// restart or move is found again when the new owner's pending scan runs the sweep.
	pub(crate) deleted_pins: BTreeMap<Scope, DeletedPinWait>,
	/// The scopes of `deleted_pins` that wait on each child group, so an archive settles the
	/// scopes naming the child without a scan.
	pub(crate) deleted_pin_scopes: BTreeMap<GroupId, BTreeSet<Scope>>,
	/// Deleted incarnations whose held sweep found no room under `max_group_pin_wait_children`
	/// for any of its pins' children, so nothing waits for them. Once a wait frees room,
	/// [`resume_skipped_deleted_pins`] queues one of their sweeps again, which reports its
	/// children again. Leaves when its sweep is dispatched. At most
	/// `max_group_pin_wait_children` scopes; one past that is left to the next pending scan.
	pub(crate) deleted_pins_skipped: BTreeSet<Scope>,
	/// The skipped scope whose sweep [`resume_skipped_deleted_pins`] queued and the store has
	/// not dispatched yet, so at most one resumed sweep waits in the store queue at a time.
	pub(crate) deleted_pins_resuming: Option<Scope>,
}

/// The fork pins of one deleted incarnation that hold its sweep; see `Groups::deleted_pins`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct DeletedPinWait {
	/// The child groups the pins wait on that the cache does not know archived.
	pub waiting: BTreeSet<GroupId>,
	/// The child groups the cache learned archived, which the next sweep drops from the pins.
	pub archived: BTreeSet<GroupId>,
}

/// How far a registered grouped log applies before its fork pin, from the `Latest` barrier
/// its members showed when it registered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum PinFloor {
	/// The members proved entries committed through `through`, above the applied position:
	/// the pin waits until the log applied through it.
	Proven { through: u64 },
	/// An in-sync member held a tail up to `frontier` that the members did not prove
	/// committed, under claims no newer than `claim_epoch`. It may hold commits a dead or
	/// earlier claimant acknowledged, so the pin waits until
	/// `filesystem::tail_proven_through` resolves the floor to `Proven` (a recovery's claim
	/// entry or the writer's next quorum proves it); meanwhile the log applies what its
	/// members prove, up to `frontier`.
	Unproven { frontier: u64, claim_epoch: u64 },
}

/// A pass over `key`'s fork pins in flight, which carries the archived children of its group
/// the cache learned through archive order `through`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ArchivePass {
	key: VolumeLogKey,
	group: GroupId,
	through: u64,
}

/// The entries due a record read that wait for a slot of `max_group_reads_in_flight`, each
/// lane oldest first. An entry is queued when it comes due (created unread, marked unread
/// again, a fork wanted, a retry fired, a volume registered) and leaves when it is popped or
/// removed. A popped entry that is no longer due is skipped. Every due entry without a read in
/// flight or a retry armed is in at least one lane; one may be in both, when a volume of a
/// background entry registers or an open first uses an alarm-only entry, and the second pop
/// skips it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ReadQueue {
	/// Entries whose reads hold applies, opens, fork stamps or acquires, by queue order.
	first: BTreeMap<u64, GroupId>,
	/// Entries with no loaded volume made only to learn a waited child's status or to answer
	/// due alarms, by queue order. Read after `first`, one in every
	/// `READS_BEFORE_BACKGROUND_READ + 1` starts while both lanes hold entries.
	background: BTreeMap<u64, GroupId>,
	/// The next queue order to hand out.
	next_seq: u64,
	/// Reads started from `first` since the last one from `background`.
	first_streak: u32,
}

impl ReadQueue {
	/// Places queued in the first lane, including ones whose entry stopped being due.
	pub(crate) fn first_len(&self) -> usize {
		self.first.len()
	}

	/// Places queued in the background lane, including ones whose entry stopped being due.
	pub(crate) fn background_len(&self) -> usize {
		self.background.len()
	}
}

/// Which lane of the read queue an entry is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lane {
	First,
	Background,
}

impl Groups {
	/// The entry of `group`, created and queued for its record read when the cache holds none.
	/// An alarm-only entry stops being one, and a read it is due moves to its lane now.
	fn entry(&mut self, group: GroupId) -> &mut GroupEntry {
		match self.entries.get_mut(&group) {
			None => {
				self.entries.insert(group, GroupEntry::default());
				self.unswept.insert(group);
				self.queue_read(group);
			}
			Some(entry) if entry.alarm_only => {
				entry.alarm_only = false;
				self.queue_read(group);
			}
			Some(_) => {}
		}
		self.entries.entry(group).or_default()
	}

	/// The entry of `group` for a due alarm that waits on its record. One the cache does not
	/// hold is created alarm-only, so its read goes in the background lane behind the reads
	/// that hold applies and opens. It leaves once its read ended and the index reads that
	/// read's end started finished, unless another use promoted it or alarms keep it for an
	/// overflow's discovery pass.
	fn alarm_entry(&mut self, group: GroupId) -> &mut GroupEntry {
		if !self.entries.contains_key(&group) {
			self.entries.insert(
				group,
				GroupEntry {
					alarm_only: true,
					..GroupEntry::default()
				},
			);
			self.unswept.insert(group);
			self.queue_read(group);
		}
		self.entries.entry(group).or_default()
	}

	/// Queues `group`'s entry for a read if it is due one and not already queued in its lane.
	/// An entry with no loaded volume that a pin wait names, or that only alarms asked for,
	/// goes in the background lane, every other entry in the first lane.
	fn queue_read(&mut self, group: GroupId) {
		let pin_wait = self.wait_refs.contains_key(&group);
		let Some(entry) = self.entries.get_mut(&group) else {
			return;
		};
		if !entry.due() {
			return;
		}
		let background = entry.loaded.is_empty() && (pin_wait || entry.alarm_only);
		let seq = self.read_queue.next_seq;
		if background {
			if entry.queued_background.is_some() {
				return;
			}
			entry.queued_background = Some(seq);
			self.read_queue.background.insert(seq, group);
		} else {
			if entry.queued_first.is_some() {
				return;
			}
			entry.queued_first = Some(seq);
			self.read_queue.first.insert(seq, group);
		}
		self.read_queue.next_seq += 1;
	}

	/// Pops the next queued entry: from the background lane once `READS_BEFORE_BACKGROUND_READ`
	/// reads started from the first lane in a row, or when the first lane is empty.
	fn pop_queued(&mut self) -> Option<(GroupId, Lane)> {
		let queue = &mut self.read_queue;
		let background_turn = !queue.background.is_empty()
			&& (queue.first.is_empty() || queue.first_streak >= READS_BEFORE_BACKGROUND_READ);
		let (lane, group) = if background_turn {
			let (_, group) = queue.background.pop_first()?;
			(Lane::Background, group)
		} else {
			let (_, group) = queue.first.pop_first()?;
			(Lane::First, group)
		};
		if let Some(entry) = self.entries.get_mut(&group) {
			match lane {
				Lane::First => entry.queued_first = None,
				Lane::Background => entry.queued_background = None,
			}
		}
		Some((group, lane))
	}
}

/// A child of one fork of a group, as the pin path needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct CachedChild {
	pub child: GroupId,
	pub archived: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct GroupEntry {
	/// The record, once read. A group whose volume holds no record reads as a root with no
	/// forks, which `default`'s root group is until its volume is created at rollout.
	pub record: Option<GroupRecord>,
	/// The read of the record finished, whether or not it found one.
	pub read_done: bool,
	/// The last fork number drawn from this group that the maintainer knows: the record's,
	/// raised by events and by the acquires of children.
	pub latest_fork: u64,
	/// The fork child rows read so far, by fork number.
	pub fork_children: BTreeMap<u64, CachedChild>,
	/// Fork numbers whose rows a pin or an acquire needs and the cache lacks. Changed only
	/// through `want` and `unwant`, which keep `Groups::wanted_forks`.
	pub wanted: BTreeSet<u64>,
	/// Registered volumes of this group on the tablet; the entry leaves with the last. An
	/// archive of the group closes each of them.
	pub loaded: BTreeSet<VolumeLogKey>,
	/// The read in flight.
	pub read: Option<OpId>,
	/// A failed read's retry timer is armed; nothing reads until it fires.
	pub retry_armed: bool,
	/// Volumes whose apply waits for this entry's read, marked candidates again when it
	/// completes.
	pub held: BTreeSet<VolumeLogKey>,
	/// Registered base volumes whose load left fork pins waiting, with the waited-on child
	/// groups whose status the cache does not know yet. Each child leaves once its status is
	/// known, from this entry's fork children or from the child group's own record, which the
	/// cache reads for it; every read that finds one archived queues the volume's pin pass
	/// again, and the volume leaves with its last unknown child. Capped tablet-wide by
	/// `max_group_pin_wait_children`; a volume's waits go when it unregisters.
	pub pin_waits: BTreeMap<VolumeLogKey, BTreeSet<GroupId>>,
	/// An archive event named this group. It is kept apart from `record` because the event may
	/// arrive before the record is read, or while a read whose snapshot predates the archive
	/// is in flight; `read_completed` merges it into the record, since an archive is final.
	pub archived_seen: bool,
	/// A gap, or a fork event naming this group as a child the cache holds no record for,
	/// marked the entry for another read while `read` was in flight. That read's snapshot may
	/// predate the event, so `read_completed` leaves `read_done` false and the entry is read
	/// again. Set only while a read is in flight; every completion of that read clears it.
	pub gap_pending: bool,
	/// Reads of this entry that failed in a row, which set the retry's backoff. Reset by a
	/// read that succeeds.
	pub failed_reads: u32,
	/// Only due alarms asked for this entry: its read goes in the background lane while no
	/// volume of the group is loaded, and its record starts no lineage walk. Cleared for good
	/// by the first other use of the entry (a load, an open, a pin, a stamp or an acquire).
	pub alarm_only: bool,
	/// The entry's place in the read queue's first lane, while it is there.
	pub queued_first: Option<u64>,
	/// The entry's place in the read queue's background lane, while it is there.
	pub queued_background: Option<u64>,
	/// The lowest fork after an idle parent's last pin that the store asked this entry's
	/// children from, while planning that parent's pin for a group fork acquire. Forks below
	/// it are covered by the pins of every idle parent planned so far, so an acquire reads and
	/// sends only the children from here through its fork, not every fork the group made.
	/// `None` until the store first asks; it only falls.
	pub idle_pin_floor: Option<u64>,
	/// The archived fork children still cached, by the order the cache learned them archived
	/// (starting at 1), each with its fork number. Each leaves `fork_children` once a pass
	/// applied it to every registered volume of the group and none of their next pins needs
	/// it; see `prune_archived`.
	pub archived_order: BTreeMap<u64, u64>,
	/// The last order `archived_order` handed out.
	pub last_archive_seq: u64,
	/// For each registered base volume of the group, the archive order through which a pass
	/// over its pins dropped every archived child the cache knew, and its pin covers each
	/// one's fork. A pass carries only the archived children past it.
	pub passed: BTreeMap<VolumeLogKey, u64>,
	/// How many registered base volumes stand at each value of `passed`, so the lowest is a
	/// lookup.
	pub passed_counts: BTreeMap<u64, u32>,
}

impl GroupEntry {
	/// Whether the entry needs a read and has none in flight or waiting on a retry.
	fn due(&self) -> bool {
		self.read.is_none() && !self.retry_armed && (!self.read_done || !self.wanted.is_empty())
	}

	/// Whether the pin path knows the child of every fork in `range`.
	fn knows(&self, range: impl Iterator<Item = u64>) -> bool {
		let mut range = range;
		range.all(|fork| self.fork_children.contains_key(&fork))
	}
}

/// The group whose forks pin `key`'s volume, or `None` for a volume outside any group.
pub(crate) fn group_of(key: &VolumeLogKey) -> Option<GroupId> {
	depot_types::keys::partition::forkable_group(&key.0.volume_key.partition_key)
}

/// Whether `key`'s group is archived, as the cache knows it: `None` outside a group or while
/// the group's record is still to be read, unless an archive event already named the group,
/// since an archive is final. A group with no entry gets one, and an unread entry's read starts
/// now, beside the caller's record lookup, rather than after it.
pub(crate) fn archived(
	state: &mut MaintainerState,
	key: &depot_types::VolumeKey,
	fx: &mut Effects,
) -> Option<bool> {
	let group = depot_types::keys::partition::forkable_group(&key.partition_key)?;
	let archived = entry_archived(state.groups.entry(group));
	if archived.is_none() {
		drive(state, fx);
	}
	archived
}

/// Whether `group` is archived, as the cache knows it, creating nothing: `None` while the cache
/// holds no entry or its record is still to be read and no archive event named it.
pub(crate) fn cached_archived(state: &MaintainerState, group: GroupId) -> Option<bool> {
	state.groups.entries.get(&group).and_then(entry_archived)
}

/// Makes sure the cache holds an entry for `group`, which a due alarm waits on. A new one is
/// alarm-only, queued for its read in the background lane behind the reads that hold applies
/// and opens; the caller drives the read.
pub(crate) fn alarm_entry(state: &mut MaintainerState, group: GroupId) {
	state.groups.alarm_entry(group);
}

/// Whether `group`'s entry is alarm-only and nothing else needs it: [`unused_alarm_entry`],
/// and no due key waits on its read and no index read its read's end started is pending
/// ([`super::alarms::holds_entry`]). Such an entry may leave.
pub(crate) fn idle_alarm_entry(state: &MaintainerState, group: GroupId) -> bool {
	unused_alarm_entry(state, group) && !super::alarms::holds_entry(state, group)
}

/// Whether `group`'s entry is alarm-only and nothing outside alarms needs it: no loaded volume,
/// no read in flight or retry armed, and nothing held, wanted or pin-waited on it.
pub(crate) fn unused_alarm_entry(state: &MaintainerState, group: GroupId) -> bool {
	let Some(entry) = state.groups.entries.get(&group) else {
		return false;
	};
	entry.alarm_only
		&& entry.loaded.is_empty()
		&& entry.read.is_none()
		&& !entry.retry_armed
		&& entry.held.is_empty()
		&& entry.wanted.is_empty()
		&& entry.pin_waits.is_empty()
		&& !state.groups.wait_refs.contains_key(&group)
}

/// Drops `group`'s entry if [`idle_alarm_entry`] holds, so alarm-only entries do not
/// accumulate on a tablet without volume churn.
pub(crate) fn drop_idle_alarm_entry(state: &mut MaintainerState, group: GroupId) {
	if idle_alarm_entry(state, group) {
		remove_entry(state, group);
	} else if state.groups.entries.contains_key(&group) {
		// The alarms' use of the entry ended; another use may keep it.
		state.groups.unswept.insert(group);
	}
}

/// Whether the cache already knows `key`'s group is archived. Creates and reads nothing, so
/// a group with no entry, or one still to be read that no archive event named, answers
/// `false`.
pub(crate) fn known_archived(state: &MaintainerState, key: &depot_types::VolumeKey) -> bool {
	let Some(group) = depot_types::keys::partition::forkable_group(&key.partition_key) else {
		return false;
	};
	state
		.groups
		.entries
		.get(&group)
		.and_then(entry_archived)
		.unwrap_or(false)
}

/// Whether `entry`'s group is archived: `None` while its record is still to be read and no
/// archive event named it.
fn entry_archived(entry: &GroupEntry) -> Option<bool> {
	if entry.archived_seen {
		return Some(true);
	}
	if !entry.read_done {
		return None;
	}
	Some(entry.record.as_ref().is_some_and(|record| {
		record.status == rivet_data::generated::depot_group_v1::GroupStatus::Archived
	}))
}

/// The fork stamp a volume created or deleted now in `key`'s group records: `Ok(None)` outside a
/// group, `Ok(Some(latest_fork))` once the group's record was read. `Err(group)` while it is
/// still to be read: a missing stamp would read as fork 0, so the caller waits for the read,
/// which this starts.
///
/// The stamp is as current as the record the read returned, raised by every event since. A
/// record served by the process's group record cache is current as of the last completed
/// events poll of every shard, so a stamp may lag a fork drawn within one
/// `depot.events.poll_interval_ms`: the volume then reads in that fork's child as if it was
/// created just before the fork, with the writes it applied until its pin logged once the fork
/// event arrived. A fork acquire raises the parent tablet's `latest_fork` itself, so the pin
/// path tolerates the same lag, and the child never misses a write from before its fork.
pub(crate) fn fork_stamp(
	state: &mut MaintainerState,
	key: &depot_types::VolumeKey,
	fx: &mut Effects,
) -> Result<Option<u64>, GroupId> {
	let Some(group) = depot_types::keys::partition::forkable_group(&key.partition_key) else {
		return Ok(None);
	};
	let entry = state.groups.entry(group);
	if entry.read_done {
		return Ok(Some(entry.latest_fork));
	}
	drive(state, fx);
	Err(group)
}

/// Whether restoring a grouped incarnation created at fork `created_at_fork` would hide it from
/// a child group: the group forked at `latest_fork` since, and a child looks the incarnation up
/// by its name, which a restore points at the restored incarnation. Until a restored incarnation
/// links the one it replaces, such a restore is refused. A child forked before the incarnation
/// was created never reads it, so a restore of an incarnation no fork has passed is safe.
pub(crate) fn restore_hides_from_children(created_at_fork: Option<u64>, latest_fork: u64) -> bool {
	latest_fork > created_at_fork.unwrap_or(0)
}

/// Whether the cache already knows that `key`'s group forked since `created_at_fork`, so a
/// restore of that incarnation is refused at admission; see [`restore_hides_from_children`].
/// A group the cache has not read yet is checked again when the restore stamps its fork.
pub(crate) fn known_forked_since(
	state: &MaintainerState,
	key: &depot_types::VolumeKey,
	created_at_fork: Option<u64>,
) -> bool {
	let Some(group) = depot_types::keys::partition::forkable_group(&key.partition_key) else {
		return false;
	};
	state.groups.entries.get(&group).is_some_and(|entry| {
		entry.read_done && restore_hides_from_children(created_at_fork, entry.latest_fork)
	})
}

/// Whether this grant's selected versions can record a fork stamp of `key`: always outside a
/// group, and inside one only at the `depot.fs_rows` and `depot.maintainer_log` versions that
/// carry a group's fork numbers.
pub(crate) fn records_group_forks(state: &MaintainerState, key: &depot_types::VolumeKey) -> bool {
	if depot_types::keys::partition::forkable_group(&key.partition_key).is_none() {
		return true;
	}
	let versions = &state.config.write_versions;
	versions.fs_rows >= FORK_PIN_ROW_VERSION && versions.maintainer_log >= FORK_PIN_LOG_VERSION
}

/// The fork stamp `key`'s record carries for `stamp` under this grant. Every reader takes no
/// stamp as fork 0, so a grant that cannot record a group's fork numbers records a stamp of
/// fork 0 as none, and a group that never forked keeps working under it. A stamp past fork 0
/// is kept, and such a grant cannot record it.
pub(crate) fn recorded_stamp(
	state: &MaintainerState,
	key: &depot_types::VolumeKey,
	stamp: Option<u64>,
) -> Option<u64> {
	match stamp {
		Some(0) if !records_group_forks(state, key) => None,
		stamp => stamp,
	}
}

/// The lineage of `group`, nearest ancestor first, each with the fork number by which the next
/// nearer group left it. `Err(unread)` names the first group whose record is still to be read;
/// such entries are created unloaded and read by the next drive. A lineage that does not end
/// at a root names `group` itself, whose waiting opens then expire.
pub(crate) fn lineage(
	state: &mut MaintainerState,
	group: GroupId,
) -> Result<Vec<(GroupId, u64)>, GroupId> {
	let mut lineage = Vec::new();
	let mut current = group;
	for _ in 0..=u8::MAX {
		let entry = state.groups.entry(current);
		if !entry.read_done {
			return Err(current);
		}
		let Some(base) = entry
			.record
			.as_ref()
			.and_then(|record| record.base.as_ref())
		else {
			return Ok(lineage);
		};
		lineage.push((base.parent, base.fork_number));
		current = base.parent;
	}
	tracing::warn!(group = ?uuid::Uuid::from_bytes(group), "a group's lineage does not end at a root");
	Err(group)
}

/// What an open of a grouped volume with no record does before creating anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LazyFork {
	/// The volume is outside any group, or its group is a root: nothing to fork from.
	None,
	/// This group, the volume's own or an ancestor, is still to be read; the open waits for
	/// the read.
	Pending(GroupId),
	/// The fork from the nearest ancestor to start.
	Spec(FsForkSpec),
}

/// The deterministic operation id of the lazy fork of `key` as of fork `fork_number`: retried
/// and concurrent opens converge on one job.
fn lazy_fork_operation_id(key: &depot_types::VolumeKey, fork_number: u64) -> [u8; 16] {
	let mut input = Vec::with_capacity(key.partition_key.len() + 64);
	input.extend_from_slice(&key.partition_key);
	input.push(0);
	if let Some(name) = &key.name {
		input.extend_from_slice(name);
	}
	input.push(0);
	input.extend_from_slice(&fork_number.to_be_bytes());
	xxhash_rust::xxh3::xxh3_128(&input).to_be_bytes()
}

/// The spec of a lazy fork of `key` from the lineage level `level`, counted from the nearest
/// ancestor.
fn lazy_fork_spec_at(
	state: &MaintainerState,
	key: &depot_types::VolumeKey,
	lineage: &[(GroupId, u64)],
	level: usize,
) -> Option<FsForkSpec> {
	let (ancestor, fork_number) = *lineage.get(level)?;
	let parent_volume_key = depot_types::VolumeKey {
		partition_key: depot_types::keys::partition::in_group(&key.partition_key, ancestor).into(),
		name: key.name.clone(),
	};
	Some(FsForkSpec {
		operation_id: lazy_fork_operation_id(key, fork_number),
		parent_volume_key,
		parent_region: state.config.local_region,
		destination_volume_key: key.clone(),
		destination_region: state.config.local_region,
		selector: FsForkSelector::FsForkGroup(FsForkGroup { fork_number }),
		minimum: None,
		restore: None,
	})
}

/// The lazy fork an open of `key` starts, from the nearest ancestor.
pub(crate) fn lazy_fork(state: &mut MaintainerState, key: &depot_types::VolumeKey) -> LazyFork {
	let Some(group) = depot_types::keys::partition::forkable_group(&key.partition_key) else {
		return LazyFork::None;
	};
	let lineage = match lineage(state, group) {
		Ok(lineage) => lineage,
		Err(unread) => return LazyFork::Pending(unread),
	};
	match lazy_fork_spec_at(state, key, &lineage, 0) {
		Some(spec) => LazyFork::Spec(spec),
		None => LazyFork::None,
	}
}

/// Where a lazy walk goes after an ancestor said nothing about the fork.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NextLazyFork {
	/// The fork from the next ancestor up.
	Next(FsForkSpec),
	/// The ancestor asked was the root: the volume never existed in the lineage.
	Root,
	/// The lineage is not in the cache, being read or dropped; the walk waits for it.
	Unknown,
}

/// The step after the ancestor `spec` asks, which said nothing about the fork.
pub(crate) fn next_lazy_fork(
	state: &mut MaintainerState,
	spec: &FsForkSpec,
	fx: &mut Effects,
) -> NextLazyFork {
	let key = &spec.destination_volume_key;
	let (Some(group), Some(current)) = (
		depot_types::keys::partition::forkable_group(&key.partition_key),
		depot_types::keys::partition::forkable_group(&spec.parent_volume_key.partition_key),
	) else {
		return NextLazyFork::Root;
	};
	let Ok(lineage) = lineage(state, group) else {
		drive(state, fx);
		return NextLazyFork::Unknown;
	};
	let Some(level) = lineage
		.iter()
		.position(|(ancestor, _)| *ancestor == current)
	else {
		return NextLazyFork::Unknown;
	};
	match lazy_fork_spec_at(state, key, &lineage, level + 1) {
		Some(next) => NextLazyFork::Next(next),
		None => NextLazyFork::Root,
	}
}

/// A grouped volume registered: its group gets an entry, read on the next drive.
pub(crate) fn volume_registered(state: &mut MaintainerState, key: &VolumeLogKey) {
	let Some(group) = group_of(key) else {
		return;
	};
	note_pin_floor(state, key);
	let entry = state.groups.entry(group);
	entry.loaded.insert(key.clone());
	// A base volume's pins have had no pass the cache counts until one completes.
	if depot_types::is_claim_log(&key.0.volume_key).is_none() && !entry.passed.contains_key(key) {
		set_passed(entry, key, 0);
	}
	// An open admitted before the cache knew the group archived may finish its load after the
	// archive closed the group's registered volumes. The volume is closed as that archive
	// would have closed it, so its writer is fenced instead of committing in an archived group.
	if entry_archived(entry) == Some(true) {
		state.groups.archive_closes.insert(key.clone());
		state.groups.archive_fences.insert(key.clone());
	}
	// An entry that existed only for a pin wait or for alarms now holds this volume's apply,
	// so its read moves to the first lane.
	state.groups.queue_read(group);
}

/// A grouped volume left memory: its group's entry leaves with its last volume. The drive at
/// the end offers held volumes the dropped entries' wanted forks made room for and starts reads
/// that wait for a slot.
pub(crate) fn volume_forgotten(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	state.groups.pin_floors.remove(key);
	let Some(group) = group_of(key) else {
		return;
	};
	let Some(entry) = state.groups.entries.get_mut(&group) else {
		return;
	};
	entry.loaded.remove(key);
	entry.held.remove(key);
	unset_passed(entry, key);
	let unloaded = entry.loaded.is_empty();
	state.groups.archive_fences.remove(key);
	state.groups.archive_closes.remove(key);
	if let Some(children) = entry.pin_waits.remove(key) {
		state.groups.pin_wait_children -= children.len();
		let released = release_wait_refs(&mut state.groups, children);
		drop_released_children(state, released);
	}
	// The volume may have been the last that had not had a pass apply an archived child.
	prune_archived(&mut state.groups, group);
	if unloaded {
		state.groups.unswept.insert(group);
		let mut unswept = std::mem::take(&mut state.groups.unswept);
		unswept.extend(state.awaiting_lazy_fork.take_released_groups());
		sweep(state, unswept);
	}
	drive(state, fx);
}

/// Sets `key`'s value in `entry.passed`, keeping `passed_counts`.
fn set_passed(entry: &mut GroupEntry, key: &VolumeLogKey, through: u64) {
	unset_passed(entry, key);
	entry.passed.insert(key.clone(), through);
	*entry.passed_counts.entry(through).or_default() += 1;
}

/// Removes `key` from `entry.passed`, keeping `passed_counts`.
fn unset_passed(entry: &mut GroupEntry, key: &VolumeLogKey) {
	let Some(previous) = entry.passed.remove(key) else {
		return;
	};
	if let Some(count) = entry.passed_counts.get_mut(&previous) {
		*count -= 1;
		if *count == 0 {
			entry.passed_counts.remove(&previous);
		}
	}
}

/// Caches `child` as the child of fork `fork_number` of `group`'s entry, keeping a child the
/// entry already caches for that fork, and marks it archived when `archived`. A child newly
/// marked archived takes the next archive order.
fn cache_child(
	groups: &mut Groups,
	group: GroupId,
	fork_number: u64,
	child: GroupId,
	archived: bool,
) {
	let Some(entry) = groups.entries.get_mut(&group) else {
		return;
	};
	let cached = entry
		.fork_children
		.entry(fork_number)
		.or_insert(CachedChild {
			child,
			archived: false,
		});
	let newly_archived = archived && !cached.archived;
	cached.archived |= archived;
	let child = cached.child;
	if newly_archived {
		entry.last_archive_seq += 1;
		entry
			.archived_order
			.insert(entry.last_archive_seq, fork_number);
	}
	groups.child_forks.insert(child, (group, fork_number));
}

/// Drops from `group`'s entry the archived children that a pass applied to every registered
/// base volume of the group, each of whose pins covers the child's fork, so no pin and no pass
/// needs them again. A fork at or above the entry's `idle_pin_floor` stays, since an idle
/// parent's acquire plans its pin with those. A volume registered later learns a dropped
/// child's status from the child's own record when its load reports the pins waiting on it.
fn prune_archived(groups: &mut Groups, group: GroupId) {
	let Some(entry) = groups.entries.get_mut(&group) else {
		return;
	};
	// With no registered base volume, the entry serves only idle parents and lineage walks,
	// and leaves with its last use.
	let Some((&through, _)) = entry.passed_counts.first_key_value() else {
		return;
	};
	let floor = entry.idle_pin_floor;
	let applied: Vec<(u64, u64)> = entry
		.archived_order
		.range(..=through)
		.map(|(seq, fork)| (*seq, *fork))
		.filter(|(_, fork)| floor.is_none_or(|floor| *fork < floor))
		.collect();
	for (seq, fork) in applied {
		entry.archived_order.remove(&seq);
		let Some(child) = entry.fork_children.remove(&fork) else {
			continue;
		};
		if groups.child_forks.get(&child.child) == Some(&(group, fork)) {
			groups.child_forks.remove(&child.child);
		}
	}
}

/// Records pass `op_id` over a volume's fork pins, which carries the archived children of the
/// volume's group past its `passed`, so its completion can advance that.
pub(crate) fn pass_started(state: &mut MaintainerState, op_id: OpId, key: &VolumeLogKey) {
	let Some(group) = group_of(key) else {
		return;
	};
	let Some(entry) = state.groups.entries.get(&group) else {
		return;
	};
	let through = entry.last_archive_seq;
	state.groups.archive_passes.insert(
		op_id,
		ArchivePass {
			key: key.clone(),
			group,
			through,
		},
	);
}

/// Takes the record of pass `op_id`, before its operation finishes, so a completion handler
/// can apply it once it knows the pass succeeded.
pub(crate) fn take_pass(state: &mut MaintainerState, op_id: OpId) -> Option<ArchivePass> {
	state.groups.archive_passes.remove(&op_id)
}

/// Pass `pass` completed: every archived child it carried is dropped from its volume's pins.
/// The volume's `passed` advances through them, short of the first whose fork its pin does not
/// cover yet, since the next pin of the volume names that fork's child; then the children
/// every registered volume has had applied leave the cache.
pub(crate) fn pass_applied(state: &mut MaintainerState, pass: Option<ArchivePass>) {
	let Some(pass) = pass else {
		return;
	};
	let pinned = state
		.filesystem
		.volumes
		.get(&pass.key)
		.map_or(0, |volume| pinned_through(&volume.record));
	let Some(entry) = state.groups.entries.get_mut(&pass.group) else {
		return;
	};
	let Some(previous) = entry.passed.get(&pass.key).copied() else {
		return;
	};
	if pass.through <= previous {
		return;
	}
	let mut covered = pass.through;
	for (seq, fork) in entry.archived_order.range(previous + 1..=pass.through) {
		if *fork > pinned {
			covered = seq - 1;
			break;
		}
	}
	if covered > previous {
		set_passed(entry, &pass.key, covered);
		prune_archived(&mut state.groups, pass.group);
	}
}

/// Checks that the tablet-wide counts match the entries they sum.
pub(crate) fn validate(state: &MaintainerState) -> anyhow::Result<()> {
	let wanted: usize = state
		.groups
		.entries
		.values()
		.map(|entry| entry.wanted.len())
		.sum();
	let waiting: usize = state
		.groups
		.entries
		.values()
		.flat_map(|entry| entry.pin_waits.values())
		.chain(state.groups.deleted_pins.values().map(|wait| &wait.waiting))
		.map(BTreeSet::len)
		.sum();
	anyhow::ensure!(
		wanted == state.groups.wanted_forks,
		"group cache wanted forks count {} but entries hold {wanted}",
		state.groups.wanted_forks
	);
	anyhow::ensure!(
		waiting == state.groups.pin_wait_children,
		"group cache pin wait children count {} but entries hold {waiting}",
		state.groups.pin_wait_children
	);
	let mut refs: BTreeMap<GroupId, u32> = BTreeMap::new();
	for children in state
		.groups
		.entries
		.values()
		.flat_map(|entry| entry.pin_waits.values())
		.chain(state.groups.deleted_pins.values().map(|wait| &wait.waiting))
	{
		for child in children {
			*refs.entry(*child).or_default() += 1;
		}
	}
	anyhow::ensure!(
		refs == state.groups.wait_refs,
		"group cache pin wait references do not match the waits that name them"
	);
	let mut scopes: BTreeMap<GroupId, BTreeSet<Scope>> = BTreeMap::new();
	for (scope, wait) in &state.groups.deleted_pins {
		for child in &wait.waiting {
			scopes.entry(*child).or_default().insert(scope.clone());
		}
	}
	anyhow::ensure!(
		scopes == state.groups.deleted_pin_scopes,
		"the deleted incarnations' pin wait index does not match their waits"
	);
	for key in &state.groups.archive_fences {
		anyhow::ensure!(
			group_of(key)
				.and_then(|group| state.groups.entries.get(&group))
				.is_some_and(|entry| entry.loaded.contains(key)),
			"an archive fence names a volume its group's entry does not hold registered"
		);
	}
	for key in &state.groups.archive_closes {
		anyhow::ensure!(
			state.groups.archive_fences.contains(key),
			"an archive close names a volume with no archive fence"
		);
	}
	let mut acquires: BTreeMap<GroupId, u32> = BTreeMap::new();
	for pending in state.pending_fork_references.values() {
		if let Some(group) = acquire_group(pending.request()) {
			*acquires.entry(group).or_default() += 1;
		}
	}
	anyhow::ensure!(
		acquires == state.groups.acquire_refs,
		"group cache acquire references do not match the pending group fork acquires"
	);
	for (child, (parent, fork)) in &state.groups.child_forks {
		anyhow::ensure!(
			state
				.groups
				.entries
				.get(parent)
				.and_then(|entry| entry.fork_children.get(fork))
				.is_some_and(|cached| cached.child == *child),
			"the group cache's child index names a fork child its entry does not hold"
		);
	}
	for (parent, children) in &state.groups.based_on {
		anyhow::ensure!(
			!children.is_empty(),
			"the group cache indexes a parent with no children"
		);
		for child in children {
			anyhow::ensure!(
				state.groups.entries.get(child).and_then(base_parent) == Some(*parent),
				"the group cache's base index names a child whose record does not name its parent"
			);
		}
	}
	for entry in state.groups.entries.values() {
		let counted: u32 = entry.passed_counts.values().sum();
		anyhow::ensure!(
			counted as usize == entry.passed.len(),
			"a group entry's pass counts do not match its registered volumes"
		);
		for fork in entry.archived_order.values() {
			anyhow::ensure!(
				entry
					.fork_children
					.get(fork)
					.is_some_and(|child| child.archived),
				"a group entry orders an archived child it does not hold archived"
			);
		}
	}
	for (group, _) in state.awaiting_lazy_fork.group_reads() {
		anyhow::ensure!(
			state.groups.entries.contains_key(group),
			"an open waits for the read of a group the cache has no entry for"
		);
	}
	let reads = state
		.operations
		.values()
		.filter(|operation| matches!(operation, Operation::GroupRead { .. }))
		.count();
	anyhow::ensure!(
		reads == state.groups.reads_in_flight,
		"group cache reads in flight count {} but {reads} group read operations are open",
		state.groups.reads_in_flight
	);
	let queue = &state.groups.read_queue;
	for (lane, items) in [
		(Lane::First, &queue.first),
		(Lane::Background, &queue.background),
	] {
		for (seq, group) in items {
			let queued = state
				.groups
				.entries
				.get(group)
				.and_then(|entry| match lane {
					Lane::First => entry.queued_first,
					Lane::Background => entry.queued_background,
				});
			anyhow::ensure!(
				queued == Some(*seq),
				"group read queue names an entry that does not record its place"
			);
		}
	}
	for (group, entry) in &state.groups.entries {
		let in_first = entry
			.queued_first
			.is_some_and(|seq| queue.first.get(&seq) == Some(group));
		let in_background = entry
			.queued_background
			.is_some_and(|seq| queue.background.get(&seq) == Some(group));
		anyhow::ensure!(
			entry.queued_first.is_none() || in_first,
			"a group entry records a place in the first read lane that the lane lacks"
		);
		anyhow::ensure!(
			entry.queued_background.is_none() || in_background,
			"a group entry records a place in the background read lane that the lane lacks"
		);
		anyhow::ensure!(
			!entry.due() || in_first || in_background,
			"a group entry is due a read but is not queued for one"
		);
		anyhow::ensure!(
			!entry.gap_pending || entry.read.is_some(),
			"a group entry is marked for another read with no read in flight"
		);
	}
	Ok(())
}

/// Removes an entry and the counts and indexes it held. A read in flight keeps its slot until
/// it answers for the entry that is gone. Callers never remove an entry something still uses
/// (see [`in_use`]), so no open, alarm or acquire waits for the read of an entry removed here.
/// The entry's parent may now be unused, so it is checked at the next sweep.
///
/// The entry's own pin waits are released without dropping the children they free: an entry
/// leaves only with no loaded volume, and each volume's waits leave when it unregisters, so
/// they are empty here.
fn remove_entry(state: &mut MaintainerState, group: GroupId) {
	let groups = &mut state.groups;
	let Some(entry) = groups.entries.remove(&group) else {
		return;
	};
	groups.wanted_forks -= entry.wanted.len();
	groups.pin_wait_children -= entry.pin_waits.values().map(BTreeSet::len).sum::<usize>();
	groups.cut_short.remove(&group);
	groups.unswept.remove(&group);
	if let Some(seq) = entry.queued_first {
		groups.read_queue.first.remove(&seq);
	}
	if let Some(seq) = entry.queued_background {
		groups.read_queue.background.remove(&seq);
	}
	let parent = base_parent(&entry);
	for children in entry.pin_waits.into_values() {
		release_wait_refs(groups, children);
	}
	for (fork, child) in &entry.fork_children {
		if groups.child_forks.get(&child.child) == Some(&(group, *fork)) {
			groups.child_forks.remove(&child.child);
		}
	}
	if let Some(parent) = parent {
		if let Some(children) = groups.based_on.get_mut(&parent) {
			children.remove(&group);
			if children.is_empty() {
				groups.based_on.remove(&parent);
			}
		}
		groups.unswept.insert(parent);
	}
}

/// The group `entry`'s read record names as its base.
fn base_parent(entry: &GroupEntry) -> Option<GroupId> {
	entry
		.record
		.as_ref()
		.and_then(|record| record.base.as_ref())
		.map(|base| base.parent)
}

/// Whether anything still uses `group`'s entry: a loaded volume of the group, a pin wait that
/// names it, a pending acquire of one of its volumes, a cached child whose lineage it is, a
/// waiting open's lineage walk or fork stamp, or a due alarm. Each is an index lookup.
fn in_use(state: &MaintainerState, group: GroupId) -> bool {
	let Some(entry) = state.groups.entries.get(&group) else {
		return false;
	};
	!entry.loaded.is_empty()
		|| state.groups.wait_refs.contains_key(&group)
		|| state.groups.acquire_refs.contains_key(&group)
		|| state.groups.based_on.contains_key(&group)
		|| state.awaiting_lazy_fork.uses_group(group)
		|| super::alarms::keeps_entry(state, group)
}

/// Drops the entries of `groups` that nothing uses any more, and each ancestor that only a
/// dropped entry used, walking up from each dropped one. Costs the groups given plus the
/// ancestors dropped, never a scan of the entries.
fn sweep(state: &mut MaintainerState, groups: BTreeSet<GroupId>) {
	let mut pending = groups;
	while let Some(group) = pending.pop_first() {
		if !state.groups.entries.contains_key(&group) || in_use(state, group) {
			continue;
		}
		let parent = state.groups.entries.get(&group).and_then(base_parent);
		remove_entry(state, group);
		if let Some(parent) = parent {
			pending.insert(parent);
		}
	}
}

/// An admitted group fork acquire of `request`'s parent group keeps that group's entry until
/// it is answered; see `Groups::acquire_refs`.
pub(crate) fn acquire_admitted(
	state: &mut MaintainerState,
	request: &rivet_data::generated::depot_client_protocol_v2::FsForkReferenceRequest,
) {
	if let Some(group) = acquire_group(request) {
		*state.groups.acquire_refs.entry(group).or_default() += 1;
	}
}

/// A group fork acquire of `request` was answered: its group's entry no longer stays for it,
/// and leaves at the next sweep if nothing else uses it.
pub(crate) fn acquire_ended(
	state: &mut MaintainerState,
	request: &rivet_data::generated::depot_client_protocol_v2::FsForkReferenceRequest,
) {
	let Some(group) = acquire_group(request) else {
		return;
	};
	let Some(count) = state.groups.acquire_refs.get_mut(&group) else {
		return;
	};
	*count -= 1;
	if *count == 0 {
		state.groups.acquire_refs.remove(&group);
		state.groups.unswept.insert(group);
	}
}

/// The parent group of a group fork acquire, or `None` for any other reference request.
fn acquire_group(
	request: &rivet_data::generated::depot_client_protocol_v2::FsForkReferenceRequest,
) -> Option<GroupId> {
	use rivet_data::generated::depot_client_protocol_v2::FsForkReferenceRequest;
	let FsForkReferenceRequest::FsForkAcquire(acquire) = request else {
		return None;
	};
	if !matches!(acquire.spec.selector, FsForkSelector::FsForkGroup(_)) {
		return None;
	}
	depot_types::keys::partition::forkable_group(&acquire.spec.parent_volume_key.partition_key)
}

/// Counts `children` as named by one more volume's pin wait.
fn add_wait_refs(groups: &mut Groups, children: &BTreeSet<GroupId>) {
	for child in children {
		*groups.wait_refs.entry(*child).or_default() += 1;
	}
}

/// Counts `children` as named by one fewer volume's pin wait, and returns the ones no wait
/// names any more.
fn release_wait_refs(
	groups: &mut Groups,
	children: impl IntoIterator<Item = GroupId>,
) -> Vec<GroupId> {
	let mut released = Vec::new();
	for child in children {
		let Some(count) = groups.wait_refs.get_mut(&child) else {
			continue;
		};
		*count -= 1;
		if *count == 0 {
			groups.wait_refs.remove(&child);
			released.push(child);
		}
	}
	released
}

/// Drops the entries of `children`, which no pin wait names any more, unless something else
/// still uses them. Such an entry was made to learn the child's status, or a lineage walk made
/// it; dropping one a lineage walk still uses costs that walk one record read again, never a
/// wrong answer, since every entry is read before it is used.
fn drop_released_children(state: &mut MaintainerState, children: Vec<GroupId>) {
	if children.is_empty() {
		return;
	}
	sweep(state, children.into_iter().collect());
}

/// Adds the forks of `forks` that the cache lacks to `group`'s wanted forks, until the
/// tablet's `max_group_wanted_forks`. A caller that needed the rest asks again once reads
/// drain these: an acquire is retried, and a held apply of an entry cut short is offered again
/// by [`offer_cut_short`] once the tablet wants fewer forks than the cap, since its own entry
/// may have no read coming.
fn want(state: &mut MaintainerState, group: GroupId, forks: impl Iterator<Item = u64>) {
	let cap = state.config.max_group_wanted_forks as usize;
	let groups = &mut state.groups;
	let Some(entry) = groups.entries.get_mut(&group) else {
		return;
	};
	let mut added = false;
	for fork in forks {
		if entry.fork_children.contains_key(&fork) || entry.wanted.contains(&fork) {
			continue;
		}
		if groups.wanted_forks >= cap {
			// Logged once per entry each time it is cut short, not on every retried acquire.
			if groups.cut_short.insert(group) {
				tracing::debug!(
					group = ?uuid::Uuid::from_bytes(group),
					cap,
					"the group cache wants depot.groups.max_wanted_forks forks; the rest are asked for after reads drain them"
				);
			}
			break;
		}
		entry.wanted.insert(fork);
		groups.wanted_forks += 1;
		added = true;
	}
	if added {
		groups.queue_read(group);
	}
}

/// Offers the held volumes of the entries cut short at `max_group_wanted_forks` for apply
/// again once the tablet wants fewer forks than the cap. Each apply decides anew and holds
/// again, wanting what now fits, so an entry cut short again goes back into the set. Does
/// nothing while the cap stays full, so it never loops reads or holds.
fn offer_cut_short(state: &mut MaintainerState) {
	if state.groups.cut_short.is_empty()
		|| state.groups.wanted_forks >= state.config.max_group_wanted_forks as usize
	{
		return;
	}
	for group in std::mem::take(&mut state.groups.cut_short) {
		let Some(entry) = state.groups.entries.get_mut(&group) else {
			continue;
		};
		let held = std::mem::take(&mut entry.held);
		for key in held {
			super::filesystem::mark_candidate(state, &key);
		}
	}
}

/// Removes `fork` from `entry`'s wanted forks and from the tablet's count.
fn unwant(wanted_forks: &mut usize, entry: &mut GroupEntry, fork: u64) {
	if entry.wanted.remove(&fork) {
		*wanted_forks -= 1;
	}
}

/// Offers the volumes of entries cut short again when there is room, and starts the queued
/// record reads, up to `max_group_reads_in_flight` in flight. Entries the cache made only to
/// learn a waited child's status or to answer due alarms read after the others, so the reads
/// that hold applies and opens are not starved behind a load's children or an alarm burst,
/// but one in every `READS_BEFORE_BACKGROUND_READ + 1` starts goes to them while both are
/// queued, so a steady stream of the others cannot delay them without bound. The rest start
/// as reads finish, since every completion drives again. Each call costs the reads it starts plus the queued
/// entries it skips because they stopped being due, never a scan of the entries.
///
/// A group volume that stays unreachable keeps its entry's wanted forks through every
/// backoff, so at `max_group_wanted_forks` it holds back the pins of other groups' volumes
/// until it answers or its last volume unregisters.
pub(crate) fn drive(state: &mut MaintainerState, fx: &mut Effects) {
	offer_cut_short(state);
	while state.groups.reads_in_flight < state.config.max_group_reads_in_flight as usize {
		let Some((group, lane)) = state.groups.pop_queued() else {
			break;
		};
		let due = state
			.groups
			.entries
			.get(&group)
			.is_some_and(GroupEntry::due);
		if !due {
			continue;
		}
		read(state, group, fx);
		let queue = &mut state.groups.read_queue;
		match lane {
			Lane::First => queue.first_streak = queue.first_streak.saturating_add(1),
			Lane::Background => queue.first_streak = 0,
		}
	}
}

/// Starts `group`'s read, taking a slot of `max_group_reads_in_flight`. Only `drive` calls
/// this, for a due entry while a slot is free.
fn read(state: &mut MaintainerState, group: GroupId, fx: &mut Effects) {
	let Some(entry) = state.groups.entries.get(&group) else {
		return;
	};
	let fork_numbers: Vec<u64> = entry
		.wanted
		.iter()
		.copied()
		.take(GROUP_READ_MAX_CHILDREN)
		.collect();
	let op_id = state.begin(
		Operation::GroupRead {
			group,
			fork_numbers: fork_numbers.clone(),
		},
		fx,
	);
	state.groups.reads_in_flight += 1;
	if let Some(entry) = state.groups.entries.get_mut(&group) {
		entry.read = Some(op_id);
	}
	fx.execute(MaintainerEffect::GroupReadRequest {
		op_id,
		group,
		fork_numbers,
	});
}

/// A fenced lifetime took group read `op_id` of `group` with every other operation, so it never
/// answers: its slot frees, and its entry no longer waits on it and is queued to read again,
/// as an entry with no read in flight is, so the cache's count of reads in flight keeps matching
/// the open operations.
pub(crate) fn read_revoked(state: &mut MaintainerState, op_id: OpId, group: GroupId) {
	state.groups.reads_in_flight = state.groups.reads_in_flight.saturating_sub(1);
	let Some(entry) = state.groups.entries.get_mut(&group) else {
		return;
	};
	if entry.read != Some(op_id) {
		return;
	}
	entry.read = None;
	// An event that marked the entry for another read while this one was in flight leaves it
	// unread, as the read's own completion would have.
	if std::mem::take(&mut entry.gap_pending) {
		entry.read_done = false;
	}
	state.groups.queue_read(group);
}

/// A group read finished. The record and the rows it carried join the entry, the volumes the
/// entry held are offered for apply again, and a failed read, or one that found no record in a
/// lagging snapshot, is retried after the retry delay.
pub(crate) fn read_completed(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<GroupReadResponse, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&op_id),
		Some(Operation::GroupRead { .. })
	) {
		return;
	}
	let Some(Operation::GroupRead {
		group,
		fork_numbers: asked,
	}) = state.finish(op_id, fx)
	else {
		unreachable!("checked above")
	};
	// The store answered, or the operation timed out, so the read's slot frees whether or not
	// its entry is still cached.
	state.groups.reads_in_flight -= 1;
	// A snapshot behind the group volume's commits, as inside a close-notice window, may lack
	// a record already written, so finding none there proves nothing. Such an answer is a
	// failed read: it keeps the held volumes, the asked forks and the lineage walk waiting and
	// retries after the backoff, rather than ending the walk at a group that may exist.
	let lagging_without_record = matches!(
		&result,
		Ok(response) if response.record.is_none() && response.commit_lag > 0
	);
	let result = match result {
		Ok(response) if lagging_without_record => Err(format!(
			"the group record read found no record in a snapshot lagging its commits by {}",
			response.commit_lag
		)),
		other => other,
	};
	// Children an archive event named, which this read's rows may predate.
	let archived_seen: BTreeSet<GroupId> = match &result {
		Ok(response) => response
			.children
			.iter()
			.map(|row| row.child.child)
			.filter(|child| {
				state
					.groups
					.entries
					.get(child)
					.is_some_and(|entry| entry.archived_seen)
			})
			.collect(),
		Err(_) => BTreeSet::new(),
	};
	let groups = &mut state.groups;
	let Some(entry) = groups.entries.get_mut(&group) else {
		// The entry left with its last volume; opens waiting for it walk their lineage again,
		// which reads it anew. The freed slot starts the next queued read.
		super::lazy_fork_waiters::group_read_ended(state, group);
		super::alarms::group_read_ended(state, group, fx);
		drive(state, fx);
		return;
	};
	if entry.read != Some(op_id) {
		// The entry left and came back while this read was in flight; its own read answers
		// for it. The freed slot starts the next queued read.
		drive(state, fx);
		return;
	}
	entry.read = None;
	// A failed read leaves `read_done` as it was, and its retry reads after any event anyway.
	let reread = std::mem::take(&mut entry.gap_pending);
	let response = match result {
		Ok(response) => response,
		Err(error) => {
			let label = if lagging_without_record {
				"lagging"
			} else {
				"failed"
			};
			crate::metrics::GROUP_READS
				.with_label_values(&[label])
				.inc();
			// Each failure in a row doubles the wait up to a cap, and only the first failure
			// and the one that reaches the cap warn, so a read that keeps failing neither
			// retries every `retry_ms` nor floods the log.
			entry.failed_reads = entry.failed_reads.saturating_add(1);
			// A lagging answer ends with its group volume's close-notice window, so its retry
			// starts at that window rather than at the longer retry for an unreachable volume:
			// opens of the group's absent volumes wait for this read.
			let retry_ms = if lagging_without_record {
				CLOSE_NOTICE_WAIT_MS.min(state.config.tuning().retry_ms)
			} else {
				state.config.tuning().retry_ms
			};
			let backoff_ms = |failures: u32| {
				retry_ms
					.saturating_mul(1u64 << failures.saturating_sub(1).min(16))
					.min(GROUP_READ_MAX_BACKOFF_MS.max(retry_ms))
			};
			let delay_ms = backoff_ms(entry.failed_reads);
			let reached_cap =
				entry.failed_reads > 1 && delay_ms > backoff_ms(entry.failed_reads - 1);
			// A lagging answer is expected for one close-notice window, so only one that keeps
			// lagging up to the cap warns.
			let warn_first = entry.failed_reads == 1 && !lagging_without_record;
			if warn_first || (reached_cap && delay_ms >= GROUP_READ_MAX_BACKOFF_MS) {
				tracing::warn!(
					%error,
					group = ?uuid::Uuid::from_bytes(group),
					failed_reads = entry.failed_reads,
					delay_ms,
					"a group record read failed, likely an unreachable group volume; the group's volumes apply after the retry"
				);
			} else {
				tracing::debug!(
					%error,
					group = ?uuid::Uuid::from_bytes(group),
					failed_reads = entry.failed_reads,
					delay_ms,
					"a group record read failed again"
				);
			}
			entry.retry_armed = true;
			fx.arm_timer_after(MaintainerTimer::GroupReadRetry(group), delay_ms);
			drive(state, fx);
			return;
		}
	};
	crate::metrics::GROUP_READS.with_label_values(&["ok"]).inc();
	entry.failed_reads = 0;
	if let Some(record) = &response.record {
		entry.latest_fork = entry.latest_fork.max(record.latest_fork);
	}
	// What the cache learned only moves forward: an event applied while the read was in flight
	// may be newer than the read, and an archive is final, whether the event found the record
	// in the cache or arrived before it.
	let archived = entry.archived_seen
		|| entry.record.as_ref().is_some_and(|record| {
			record.status == rivet_data::generated::depot_group_v1::GroupStatus::Archived
		});
	// A read that finds the group archived when no event said so, as after a gap, closes its
	// volumes as the event would have.
	let archived_by_read = !archived
		&& response.record.as_ref().is_some_and(|record| {
			record.status == rivet_data::generated::depot_group_v1::GroupStatus::Archived
		});
	match response.record {
		Some(mut record) => {
			if archived {
				record.status = rivet_data::generated::depot_group_v1::GroupStatus::Archived;
			}
			record.latest_fork = record.latest_fork.max(entry.latest_fork);
			entry.record = Some(record);
		}
		None if entry.record.is_some() => {}
		None => entry.record = None,
	}
	// An event that marked the entry for another read while this read was in flight may be
	// newer than this read's snapshot, so the entry stays unread and is read again.
	entry.read_done = !reread;
	let base = base_parent(entry);
	let mut rows = Vec::with_capacity(response.children.len());
	for row in response.children {
		unwant(&mut groups.wanted_forks, entry, row.fork_number);
		let archived = row.child.archived || archived_seen.contains(&row.child.child);
		rows.push((row.fork_number, row.child.child, archived));
	}
	// A fork number the volume does not hold a row for was asked for in error, or its fork is
	// still being drawn; asking again would loop, so it is dropped and asked for again by the
	// next pin that needs it. Forks this read did not ask for, past one read's bound, stay
	// wanted and the next drive reads them.
	for fork in &asked {
		unwant(&mut groups.wanted_forks, entry, *fork);
	}
	// An entry read again stays unread, so the volumes held for it and the opens waiting for
	// it would only wait again; they are released after the next read instead.
	let held = if reread {
		BTreeSet::new()
	} else {
		std::mem::take(&mut entry.held)
	};
	// An entry with no loaded volume exists for a lineage walk, which goes on to the next
	// ancestor as each record arrives rather than one level per retried open. An alarm-only
	// entry needs only its own record.
	let next_ancestor = if entry.loaded.is_empty() && !entry.alarm_only {
		entry
			.record
			.as_ref()
			.and_then(|record| record.base.as_ref())
			.map(|base| base.parent)
	} else {
		None
	};
	for (fork_number, child, archived) in rows {
		cache_child(&mut state.groups, group, fork_number, child, archived);
	}
	// The record's base is fixed, so a group is indexed under its parent once.
	if let Some(parent) = base {
		state
			.groups
			.based_on
			.entry(parent)
			.or_default()
			.insert(group);
	}
	if let Some(parent) = next_ancestor {
		state.groups.entry(parent);
	}
	for key in held {
		super::filesystem::mark_candidate(state, &key);
	}
	// Forks past this read's bound stay wanted and are read next.
	state.groups.queue_read(group);
	settle_pin_waits(state, group);
	match child_learned(state, group) {
		Some(parent) => settle_pin_waits(state, parent),
		// A waited child whose read found no record cannot name its parent; the waits that
		// name it settle with it counted live.
		None if state.groups.wait_refs.contains_key(&group) => settle_waits_naming(state, group),
		None => {}
	}
	if archived_by_read {
		close_archived(state, group, fx);
	}
	if state.groups.deleted_pin_scopes.contains_key(&group)
		&& child_status(&state.groups.entries, &BTreeMap::new(), group) == Some(true)
	{
		deleted_child_archived(state, group);
	}
	if !reread {
		super::lazy_fork_waiters::group_read_ended(state, group);
		super::alarms::group_read_ended(state, group, fx);
		super::fork_references::pins_progressed(state, None, fx);
	}
	drive(state, fx);
}

/// Settles the pin waits of every entry whose waits name `child`. Only for a child whose
/// record does not say which parent it left, which is rare, so the scan over the entries with
/// waits stays off the common path.
fn settle_waits_naming(state: &mut MaintainerState, child: GroupId) {
	let parents: Vec<GroupId> = state
		.groups
		.entries
		.iter()
		.filter(|(_, entry)| {
			entry
				.pin_waits
				.values()
				.any(|children| children.contains(&child))
		})
		.map(|(group, _)| *group)
		.collect();
	for parent in parents {
		settle_pin_waits(state, parent);
	}
}

/// Passes what the cache knows of `group`'s own record to its parent's entry, if the parent
/// is cached: the child of the fork `group` left its parent by, with its status. This is how
/// a child group read for a pin wait answers the wait, and it saves the parent a fork row
/// read. Returns the parent so its waits can settle.
fn child_learned(state: &mut MaintainerState, group: GroupId) -> Option<GroupId> {
	let record = state.groups.entries.get(&group)?.record.as_ref()?;
	let base = record.base.as_ref()?;
	let parent = base.parent;
	let fork_number = base.fork_number;
	let archived = record.status == rivet_data::generated::depot_group_v1::GroupStatus::Archived;
	let groups = &mut state.groups;
	let entry = groups.entries.get_mut(&parent)?;
	unwant(&mut groups.wanted_forks, entry, fork_number);
	cache_child(groups, parent, fork_number, group, archived);
	Some(parent)
}

/// A registered volume's load reported the child groups its kept fork pins still wait on,
/// and the fork ranges of those pins. The cache learns each one's status: from the fork
/// children it already holds, else by whichever costs fewer reads. Reading each child
/// group's own record costs one read per unknown child; reading the parent's fork child
/// rows over the pins' ranges costs one read per page of `GROUP_READ_MAX_CHILDREN` forks the
/// cache lacks, and each row carries its child's archived flag. So the fork rows are read
/// when the unknown children outnumber those pages, as a dense range of live previews does,
/// and the children's records otherwise, as a few waiters over many idle forks do. The fork
/// rows are read only when every fork the cache lacks fits under `max_group_wanted_forks`;
/// otherwise the children's records are read, so no child is left waiting on a range that was
/// cut short. Either way the reads are bounded in flight by `max_group_reads_in_flight`.
///
/// The volume's pin pass is queued whenever one of them is learned archived. This is how an
/// archive that happened while the tablet did not hold the group, after a move, a restart or
/// the group's last unload, reaches the volume's pins without waiting for its next pin.
///
/// Only a base volume reports: a claim log's load reads the same pin rows and reports
/// nothing, so the waits and passes of one volume are keyed by its base log alone. A load
/// that would take the tablet past `max_group_pin_wait_children` learns only what fits; its
/// other children stay waiting in the store until the volume's next pin or load.
pub(crate) fn pins_loaded(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	fork_pins: crate::tablet_maintainer_machine::types::LoadedForkPins,
	fx: &mut Effects,
) {
	let waiting = fork_pins.waiting;
	if waiting.is_empty() || depot_types::is_claim_log(&key.0.volume_key).is_some() {
		return;
	}
	let Some(group) = group_of(key) else {
		return;
	};
	let cap = state.config.max_group_pin_wait_children as usize;
	let groups = &mut state.groups;
	// Registration created the entry; a volume without one is not registered.
	let Some(entry) = groups.entries.get_mut(&group) else {
		return;
	};
	let previous = entry.pin_waits.remove(key);
	if let Some(previous) = &previous {
		groups.pin_wait_children -= previous.len();
	}
	let room = cap.saturating_sub(groups.pin_wait_children);
	if waiting.len() > room {
		tracing::debug!(
			?key,
			waiting = waiting.len(),
			room,
			"the group cache learns children for depot.groups.max_pin_wait_children waits; the rest wait for the volume's next pin or load"
		);
	}
	let children: BTreeSet<GroupId> = waiting.into_iter().take(room).collect();
	if !children.is_empty() {
		groups.pin_wait_children += children.len();
		entry.pin_waits.insert(key.clone(), children.clone());
		add_wait_refs(groups, &children);
	}
	// The previous wait's references go after the new ones are counted, so a child both name
	// keeps its entry.
	if let Some(previous) = previous {
		let released = release_wait_refs(&mut state.groups, previous);
		drop_released_children(state, released);
	}
	if children.is_empty() {
		// The released children's entries may have left with their wanted forks.
		drive(state, fx);
		return;
	}
	// A child group the cache already read, for a volume of its own, tells its parent's
	// entry now, so the pass finds it among the archived children.
	for child in &children {
		child_learned(state, *child);
	}
	settle_pin_waits(state, group);
	let Some(entry) = state.groups.entries.get(&group) else {
		drive(state, fx);
		return;
	};
	let still_unknown = entry.pin_waits.get(key).cloned().unwrap_or_default();
	if still_unknown.is_empty() {
		drive(state, fx);
		return;
	}
	let known: BTreeMap<GroupId, bool> = entry
		.fork_children
		.values()
		.map(|child| (child.child, child.archived))
		.collect();
	// The forks are counted from the ranges' lengths, never listed, so a pin over a million
	// idle forks costs the cache what it holds in that range, not a million steps. A fork is
	// either known or wanted, never both, so the forks the cache lacks less the wanted ones
	// are the ones `want` would add.
	let ranges = merged_ranges(&fork_pins.waiting_forks);
	let mut missing_forks: u64 = 0;
	let mut new_forks: u64 = 0;
	for (first, last) in &ranges {
		let len = (last - first).saturating_add(1);
		let known = entry.fork_children.range(*first..=*last).count() as u64;
		let wanted = entry.wanted.range(*first..=*last).count() as u64;
		let missing = len.saturating_sub(known);
		missing_forks = missing_forks.saturating_add(missing);
		new_forks = new_forks.saturating_add(missing.saturating_sub(wanted));
	}
	let pages = missing_forks.div_ceil(GROUP_READ_MAX_CHILDREN as u64);
	let wanted_room =
		(state.config.max_group_wanted_forks as usize).saturating_sub(state.groups.wanted_forks);
	if still_unknown.len() as u64 > pages && new_forks <= wanted_room as u64 {
		// Fewer reads by page: the parent's entry reads the fork rows, which settle the waits.
		// Only now are the missing forks listed, by stepping over the gaps between the known
		// ones, and `want` skips the wanted ones. There are at most `new_forks` plus the
		// entry's wanted forks, both within `max_group_wanted_forks`.
		let forks = forks_missing_from(&entry.fork_children, &ranges);
		want(state, group, forks.into_iter());
	} else {
		// Fewer reads by child, or the range does not fit: each child the parent's entry does
		// not know gets an entry of its own, whose record the next drive reads.
		for child in still_unknown {
			if child_status(&state.groups.entries, &known, child).is_none() {
				state.groups.entry(child);
			}
		}
	}
	drive(state, fx);
}

/// The forks of `ranges` that `fork_children` lacks, in order. Steps over each range's gaps
/// between the known forks, so it costs the known forks in the ranges plus the forks it
/// returns, not the ranges' lengths.
fn forks_missing_from(
	fork_children: &BTreeMap<u64, CachedChild>,
	ranges: &[(u64, u64)],
) -> Vec<u64> {
	let mut missing = Vec::new();
	for (first, last) in ranges {
		let mut next = Some(*first);
		for known in fork_children.range(*first..=*last).map(|(fork, _)| *fork) {
			if let Some(start) = next {
				missing.extend(start..known);
			}
			next = known.checked_add(1);
		}
		if let Some(start) = next {
			missing.extend(start..=*last);
		}
	}
	missing
}

/// `ranges` sorted by their first fork with overlapping and adjacent ranges joined, so each
/// fork is counted once. A range whose first fork is past its last is dropped.
fn merged_ranges(ranges: &[(u64, u64)]) -> Vec<(u64, u64)> {
	let mut sorted: Vec<(u64, u64)> = ranges
		.iter()
		.copied()
		.filter(|(first, last)| first <= last)
		.collect();
	sorted.sort_unstable();
	let mut merged: Vec<(u64, u64)> = Vec::with_capacity(sorted.len());
	for (first, last) in sorted {
		match merged.last_mut() {
			Some((_, previous_last)) if first <= previous_last.saturating_add(1) => {
				*previous_last = (*previous_last).max(last);
			}
			Some(_) | None => merged.push((first, last)),
		}
	}
	merged
}

/// The status of `child`, a fork child of `group`, as the cache knows it: `Some(true)` once
/// archived, `Some(false)` while known live, `None` while unknown. `known` maps the children of
/// `group`'s cached forks to their status.
///
/// A child whose own read found no record counts as live: a waited child was forked with a
/// record, so none means it is not written yet, and the read is not asked again. The wait
/// lets it go and the pin keeps waiting on it in the store, which keeps more than needed and
/// never less, until the volume's next pin or load reports it again.
fn child_status(
	entries: &BTreeMap<GroupId, GroupEntry>,
	known: &BTreeMap<GroupId, bool>,
	child: GroupId,
) -> Option<bool> {
	let from_record = entries
		.get(&child)
		.filter(|entry| entry.read_done)
		.map(|entry| {
			entry.archived_seen
				|| entry.record.as_ref().is_some_and(|record| {
					record.status == rivet_data::generated::depot_group_v1::GroupStatus::Archived
				})
		});
	match (known.get(&child).copied(), from_record) {
		(Some(archived), Some(from_record)) => Some(archived || from_record),
		(Some(archived), None) | (None, Some(archived)) => Some(archived),
		(None, None) => None,
	}
}

/// Settles the pin waits of `group`'s volumes with what the cache knows: every child whose
/// status is known leaves its volume's wait, a volume whose wait just learned an archived
/// child gets its pin pass queued, and a volume leaves with its last unknown child. Runs only
/// while the tablet serves, since a pass queued earlier could not start; the waits stay until
/// [`began_serving`] settles them.
fn settle_pin_waits(state: &mut MaintainerState, group: GroupId) {
	if state.phase != super::MaintainerPhase::Serving {
		return;
	}
	let Some(entry) = state.groups.entries.get_mut(&group) else {
		return;
	};
	if entry.pin_waits.is_empty() {
		return;
	}
	let mut waits = std::mem::take(&mut entry.pin_waits);
	let known: BTreeMap<GroupId, bool> = entry
		.fork_children
		.values()
		.map(|child| (child.child, child.archived))
		.collect();
	let entries = &state.groups.entries;
	let mut resolved = Vec::new();
	let mut passes = Vec::new();
	waits.retain(|key, children| {
		let mut archived = false;
		children.retain(|child| match child_status(entries, &known, *child) {
			Some(status) => {
				archived |= status;
				resolved.push(*child);
				false
			}
			None => true,
		});
		if archived {
			passes.push(key.clone());
		}
		!children.is_empty()
	});
	state.groups.pin_wait_children -= resolved.len();
	if let Some(entry) = state.groups.entries.get_mut(&group) {
		entry.pin_waits = waits;
	}
	let released = release_wait_refs(&mut state.groups, resolved);
	drop_released_children(state, released);
	for key in passes {
		let work = super::store::StoreWork::ForkPinPass { key };
		if !state.store_queue.contains(&work) {
			state.store_queue.push_back(work);
		}
	}
}

/// The tablet began serving: settles the pin waits that loads during its opening left, which
/// could queue no pass before. The drive at the end offers held volumes the dropped entries'
/// wanted forks made room for and starts reads that wait for a slot.
pub(crate) fn began_serving(state: &mut MaintainerState, fx: &mut Effects) {
	let groups: Vec<GroupId> = state
		.groups
		.entries
		.iter()
		.filter(|(_, entry)| !entry.pin_waits.is_empty())
		.map(|(group, _)| *group)
		.collect();
	for group in groups {
		settle_pin_waits(state, group);
	}
	drive(state, fx);
}

/// Starts a queued fork pin pass of a volume that is still registered, with the archived
/// children the cache knows now.
pub(crate) fn dispatch_pin_pass(state: &mut MaintainerState, key: VolumeLogKey, fx: &mut Effects) {
	if !state.filesystem.volumes.contains_key(&key) {
		return;
	}
	let archived_children = archived_children(state, &key);
	if archived_children.is_empty() {
		return;
	}
	let op_id = state.begin(Operation::ForkPinPass { key: key.clone() }, fx);
	pass_started(state, op_id, &key);
	fx.execute(MaintainerEffect::ForkPinPassRequest {
		op_id,
		key,
		archived_children,
	});
}

/// A fork pin pass finished. A failed pass changes nothing: the pins keep waiting, which keeps
/// more than needed and never less, and the volume's next pin or load runs the pass again.
pub(crate) fn pin_pass_completed(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<(), String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&op_id),
		Some(Operation::ForkPinPass { .. })
	) {
		return;
	}
	let pass = take_pass(state, op_id);
	let Some(Operation::ForkPinPass { key }) = state.finish(op_id, fx) else {
		unreachable!("checked above")
	};
	if result.is_ok() {
		pass_applied(state, pass);
	}
	if let Err(error) = result {
		tracing::warn!(
			%error,
			?key,
			"a fork pin pass failed, likely a store error; the pins keep waiting until the volume's next pin or load"
		);
	}
}

/// The retry of a failed read, if its entry is still wanted.
pub(crate) fn retry(state: &mut MaintainerState, group: GroupId, fx: &mut Effects) {
	if let Some(entry) = state.groups.entries.get_mut(&group) {
		entry.retry_armed = false;
		state.groups.queue_read(group);
		drive(state, fx);
	} else {
		// The entry left with its last volume; opens waiting for it walk their lineage again,
		// and alarms waiting for it ask for it again.
		super::lazy_fork_waiters::group_read_ended(state, group);
		super::alarms::group_read_ended(state, group, fx);
	}
}

/// Applies a page of events to the cached entries they name; others are ignored.
/// Events may have been missed: every cached group record is read again, since a missed fork
/// would leave a parent's writes unpinned and a missed archive would leave a group open.
/// An entry whose read is in flight is read again after it, since that read's snapshot may
/// predate the missed events.
pub(crate) fn gap(state: &mut MaintainerState, fx: &mut Effects) {
	for entry in state.groups.entries.values_mut() {
		entry.read_done = false;
		if entry.read.is_some() {
			entry.gap_pending = true;
		}
	}
	let groups: Vec<GroupId> = state.groups.entries.keys().copied().collect();
	for group in groups {
		state.groups.queue_read(group);
	}
	drive(state, fx);
}

/// Applies the group events the process polled. A fork event raises its parent's
/// `latest_fork`, which is how a tablet maintainer usually learns of a fork; see [`pin`] for
/// where that cuts each of the parent's volumes.
pub(crate) fn events(state: &mut MaintainerState, events: Vec<Event>, fx: &mut Effects) {
	for event in events {
		match event {
			Event::EventGroupForked(fork) => {
				// A child group read before its record was written cached as a root; its
				// fork's event says it has a base, so it is read again.
				if let Some(child) = state.groups.entries.get_mut(&fork.child)
					&& child.record.is_none()
				{
					child.read_done = false;
					if child.read.is_some() {
						child.gap_pending = true;
					}
					state.groups.queue_read(fork.child);
				}
				let groups = &mut state.groups;
				let Some(entry) = groups.entries.get_mut(&fork.parent) else {
					continue;
				};
				entry.latest_fork = entry.latest_fork.max(fork.fork_number);
				unwant(&mut groups.wanted_forks, entry, fork.fork_number);
				cache_child(groups, fork.parent, fork.fork_number, fork.child, false);
			}
			Event::EventGroupArchived(archive) => {
				// The one entry that caches the group as a fork child learns it is archived.
				if let Some((parent, fork_number)) =
					state.groups.child_forks.get(&archive.group).copied()
				{
					cache_child(&mut state.groups, parent, fork_number, archive.group, true);
				}
				// The archive is kept on the entry whether or not its record was read: a read
				// in flight, of the group's own record or of the fork rows a pin wait reads
				// for it, may answer from a snapshot before the archive, and
				// `read_completed` keeps the archive over it. A waited child with no entry
				// gets one, read by the next drive, so the archive outlives the wait's read
				// whichever answers first.
				let waited = state.groups.wait_refs.contains_key(&archive.group);
				// A cached entry is updated in place, so an alarm-only one stays in its lane.
				let entry = if state.groups.entries.contains_key(&archive.group) {
					state.groups.entries.get_mut(&archive.group)
				} else if waited {
					Some(state.groups.entry(archive.group))
				} else {
					None
				};
				if let Some(entry) = entry {
					entry.archived_seen = true;
					if let Some(record) = entry.record.as_mut() {
						record.status =
							rivet_data::generated::depot_group_v1::GroupStatus::Archived;
					}
				}
				close_archived(state, archive.group, fx);
				super::alarms::group_archived(state, archive.group, fx);
				deleted_child_archived(state, archive.group);
			}
		}
	}
	drive(state, fx);
}

/// Closes the registered volumes of `group`, which an archive event named. Admission refuses
/// every later open of the group's volumes; this closes what was already open, so nothing
/// keeps reading or committing in an archived group. The command host drops each volume's slot
/// and the store revokes its read sessions at once, and every volume's writer is fenced by a
/// recovery claim at a newer epoch, which the shards check on each append, as soon as a
/// archive fence slot is free ([`drive_archive_fences`]). A writer refused this way reopens and
/// its open is refused as archived: a refusal, never a recovery. Parked lazy fork jobs of the
/// group's volumes are dropped, since no open will re-drive them. O(registered volumes of the
/// group plus parked lazy fork jobs), only on the event.
fn close_archived(state: &mut MaintainerState, group: GroupId, fx: &mut Effects) {
	super::forks::group_archived(state, group, fx);
	let Some(entry) = state.groups.entries.get(&group) else {
		return;
	};
	if entry.loaded.is_empty() {
		return;
	}
	let keys: Vec<VolumeLogKey> = entry.loaded.iter().cloned().collect();
	crate::metrics::GROUP_ARCHIVE_CLOSES.inc_by(keys.len() as u64);
	state.groups.archive_fences.extend(keys.iter().cloned());
	fx.execute(MaintainerEffect::CloseArchivedVolumes { keys });
}

/// Closes the volumes that registered into a group already known archived ([`volume_registered`]),
/// as the archive would have closed them, and drops the parked lazy fork jobs of their groups,
/// since every later open of those groups is refused and none would re-drive them. O(closed
/// volumes plus parked lazy fork jobs), only when a volume registered into an archived group.
fn close_registered_archived(state: &mut MaintainerState, fx: &mut Effects) {
	if state.groups.archive_closes.is_empty() {
		return;
	}
	let keys: Vec<VolumeLogKey> = std::mem::take(&mut state.groups.archive_closes)
		.into_iter()
		.collect();
	let groups: BTreeSet<GroupId> = keys.iter().filter_map(group_of).collect();
	for group in groups {
		super::forks::group_archived(state, group, fx);
	}
	crate::metrics::GROUP_ARCHIVE_CLOSES.inc_by(keys.len() as u64);
	fx.execute(MaintainerEffect::CloseArchivedVolumes { keys });
}

/// Starts the recovery claim that fences the writer of each volume of an archived group still
/// waiting for one. A volume waits while lifecycle, recovery or unload work holds it, while
/// its creation is unacknowledged or its members lack an in-sync quorum, and while every
/// slot of `max_archive_fences` is taken; the next drive tries it again. The fences have their
/// own slots so an archive never takes the `max_repairs` slots crash repairs need. Runs after the
/// lifecycle drives and again at the end of the drives, since the filesystem drive registers
/// volumes after the first pass (an acknowledged creation, a replay), so a volume
/// registering into an archived group is closed and fenced within the event that registered it.
/// Costs nothing while no archived group has a registered volume. First closes the volumes that
/// registered into a group already known archived ([`close_registered_archived`]).
pub(crate) fn drive_archive_fences(state: &mut MaintainerState, fx: &mut Effects) {
	close_registered_archived(state, fx);
	if state.phase != super::MaintainerPhase::Serving || state.groups.archive_fences.is_empty() {
		return;
	}
	let keys: Vec<VolumeLogKey> = state.groups.archive_fences.iter().cloned().collect();
	for key in keys {
		if state.operations.index().counts.archive_fence >= state.config.max_archive_fences {
			return;
		}
		let Some(volume) = state.loaded_volumes.get(&key) else {
			continue;
		};
		if state.pending.contains_key(&key.0.volume_key)
			|| super::repair::holds_key(state, &key.0.volume_key)
			|| state.unloading.contains_key(&key)
			|| super::deletion::ambiguous(state, &key.0.volume_key)
			|| volume
				.creation
				.as_ref()
				.is_some_and(|creation| !creation.acknowledged)
			|| !super::membership::quorum(&state.tablet.replica_set, |worker| {
				volume.members.get(worker).is_some_and(|replica| {
					replica.status
						== rivet_data::generated::shared_depot_types_v1::VolumeLogReplicaStatus::InSync
				})
			}) {
			continue;
		}
		let Some(row) = volume.row(&key) else {
			continue;
		};
		state.groups.archive_fences.remove(&key);
		if row.status == rivet_data::generated::shared_depot_fs_v2::FsVolumeStatus::Deleted {
			continue;
		}
		// The recovery claims every member at an epoch above the writer's and commits a claim
		// entry; once it ends the volume is released, and unloads when idle.
		super::repair::recover(
			state,
			key,
			row,
			super::repair::RecoveryPurpose::ArchiveFence,
			fx,
		);
	}
}

/// What the pin path decided for a volume with entries to apply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PinNeed {
	/// Nothing to pin: the volume is outside any group or stands pinned through the group's
	/// latest fork.
	None,
	/// The group's entry lacks the record or children the pin needs; the volume waits for a
	/// read.
	Read,
	/// The group was forked since the volume's last pin: an op must be logged first.
	Pin,
	/// The group was forked since the volume's last pin, and the volume applies the entries
	/// committed below its pin floor first: they apply now and the pin follows them.
	Apply,
	/// A pin is needed and this grant's maintainer log version cannot carry one; the apply
	/// waits for a grant that can.
	Unsupported,
}

/// The fork number the volume stands pinned for, as its record says; a lazily forked or
/// restored incarnation starts at the fork it was created at.
pub(crate) fn pinned_through(record: &depot_fs_core::FsVolumeRecord) -> u64 {
	record
		.fork_pinned_through
		.or(record.created_at_fork)
		.unwrap_or(0)
}

/// Decides, without changing anything, whether `key`'s next apply may proceed, must wait for
/// a group read, or needs a pin logged first.
pub(crate) fn pin_need(state: &MaintainerState, key: &VolumeLogKey) -> PinNeed {
	// A claim log writes its base volume's rows, so its apply waits on the base's pin; its
	// own floor decides whether it applies first.
	let need = match depot_types::is_claim_log(&key.0.volume_key) {
		Some(_) => fork_pin_need(state, &super::claims::base_of(key).0),
		None => fork_pin_need(state, key),
	};
	match need {
		PinNeed::Pin if below_pin_floor(state, key) => PinNeed::Apply,
		other => other,
	}
}

/// Whether a fork pin of `key`'s volume waits for an [`PinFloor::Unproven`] floor of one of
/// its logs: the members hold a tail nothing proved committed, which only the holder's next
/// write, a recovery or a proof the holder is asked for resolves. A group acquire parked on it
/// is refused as `TailUnproven`, so the fork's caller can have the tail proven.
pub(crate) fn pin_waits_for_tail(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	super::claims::volume_logs(key).iter().any(|log| {
		matches!(
			state.groups.pin_floors.get(log),
			Some(PinFloor::Unproven { .. })
		)
	})
}

/// The logs of `key`'s volume whose pin floor is [`PinFloor::Unproven`]. Bounded by the
/// volume's logs.
pub(crate) fn unproven_logs(state: &MaintainerState, key: &VolumeLogKey) -> Vec<VolumeLogKey> {
	super::claims::volume_logs(key)
		.into_iter()
		.filter(|log| {
			matches!(
				state.groups.pin_floors.get(log),
				Some(PinFloor::Unproven { .. })
			)
		})
		.collect()
}

/// Whether `key` has applied less than its pin floor, or its floor waits for its tail to be
/// proven.
fn below_pin_floor(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	let Some(floor) = state.groups.pin_floors.get(key) else {
		return false;
	};
	match floor {
		PinFloor::Proven { through } => state
			.filesystem
			.volumes
			.get(key)
			.is_some_and(|volume| volume.applied.volume_seq < *through),
		PinFloor::Unproven { .. } => true,
	}
}

/// The seq `key`'s apply stops at while a fork pin waits for it to reach its pin floor, so
/// no entry committed after the floor applies before the pin. `None` when no pin waits on
/// the floor.
pub(crate) fn pin_apply_limit(state: &MaintainerState, key: &VolumeLogKey) -> Option<u64> {
	let limit = match *state.groups.pin_floors.get(key)? {
		PinFloor::Proven { through } => through,
		PinFloor::Unproven { frontier, .. } => frontier,
	};
	match pin_need(state, key) {
		PinNeed::Apply => Some(limit),
		PinNeed::None | PinNeed::Read | PinNeed::Pin | PinNeed::Unsupported => None,
	}
}

/// Records `key`'s pin floor as it registers, from the `Latest` barrier over its members:
/// the highest seq they prove committed when that is above what it applied, or the tail they
/// hold unproven. A log whose members prove their whole tail and hold nothing above the
/// applied position gets none. See `Groups::pin_floors`.
fn note_pin_floor(state: &mut MaintainerState, key: &VolumeLogKey) {
	let Some(volume) = state.filesystem.volumes.get(key) else {
		return;
	};
	let applied = volume.applied.volume_seq;
	let floor = match super::filesystem::latest_barrier(state, key, applied) {
		LatestBarrier::Proven { through } if through > applied => PinFloor::Proven { through },
		LatestBarrier::Proven { .. } => return,
		LatestBarrier::Unproven {
			frontier,
			claim_epoch,
		} => PinFloor::Unproven {
			frontier,
			claim_epoch,
		},
	};
	state.groups.pin_floors.insert(key.clone(), floor);
}

/// `key`'s members were observed: an unproven pin floor whose tail they now prove, or under
/// a newer claim they prove, resolves at most at its frontier, the tail the members held when
/// this grant registered the log, which holds every commit acknowledged before then. The
/// observation that proves it may prove later entries too, such as the holder's first write
/// after the fork or an empty entry it commits to prove its tail; those stay above the pin.
/// The log is marked so the drive applies through the floor and then pins. Costs one map
/// lookup for any other log.
///
/// Under a newer proven claim the floor should end at that claim's entry, which a recovery
/// commits just above what it kept; the replica states carry no position of it, so the
/// frontier bounds it instead. A recovery that dropped an uncommitted part of the frontier
/// lets the new claimant's entries up to the frontier under the pin.
pub(crate) fn members_observed(state: &mut MaintainerState, key: &VolumeLogKey) {
	let Some(PinFloor::Unproven {
		frontier,
		claim_epoch,
	}) = state.groups.pin_floors.get(key).copied()
	else {
		return;
	};
	let Some(proven) = super::filesystem::tail_proven_through(state, key, frontier, claim_epoch)
	else {
		return;
	};
	let through = proven.min(frontier);
	let applied = state
		.filesystem
		.volumes
		.get(key)
		.map_or(0, |volume| volume.applied.volume_seq);
	if through > applied {
		state
			.groups
			.pin_floors
			.insert(key.clone(), PinFloor::Proven { through });
	} else {
		state.groups.pin_floors.remove(key);
	}
	super::filesystem::mark_candidate(state, key);
}

/// `key` applied through `applied_seq`: a proven pin floor it reached leaves, and the pin it
/// held back is logged by the next drive.
pub(crate) fn applied_through(state: &mut MaintainerState, key: &VolumeLogKey, applied_seq: u64) {
	if state
		.groups
		.pin_floors
		.get(key)
		.is_some_and(|floor| match floor {
			PinFloor::Proven { through } => *through <= applied_seq,
			PinFloor::Unproven { .. } => false,
		}) {
		state.groups.pin_floors.remove(key);
	}
}

/// The pin decision of a base volume, before its pin floor is considered.
fn fork_pin_need(state: &MaintainerState, key: &VolumeLogKey) -> PinNeed {
	let Some(group) = group_of(key) else {
		return PinNeed::None;
	};
	let Some(volume) = state.filesystem.volumes.get(key) else {
		return PinNeed::None;
	};
	let Some(entry) = state.groups.entries.get(&group) else {
		return PinNeed::Read;
	};
	if !entry.read_done {
		return PinNeed::Read;
	}
	let pinned = pinned_through(&volume.record);
	if entry.latest_fork <= pinned {
		return PinNeed::None;
	}
	if !entry.knows(pinned + 1..=entry.latest_fork) {
		return PinNeed::Read;
	}
	if state.config.write_versions.maintainer_log < FORK_PIN_LOG_VERSION
		|| state.config.write_versions.fs_rows < FORK_PIN_ROW_VERSION
	{
		return PinNeed::Unsupported;
	}
	PinNeed::Pin
}

/// A grouped volume's apply held because this grant cannot log a pin; the next grant of the
/// tablet selects the versions anew.
pub(crate) fn unsupported(state: &MaintainerState, key: &VolumeLogKey) {
	tracing::warn!(
		?key,
		version = state.config.write_versions.maintainer_log,
		"a grouped volume needs a fork pin but this grant's maintainer log version has none; its apply waits for the next grant"
	);
}

/// Holds `key`'s apply on its group: the entry notes the volume and asks for what it lacks.
/// The volume is offered again when the entry's read completes, or, when `want` was cut short
/// at the tablet's cap and no read of the entry may be coming, once the cap has room.
pub(crate) fn hold_for_read(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	let Some(group) = group_of(key) else {
		return;
	};
	let pinned = state
		.filesystem
		.volumes
		.get(key)
		.map(|volume| pinned_through(&volume.record))
		.unwrap_or(0);
	let entry = state.groups.entry(group);
	entry.held.insert(key.clone());
	if entry.read_done {
		let latest = entry.latest_fork;
		want(state, group, pinned + 1..=latest);
	}
	drive(state, fx);
}

/// Whether a group fork acquire of `key`'s volume may go to the store now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AcquireGate {
	/// The volume stands pinned through the fork.
	Ready,
	/// A pin or a read of the group is on its way; the acquire is answered busy and retried.
	Busy,
}

/// An acquire names fork `fork_number` of `key`'s group: the cache learns the fork at
/// admission, so every later answer knows it, whatever the acquire's own outcome. A group with
/// no entry gets one, whose read starts now.
pub(crate) fn observe_fork(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	fork_number: u64,
	fx: &mut Effects,
) {
	let Some(group) = group_of(key) else {
		return;
	};
	let entry = state.groups.entry(group);
	entry.latest_fork = entry.latest_fork.max(fork_number);
	drive(state, fx);
}

/// Whether the tablet's store writes rows a group fork needs: `depot.fs_rows` version 2.
/// The store writes at the lower of the grant's selected version and the durable mark, so
/// both must reach it. A grant that selected a lower version never does, whatever the record
/// says. Otherwise a fresh grant reaches it only once a pass makes the raised mark durable; a
/// tablet below it asks promptly for the pass that carries the mark, instead of waiting for
/// data to come due, and the fork is refused for a retry meanwhile. The in-memory record
/// leads the store's while that pass uploads, so the mark counts only once it completed.
pub(crate) fn rows_ready(state: &mut MaintainerState, fx: &mut Effects) -> bool {
	if !rows_selected(state) {
		return false;
	}
	if state.maintainer_log.raising_marks.is_some() {
		return false;
	}
	let marks = state
		.maintainer_log
		.record
		.as_ref()
		.map_or(&[][..], |record| record.version_marks.as_slice());
	let mark = super::maintainer_log::rows_mark(marks);
	if mark >= FORK_PIN_ROW_VERSION {
		return true;
	}
	super::maintainer_log::request_mark_pass(state, fx);
	false
}

/// Whether this grant selected the `depot.fs_rows` version a group fork needs, so its store
/// writes those rows once a pass makes the raised mark durable. A grant that selected an older
/// version never does.
pub(crate) fn rows_selected(state: &MaintainerState) -> bool {
	state.config.write_versions.fs_rows >= FORK_PIN_ROW_VERSION
}

/// Whether the store writes the rows a fork pin needs now, as [`rows_ready`] decides, without
/// asking for the pass that raises the mark.
fn writes_pin_rows(state: &MaintainerState) -> bool {
	if state.config.write_versions.fs_rows < FORK_PIN_ROW_VERSION
		|| state.maintainer_log.raising_marks.is_some()
	{
		return false;
	}
	let marks = state
		.maintainer_log
		.record
		.as_ref()
		.map_or(&[][..], |record| record.version_marks.as_slice());
	super::maintainer_log::rows_mark(marks) >= FORK_PIN_ROW_VERSION
}

/// An acquire names fork `fork_number` of `key`'s group: the cache learns the fork, so every
/// later answer knows it, and the volume is pinned through it before the store answers.
pub(crate) fn acquire_fork(
	state: &mut MaintainerState,
	key: Option<&VolumeLogKey>,
	volume_key: &depot_types::VolumeKey,
	fork_number: u64,
	fx: &mut Effects,
) -> AcquireGate {
	let Some(group) = depot_types::keys::partition::forkable_group(&volume_key.partition_key)
	else {
		return AcquireGate::Ready;
	};
	let entry = state.groups.entry(group);
	entry.latest_fork = entry.latest_fork.max(fork_number);
	if !rows_ready(state, fx) {
		return AcquireGate::Busy;
	}
	// A volume the tablet holds registered is pinned through the fork by its own pin path.
	if let Some(key) = key
		&& let Some(volume) = state.filesystem.volumes.get(key)
	{
		if pinned_through(&volume.record) >= fork_number {
			return AcquireGate::Ready;
		}
		return match pin_need(state, key) {
			PinNeed::None => AcquireGate::Ready,
			PinNeed::Read => {
				hold_for_read(state, key, fx);
				AcquireGate::Busy
			}
			PinNeed::Pin | PinNeed::Apply => {
				pin(state, key, fx);
				AcquireGate::Busy
			}
			PinNeed::Unsupported => {
				unsupported(state, key);
				AcquireGate::Busy
			}
		};
	}
	// A volume loaded but not registered yet registers first, then pins as above. A deleted
	// incarnation never registers again; its delete logged its pin, which the store answers
	// from.
	if let Some(key) = key
		&& state
			.loaded_volumes
			.get(key)
			.is_some_and(|volume| !volume.is_deleted())
	{
		super::filesystem::load(state, key.clone());
		return AcquireGate::Busy;
	}
	// An idle volume's pin is planned by the store at its stored cut, with the fork children
	// the cache knows from `idle_pin_floor` through this fork, and logged before the acquire.
	// The store asks for the forks after the volume's last pin when those do not cover them;
	// see `idle_pin_children_needed`. Until the group's record and the children from the
	// floor are read, the acquire waits.
	let Some(entry) = state.groups.entries.get_mut(&group) else {
		return AcquireGate::Busy;
	};
	if !entry.read_done {
		drive(state, fx);
		return AcquireGate::Busy;
	}
	if let Some(floor) = entry.idle_pin_floor
		&& floor <= fork_number
		&& !entry.knows(floor..=fork_number)
	{
		want(state, group, floor..=fork_number);
		drive(state, fx);
		return AcquireGate::Busy;
	}
	AcquireGate::Ready
}

/// The store found that an idle parent of `group` needs the children of every fork from
/// `first_fork` through `fork_number` to plan its pin. The entry's floor falls to
/// `first_fork`, and the missing children are wanted. Returns whether the cache already knows
/// them all, so the acquire plans again at once; otherwise it waits for the read.
pub(crate) fn idle_pin_children_needed(
	state: &mut MaintainerState,
	group: GroupId,
	first_fork: u64,
	fork_number: u64,
	fx: &mut Effects,
) -> bool {
	let Some(entry) = state.groups.entries.get_mut(&group) else {
		return false;
	};
	entry.idle_pin_floor = Some(
		entry
			.idle_pin_floor
			.map_or(first_fork, |floor| floor.min(first_fork)),
	);
	if first_fork > fork_number || entry.knows(first_fork..=fork_number) {
		return true;
	}
	want(state, group, first_fork..=fork_number);
	drive(state, fx);
	false
}

/// The fork children of a group fork acquire's parent group the cache knows, from the group's
/// `idle_pin_floor` through the acquired fork, which the store plans an idle parent's pin with;
/// empty before the store named a floor, and `None` for any other request or while the cache
/// lacks one of them.
pub(crate) fn known_children(
	state: &MaintainerState,
	request: &rivet_data::generated::depot_client_protocol_v2::FsForkReferenceRequest,
) -> Option<GroupChildren> {
	use rivet_data::generated::depot_client_protocol_v2::FsForkReferenceRequest;
	let FsForkReferenceRequest::FsForkAcquire(acquire) = request else {
		return None;
	};
	let FsForkSelector::FsForkGroup(fork) = &acquire.spec.selector else {
		return None;
	};
	let group = depot_types::keys::partition::forkable_group(
		&acquire.spec.parent_volume_key.partition_key,
	)?;
	let entry = state.groups.entries.get(&group)?;
	if !entry.read_done {
		return None;
	}
	// Before the store first names a floor, the plan carries no children and the store
	// answers with the forks it needs.
	let Some(floor) = entry
		.idle_pin_floor
		.filter(|floor| *floor <= fork.fork_number)
	else {
		return Some(Default::default());
	};
	if !entry.knows(floor..=fork.fork_number) {
		return None;
	}
	Some(GroupChildren {
		children: entry
			.fork_children
			.range(floor..=fork.fork_number)
			.map(|(fork, child)| (*fork, (child.child, child.archived)))
			.collect(),
	})
}

/// The children of `key`'s group the cache knows to be archived and has not seen a pass apply
/// to `key`'s pins, which the pin pass drops from the volume's fork pins when the volume's next
/// pin applies or the volume loads. The pass uses only what the cache already holds, so it
/// costs no read: a child the cache does not know, or does not know as archived, stays waiting
/// until a later pass knows it, which keeps more than needed and never less. Empty outside a
/// group or without an entry. Costs the children it returns.
pub(crate) fn archived_children(state: &MaintainerState, key: &VolumeLogKey) -> Vec<GroupId> {
	let Some(group) = group_of(key) else {
		return Vec::new();
	};
	let Some(entry) = state.groups.entries.get(&group) else {
		return Vec::new();
	};
	let passed = entry.passed.get(key).copied().unwrap_or(0);
	entry
		.archived_order
		.range(passed + 1..)
		.filter_map(|(_, fork)| entry.fork_children.get(fork))
		.map(|child| child.child)
		.collect()
}

/// Logs the fork pin of `key`'s volume at its applied cut, which holds the volume's apply until
/// the op is in the store. Nothing is logged while another op or an in-flight apply of the
/// volume could pass the cut; the next drive tries again.
///
/// The cut is where this maintainer is in the volume's log when it learns of the fork, from the
/// events poll or a fork acquire, not where the log was when the fork's API call returned. This
/// is intended. A parent write applied before the maintainer learns of the fork lands in the
/// child. The API's best-effort `settled` wait gives the events poll time to deliver the fork
/// first, but a lagging poll or a regrant of the tablet between the fork and the write, as after
/// a crash, a deploy's restart or a move, can still let a write or destroy made just after the
/// fork into the child. Writes acknowledged before the fork call are never affected: every fork
/// includes them.
pub(crate) fn pin(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	if depot_types::is_claim_log(&key.0.volume_key).is_some() {
		let (base, _) = super::claims::base_of(key);
		return pin(state, &base, fx);
	}
	let Some(group) = group_of(key) else {
		return;
	};
	if !super::maintainer_log::admits(state)
		|| super::maintainer_log::replay::pending_for(state, &key.0.volume_key)
		|| super::maintainer_log::replay::has_ops(state, key)
		|| super::drain::held(state, key)
		|| state
			.operations
			.index()
			.filesystem_applies
			.contains_key(key)
	{
		return;
	}
	let Some(volume) = state.filesystem.volumes.get(key) else {
		return;
	};
	let Some(entry) = state.groups.entries.get(&group) else {
		return;
	};
	let pinned = pinned_through(&volume.record);
	let first_fork = pinned + 1;
	let last_fork = entry.latest_fork;
	if last_fork < first_fork {
		return;
	}
	// A log below its pin floor applies the entries committed before this grant registered
	// it first; the pin is logged once it has, at a cut that holds them.
	let volume_logs = super::claims::volume_logs(key);
	let mut below_floor = false;
	for log in &volume_logs {
		members_observed(state, log);
		if below_pin_floor(state, log) {
			below_floor = true;
			super::filesystem::mark_candidate(state, log);
		}
	}
	if below_floor {
		return;
	}
	// The store writes a pin row only once the pass that raises the `depot.fs_rows` mark to the
	// version this grant selected completed, which a grant after a crash may not have run yet.
	// Until then the pin waits for that pass as an apply held on the mark, whose completion
	// offers the volume again. A pin logged sooner would fail in the store, which fences this
	// maintainer and every successor that finds the same mark, before any of them runs the pass.
	if !writes_pin_rows(state) {
		// A pin already held for the pass asks for nothing more; the pass's completion offers it.
		if !state.filesystem.applies_held.contains_key(key) {
			super::filesystem::hold_for_rows_mark(state, key, "fork_pin", fx);
		}
		return;
	}
	let Some(entry) = state.groups.entries.get(&group) else {
		return;
	};
	// The cut spans every claim log of the volume at its applied position; a claim log not
	// registered yet has no position to pin, so the pin waits for it.
	let mut logs = Vec::new();
	for log in volume_logs.iter().cloned() {
		let Some(log_volume) = state.filesystem.volumes.get(&log) else {
			tracing::debug!(?key, ?log, "a fork pin waits for a claim log to register");
			return;
		};
		let (_, claim_log) = super::claims::base_of(&log);
		logs.push(FsClaimLogPosition {
			claim_log,
			position: log_volume.applied.clone(),
		});
	}
	let waiting: Vec<GroupId> = (first_fork..=last_fork)
		.filter_map(|fork| entry.fork_children.get(&fork))
		.filter(|child| !child.archived)
		.map(|child| child.child)
		.collect();
	let op = MaintainerOp::ForkPinOp(ForkPinOp {
		volume_log_key: key.0.clone(),
		cut: FsCut { logs },
		first_fork,
		last_fork,
		waiting,
		created_at_unix_ms: fx.clocks().unix_ms,
	});
	crate::metrics::GROUP_FORK_PINS
		.with_label_values(&["logged"])
		.inc();
	for log in &volume_logs {
		state.groups.pin_floors.remove(log);
	}
	super::maintainer_log::replay::submit_live(state, op, key.clone());
}

/// The sweep of deleted incarnation `scope` found only fork pins holding it, which wait on
/// `waiting`. The cache waits for each to be archived (see `Groups::deleted_pins`): children it
/// already knows archived go straight to the next sweep, the others are waited on, and their
/// entries are read to learn a status an archive event did not bring. Once nothing is left to
/// wait on, the sweep is queued again. Children past `max_group_pin_wait_children` are not
/// waited on; the next sweep, once the waited ones are archived, reports them again.
pub(crate) fn deleted_pins_held(
	state: &mut MaintainerState,
	scope: Scope,
	waiting: Vec<GroupId>,
	fx: &mut Effects,
) {
	forget_deleted_wait(state, &scope);
	let cap = state.config.max_group_pin_wait_children as usize;
	let room = cap.saturating_sub(state.groups.pin_wait_children);
	let mut wait = DeletedPinWait::default();
	let mut skipped = 0usize;
	for child in waiting {
		if child_status(&state.groups.entries, &BTreeMap::new(), child) == Some(true) {
			wait.archived.insert(child);
		} else if wait.waiting.len() < room {
			wait.waiting.insert(child);
		} else {
			skipped += 1;
		}
	}
	if skipped > 0 {
		crate::metrics::GROUP_PIN_WAIT_CHILDREN_SKIPPED.inc_by(skipped as u64);
		tracing::warn!(
			?scope,
			skipped,
			room,
			"a deleted volume's fork pins wait on more child groups than depot.groups.max_pin_wait_children leaves room for; the rest are waited on after these are archived"
		);
	}
	if wait.waiting.is_empty() {
		if wait.archived.is_empty() {
			// Nothing fits: the pending sweep row stays and the scope waits for room, which
			// `resume_skipped_deleted_pins` gives it back once a wait frees some.
			if state.groups.deleted_pins_skipped.len() < cap {
				state.groups.deleted_pins_skipped.insert(scope);
			} else {
				tracing::warn!(
					?scope,
					"more deleted volumes wait for room under depot.groups.max_pin_wait_children than it allows; the tablet's next pending scan runs this sweep again"
				);
			}
			return;
		}
		state.groups.deleted_pins.insert(scope.clone(), wait);
		queue_deleted_sweep(state, scope);
		return;
	}
	state.groups.pin_wait_children += wait.waiting.len();
	add_wait_refs(&mut state.groups, &wait.waiting);
	for child in &wait.waiting {
		state
			.groups
			.deleted_pin_scopes
			.entry(*child)
			.or_default()
			.insert(scope.clone());
		state.groups.entry(*child);
	}
	state.groups.deleted_pins.insert(scope, wait);
	drive(state, fx);
}

/// The cache learned `child` archived: every deleted incarnation whose pins wait on it moves
/// it to the archived children of its next sweep, and one left waiting on nothing has its
/// sweep queued. Costs the scopes that wait on the child.
fn deleted_child_archived(state: &mut MaintainerState, child: GroupId) {
	let Some(scopes) = state.groups.deleted_pin_scopes.remove(&child) else {
		return;
	};
	let mut released = Vec::new();
	for scope in scopes {
		let Some(wait) = state.groups.deleted_pins.get_mut(&scope) else {
			continue;
		};
		if wait.waiting.remove(&child) {
			wait.archived.insert(child);
			state.groups.pin_wait_children -= 1;
			released.push(child);
		}
		if wait.waiting.is_empty() {
			queue_deleted_sweep(state, scope);
		}
	}
	let released = release_wait_refs(&mut state.groups, released);
	drop_released_children(state, released);
}

/// Queues the pending sweep of deleted incarnation `scope` once, which carries its archived
/// children when it is dispatched.
fn queue_deleted_sweep(state: &mut MaintainerState, scope: Scope) {
	let work = super::store::StoreWork::SweepPending { scope };
	if !state.store_queue.contains(&work) {
		state.store_queue.push_back(work);
	}
}

/// Queues the sweep of one deleted incarnation whose held sweep found no room for any child
/// (`Groups::deleted_pins_skipped`) once `max_group_pin_wait_children` has room again, so its
/// sweep reports its children again and waits on what now fits. Runs at the end of every
/// event; costs nothing while no scope is skipped. One resumed sweep waits in the store queue
/// at a time, and a scope that finds no room again goes back to the set, so the waits that
/// free room drive the resumes and no timer is needed.
pub(crate) fn resume_skipped_deleted_pins(state: &mut MaintainerState) {
	if state.groups.deleted_pins_skipped.is_empty() || state.groups.deleted_pins_resuming.is_some()
	{
		return;
	}
	let cap = state.config.max_group_pin_wait_children as usize;
	if state.groups.pin_wait_children >= cap {
		return;
	}
	let Some(scope) = state.groups.deleted_pins_skipped.pop_first() else {
		return;
	};
	state.groups.deleted_pins_resuming = Some(scope.clone());
	queue_deleted_sweep(state, scope);
}

/// Ends the wait of deleted incarnation `scope`, if any, releasing the children it still
/// waits on; returns its archived children.
fn forget_deleted_wait(state: &mut MaintainerState, scope: &Scope) -> Vec<GroupId> {
	if state.groups.deleted_pins_resuming.as_ref() == Some(scope) {
		state.groups.deleted_pins_resuming = None;
	}
	let Some(wait) = state.groups.deleted_pins.remove(scope) else {
		return Vec::new();
	};
	state.groups.pin_wait_children -= wait.waiting.len();
	for child in &wait.waiting {
		if let Some(scopes) = state.groups.deleted_pin_scopes.get_mut(child) {
			scopes.remove(scope);
			if scopes.is_empty() {
				state.groups.deleted_pin_scopes.remove(child);
			}
		}
	}
	let released = release_wait_refs(&mut state.groups, wait.waiting);
	drop_released_children(state, released);
	wait.archived.into_iter().collect()
}

/// The archived children the sweep of deleted incarnation `scope` drops from its fork pins,
/// at its dispatch. The wait ends with it: the sweep reports what is still held.
pub(crate) fn take_deleted_archived(state: &mut MaintainerState, scope: &Scope) -> Vec<GroupId> {
	state.groups.deleted_pins_skipped.remove(scope);
	forget_deleted_wait(state, scope)
}
