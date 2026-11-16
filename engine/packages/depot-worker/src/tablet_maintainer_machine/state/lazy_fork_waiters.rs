//! Opens of a grouped volume with no record that wait for the volume's lazy fork job, or for
//! a group read the open's lineage walk or fork stamp needs.
//!
//! Every open of such a volume waits behind the work already in progress instead of being
//! refused for it, so concurrent and retried opens of one volume share one job and one read.
//! The only refusal is at admission, when the volume already holds
//! `max_lazy_fork_waiters_per_volume` waiting opens. Each waiting open keeps its own deadline
//! timer, armed just short of its caller's deadline (see `arm_lazy_fork_deadline`), and an
//! expiry answers only that open.
//!
//! An open admitted again is never refused for a transient reason (a full budget, work in flight
//! on its key, an owner handoff, an apply far behind): the key parks by that reason until the
//! event that clears it, and only then is offered again, so the drive after every event visits
//! only the keys some event woke. A full budget is retried as soon as it has room, one key at a
//! time; an owner handoff when it ends; work on the key when an event or a finished operation
//! names the key, or its open in flight leaves. A key no such event wakes keeps its opens until
//! their deadlines. An open whose lineage walk parks on a group already read, because the
//! cached lineage does not end at a root, holds a waiting slot until its deadline; nothing
//! releases it earlier.

use super::{MaintainerPhase, MaintainerState};
use crate::tablet_maintainer_machine::{
	Effects,
	types::{OpenVolume, RequestId},
};
use depot_types::VolumeKey;
use rivet_keys::keys::codec::GroupId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// The waiting opens of every volume key, with the index of keys being admitted again.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct LazyForkWaiters {
	/// The waiting opens by volume key. A key leaves with its last open.
	keys: BTreeMap<VolumeKey, KeyWaiters>,
	/// The keys whose wait is `Wait::Admitting` and that the next drive offers for admission:
	/// each wait that ended, and each parked key an event woke since.
	admitting: BTreeSet<VolumeKey>,
	/// The keys whose wait is `Wait::Admitting` that admission refused for a transient reason,
	/// by that reason. Each is offered again only on the event that can clear it, so the drive
	/// after every event never visits them.
	parked: BTreeMap<VolumeKey, Parked>,
	/// `parked` by reason: the keys refused for a full request or row read budget, offered one
	/// at a time while the budget has room.
	parked_capacity: BTreeSet<VolumeKey>,
	/// The keys refused for an owner handoff, offered when it ends.
	parked_handoff: BTreeSet<VolumeKey>,
	/// The keys refused for work on the key itself, offered when an event or a finished
	/// operation names the key, or its open in flight leaves.
	parked_key: BTreeSet<VolumeKey>,
	/// The keys whose wait is `Wait::GroupRead` of each group, so a group read's end visits
	/// only the keys waiting for it.
	by_group: BTreeMap<GroupId, BTreeSet<VolumeKey>>,
	/// How many keys use each group's cache entry: a key waiting to read a group uses it and
	/// its own group, and a key being admitted again uses its own group. The group cache keeps
	/// an entry while this counts it.
	group_uses: BTreeMap<GroupId, u32>,
	/// Groups whose count in `group_uses` reached zero since the group cache last took them,
	/// so it checks only those entries for leaving.
	released_groups: BTreeSet<GroupId>,
	/// Waiting opens across every key.
	callers: usize,
}

/// Why admission last refused a key's oldest open, which decides the event that offers it
/// again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Parked {
	/// The request or row read budget is full.
	Capacity,
	/// An owner handoff bars creating.
	Handoff,
	/// Work on the key itself: recovered log ops, a fork, an open or a row read of it in
	/// flight, or its apply far behind.
	Key,
}

impl Parked {
	/// The parking reason of an admission refusal reason from `lifecycle::not_now`.
	fn of(reason: &str) -> Self {
		match reason {
			"request_capacity" | "row_read_capacity" => Parked::Capacity,
			"owner_handoff" => Parked::Handoff,
			// "log_replay", "fork_hold", "key_opening", "key_row_read" and "apply_behind",
			// each of which an event naming the key can clear.
			_ => Parked::Key,
		}
	}
}

/// The opens of one volume key and what they wait for.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct KeyWaiters {
	wait: Wait,
	/// Oldest first. Never empty while the key is held. It may hold one open past the cap:
	/// an open admitted again from it that waits once more returns to its front whatever its
	/// length.
	callers: VecDeque<OpenVolume>,
}

