//! Bounded discovery passes advance to a fixed domain-deadline horizon without rewinding.
//! Omitted hints request a later pass; future hints never prevent a pass from finishing.
use super::{MaintainerPhase, MaintainerState, Operation};
use crate::{
	alarms::{Request as AlarmRequest, Response as AlarmResponse},
	tablet_maintainer_machine::{
		Effects,
		effect::MaintainerEffect,
		types::{MaintainerTimer, OpId},
	},
};
use depot_types::{VolumeLogKey, alarm::revision};
use rivet_data::generated::{
	depot_alarm_v1::{AlarmCurrent, AlarmRetryRecord},
	depot_tablet_coordinator_protocol_v1::{
		TabletCoordinatorRejectResponse as CoordinatorReject,
		TabletCoordinatorResponse as CoordinatorResponse,
	},
	shared_depot_alarms_v1::{AlarmOwner, AlarmSummary},
	shared_depot_fs_v2::{FsVolumeRecord, FsVolumeStatus},
};
use rivet_keys::keys::codec::GroupId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The most due keys one group's record read holds. One key is enough to drive the read, and
/// a few more cover the handful of due alarms a small group usually has, so they deliver at
/// once when the read succeeds. The rest of a large group's due keys are dropped and found
/// again by a discovery pass armed when the read ends, which the cached entry then lets
/// deliver without waiting. A fixed sub-limit of `max_candidates` rather than its own setting:
/// it only keeps one group whose read keeps failing from taking every wait from other groups'
/// alarms.
const GROUP_WAITS_PER_GROUP: usize = 4;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Alarms {
	pub enabled: bool,
	pub owner: Option<AlarmOwner>,
	pub owner_id: [u8; 16],
	candidates: Candidates,
	/// At most one invalidation per occupied volume, bounded by active and parked work.
	deferred: BTreeMap<depot_types::VolumeKey, VolumeLogKey>,
	scan: Scan,
	armed_at: Option<u64>,
	/// Earliest retry pass for an omitted hint; independent of cached future deadlines.
	rescan_after: Option<u64>,
	/// Alternate ready retries and fresh candidates when both can use the next slot.
	prefer_queued: bool,
	/// Due keys whose group record the cache has not read, by group. Neither read nor called
	/// until the read ends, since an archived group's alarm must never fire; each read that
	/// ends forgets or reads its keys by what it learned. The cache keeps these groups' entries
	/// while keys wait on them. Capped with `group_waiting` by `max_candidates`, and at
	/// `GROUP_WAITS_PER_GROUP` keys for each group.
	group_waits: BTreeMap<GroupId, GroupWait>,
	/// The keys `group_waits` holds, kept with each change.
	group_waiting: usize,
	/// A due key was dropped because `group_waiting` was at `max_candidates`. The next read
	/// that ends and frees waits arms a discovery pass to find it again; a pass armed sooner
	/// would page and drop it again while every waited read still fails.
	group_waits_overflowed: bool,
	/// Alarm-only group entries kept after their read ended because keys of their group were
	/// dropped at `GROUP_WAITS_PER_GROUP`, with the scan epoch at the read's end. The discovery
	/// pass armed for those keys finds the group's record cached and delivers them without
	/// another read; the restart of the pass after it drops each entry still idle. Bounded by
	/// `max_candidates`; an entry past it is dropped at once and its keys read the group again.
	overflow_entries: BTreeMap<GroupId, u64>,
	/// Due keys whose index read the end of their group's record read started, by group. The
	/// read's completion asks the group cache again, so the group's entry stays cached until
	/// each of these reads finishes, completed, dropped, evicted or ended by a phase change; a
	/// dropped entry would make that completion wait for another group read, and its end start
	/// another index read, forever. Each key has its read pending, so the map is bounded by
	/// `max_operations` plus the parked cap `max_candidates`. Sets are never empty.
	read_handoffs: BTreeMap<GroupId, BTreeSet<VolumeLogKey>>,
	pub initialization: Option<rivet_data::generated::depot_tablet_manifest_v1::AlarmIndexSource>,
	pub reporting_ready: bool,
	pub split_manifest: Option<u64>,
}

/// The alarm candidates by due time, with the grouped ones indexed by group so an archive
/// event touches only its own group's candidates. Bounded by `max_candidates`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Candidates {
	queue: rivet_state_machine::IndexedDeadlineQueue<VolumeLogKey>,
	/// The queued keys of each group, kept with every change to `queue`. Sets are never empty.
	by_group: BTreeMap<GroupId, BTreeSet<VolumeLogKey>>,
}

impl Candidates {
	/// Queues `key` at `at` unless it is queued earlier.
	fn schedule_earliest(&mut self, key: VolumeLogKey, at: u64) -> u64 {
		if let Some(group) = super::groups::group_of(&key) {
			self.by_group.entry(group).or_default().insert(key.clone());
		}
		self.queue.schedule_earliest(key, at)
	}

	/// Removes `key`, returning the time it was queued at.
	pub(crate) fn cancel(&mut self, key: &VolumeLogKey) -> Option<u64> {
		let at = self.queue.cancel(key)?;
		self.unindex(key);
		Some(at)
	}

	/// Removes and returns up to `limit` due keys in `(at, key)` order.
	fn pop_due(&mut self, now: u64, limit: usize) -> Vec<(u64, VolumeLogKey)> {
		let due = self.queue.pop_due(now, limit);
		for (_, key) in &due {
			self.unindex(key);
		}
		due
	}

	fn unindex(&mut self, key: &VolumeLogKey) {
		let Some(group) = super::groups::group_of(key) else {
			return;
		};
		if let Some(keys) = self.by_group.get_mut(&group) {
			keys.remove(key);
			if keys.is_empty() {
				self.by_group.remove(&group);
			}
		}
	}

	/// The queued keys of `group`.
	pub(crate) fn of_group(&self, group: GroupId) -> impl Iterator<Item = &VolumeLogKey> {
		self.by_group.get(&group).into_iter().flatten()
	}

	pub(crate) fn scheduled_at(&self, key: &VolumeLogKey) -> Option<u64> {
		self.queue.scheduled_at(key)
	}

	pub(crate) fn next_at(&self) -> Option<u64> {
		self.queue.next_at()
	}

	pub(crate) fn len(&self) -> usize {
		self.queue.len()
	}

	fn iter(&self) -> impl DoubleEndedIterator<Item = &(u64, VolumeLogKey)> {
		self.queue.iter()
	}

	/// Checks the deadline queue and that the group index holds exactly its grouped keys.
	fn debug_validate(&self) -> anyhow::Result<()> {
		self.queue.debug_validate()?;
		let mut indexed = 0;
		for (group, keys) in &self.by_group {
			anyhow::ensure!(!keys.is_empty(), "empty alarm candidate group set");
			for key in keys {
				anyhow::ensure!(
					super::groups::group_of(key) == Some(*group)
						&& self.queue.scheduled_at(key).is_some(),
					"alarm candidate group index names a key not queued for that group"
				);
			}
			indexed += keys.len();
		}
		let grouped = self
			.queue
			.iter()
			.filter(|(_, key)| super::groups::group_of(key).is_some())
			.count();
		anyhow::ensure!(
			indexed == grouped,
			"alarm candidate group index misses a queued key"
		);
		Ok(())
	}
}

/// The due keys one group's record read holds.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct GroupWait {
	/// At most `GROUP_WAITS_PER_GROUP` keys, never empty.
	keys: BTreeSet<VolumeLogKey>,
	/// A further due key of the group was dropped at `GROUP_WAITS_PER_GROUP`: the read's end
	/// arms a discovery pass, and keeps the group's entry for it.
	overflowed: bool,
}