/// Which end of a key's queue an open joins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum End {
	/// A new open, behind the opens already waiting.
	Back,
	/// An open admitted again from the queue that must wait once more, ahead of the opens
	/// that arrived after it.
	Front,
}

/// What a key's waiting opens wait for.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Wait {
	/// The key's lazy fork job, which publishes the volume's record or finds the volume absent
	/// at the fork. The job's end moves the key to `Admitting`.
	Job,
	/// The read of this group, which the open's lineage walk or the creation's fork stamp
	/// needs. Its completion, or the loss of its cache entry, moves the key to `Admitting`.
	GroupRead(GroupId),
	/// The wait ended. Admission refuses a second open of a key while one is in flight, so
	/// the opens are admitted again one at a time, each once the key has no open, record
	/// lookup or fork job in flight.
	Admitting,
}

impl LazyForkWaiters {
	/// Whether opens of `key` wait here.
	pub(crate) fn contains_key(&self, key: &VolumeKey) -> bool {
		self.keys.contains_key(key)
	}

	/// Waiting opens across every key.
	pub(crate) fn len(&self) -> usize {
		self.callers
	}

	/// Whether the open `id` of `key` waits here.
	pub(crate) fn holds(&self, key: &VolumeKey, id: RequestId) -> bool {
		self.keys
			.get(key)
			.is_some_and(|waiters| waiters.callers.iter().any(|caller| caller.request_id == id))
	}

	/// Every group a waiting open's lineage walk or fork stamp waits to read, with the keys
	/// waiting for it.
	pub(crate) fn group_reads(&self) -> impl Iterator<Item = (&GroupId, &BTreeSet<VolumeKey>)> {
		self.by_group.iter()
	}

	/// Whether a waiting open uses `group`'s cache entry; see `group_uses`.
	pub(crate) fn uses_group(&self, group: GroupId) -> bool {
		self.group_uses.contains_key(&group)
	}

	/// Takes the groups no waiting open uses any more since the last call.
	pub(crate) fn take_released_groups(&mut self) -> BTreeSet<GroupId> {
		std::mem::take(&mut self.released_groups)
	}

	/// Checks that the indexes match the waits they index.
	pub(crate) fn validate(&self) -> anyhow::Result<()> {
		let mut uses: BTreeMap<GroupId, u32> = BTreeMap::new();
		for (key, waiters) in &self.keys {
			for group in used_groups(key, &waiters.wait) {
				*uses.entry(group).or_default() += 1;
			}
			let indexed = match &waiters.wait {
				Wait::Admitting => match self.parked.get(key) {
					None => self.admitting.contains(key),
					Some(reason) => {
						!self.admitting.contains(key) && self.parked_keys(*reason).contains(key)
					}
				},
				Wait::GroupRead(group) => self
					.by_group
					.get(group)
					.is_some_and(|keys| keys.contains(key)),
				Wait::Job => true,
			};
			anyhow::ensure!(
				indexed,
				"a waiting key is missing from the index of its wait"
			);
		}
		anyhow::ensure!(
			uses == self.group_uses,
			"lazy fork waiters' group uses do not match their waits"
		);
		let indexed = self.admitting.len()
			+ self.parked_capacity.len()
			+ self.parked_handoff.len()
			+ self.parked_key.len();
		let admitting = self
			.keys
			.values()
			.filter(|waiters| waiters.wait == Wait::Admitting)
			.count();
		anyhow::ensure!(
			indexed == admitting && self.parked.len() + self.admitting.len() == admitting,
			"lazy fork waiters index keys being admitted that do not wait for admission"
		);
		Ok(())
	}

	/// Removes and returns every waiting open, for a fence that answers them all.
	pub(crate) fn take_all(&mut self) -> Vec<OpenVolume> {
		let taken = std::mem::take(self);
		taken
			.keys
			.into_values()
			.flat_map(|waiters| waiters.callers)
			.collect()
	}

	/// Queues `caller` behind the key's other opens, or ahead of them when it is an open
	/// admitted again from this queue (`End::Front`), so the oldest still goes first.
	fn push(&mut self, key: VolumeKey, caller: OpenVolume, wait: Wait, end: End) {
		// A new key starts at `Wait::Job`, which no index holds, so `set_wait` indexes it once.
		let waiters = self.keys.entry(key.clone()).or_insert(KeyWaiters {
			wait: Wait::Job,
			callers: VecDeque::new(),
		});
		match end {
			End::Back => waiters.callers.push_back(caller),
			End::Front => waiters.callers.push_front(caller),
		}
		self.callers += 1;
		self.set_wait(&key, wait);
	}