/// What the group cache allows for a key's alarm before it is read or called.
enum GroupGate {
	/// The key is outside any group, or its group is known live.
	Deliver,
	/// The key's group is archived: its index rows are forgotten instead.
	Forget,
	/// The key's group record is still to be read.
	Wait(GroupId),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Attempt {
	/// The projection waits for its own conflicting store work; no I/O or timer is running.
	Queued(AlarmRequest),
	/// Executor work is in flight; expiry cancels before releasing capacity.
	Running(AlarmRequest),
	/// Keep capacity until cancellation completes, then retry the same immutable I/O body.
	Cancelling(AlarmRequest),
	/// No future or execution slot is held; the retry deadline owns the immutable body.
	Waiting(AlarmRequest),
	/// A replacement invalidated the work; retain capacity until its cancellation completes.
	Obsolete(AlarmRequest),
}
impl Attempt {
	pub(crate) fn request(&self) -> &AlarmRequest {
		match self {
			Self::Queued(r)
			| Self::Running(r)
			| Self::Cancelling(r)
			| Self::Waiting(r)
			| Self::Obsolete(r) => r,
		}
	}
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Scan {
	after: Option<bytes::Bytes>,
	/// Fixed at the first page; callback retries cannot move the end of a live pass.
	through_unix_ms: Option<i64>,
	/// EOF or a page beyond the fixed horizon, whose earliest future wake starts the next pass.
	exhausted: bool,
	epoch: u64,
}

/// Only the derived alarm index is visited, never the unrelated operation map.
fn pending(state: &MaintainerState) -> Vec<(OpId, Attempt)> {
	state
		.operations
		.index()
		.alarms
		.iter()
		.filter_map(|id| match state.operations.get(id) {
			Some(Operation::Alarm(attempt)) => Some((*id, attempt.clone())),
			_ => None,
		})
		.collect()
}
fn pending_volume(state: &MaintainerState, key: &depot_types::VolumeKey) -> Vec<(OpId, Attempt)> {
	state
		.operations
		.index()
		.alarms_by_volume
		.get(key)
		.into_iter()
		.flatten()
		.filter_map(|id| match state.operations.get(id) {
			Some(Operation::Alarm(attempt)) => Some((*id, attempt.clone())),
			_ => None,
		})
		.collect()
}
/// Projection work obeys the same per-volume and tablet barriers as filesystem replay.
fn blocked(state: &MaintainerState, request: &AlarmRequest) -> bool {
	if !request.uses_store() {
		return false;
	}
	let tablet = super::footprint::TabletInFlight::of(state);
	let volume = request
		.key()
		.map(|key| super::footprint::StoreVolume::from(&key.0.volume_key));
	tablet.exclusive
		|| tablet.pending_scan
		|| state.operations.index().store.keyed >= state.config.max_keyed_in_flight as usize
		|| volume.is_some_and(|volume| {
			super::footprint::volume_conflict(state, &volume, super::footprint::VolumeAccess::Keyed)
		})
}
/// Starts `request`, or parks it queued when its store work is blocked or every keyed
/// execution slot of `max_operations` is taken; `drive` starts queued work as slots free.
fn start(state: &mut MaintainerState, request: AlarmRequest, fx: &mut Effects) {
	let slots_full = request.key().is_some()
		&& state.operations.index().counts.alarm_keyed >= state.config.alarms.max_operations;
	if slots_full || blocked(state, &request) {
		park(state, Attempt::Queued(request), fx);
	} else {
		let id = state.begin(Operation::Alarm(Attempt::Running(request.clone())), fx);
		fx.execute(MaintainerEffect::AlarmRequest { op_id: id, request });
	}
}
fn wait(state: &mut MaintainerState, request: AlarmRequest, fx: &mut Effects) {
	park(state, Attempt::Waiting(request), fx);
}
fn rediscover_at(state: &mut MaintainerState, at: u64, now: u64) {
	let at = if at > now {
		at
	} else {
		now.saturating_add(state.config.alarms.retry_min_ms)
	};
	state.alarms.rescan_after = Some(state.alarms.rescan_after.map_or(at, |old| old.min(at)));
}
fn rediscover(state: &mut MaintainerState, fx: &Effects) {
	rediscover_at(state, fx.now_boot_ms(), fx.now_boot_ms());
}
/// What the group cache says of `key`'s group. Asking creates nothing; the caller that waits
/// creates the group's entry and starts its read.
fn group_gate(state: &MaintainerState, key: &VolumeLogKey) -> GroupGate {
	let Some(group) = super::groups::group_of(key) else {
		return GroupGate::Deliver;
	};
	match super::groups::cached_archived(state, group) {
		Some(true) => GroupGate::Forget,
		Some(false) => GroupGate::Deliver,
		None => GroupGate::Wait(group),
	}
}
/// Holds due `key` until `group`'s record read ends, and starts the read if a slot is free.
/// At the tablet's cap, or once `group` holds `GROUP_WAITS_PER_GROUP` keys, the hint is
/// dropped, and the end of a waited read arms the pass that finds its index row again.
fn wait_for_group(
	state: &mut MaintainerState,
	group: GroupId,
	key: VolumeLogKey,
	fx: &mut Effects,
) {
	let held = state.alarms.group_waits.get(&group);
	let waiting = held.is_some_and(|wait| wait.keys.contains(&key));
	let group_full = held.is_some_and(|wait| wait.keys.len() >= GROUP_WAITS_PER_GROUP);
	if !waiting {
		if group_full {
			crate::metrics::ALARM_HINTS_DROPPED
				.with_label_values(&["group_full"])
				.inc();
			if let Some(wait) = state.alarms.group_waits.get_mut(&group) {
				wait.overflowed = true;
			}
			return;
		}
		if state.alarms.group_waiting >= state.config.alarms.max_candidates as usize {
			crate::metrics::ALARM_HINTS_DROPPED
				.with_label_values(&["group_waits_full"])
				.inc();
			state.alarms.group_waits_overflowed = true;
			return;
		}
		state
			.alarms
			.group_waits
			.entry(group)
			.or_default()
			.keys
			.insert(key);
		state.alarms.group_waiting += 1;
		super::groups::alarm_entry(state, group);
	}
	super::groups::drive(state, fx);
}
/// `group`'s record read ended, or its entry left the cache. The keys waiting on it are
/// forgotten or read now by what the read learned, or offered again as due candidates when the
/// cache no longer knows the group. A dropped key, of this group or at the tablet's wait cap,
/// arms the discovery pass that finds it again. The group's entry, if only alarms used it,
/// leaves once nothing needs it, so idle alarm-only entries do not accumulate.
pub(crate) fn group_read_ended(state: &mut MaintainerState, group: GroupId, fx: &mut Effects) {
	let wait = state.alarms.group_waits.remove(&group);
	let overflowed = wait.as_ref().is_some_and(|wait| wait.overflowed);
	if let Some(wait) = wait {
		state.alarms.group_waiting -= wait.keys.len();
		let verdict = super::groups::cached_archived(state, group);
		for key in wait.keys {
			match verdict {
				Some(archived) if state.phase == MaintainerPhase::Serving => {
					offer_gated(state, key, archived, fx)
				}
				Some(_) | None => candidate(state, key, fx.now_boot_ms(), fx.now_boot_ms()),
			}
		}
		if std::mem::take(&mut state.alarms.group_waits_overflowed) || overflowed {
			rediscover(state, fx);
		}
	}

	// An overflowed group keeps its entry for the pass that finds its dropped keys, within
	// the cap; any other idle alarm-only entry leaves now, or once the index reads handed off
	// above finish.
	if overflowed
		&& state.alarms.overflow_entries.len() < state.config.alarms.max_candidates as usize
		&& super::groups::unused_alarm_entry(state, group)
	{
		let epoch = state.alarms.scan.epoch;
		state.alarms.overflow_entries.insert(group, epoch);
		return;
	}
	if !state.alarms.overflow_entries.contains_key(&group) {
		super::groups::drop_idle_alarm_entry(state, group);
	}
}
/// Forgets or reads due `key`, whose group the cache just learned is `archived` or live,
/// without offering it to the drive, since the group's entry may leave before the drive would
/// ask. A key whose volume has alarm work in flight waits for that work as a candidate would.
fn offer_gated(state: &mut MaintainerState, key: VolumeLogKey, archived: bool, fx: &mut Effects) {
	if state.alarms.owner.is_none() {
		return;
	}
	if state
		.operations
		.index()
		.alarms_by_volume
		.contains_key(&key.0.volume_key)
	{
		state
			.alarms
			.deferred
			.entry(key.0.volume_key.clone())
			.or_insert(key);
		return;
	}
	state.alarms.candidates.cancel(&key);
	if archived {
		start(state, AlarmRequest::Forget { key }, fx);
		return;
	}
	start(state, AlarmRequest::Read { key: key.clone() }, fx);
	// A read dropped at the parked cap is pending nowhere; rediscovery finds its row again.
	if read_pending(state, &key) {
		if let Some(group) = super::groups::group_of(&key) {
			state
				.alarms
				.read_handoffs
				.entry(group)
				.or_default()
				.insert(key);
		}
	}
}
/// Whether an index read of `key` is pending in any attempt: running, cancelling, obsolete,
/// queued or waiting for its retry.
fn read_pending(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	let Some(ids) = state
		.operations
		.index()
		.alarms_by_volume
		.get(&key.0.volume_key)
	else {
		return false;
	};
	ids.iter().any(|id| match state.operations.get(id) {
		Some(Operation::Alarm(attempt)) => {
			matches!(attempt.request(), AlarmRequest::Read { key: read } if read == key)
		}
		_ => false,
	})
}
/// Releases `key`'s handoff once no index read of it is pending, and drops its group's entry
/// once the group's last handed-off read finished and nothing else needs the entry. Every path
/// that ends a read calls this after its own transition, so a completion asks the group cache
/// while the entry is still there.
fn read_settled(state: &mut MaintainerState, key: &VolumeLogKey) {
	let Some(group) = super::groups::group_of(key) else {
		return;
	};
	let handed_off = state
		.alarms
		.read_handoffs
		.get(&group)
		.is_some_and(|keys| keys.contains(key));
	if !handed_off || read_pending(state, key) {
		return;
	}
	let Some(keys) = state.alarms.read_handoffs.get_mut(&group) else {
		return;
	};
	keys.remove(key);
	if !keys.is_empty() {
		return;
	}
	state.alarms.read_handoffs.remove(&group);
	if !state.alarms.overflow_entries.contains_key(&group) {
		super::groups::drop_idle_alarm_entry(state, group);
	}
}
/// Whether alarms need `group`'s cache entry: due keys wait on its read, or index reads that
/// its read's end started are still pending.
pub(crate) fn holds_entry(state: &MaintainerState, group: GroupId) -> bool {
	state.alarms.group_waits.contains_key(&group) || state.alarms.read_handoffs.contains_key(&group)
}
/// The lifetime ended and its operations were dropped without completions: no handed-off read
/// is pending any more.
pub(crate) fn lifetime_revoked(state: &mut MaintainerState) {
	state.alarms.read_handoffs.clear();
}
/// Drops the alarm-only entries kept for an overflow's discovery pass once the pass after it
/// starts, each only if still idle.
fn drop_overflow_entries(state: &mut MaintainerState) {
	let epoch = state.alarms.scan.epoch;
	let ended: Vec<GroupId> = state
		.alarms
		.overflow_entries
		.iter()
		.filter(|(_, kept_at)| epoch.wrapping_sub(**kept_at) >= 2)
		.map(|(group, _)| *group)
		.collect();
	for group in ended {
		state.alarms.overflow_entries.remove(&group);
		super::groups::drop_idle_alarm_entry(state, group);
	}
}
/// Whether alarms keep `group`'s cache entry: due keys wait on its read, a handed-off index
/// read its read's end started is pending, or an overflow's discovery pass keeps it.
pub(crate) fn keeps_entry(state: &MaintainerState, group: GroupId) -> bool {
	holds_entry(state, group) || state.alarms.overflow_entries.contains_key(&group)
}

/// Parked bodies have a separate bounded budget, so failures cannot reserve execution slots.
fn park(state: &mut MaintainerState, attempt: Attempt, fx: &mut Effects) {
	if attempt.request().key().is_some()
		&& state.operations.index().alarm_parked.len()
			>= state.config.alarms.max_candidates as usize
	{
		// Reads and forgets have no uncertain mutation to retain; their canonical index entry
		// survives, or was already deleted, and a later pass finds it again.
		if droppable(&attempt) {
			crate::metrics::ALARM_HINTS_DROPPED
				.with_label_values(&["parked_full"])
				.inc();
			if let Some(key) = attempt.request().key() {
				state.alarms.candidates.cancel(key);
				state.alarms.deferred.remove(&key.0.volume_key);
			}
			rediscover(state, fx);
			return;
		}
		let evictable = state.operations.index().alarm_parked.iter().copied().find(
			|id| matches!(state.operations.get(id), Some(Operation::Alarm(parked)) if droppable(parked)),
		);
		if let Some(id) = evictable {
			crate::metrics::ALARM_HINTS_DROPPED
				.with_label_values(&["parked_full"])
				.inc();
			if let Some(Operation::Alarm(evicted)) = state.finish(id, fx) {
				if let Some(key) = evicted.request().key() {
					state.alarms.deferred.remove(&key.0.volume_key);
					read_settled(state, key);
				}
			}
			rediscover(state, fx);
		} else {
			// Never discard or rebase an uncertain write to make room. End this owner
			// if its bounded retention is exhausted; recovery uses the canonical log.
			super::fence(state, fx);
			return;
		}
	}
	state.begin(Operation::Alarm(attempt), fx);
}
/// Whether parked `attempt` holds no uncertain mutation, so it may be dropped at the parked cap
/// and left to rediscovery: a read, or any forget. A forget is an idempotent read-then-delete
/// of the index rows with no execution in flight once parked, since a failed or timed-out
/// forget waits only after its cancellation completed. A dropped forget either already deleted
/// the rows or leaves them for a later pass, so no forget fences the maintainer.
fn droppable(attempt: &Attempt) -> bool {
	match attempt {
		Attempt::Queued(request)
		| Attempt::Running(request)
		| Attempt::Cancelling(request)
		| Attempt::Waiting(request)
		| Attempt::Obsolete(request) => matches!(
			request,
			AlarmRequest::Read { .. } | AlarmRequest::Forget { .. }
		),
	}
}
fn obsolete(state: &mut MaintainerState, id: OpId, fx: &mut Effects) {
	match state.operations.get(&id).cloned() {
		Some(Operation::Alarm(Attempt::Running(request) | Attempt::Cancelling(request))) => {
			state
				.operations
				.insert(id, Operation::Alarm(Attempt::Obsolete(request)));
			fx.execute(MaintainerEffect::CancelAlarmRequest { op_id: id });
		}
		Some(Operation::Alarm(Attempt::Queued(request) | Attempt::Waiting(request))) => {
			state.finish(id, fx);
			if let Some(key) = request.key() {
				read_settled(state, key);
			}
		}
		_ => {}
	}
}

/// Preserve admitted hints and their deadlines. Future hints do not reserve page
/// capacity, and due hints must keep their place while a new pass discovers more work.
fn restart_scan(state: &mut MaintainerState) {
	state.alarms.rescan_after = None;
	state.alarms.scan.after = None;
	state.alarms.scan.through_unix_ms = None;
	state.alarms.scan.exhausted = false;
	state.alarms.scan.epoch = state.alarms.scan.epoch.wrapping_add(1);
	drop_overflow_entries(state);
}
fn candidate(state: &mut MaintainerState, key: VolumeLogKey, at_boot_ms: u64, now: u64) {
	if state.alarms.owner.is_none() {
		return;
	}
	let key = key.detach();
	if state
		.operations
		.index()
		.alarms_by_volume
		.contains_key(&key.0.volume_key)
	{
		state
			.alarms
			.deferred
			.entry(key.0.volume_key.clone())
			.or_insert_with(|| key.clone());
		return;
	}
	let cap = state.config.alarms.max_candidates as usize;
	if state.alarms.candidates.len() >= cap && state.alarms.candidates.scheduled_at(&key).is_none()
	{
		// Keep the earliest hints without rewinding this pass. Rewinding on every
		// eviction lets repeated failures or new commits starve later index rows.
		let last = state.alarms.candidates.iter().next_back().cloned();
		if let Some((at, last)) = last {
			if at_boot_ms >= at {
				rediscover_at(state, at_boot_ms, now);
				return;
			}
			state.alarms.candidates.cancel(&last);
			rediscover_at(state, at, now);
		}
	}
	state.alarms.candidates.schedule_earliest(key, at_boot_ms);
}
fn due_at(
	state: &MaintainerState,
	key: &VolumeLogKey,
	current: &AlarmCurrent,
	fx: &Effects,
) -> Option<u64> {
	let pending = current.summary.pending.as_ref()?;
	let domain = fx.now_boot_ms().saturating_add(
		pending
			.deadline_unix_ms
			.saturating_sub(fx.clocks().unix_ms)
			.max(0) as u64,
	);
	let retry = current.retry.as_ref().filter(|retry| {
		Some(&retry.revision) == revision(key.0.volume_incarnation, &current.summary).as_ref()
	});
	let retry = retry.map_or(0, |retry| {
		if retry.owner_id == state.alarms.owner_id {
			retry.next_boot_ms
		} else {
			fx.now_boot_ms().saturating_add(
				retry
					.next_unix_ms
					.saturating_sub(fx.clocks().unix_ms)
					.max(0) as u64,
			)
		}
	});
	Some(domain.max(retry))
}
fn current_candidate(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	current: &AlarmCurrent,
	fx: &Effects,
) {
	if let Some(at) = due_at(state, &key, current, fx) {
		candidate(state, key, at, fx.now_boot_ms());
	}
}

/// The committed postimage supplies the new deadline and invalidates the previous attempt.
pub(crate) fn changed(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	summary: Option<&AlarmSummary>,
	fx: &mut Effects,
) {
	state.alarms.deferred.remove(&key.0.volume_key);
	if let Some(summary) = summary {
		for (id, attempt) in pending_volume(state, &key.0.volume_key) {
			let stale = match attempt.request() {
				AlarmRequest::Call {
					key: old,
					summary: delivered,
					..
				}
				| AlarmRequest::Retry {
					key: old,
					summary: delivered,
					..
				} => {
					old == &key
						&& revision(key.0.volume_incarnation, delivered)
							!= revision(key.0.volume_incarnation, summary)
				}
				AlarmRequest::Read { key: old } | AlarmRequest::Forget { key: old } => old == &key,
				AlarmRequest::Page { .. } => false,
			};
			if stale {
				obsolete(state, id, fx);
			}
		}
		state.alarms.candidates.cancel(&key);
		current_candidate(
			state,
			key,
			&AlarmCurrent {
				summary: summary.clone(),
				retry: None,
			},
			fx,
		);
	} else {
		candidate(state, key, fx.now_boot_ms(), fx.now_boot_ms());
	}
}
pub(crate) fn published(state: &mut MaintainerState, record: &FsVolumeRecord, fx: &mut Effects) {
	let key = VolumeLogKey::from(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
		volume_key: record.volume_key(),
		volume_incarnation: record.volume_incarnation,
	});
	if record.status != FsVolumeStatus::Deleted
		|| state
			.alarms
			.deferred
			.get(&key.0.volume_key)
			.is_some_and(|old| old.0.volume_incarnation == key.0.volume_incarnation)
	{
		state.alarms.deferred.remove(&key.0.volume_key);
	}
	for (id, attempt) in pending_volume(state, &key.0.volume_key) {
		if attempt.request().key().is_some_and(|old| {
			if record.status == FsVolumeStatus::Deleted {
				old.0.volume_incarnation == key.0.volume_incarnation
			} else {
				old.0.volume_incarnation != key.0.volume_incarnation
			}
		}) {
			obsolete(state, id, fx);
		}
	}
	state.alarms.candidates.cancel(&key);
	if record.status == FsVolumeStatus::Deleted {
		return;
	}
	// A lazy fork indexes no alarm, whether a read or a command made it: the child's first
	// command arms its inherited alarm in its own commit. This is intentional: it keeps a fork
	// O(1), a fork cannot wake every inherited alarm at once and overload the child's pools, and
	// reading an actor does not start it. Two later options are not built: the parent fires its
	// children's alarms, or a colocated import places volumes by root group and copies alarm
	// entries on `GROUP_FORKED`.
	if record.lazy_fork.is_some() {
		return;
	}
	candidate(state, key, fx.now_boot_ms(), fx.now_boot_ms());
}

/// `group` was archived: the alarms of its volumes never fire again. Its scheduled
/// candidates, found through the group index without visiting other groups', and the keys
/// waiting on its read come due now, and the drive forgets their index
/// rows under `max_operations`, since the cache holds the archive (the event handler kept it on
/// the entry of every group that is cached or waited on). The rows of its other volumes are
/// forgotten when they come due: the drive reads an unknown group before it delivers.
pub(crate) fn group_archived(state: &mut MaintainerState, group: GroupId, fx: &mut Effects) {
	let keys: Vec<VolumeLogKey> = state.alarms.candidates.of_group(group).cloned().collect();
	for key in keys {
		state.alarms.candidates.cancel(&key);
		candidate(state, key, fx.now_boot_ms(), fx.now_boot_ms());
	}
	group_read_ended(state, group, fx);
}
pub(crate) fn tick(state: &mut MaintainerState, _fx: &mut Effects) {
	state.alarms.armed_at = None;
}

pub(crate) fn drive(state: &mut MaintainerState, fx: &mut Effects) {
	if state.alarms.owner.is_none() {
		return;
	}
	if state.phase != MaintainerPhase::Serving {
		if state.alarms.armed_at.take().is_some() {
			fx.cancel_timer(MaintainerTimer::Alarms);
		}
		for (id, attempt) in pending(state) {
			match attempt {
				Attempt::Running(_) | Attempt::Cancelling(_) | Attempt::Obsolete(_) => {
					fx.execute(MaintainerEffect::CancelAlarmRequest { op_id: id })
				}
				Attempt::Queued(request) | Attempt::Waiting(request) => {
					state.finish(id, fx);
					if let Some(key) = request.key() {
						read_settled(state, key);
					}
				}
			}
		}
		return;
	}
	if state.alarms.scan.exhausted
		&& state
			.alarms
			.rescan_after
			.is_some_and(|at| at <= fx.now_boot_ms())
	{
		restart_scan(state);
	}
	// While tablet-wide store work holds every store access, no keyed candidate is popped: its
	// read or forget would only park, and past the parked cap be dropped and paged again. The
	// end of that work is an event, whose drive pops them.
	let tablet = super::footprint::TabletInFlight::of(state);
	let tablet_blocked = tablet.exclusive || tablet.pending_scan;
	while state.phase == MaintainerPhase::Serving
		&& state.operations.index().counts.alarm_keyed < state.config.alarms.max_operations
	{
		let queued = state.operations.index().alarm_queued.iter().find_map(|id| {
			match state.operations.get(id) {
				Some(Operation::Alarm(Attempt::Queued(request))) if !blocked(state, request) => {
					Some((*id, request.clone()))
				}
				_ => None,
			}
		});
		let due = state
			.alarms
			.candidates
			.next_at()
			.is_some_and(|at| at <= fx.now_boot_ms());
		if let Some((id, request)) = queued.filter(|_| state.alarms.prefer_queued || !due) {
			state
				.operations
				.insert(id, Operation::Alarm(Attempt::Running(request.clone())));
			let timeout = state.config.alarms.io_timeout_ms;
			fx.arm_timer_after(MaintainerTimer::Operation(id), timeout);
			fx.execute(MaintainerEffect::AlarmRequest { op_id: id, request });
			state.alarms.prefer_queued = false;
			continue;
		}
		if tablet_blocked {
			break;
		}
		let Some((_, key)) = state.alarms.candidates.pop_due(fx.now_boot_ms(), 1).pop() else {
			break;
		};
		// Completion of the occupying operation revalidates the key, so no busy-wait timer is needed.
		if state
			.operations
			.index()
			.alarms_by_volume
			.contains_key(&key.0.volume_key)
		{
			state
				.alarms
				.deferred
				.entry(key.0.volume_key.clone())
				.or_insert(key);
			continue;
		}
		// A volume of an archived group forgets its alarm instead of reading it, and one whose
		// group is still to be read waits for the read, so it is never called meanwhile.
		match group_gate(state, &key) {
			GroupGate::Deliver => start(state, AlarmRequest::Read { key }, fx),
			GroupGate::Forget => start(state, AlarmRequest::Forget { key }, fx),
			GroupGate::Wait(group) => {
				wait_for_group(state, group, key, fx);
				continue;
			}
		}
		state.alarms.prefer_queued = true;
	}
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	if !state.alarms.scan.exhausted
		&& state.operations.index().alarm_pages.is_empty()
		&& (state.alarms.candidates.len() < state.config.alarms.max_candidates as usize
			|| state
				.alarms
				.candidates
				.next_at()
				.is_some_and(|at| at > fx.now_boot_ms()))
	{
		state
			.alarms
			.scan
			.through_unix_ms
			.get_or_insert(fx.clocks().unix_ms);
		start(
			state,
			AlarmRequest::Page {
				after: state.alarms.scan.after.clone(),
				epoch: state.alarms.scan.epoch,
				limit: (state.config.alarms.page_size as usize).min(
					(state.config.alarms.max_candidates as usize - state.alarms.candidates.len())
						.max(1),
				),
			},
			fx,
		);
	}
	let slots_free =
		state.operations.index().counts.alarm_keyed < state.config.alarms.max_operations;
	let mut next = if slots_free && !tablet_blocked {
		state.alarms.candidates.next_at()
	} else {
		None
	};
	if state.alarms.scan.exhausted
		&& let Some(at) = state.alarms.rescan_after
	{
		next = Some(next.map_or(at, |next| next.min(at)));
	}
	if next != state.alarms.armed_at {
		state.alarms.armed_at = next;
		match next {
			Some(at) => fx.arm_timer_at(MaintainerTimer::Alarms, at),
			None => fx.cancel_timer(MaintainerTimer::Alarms),
		}
	}
}
pub(crate) fn expired(state: &mut MaintainerState, id: OpId, fx: &mut Effects) {
	match state.operations.get(&id).cloned() {
		Some(Operation::Alarm(Attempt::Running(request))) => {
			state
				.operations
				.insert(id, Operation::Alarm(Attempt::Cancelling(request)));
			fx.execute(MaintainerEffect::CancelAlarmRequest { op_id: id });
		}
		Some(Operation::Alarm(Attempt::Cancelling(_) | Attempt::Obsolete(_))) => {
			fx.execute(MaintainerEffect::CancelAlarmRequest { op_id: id })
		}
		Some(Operation::Alarm(Attempt::Waiting(request))) => {
			if state.phase == MaintainerPhase::Serving {
				state
					.operations
					.insert(id, Operation::Alarm(Attempt::Queued(request)));
			} else {
				state.finish(id, fx);
				if let Some(key) = request.key() {
					read_settled(state, key);
				}
			}
		}
		_ => {}
	}
}
/// An alarm operation's executor answered. A handed-off index read is released after the
/// completion's own transition, which asks the group cache while the entry is still cached.
pub(crate) fn completed(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<AlarmResponse, String>,
	fx: &mut Effects,
) {
	let read = match state.operations.get(&id) {
		Some(Operation::Alarm(attempt)) => match attempt.request() {
			AlarmRequest::Read { key } => Some(key.clone()),
			AlarmRequest::Page { .. }
			| AlarmRequest::Call { .. }
			| AlarmRequest::Retry { .. }
			| AlarmRequest::Forget { .. } => None,
		},
		_ => None,
	};
	attempt_completed(state, id, result, fx);
	if let Some(key) = read {
		read_settled(state, &key);
	}
}
fn attempt_completed(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<AlarmResponse, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(Operation::Alarm(
			Attempt::Running(_) | Attempt::Cancelling(_) | Attempt::Obsolete(_)
		))
	) {
		return;
	}
	let Some(Operation::Alarm(attempt)) = state.finish(id, fx) else {
		return;
	};
	let recheck = attempt
		.request()
		.key()
		.and_then(|key| state.alarms.deferred.remove(&key.0.volume_key));
	let rechecked = recheck.is_some();
	if let Some(latest) = recheck {
		// A live attempt already validates its own key. Reoffering the same hint
		// before parking its retry can evict the next scan row and rewind discovery.
		if matches!(attempt, Attempt::Obsolete(_)) || attempt.request().key() != Some(&latest) {
			candidate(state, latest, fx.now_boot_ms(), fx.now_boot_ms());
		}
	}
	let (request, result) = match attempt {
		Attempt::Running(request) => (request, result),
		Attempt::Cancelling(request) if matches!(request, AlarmRequest::Call { .. }) => {
			(request, Err("alarm callback timed out".to_owned()))
		}
		Attempt::Cancelling(request) => {
			if state.phase == MaintainerPhase::Serving {
				wait(state, request, fx);
			}
			return;
		}
		Attempt::Obsolete(request) => {
			if !rechecked {
				if let Some(key) = request.key() {
					candidate(state, key.clone(), fx.now_boot_ms(), fx.now_boot_ms());
				}
			}
			return;
		}
		Attempt::Waiting(_) | Attempt::Queued(_) => return,
	};
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	let result = match result {
		Err(_) if matches!(request, AlarmRequest::Call { .. }) => Ok(AlarmResponse::Called {
			busy: false,
			randomness: 0,
		}),
		result => result,
	};
	match (request, result) {
		(request, Err(_)) => wait(state, request, fx),
		(AlarmRequest::Page { epoch, .. }, Ok(AlarmResponse::Page { rows, done })) => {
			if epoch != state.alarms.scan.epoch {
				return;
			}
			let mut reached_horizon = false;
			for (physical, row) in rows {
				state.alarms.scan.after = Some(physical);
				let key = VolumeLogKey::from(row.volume_log_key);
				if super::in_tablet(state, &key.0.volume_key) {
					// A row of a group the cache knows is archived is due at once, and the drive
					// forgets it within `max_operations`. An unknown group is not read here: the
					// drive reads it when one of its rows comes due.
					if super::groups::known_archived(state, &key.0.volume_key) {
						candidate(state, key, fx.now_boot_ms(), fx.now_boot_ms());
						continue;
					}
					let current = AlarmCurrent {
						summary: row.summary,
						retry: row.retry,
					};
					if let Some(eligible) = depot_types::alarm::index_deadline(&current.summary)
						&& state
							.alarms
							.scan
							.through_unix_ms
							.is_some_and(|through| eligible > through)
					{
						reached_horizon = true;
						if let Some(at) = due_at(state, &key, &current, fx) {
							rediscover_at(state, at, fx.now_boot_ms());
						}
					}
					current_candidate(state, key, &current, fx);
				}
			}
			state.alarms.scan.exhausted = done || reached_horizon;
		}
		(AlarmRequest::Read { key }, Ok(AlarmResponse::Read { current })) => {
			let Some(current) = current else {
				return;
			};
			let Some(at) = due_at(state, &key, &current, fx) else {
				return;
			};
			if at > fx.now_boot_ms() {
				candidate(state, key, at, fx.now_boot_ms());
				return;
			}
			// The group may have been archived, or dropped from the cache, while the read was in
			// flight.
			match group_gate(state, &key) {
				GroupGate::Deliver => {}
				GroupGate::Forget => {
					start(state, AlarmRequest::Forget { key }, fx);
					return;
				}
				GroupGate::Wait(group) => {
					wait_for_group(state, group, key, fx);
					return;
				}
			}
			start(
				state,
				AlarmRequest::Call {
					key,
					summary: current.summary,
					attempt: current.retry.map_or(0, |retry| retry.attempt),
				},
				fx,
			);
		}
		(
			AlarmRequest::Call {
				key,
				summary,
				attempt,
			},
			Ok(AlarmResponse::Called { randomness, .. }),
		) => {
			let ceiling = state
				.config
				.alarms
				.retry_min_ms
				.saturating_mul(1u64 << attempt.min(63))
				.min(state.config.alarms.retry_max_ms);
			let delay = ceiling
				.saturating_add(u64::from(randomness) % (ceiling / 4 + 1))
				.min(state.config.alarms.retry_max_ms);
			let Some(revision) = revision(key.0.volume_incarnation, &summary) else {
				return;
			};
			let record = AlarmRetryRecord {
				revision,
				next_unix_ms: fx.clocks().unix_ms.saturating_add(delay as i64),
				next_boot_ms: fx.now_boot_ms().saturating_add(delay),
				attempt: attempt.saturating_add(1),
				owner_id: state.alarms.owner_id,
			};
			start(
				state,
				AlarmRequest::Retry {
					key,
					summary,
					record,
				},
				fx,
			);
		}
		(AlarmRequest::Retry { key, .. }, Ok(AlarmResponse::Updated { current })) => {
			if let Some(current) = current {
				current_candidate(state, key, &current, fx);
			}
		}
		(AlarmRequest::Forget { .. }, Ok(AlarmResponse::Updated { .. })) => {}
		(
			AlarmRequest::Page { .. }
			| AlarmRequest::Read { .. }
			| AlarmRequest::Call { .. }
			| AlarmRequest::Retry { .. }
			| AlarmRequest::Forget { .. },
			Ok(
				AlarmResponse::Page { .. }
				| AlarmResponse::Read { .. }
				| AlarmResponse::Called { .. }
				| AlarmResponse::Updated { .. },
			),
		) => super::fence(state, fx),
	}
}

pub(crate) fn snapshot(state: &MaintainerState) -> crate::tablet_maintainer_machine::AlarmSnapshot {
	if state.alarms.owner.is_none() {
		return Default::default();
	}
	crate::tablet_maintainer_machine::AlarmSnapshot {
		candidates: state.alarms.candidates.len() as u32,
		operations: state.operations.index().counts.alarm_keyed,
		parked: state.operations.index().alarm_parked.len() as u32,
		parked_capacity: state.config.alarms.max_candidates,
		candidate_capacity: state.config.alarms.max_candidates,
		operation_capacity: state.config.alarms.max_operations,
		group_waits: state.alarms.group_waiting as u32,
		group_wait_capacity: state.config.alarms.max_candidates,
	}
}

pub(crate) fn validate(state: &MaintainerState) -> anyhow::Result<()> {
	state.alarms.candidates.debug_validate()?;
	anyhow::ensure!(
		state.operations.index().counts.alarm_keyed <= state.config.alarms.max_operations,
		"alarm execution capacity exceeded"
	);
	anyhow::ensure!(
		state.operations.index().alarm_parked.len() <= state.config.alarms.max_candidates as usize,
		"parked alarm capacity exceeded"
	);
	anyhow::ensure!(
		state.alarms.candidates.len() <= state.config.alarms.max_candidates as usize,
		"alarm frontier capacity exceeded"
	);
	anyhow::ensure!(
		state.operations.index().alarm_pages.len() <= 1,
		"concurrent alarm pages"
	);
	anyhow::ensure!(
		state.alarms.group_waiting
			== state
				.alarms
				.group_waits
				.values()
				.map(|wait| wait.keys.len())
				.sum::<usize>(),
		"alarm group wait count drifted"
	);
	anyhow::ensure!(
		state.alarms.group_waiting <= state.config.alarms.max_candidates as usize,
		"alarm group wait capacity exceeded"
	);
	anyhow::ensure!(
		state
			.alarms
			.group_waits
			.values()
			.all(|wait| !wait.keys.is_empty() && wait.keys.len() <= GROUP_WAITS_PER_GROUP),
		"alarm group waits hold an empty or overfull group"
	);
	anyhow::ensure!(
		state.alarms.overflow_entries.len() <= state.config.alarms.max_candidates as usize,
		"alarm overflow entry capacity exceeded"
	);
	anyhow::ensure!(
		state
			.alarms
			.group_waits
			.keys()
			.chain(state.alarms.read_handoffs.keys())
			.all(|group| state.groups.entries.contains_key(group)),
		"alarms wait on a group the cache holds no entry for"
	);
	for (group, keys) in &state.alarms.read_handoffs {
		anyhow::ensure!(!keys.is_empty(), "an empty alarm read handoff");
		for key in keys {
			anyhow::ensure!(
				super::groups::group_of(key) == Some(*group) && read_pending(state, key),
				"an alarm read handoff names no pending read of its group"
			);
		}
	}
	Ok(())
}

pub(crate) fn ready_response(
	state: &mut MaintainerState,
	result: Result<
		rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse,
		String,
	>,
	fx: &mut Effects,
) {
	if state.phase != super::MaintainerPhase::Opening || !state.alarms.reporting_ready {
		return;
	}
	state.alarms.reporting_ready = false;
	match result {
		Ok(CoordinatorResponse::Accepted(_)) => {
			state.alarms.initialization = None;
			super::opening::finish(state, fx);
		}
		Err(_)
		| Ok(CoordinatorResponse::TabletCoordinatorRejectResponse(
			CoordinatorReject::Overloaded | CoordinatorReject::NotCoordinator,
		)) => fx.arm_timer_after(MaintainerTimer::SplitRetry, state.config.tuning().retry_ms),
		_ => super::fence(state, fx),
	}
}