	/// Sets what the opens of `key` wait for, keeping the indexes in step. A key that waits
	/// for admission again is offered at the next drive, parked or not.
	fn set_wait(&mut self, key: &VolumeKey, wait: Wait) {
		let Some(waiters) = self.keys.get_mut(key) else {
			return;
		};
		let previous = std::mem::replace(&mut waiters.wait, wait.clone());
		self.unindex(key, &previous);
		for group in used_groups(key, &wait) {
			*self.group_uses.entry(group).or_default() += 1;
		}
		match wait {
			Wait::Admitting => {
				self.admitting.insert(key.clone());
			}
			Wait::GroupRead(group) => {
				self.by_group.entry(group).or_default().insert(key.clone());
			}
			Wait::Job => {}
		}
	}

	/// Removes `key` from the index of `wait`.
	fn unindex(&mut self, key: &VolumeKey, wait: &Wait) {
		for group in used_groups(key, wait) {
			let Some(count) = self.group_uses.get_mut(&group) else {
				continue;
			};
			*count -= 1;
			if *count == 0 {
				self.group_uses.remove(&group);
				self.released_groups.insert(group);
			}
		}
		match wait {
			Wait::Admitting => {
				self.admitting.remove(key);
				self.unpark(key);
			}
			Wait::GroupRead(group) => {
				if let Some(keys) = self.by_group.get_mut(group) {
					keys.remove(key);
					if keys.is_empty() {
						self.by_group.remove(group);
					}
				}
			}
			Wait::Job => {}
		}
	}

	/// Parks `key`, which waits for admission, until the event that clears `reason`.
	fn park(&mut self, key: &VolumeKey, reason: Parked) {
		self.admitting.remove(key);
		self.unpark(key);
		self.parked.insert(key.clone(), reason);
		self.parked_set(reason).insert(key.clone());
	}

	/// Removes `key` from the parked keys, if it is parked.
	fn unpark(&mut self, key: &VolumeKey) {
		if let Some(reason) = self.parked.remove(key) {
			self.parked_set(reason).remove(key);
		}
	}

	/// Offers parked `key` at the next drive, if it is parked for `reason`.
	fn wake(&mut self, key: &VolumeKey, reason: Parked) {
		if self.parked.get(key) == Some(&reason) {
			self.unpark(key);
			self.admitting.insert(key.clone());
		}
	}

	fn parked_keys(&self, reason: Parked) -> &BTreeSet<VolumeKey> {
		match reason {
			Parked::Capacity => &self.parked_capacity,
			Parked::Handoff => &self.parked_handoff,
			Parked::Key => &self.parked_key,
		}
	}

	fn parked_set(&mut self, reason: Parked) -> &mut BTreeSet<VolumeKey> {
		match reason {
			Parked::Capacity => &mut self.parked_capacity,
			Parked::Handoff => &mut self.parked_handoff,
			Parked::Key => &mut self.parked_key,
		}
	}

	/// Drops `key` once its last open left.
	fn release_if_empty(&mut self, key: &VolumeKey) {
		let Some(waiters) = self.keys.get(key) else {
			return;
		};
		if !waiters.callers.is_empty() {
			return;
		}
		let wait = waiters.wait.clone();
		self.keys.remove(key);
		self.unindex(key, &wait);
	}

	/// The oldest waiting open of `key`.
	fn front(&self, key: &VolumeKey) -> Option<&OpenVolume> {
		self.keys.get(key)?.callers.front()
	}

	fn pop(&mut self, key: &VolumeKey) -> Option<OpenVolume> {
		let waiters = self.keys.get_mut(key)?;
		let caller = waiters.callers.pop_front()?;
		self.callers -= 1;
		self.release_if_empty(key);
		Some(caller)
	}

	/// Removes the open `id` of `key`, returning whether it waited here.
	fn remove(&mut self, key: &VolumeKey, id: RequestId) -> bool {
		let Some(waiters) = self.keys.get_mut(key) else {
			return false;
		};
		let Some(index) = waiters
			.callers
			.iter()
			.position(|caller| caller.request_id == id)
		else {
			return false;
		};
		waiters.callers.remove(index);
		self.callers -= 1;
		self.release_if_empty(key);
		true
	}
}

/// The group cache entries a key waiting for `wait` uses: the group it waits to read, and its
/// own group while it waits for a read or for admission again, whose lineage the walk uses.
fn used_groups(key: &VolumeKey, wait: &Wait) -> Vec<GroupId> {
	let own = depot_types::keys::partition::forkable_group(&key.partition_key);
	match wait {
		Wait::GroupRead(group) => own.into_iter().chain(std::iter::once(*group)).collect(),
		Wait::Admitting => own.into_iter().collect(),
		Wait::Job => Vec::new(),
	}
}

/// Whether a new open of `key` waits here instead of running its own admission: opens of the
/// key already wait, or a lazy fork job holds the key.
pub(crate) fn joins(state: &MaintainerState, key: &VolumeKey) -> bool {
	state.awaiting_lazy_fork.contains_key(key) || super::forks::holds_lazy(state, key)
}

/// Admits a new open of a key that `joins` named, behind the opens already waiting. With
/// `max_lazy_fork_waiters_per_volume` opens of the key already waiting it is refused as
/// overloaded, which is the only refusal for work in progress.
pub(crate) fn join(state: &mut MaintainerState, caller: OpenVolume, fx: &mut Effects) {
	let key = caller.volume_key.clone();
	let cap = state.config.max_lazy_fork_waiters_per_volume as usize;
	let waiting = state
		.awaiting_lazy_fork
		.keys
		.get(&key)
		.map_or(0, |waiters| waiters.callers.len());
	if waiting >= cap {
		super::lifecycle::reject_overloaded(caller.request_id, "lazy_fork_waiters_full", fx);
		return;
	}
	if waiting + 1 == cap {
		tracing::warn!(
			?key,
			cap,
			"a grouped volume's waiting opens reached depot.groups.max_open_waiters_per_volume; more opens of it are refused until its lazy fork or group read ends"
		);
	}
	let wait = state
		.awaiting_lazy_fork
		.keys
		.get(&key)
		.map_or(Wait::Job, |waiters| waiters.wait.clone());
	super::lifecycle::arm_lazy_fork_deadline(state, &caller, fx);
	state.awaiting_lazy_fork.push(key, caller, wait, End::Back);
}

/// Parks an admitted open that found its key needs `wait` first. The open came through
/// admission, so it is never refused here. A new open finds no other open of its key waiting
/// (it would have joined them), so this open is either alone or one admitted again from the
/// queue, which goes back to the front.
pub(crate) fn park(state: &mut MaintainerState, caller: OpenVolume, wait: Wait, fx: &mut Effects) {
	super::lifecycle::arm_lazy_fork_deadline(state, &caller, fx);
	state
		.awaiting_lazy_fork
		.push(caller.volume_key.clone(), caller, wait, End::Front);
}

/// The fork job of `key` ended, having published the volume's record or found the volume
/// absent: its waiting opens are admitted again.
pub(crate) fn job_ended(state: &mut MaintainerState, key: &VolumeKey) {
	state.awaiting_lazy_fork.set_wait(key, Wait::Admitting);
}

/// Removes and returns the opens of `key` that wait for its lazy fork job, so they can be
/// answered now: the job's parent answered `TailUnproven`, and the opener can have that tail
/// proven at once instead of at its own deadline. Opens of `key` waiting for anything else
/// stay.
pub(crate) fn take_job_waiters(state: &mut MaintainerState, key: &VolumeKey) -> Vec<OpenVolume> {
	let waiters = &mut state.awaiting_lazy_fork;
	if waiters.keys.get(key).map(|waiters| &waiters.wait) != Some(&Wait::Job) {
		return Vec::new();
	}
	let mut taken = Vec::new();
	while let Some(caller) = waiters.pop(key) {
		taken.push(caller);
	}
	taken
}

/// The read of `group` ended, or its cache entry left: the opens waiting for it are admitted
/// again. Each walks its lineage once more and waits again for the next unread ancestor, if
/// any. Visits only the keys waiting for `group`.
pub(crate) fn group_read_ended(state: &mut MaintainerState, group: GroupId) {
	let Some(released) = state.awaiting_lazy_fork.by_group.get(&group).cloned() else {
		return;
	};
	for key in released {
		state.awaiting_lazy_fork.set_wait(&key, Wait::Admitting);
	}
}

/// An expired open `id` of `key` leaves, returning whether it waited here. The other opens of
/// the key keep waiting.
pub(crate) fn expired(state: &mut MaintainerState, key: &VolumeKey, id: RequestId) -> bool {
	state.awaiting_lazy_fork.remove(key, id)
}

/// Events or finished operations named `keys`: those parked for work on the key are offered
/// at the next drive.
pub(crate) fn keys_marked<'a>(
	state: &mut MaintainerState,
	keys: impl IntoIterator<Item = &'a VolumeKey>,
) {
	let waiters = &mut state.awaiting_lazy_fork;
	if waiters.parked_key.is_empty() {
		return;
	}
	for key in keys {
		waiters.wake(key, Parked::Key);
	}
}

/// An event that may change every key, such as an assignment update, happened: every key
/// parked for work on the key is offered at the next drive.
pub(crate) fn all_marked(state: &mut MaintainerState) {
	let waiters = &mut state.awaiting_lazy_fork;
	for key in std::mem::take(&mut waiters.parked_key) {
		waiters.parked.remove(&key);
		waiters.admitting.insert(key);
	}
}

/// The open in flight on `key` left, so a key parked for it is offered at the next drive.
pub(crate) fn key_freed(state: &mut MaintainerState, key: &VolumeKey) {
	state.awaiting_lazy_fork.wake(key, Parked::Key);
}

/// The owner handoff ended, so every key parked for it is offered at the next drive.
pub(crate) fn handoff_ended(state: &mut MaintainerState) {
	let waiters = &mut state.awaiting_lazy_fork;
	for key in std::mem::take(&mut waiters.parked_handoff) {
		waiters.parked.remove(&key);
		waiters.admitting.insert(key);
	}
}

/// Admits again the opens of every key offered for admission, one at a time per key, oldest
/// first, and while the request and row read budgets have room, the keys parked for them, one
/// at a time. A key whose oldest open admission refuses for a transient reason parks until
/// the event that clears it; a waiting open is never refused for one. The per-key pass stops
/// at the first open admitted into work in flight, since admission refuses a second open of a
/// key while one is in flight, and parks the key until that work ends. Runs after every event
/// and costs the keys some event woke, never a scan of the waiting keys.
pub(crate) fn drive(state: &mut MaintainerState, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	let keys = state
		.awaiting_lazy_fork
		.admitting
		.iter()
		.cloned()
		.collect::<Vec<_>>();
	for key in keys {
		admit_key(state, &key, fx);
	}
	// A budget with room takes the parked keys in order until one is refused for it again.
	while let Some(key) = state.awaiting_lazy_fork.parked_capacity.first().cloned() {
		if super::lifecycle::capacity_full(state, super::lifecycle::BudgetSlot::Held).is_some() {
			break;
		}
		state.awaiting_lazy_fork.wake(&key, Parked::Capacity);
		admit_key(state, &key, fx);
		if state.awaiting_lazy_fork.parked.get(&key) == Some(&Parked::Capacity) {
			break;
		}
	}
}

/// Admits the opens of `key`, offered for admission, oldest first, until one goes into work
/// in flight or admission refuses one for a transient reason; see [`drive`].
fn admit_key(state: &mut MaintainerState, key: &VolumeKey, fx: &mut Effects) {
	// An admission answered at once (a lasting refusal such as an archived group) leaves the
	// key free for the next open; one that waits again moves the key out of `admitting`.
	while state.awaiting_lazy_fork.admitting.contains(key) {
		let Some(front) = state.awaiting_lazy_fork.front(key).cloned() else {
			break;
		};
		// The front open is still queued and counted in the request budget; popping it for
		// `admit_open` frees that slot first.
		if let Some(reason) =
			super::lifecycle::not_now(state, &front, super::lifecycle::BudgetSlot::Held)
		{
			state.awaiting_lazy_fork.park(key, Parked::of(reason));
			break;
		}
		let Some(caller) = state.awaiting_lazy_fork.pop(key) else {
			break;
		};
		let id = caller.request_id;
		super::lifecycle::admit_open(state, caller, fx);
		if in_flight(state, key, id) {
			// The key's next open waits for this one, which frees the key when it leaves.
			if state.awaiting_lazy_fork.admitting.contains(key) {
				state.awaiting_lazy_fork.park(key, Parked::Key);
			}
			break;
		}
	}
}

/// Whether the open `id` of `key`, just admitted, is in flight or waiting rather than
/// answered.
fn in_flight(state: &MaintainerState, key: &VolumeKey, id: RequestId) -> bool {
	state
		.pending
		.get(key)
		.is_some_and(|pending| pending.caller.request_id == id)
		|| state.awaiting_lazy_fork.holds(key, id)
		|| state
			.awaiting_deletion
			.get(key)
			.is_some_and(|caller| caller.request_id == id)
		|| super::forks::holds_key(state, key)
}
