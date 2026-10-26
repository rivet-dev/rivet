//! Replica status observation. Log shards push every synced change as a notice and
//! resend a notice whose delivery failed, so a loaded volume is polled only while an
//! observation of it is needed: once after it loads, on a targeted request (a cold
//! request, repair, open, close, client hint or a notice this maintainer had to refuse),
//! and at `status_poll_ms` while a commit waits on a failed read source. An idle volume
//! costs no status reads. Callers that need a fresh observation of every loaded volume
//! request a barrier round and wait for its observation generation; callers that need only
//! one volume request a volume barrier, which reads that volume from every member and
//! costs the same however many volumes the tablet holds.

use super::{MaintainerPhase, MaintainerState, Operation, cold, volume_keys::KeyId};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_storage_worker_protocol_v1::{self as wire, RejectResponse, StorageWorkerResponse},
	shared_depot_types_v1::VolumeLogReplicaStatus,
};
use rivet_state_machine::DeadlineQueue;
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{Excluded, Unbounded};

/// Progress of the barrier round, the only status work that visits every loaded volume.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum PollCycle {
	/// No barrier round is running. Every requested generation has been observed.
	#[default]
	Idle,
	/// Visiting loaded volumes in key order for barrier `generation`, one chunk of keys
	/// per status request. `chunk` is being sent to members in order, starting at
	/// `next_member`; `after` is the last key the chunk covered, where the next chunk
	/// starts. `outstanding` counts this round's requests still in flight.
	Scanning {
		generation: u64,
		after: Option<VolumeLogKey>,
		chunk: Vec<VolumeLogKey>,
		next_member: usize,
		outstanding: u32,
	},
	/// Every chunk of barrier `generation` is sent; its `outstanding` requests remain.
	WaitingForReplies { generation: u64, outstanding: u32 },
}

/// The barrier a batched status read belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PollBarrier {
	/// The barrier round of this generation over every loaded volume.
	Round(u64),
	/// Volume barriers. Each key's progress is in `StatusPolls::volume_barriers`.
	Volumes,
}

/// A caller's wait for a fresh observation of one volume from every member, begun after
/// the request. `refresh_volume` returns it and `volume_refreshed` answers it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct VolumeBarrier {
	pub(crate) key: VolumeLogKey,
	generation: u64,
}

/// Progress of the volume barriers of one key. The entry exists only while `requested`
/// is above `observed`, so a missing entry means every barrier of the key finished.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct VolumeBarriers {
	/// The highest generation a caller waits for.
	requested: u64,
	/// The highest generation whose reads all completed.
	observed: u64,
	/// The reads in flight, or `None` while the key waits in `volume_queue`.
	reading: Option<VolumeReads>,
}

/// One volume barrier's reads of a key: it is in `volume_sending` until sent to every
/// member, and `outstanding` of the sent reads have not completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct VolumeReads {
	generation: u64,
	outstanding: u32,
}

/// One volume's place in the status schedule. `check_at` is the authority for the volume's
/// entry in `StatusPolls::queue`; `None` means no observation of the volume is due. Every
/// insertion into `loaded_volumes` schedules one poll through `track_loaded`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StatusSchedule {
	/// The interval the next scheduled poll waits after the current one is sent.
	pub interval_ms: u64,
	pub check_at: Option<u64>,
}

/// Scheduling state for status polls and barrier rounds.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct StatusPolls {
	/// Check times of loaded volumes by interned key id, ordered by time. Each entry matches
	/// its volume's `StatusSchedule::check_at`.
	queue: DeadlineQueue<KeyId>,
	/// The time `MaintainerTimer::Poll` is armed for, kept so an unchanged queue head
	/// emits no timer effect.
	armed_at: Option<u64>,
	/// Targeted and due volumes taken from the schedule, being sent to members in order
	/// starting at `next_member`. Their next checks are already scheduled.
	sending: Option<Chunk>,
	/// The highest barrier generation a caller is waiting for.
	pub(crate) requested_generation: u64,
	/// The highest barrier generation whose round has finished.
	pub(crate) observed_generation: u64,
	/// When a poll last found a change that no notice reported, for a rate-limited log.
	unnoticed_change_logged_at: Option<u64>,
	/// The last volume barrier generation handed out. Generations of all keys share it,
	/// so a later request always waits for reads begun after it.
	volume_generation: u64,
	/// Keys with an unfinished volume barrier.
	volume_barriers: BTreeMap<VolumeLogKey, VolumeBarriers>,
	/// Keys whose next volume barrier reads are not started yet.
	volume_queue: BTreeSet<VolumeLogKey>,
	/// Volume barrier reads being sent to members in order, starting at `next_member`.
	volume_sending: Option<Chunk>,
}

/// Where a replica status observation came from, for the observation counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
	Notice,
	Poll,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Chunk {
	keys: Vec<VolumeLogKey>,
	next_member: usize,
}

/// Requests a barrier round that observes every loaded volume after this call, and
/// returns its generation. A round already scanning may have passed some volumes, so
/// the request waits for the next one; repeated requests share it.
pub(crate) fn refresh_all(state: &mut MaintainerState, effects: &mut Effects) -> u64 {
	let generation = match &state.poll {
		PollCycle::Idle => state.status_polls.observed_generation + 1,
		PollCycle::Scanning { generation, .. }
		| PollCycle::WaitingForReplies { generation, .. } => generation + 1,
	};
	state.status_polls.requested_generation =
		state.status_polls.requested_generation.max(generation);
	begin_round(state);
	drive(state, effects);
	generation
}

/// Requests a barrier that observes one loaded volume from every member after this call.
/// Repeated requests for a key share its next reads. Callers that need only one volume
/// use this instead of `refresh_all`, whose cost grows with every loaded volume.
pub(crate) fn refresh_volume(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	effects: &mut Effects,
) -> VolumeBarrier {
	let polls = &mut state.status_polls;
	polls.volume_generation += 1;
	let generation = polls.volume_generation;
	let barriers = polls
		.volume_barriers
		.entry(key.clone())
		.or_insert(VolumeBarriers {
			requested: 0,
			observed: 0,
			reading: None,
		});
	barriers.requested = generation;
	if barriers.reading.is_none() {
		polls.volume_queue.insert(key.clone());
	}
	drive(state, effects);
	VolumeBarrier { key, generation }
}

/// Whether a volume barrier has finished.
pub(crate) fn volume_refreshed(state: &MaintainerState, barrier: &VolumeBarrier) -> bool {
	state
		.status_polls
		.volume_barriers
		.get(&barrier.key)
		.is_none_or(|barriers| barriers.observed >= barrier.generation)
}

/// Finishes a key's volume barrier reads once all are sent and completed, then queues
/// its next reads if a later request is waiting, or drops the entry.
fn settle_volume(state: &mut MaintainerState, key: &VolumeLogKey) {
	super::catch_up::mark(state, key);
	let polls = &mut state.status_polls;
	if polls
		.volume_sending
		.as_ref()
		.is_some_and(|chunk| chunk.keys.contains(key))
	{
		return;
	}
	let Some(barriers) = polls.volume_barriers.get_mut(key) else {
		return;
	};
	let Some(reads) = barriers.reading else {
		return;
	};
	if reads.outstanding > 0 {
		return;
	}
	barriers.reading = None;
	barriers.observed = reads.generation;
	if barriers.requested > barriers.observed {
		polls.volume_queue.insert(key.clone());
	} else {
		polls.volume_barriers.remove(key);
	}
}

/// Sends volume barrier reads within the shared status window: each chunk of queued
/// keys goes to every member, one request at a time. Returns the updated window use.
fn drive_volume_barriers(
	state: &mut MaintainerState,
	members: &[WorkerId],
	mut active: usize,
	effects: &mut Effects,
) -> usize {
	let batch = state.config.tuning().max_status_batch_volumes as usize;
	let max_requests = state.config.tuning().max_status_requests as usize;
	while active < max_requests {
		if let Some(chunk) = &mut state.status_polls.volume_sending
			&& chunk.next_member < members.len()
		{
			let keys = chunk.keys.clone();
			let worker = members[chunk.next_member].clone();
			chunk.next_member += 1;
			for key in &keys {
				if let Some(reads) = state
					.status_polls
					.volume_barriers
					.get_mut(key)
					.and_then(|barriers| barriers.reading.as_mut())
				{
					reads.outstanding += 1;
				}
			}
			send(state, keys, worker, Some(PollBarrier::Volumes), effects);
			active += 1;
			continue;
		}

		// The previous chunk is sent to every member; its keys finish as replies arrive.
		if let Some(chunk) = state.status_polls.volume_sending.take() {
			for key in &chunk.keys {
				settle_volume(state, key);
			}
		}

		// Start the next chunk. A key that cannot be observed now finishes at once, as
		// a barrier round skips it.
		let mut keys = Vec::new();
		while keys.len() < batch {
			let Some(key) = state.status_polls.volume_queue.pop_first() else {
				break;
			};
			let pollable = pollable(state, &key);
			let Some(barriers) = state.status_polls.volume_barriers.get_mut(&key) else {
				continue;
			};
			if pollable {
				barriers.reading = Some(VolumeReads {
					generation: barriers.requested,
					outstanding: 0,
				});
				keys.push(key);
			} else {
				state.status_polls.volume_barriers.remove(&key);
			}
		}
		if keys.is_empty() {
			break;
		}
		state.status_polls.volume_sending = Some(Chunk {
			keys,
			next_member: 0,
		});
	}
	active
}

/// Whether the barrier round of `generation` has finished.
pub(crate) fn refreshed(state: &MaintainerState, generation: u64) -> bool {
	state.status_polls.observed_generation >= generation
}

/// Starts the next requested barrier round when none is running.
fn begin_round(state: &mut MaintainerState) {
	if state.phase != MaintainerPhase::Serving
		|| state.poll != PollCycle::Idle
		|| state.status_polls.requested_generation <= state.status_polls.observed_generation
	{
		return;
	}
	super::stop::started(state);
	state.poll = PollCycle::Scanning {
		generation: state.status_polls.requested_generation,
		after: None,
		chunk: Vec::new(),
		next_member: 0,
		outstanding: 0,
	};
}

/// Observes one volume's members now, without a barrier round. A commit notice names
/// its volume, so a round over every loaded volume would cost work per commit that
/// grows with the number of volumes the tablet holds. The hint also resets the
/// volume's schedule, because a volume that is being written is not quiet.
pub(crate) fn request(state: &mut MaintainerState, key: VolumeLogKey, effects: &mut Effects) {
	if state.phase == MaintainerPhase::Serving && state.loaded_volumes.contains_key(&key) {
		reset(state, &key);
		state.poll_targets.insert(key);
		drive(state, effects);
	}
}

/// Whether a targeted status probe of `key` is queued or in flight.
pub(crate) fn probing(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	state.poll_targets.contains(key)
		|| state.operations.values().any(
			|operation| matches!(operation, Operation::Poll { keys, .. } if keys.contains(key)),
		)
}

/// Schedules a newly loaded volume's first poll one base interval from now. A volume
/// that is already scheduled keeps its schedule.
pub(crate) fn track(state: &mut MaintainerState, key: &VolumeLogKey) {
	let now = state.now_boot_ms;
	let base = state.config.tuning().status_poll_ms;
	let Some(volume) = state.loaded_volumes.get_mut(key) else {
		return;
	};
	if volume.status.check_at.is_some() {
		return;
	}
	let at = now + base;
	volume.status = StatusSchedule {
		interval_ms: base,
		check_at: Some(at),
	};
	let id = state.volume_keys.intern(key);
	state.status_polls.queue.schedule(id, at);
}

/// Removes a loaded volume's schedule. Every removal from `loaded_volumes` calls this
/// first, so the queue holds only live volumes.
pub(crate) fn untrack(state: &mut MaintainerState, key: &VolumeLogKey) {
	if let Some(volume) = state.loaded_volumes.get_mut(key)
		&& let Some(at) = volume.status.check_at.take()
		&& let Some(id) = state.volume_keys.find(key)
	{
		state.status_polls.queue.cancel(&id, at);
	}
}

/// Returns a volume to the base interval and brings its next poll forward to at most
/// one base interval from now. Pushes, hints, opens, closes, repairs and replies that
/// changed an observation call this.
pub(crate) fn reset(state: &mut MaintainerState, key: &VolumeLogKey) {
	let now = state.now_boot_ms;
	let base = state.config.tuning().status_poll_ms;
	let id = state.volume_keys.intern(key);
	let Some(volume) = state.loaded_volumes.get_mut(key) else {
		return;
	};
	volume.status.interval_ms = base;
	let at = state
		.status_polls
		.queue
		.schedule_earliest(id, volume.status.check_at, now + base);
	volume.status.check_at = Some(at);
}

/// Resets every loaded volume, for a membership change that adds members nobody has
/// observed yet. It runs once per change.
pub(crate) fn reset_all(state: &mut MaintainerState) {
	let keys = state.loaded_volumes.keys().cloned().collect::<Vec<_>>();
	for key in keys {
		reset(state, &key);
	}
}

/// Makes every loaded volume due now, scheduling any that is not scheduled yet. Tests
/// that drive the poll timer at a fixed clock use this to stand in for elapsed intervals.
#[doc(hidden)]
pub fn expire_all(state: &mut MaintainerState) {
	let now = state.now_boot_ms;
	let keys = state.loaded_volumes.keys().cloned().collect::<Vec<_>>();
	for key in keys {
		let id = state.volume_keys.intern(&key);
		let volume = state.loaded_volumes.get_mut(&key).expect("loaded volume");
		if volume.status.interval_ms == 0 {
			volume.status.interval_ms = state.config.tuning().status_poll_ms;
		}
		state
			.status_polls
			.queue
			.replace(id, volume.status.check_at, now);
		volume.status.check_at = Some(now);
	}
}

/// Finishes every volume barrier as if all its reads were answered with unchanged
/// observations. Tests that stand in for the replies use this.
#[doc(hidden)]
pub fn finish_volume_barriers(state: &mut MaintainerState) {
	let polls = &mut state.status_polls;
	polls.volume_sending = None;
	polls.volume_queue.clear();
	polls.volume_barriers.clear();
}

/// Whether a volume must keep the base interval: a targeted observation is pending, or a
/// commit is waiting to be applied while a read source failed. Log shards push a notice
/// for every synced change, so a commit above the applied position or an apply in flight
/// needs no poll; only a failed source is restored by a fresh observation of its member,
/// which an idle volume may never get from a notice.
fn holds_base_interval(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	if state.poll_targets.contains(key) {
		return true;
	}
	let Some(volume) = state.filesystem.volumes.get(key) else {
		return false;
	};
	if volume.unavailable_sources.is_empty() {
		return false;
	}
	let applied = volume.applied.volume_seq;
	super::filesystem::known_committed(state, key) > applied
}

/// The interval until a volume's next check, or `None` when no check is due: the base
/// while an observation is needed or when members push no notices, otherwise nothing,
/// because notices report every change and a lost one is resent.
fn next_interval(state: &MaintainerState, key: &VolumeLogKey) -> Option<u64> {
	if state.config.tuning().poll_loaded_volumes || holds_base_interval(state, key) {
		Some(state.config.tuning().status_poll_ms)
	} else {
		None
	}
}

/// Reschedules a volume whose scheduled poll is being sent now, or leaves it unscheduled
/// when no further check is due.
fn backoff(state: &mut MaintainerState, key: &VolumeLogKey) {
	let now = state.now_boot_ms;
	let interval = next_interval(state, key);
	let Some(volume) = state.loaded_volumes.get_mut(key) else {
		return;
	};
	let Some(interval) = interval else {
		volume.status = StatusSchedule::default();
		return;
	};
	let at = now + interval;
	volume.status = StatusSchedule {
		interval_ms: interval,
		check_at: Some(at),
	};
	let id = state.volume_keys.intern(key);
	state.status_polls.queue.schedule(id, at);
}

/// An observation arrived, so the next check is the base from now while an observation is
/// still expected, and otherwise none. A sooner check that is already scheduled stays.
fn observed_now(state: &mut MaintainerState, key: &VolumeLogKey) {
	let now = state.now_boot_ms;
	let interval = next_interval(state, key);
	let Some(volume) = state.loaded_volumes.get_mut(key) else {
		return;
	};
	let Some(interval) = interval else {
		return;
	};
	let id = state.volume_keys.intern(key);
	let at = state
		.status_polls
		.queue
		.schedule_earliest(id, volume.status.check_at, now + interval);
	volume.status = StatusSchedule {
		interval_ms: interval,
		check_at: Some(at),
	};
}

/// Sends one batched status read to a member; its reply is handled by `completed`.
fn send(
	state: &mut MaintainerState,
	keys: Vec<VolumeLogKey>,
	worker: WorkerId,
	barrier: Option<PollBarrier>,
	effects: &mut Effects,
) {
	let volume_log_keys = keys.iter().map(|key| key.0.clone()).collect();
	let op_id = state.begin(
		Operation::Poll {
			replica_set_version: state.tablet.replica_set.tablet_replica_set_version,
			keys,
			worker: worker.clone(),
			barrier,
		},
		effects,
	);
	// The unfenced probe answers each key with its disk state, as a sequence-zero read
	// would, without loading any segment bytes.
	effects.execute(MaintainerEffect::ReplicaRequest {
		op_id,
		worker_id: worker,
		request: wire::StorageWorkerRequest::ReadVolumeLogStatusRequest(
			wire::ReadVolumeLogStatusRequest { volume_log_keys },
		),
	});
}

/// Whether a loaded volume can be observed now. Deleting and quiescing volumes are skipped.
fn pollable(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	state.loaded_volumes.contains_key(key) && !super::filesystem::retiring(state, key)
}

/// Collects the next batch of targeted and due volumes. Targets come first; due volumes
/// fill the rest of the batch and are rescheduled as they are taken. A target's scheduled
/// check is dropped as it is sent, since this poll is that observation; its reply schedules
/// the next one if an observation is still needed, and a failed poll resets it.
fn next_batch(state: &mut MaintainerState, batch: usize) -> Vec<VolumeLogKey> {
	let mut keys = Vec::new();
	let mut taken = BTreeSet::new();
	while keys.len() < batch {
		let Some(key) = state.poll_targets.pop_first() else {
			break;
		};
		if pollable(state, &key) && taken.insert(key.clone()) {
			if !state.config.tuning().poll_loaded_volumes {
				untrack(state, &key);
			}
			keys.push(key);
		}
	}
	while keys.len() < batch {
		let due = state
			.status_polls
			.queue
			.pop_due(state.now_boot_ms, batch - keys.len());
		if due.is_empty() {
			break;
		}
		for (_, id) in due {
			let Some(key) = state.volume_keys.key(id).cloned() else {
				continue;
			};
			if let Some(volume) = state.loaded_volumes.get_mut(&key) {
				volume.status.check_at = None;
			}
			backoff(state, &key);
			if pollable(state, &key) && taken.insert(key.clone()) {
				keys.push(key);
			}
		}
	}
	keys
}

/// Bounds outstanding status RPCs while continuing promptly as peers answer. Each RPC
/// carries up to `max_status_batch_volumes` keys, so polling costs a number of requests
/// proportional to due volumes divided by the batch size, not one per volume.
pub(crate) fn drive(state: &mut MaintainerState, effects: &mut Effects) {
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	let counts = state.operations.index().counts;
	let mut active = (counts.poll + counts.promotion_list) as usize;
	let batch = state.config.tuning().max_status_batch_volumes as usize;
	let max_requests = state.config.tuning().max_status_requests as usize;

	// Targeted and scheduled observations share the status budget with barrier rounds,
	// one request at a time. A barrier round visits every volume anyway, so they wait
	// only while it is scanning.
	let members = state.members();

	// Volume barriers wait on few keys, so they go first and do not wait for a round.
	active = drive_volume_barriers(state, &members, active, effects);

	while !matches!(state.poll, PollCycle::Scanning { .. }) && active < max_requests {
		if let Some(chunk) = &mut state.status_polls.sending
			&& chunk.next_member < members.len()
		{
			let keys = chunk.keys.clone();
			let worker = members[chunk.next_member].clone();
			chunk.next_member += 1;
			send(state, keys, worker, None, effects);
			active += 1;
			continue;
		}
		let keys = next_batch(state, batch);
		if keys.is_empty() {
			state.status_polls.sending = None;
			break;
		}
		state.status_polls.sending = Some(Chunk {
			keys,
			next_member: 0,
		});
	}

	// A barrier round sends each chunk of loaded volumes to every member, one request
	// at a time within the shared window.
	while active < max_requests {
		let PollCycle::Scanning {
			generation,
			after,
			chunk,
			next_member,
			outstanding,
		} = &mut state.poll
		else {
			break;
		};
		let generation = *generation;
		if *next_member < members.len() && !chunk.is_empty() {
			let keys = chunk.clone();
			let worker = members[*next_member].clone();
			*next_member += 1;
			*outstanding += 1;
			send(
				state,
				keys,
				worker,
				Some(PollBarrier::Round(generation)),
				effects,
			);
			active += 1;
			continue;
		}
		let outstanding = *outstanding;

		// Take the next chunk of pollable volumes after the previous one.
		let lower = match after {
			Some(key) => Excluded(key.clone()),
			None => Unbounded,
		};
		let mut visited = None;
		let mut keys = Vec::new();
		for key in state
			.loaded_volumes
			.range((lower, Unbounded))
			.map(|(key, _)| key)
		{
			if keys.len() >= batch {
				break;
			}
			visited = Some(key.clone());
			if pollable(state, key) {
				keys.push(key.clone());
			}
		}
		let Some(last) = visited else {
			state.poll = PollCycle::WaitingForReplies {
				generation,
				outstanding,
			};
			break;
		};
		for key in &keys {
			state.poll_targets.remove(key);
		}
		state.poll = PollCycle::Scanning {
			generation,
			after: Some(last),
			chunk: keys,
			next_member: 0,
			outstanding,
		};
	}
	if let PollCycle::WaitingForReplies {
		generation,
		outstanding: 0,
	} = state.poll
	{
		state.poll = PollCycle::Idle;
		state.status_polls.observed_generation = generation;
		super::stop::completed(state, effects);
		begin_round(state);
		if matches!(state.poll, PollCycle::Scanning { .. }) {
			drive(state, effects);
		}
	}
}

/// Arms `MaintainerTimer::Poll` at the head of the schedule after every event. A head
/// that is already due is not armed: those volumes wait for a free status slot or the
/// end of a barrier scan, and the completion that frees it drives polling again.
pub(crate) fn arm_head(state: &mut MaintainerState, effects: &mut Effects) {
	let wanted = match state.phase {
		MaintainerPhase::Serving => state
			.status_polls
			.queue
			.next_at()
			.filter(|at| *at > state.now_boot_ms),
		MaintainerPhase::Opening
		| MaintainerPhase::Frozen
		| MaintainerPhase::Draining
		| MaintainerPhase::Stopped
		| MaintainerPhase::Fenced => None,
	};
	if wanted == state.status_polls.armed_at {
		return;
	}
	match wanted {
		Some(at) => effects.arm_timer_at(MaintainerTimer::Poll, at),
		None => effects.cancel_timer(MaintainerTimer::Poll),
	}
	state.status_polls.armed_at = wanted;
}

/// The armed poll fired. Due volumes are sent by the drive that follows.
pub(crate) fn timer_fired(state: &mut MaintainerState) {
	state.status_polls.armed_at = None;
}

/// Clears the armed marker, the unsent chunks and the volume barriers, whose waiters
/// fencing rejects, when fencing cancels the timer and every read.
pub(crate) fn fence(state: &mut MaintainerState) {
	state.status_polls.armed_at = None;
	state.status_polls.sending = None;
	state.status_polls.volume_sending = None;
	state.status_polls.volume_queue.clear();
	state.status_polls.volume_barriers.clear();
}

/// Every queued check matches a loaded volume's `check_at`, and every scheduled volume
/// is queued.
pub(crate) fn validate(state: &MaintainerState) -> anyhow::Result<()> {
	for (at, id) in state.status_polls.queue.iter() {
		anyhow::ensure!(
			state
				.volume_keys
				.key(*id)
				.and_then(|key| state.loaded_volumes.get(key))
				.is_some_and(|volume| volume.status.check_at == Some(*at)),
			"status poll queued for a volume without a matching schedule"
		);
	}
	let scheduled = state
		.loaded_volumes
		.values()
		.filter(|volume| volume.status.check_at.is_some())
		.count();
	anyhow::ensure!(
		scheduled == state.status_polls.queue.len(),
		"scheduled volume missing from the status poll queue"
	);
	let polls = &state.status_polls;
	for (key, barriers) in &polls.volume_barriers {
		anyhow::ensure!(
			barriers.requested > barriers.observed,
			"finished volume barrier kept"
		);
		anyhow::ensure!(
			barriers.reading.is_some() != polls.volume_queue.contains(key),
			"volume barrier neither reading nor queued"
		);
	}
	for key in &polls.volume_queue {
		anyhow::ensure!(
			polls.volume_barriers.contains_key(key),
			"queued volume barrier without progress"
		);
	}
	Ok(())
}

/// Splits a batched status reply into one observation per key. A failed request or a reply
/// that does not answer every key counts as a failed source for each key. Keys that were
/// deleted or unloaded while the request was in flight are skipped.
pub(crate) fn completed(
	state: &mut MaintainerState,
	keys: Vec<VolumeLogKey>,
	worker: WorkerId,
	barrier: Option<PollBarrier>,
	result: Result<StorageWorkerResponse, String>,
	effects: &mut Effects,
) {
	// Counted on completion rather than in the drive, which the debug oracle repeats.
	crate::metrics::record_idle_visits("status_poll", keys.len());
	if barrier == Some(PollBarrier::Volumes) {
		for key in &keys {
			if let Some(reads) = state
				.status_polls
				.volume_barriers
				.get_mut(key)
				.and_then(|barriers| barriers.reading.as_mut())
			{
				reads.outstanding = reads.outstanding.saturating_sub(1);
			}
		}
	}
	let settle = match barrier {
		Some(PollBarrier::Volumes) => keys.clone(),
		Some(PollBarrier::Round(_)) | None => Vec::new(),
	};
	observe_reply(state, keys, worker, barrier, result, effects);
	// Observations are recorded before a waiter can see its barrier finish.
	for key in &settle {
		settle_volume(state, key);
	}
}

/// Records the observations of one batched status reply and counts it against its round.
fn observe_reply(
	state: &mut MaintainerState,
	keys: Vec<VolumeLogKey>,
	worker: WorkerId,
	barrier: Option<PollBarrier>,
	result: Result<StorageWorkerResponse, String>,
	effects: &mut Effects,
) {
	if let Some(PollBarrier::Round(generation)) = barrier
		&& let PollCycle::Scanning {
			generation: current,
			outstanding,
			..
		}
		| PollCycle::WaitingForReplies {
			generation: current,
			outstanding,
		} = &mut state.poll
		&& *current == generation
	{
		*outstanding -= 1;
	}
	let statuses = match result {
		Ok(StorageWorkerResponse::ReadVolumeLogStatusResponse(reply))
			if reply.statuses.len() == keys.len() =>
		{
			reply.statuses
		}
		Ok(_) | Err(_) => {
			super::catch_up::refresh_learner(state, &worker, effects);
			for key in keys {
				if pollable(state, &key) {
					reset(state, &key);
					super::filesystem::source_failed(state, &key, worker.clone());
				}
			}
			return;
		}
	};
	for (key, status) in keys.into_iter().zip(statuses) {
		if !pollable(state, &key) {
			continue;
		}
		let reason = match status {
			wire::VolumeLogStatus::BelowCold(reply) => RejectResponse::BelowCold(reply),
			wire::VolumeLogStatus::VolumeLogReplicaDeleted(reply) => {
				RejectResponse::VolumeLogReplicaDeleted(reply)
			}
			wire::VolumeLogStatus::VolumeNotLoaded => RejectResponse::VolumeNotLoaded,
			wire::VolumeLogStatus::NotServing => RejectResponse::NotServing,
		};
		// A quorum-confirmed close intentionally leaves no replica on the learner.
		// Its expected absence must not invalidate discovery for every live volume.
		// NotServing still needs fresh disk/phase evidence even for a closed key.
		if matches!(reason, RejectResponse::NotServing)
			|| (matches!(reason, RejectResponse::VolumeNotLoaded)
				&& super::unload::closed_through(state, &key).is_none())
		{
			super::catch_up::refresh_learner(state, &worker, effects);
		}
		observed(
			state,
			key.clone(),
			worker.clone(),
			Ok(StorageWorkerResponse::RejectResponse(reason)),
			Source::Poll,
			effects,
		);
		super::unload::status_observed(state, &key);
	}
}

/// Status replies are hints for selecting a read source; the actual read validates its
/// incarnation again. A reply that changed the member's observation, reported disk
/// pressure or failed resets the volume's schedule.
fn observed(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	worker: WorkerId,
	result: Result<StorageWorkerResponse, String>,
	source: Source,
	effects: &mut Effects,
) {
	let Ok(StorageWorkerResponse::RejectResponse(reason)) = result else {
		reset(state, &key);
		super::filesystem::source_failed(state, &key, worker);
		// An unload waiting for the close's probe goes on with what it observed.
		super::unload::mark(state, &key);
		return;
	};
	let previous = state
		.loaded_volumes
		.get(&key)
		.and_then(|volume| volume.members.get(&worker))
		.cloned();
	let hint = match &reason {
		RejectResponse::BelowCold(reply) => reply.cold_write.clone(),
		_ => None,
	};
	if hint.as_ref().is_some_and(|hint| hint.disk_pressure) {
		reset(state, &key);
	}
	let observed = match reason {
		RejectResponse::VolumeNotLoaded => {
			if let Some(volume) = state.loaded_volumes.get_mut(&key)
				&& volume.members.remove(&worker).is_some()
			{
				super::catch_up::mark(state, &key);
				reset(state, &key);
			}
			super::member_repair::absent(state, &key, &worker, effects);
			None
		}
		RejectResponse::BelowCold(reply) => Some(reply.state),
		RejectResponse::VolumeLogReplicaDeleted(reply) => Some(reply.state),
		RejectResponse::StaleClientEpoch(_)
		| RejectResponse::ClientConflict(_)
		| RejectResponse::NotClaimed(_)
		| RejectResponse::VolumeSeqGap(_)
		| RejectResponse::Diverged(_)
		| RejectResponse::AppendedMismatch(_)
		| RejectResponse::VolumeLogReplicaIncarnationMismatch(_)
		| RejectResponse::StaleTabletReplicaSet(_)
		| RejectResponse::StaleTabletGrantEpoch(_)
		| RejectResponse::Overloaded(_)
		| RejectResponse::NotServing
		| RejectResponse::RequestTooLarge(_)
		| RejectResponse::TooManyLoadedVolumes(_)
		| RejectResponse::DiskIncarnationMismatch(_)
		| RejectResponse::DiskFull(_)
		| RejectResponse::UnsupportedPayloadVersion(_) => None,
	};
	if let Some(observed) = observed {
		let changed = previous.as_ref() != Some(&observed);
		record_observation(state, source, changed);
		// A poll found a change no notice reported yet, so poll the volume at the base
		// interval until its polls find nothing new.
		if changed && matches!(source, Source::Poll) {
			reset(state, &key);
		}
		super::stop::observe(state, &key, &worker, &observed);
		let now_in_sync = observed.status == VolumeLogReplicaStatus::InSync
			&& previous
				.as_ref()
				.is_none_or(|previous| previous.status != VolumeLogReplicaStatus::InSync);
		cold::observe(state, &key, worker, observed.clone());
		if now_in_sync {
			report_missing_members(state, &key, effects);
		}
		if state.store_queue.has_waiting_pass() {
			cold::join_waiting_pass(state, key.clone());
		} else {
			cold::schedule(state, &key, &observed, hint, effects);
		}
		observed_now(state, &key);
		// An unload waiting for the close's probe decides on this observation.
		super::unload::mark(state, &key);
	} else {
		super::filesystem::source_failed(state, &key, worker);
	}
}

/// Reports every tablet member missing from loaded volume `key` to member repair. A member
/// becoming in sync is when a repair can start: repair copies from an in-sync quorum, so a
/// member dropped while too few others were in sync, such as one a late forget removed after
/// an open whose load could not install its claim, is reloaded here instead of waiting for
/// the volume to close. The cost is one check per member, only on this transition.
fn report_missing_members(state: &mut MaintainerState, key: &VolumeLogKey, effects: &mut Effects) {
	let missing = match state.loaded_volumes.get(key) {
		Some(volume) => state
			.members()
			.into_iter()
			.filter(|worker| !volume.members.contains_key(worker))
			.collect::<Vec<_>>(),
		None => return,
	};
	for worker in missing {
		super::member_repair::absent(state, key, &worker, effects);
	}
}

/// Applies one tablet's replica status notices from `worker`, after the checks a status
/// reply gets: the maintainer serves, the sender is a member, and the shard synced this
/// grant and replica set.
///
/// The shard counts a delivered notice as done, so a notice refused here for a grant or
/// replica set the shard has not caught up with polls its volumes instead: nothing else
/// would report those changes to an idle volume. Notices from a non-member are dropped, and
/// a maintainer that is not serving observes its volumes once it serves.
pub(crate) fn notices(
	state: &mut MaintainerState,
	worker: WorkerId,
	tablet_grant_epoch: u64,
	tablet_replica_set_version: u64,
	notices: Vec<StatusNotice>,
	effects: &mut Effects,
) {
	if state.phase != MaintainerPhase::Serving || !state.members().contains(&worker) {
		return;
	}
	if tablet_grant_epoch != state.tablet.tablet_grant_epoch
		|| tablet_replica_set_version != state.tablet.replica_set.tablet_replica_set_version
	{
		for notice in notices {
			request(state, notice.volume_log_key, effects);
		}
		return;
	}
	for notice in notices {
		let key = notice.volume_log_key;
		if !pollable(state, &key) {
			continue;
		}
		let carried = match &notice.state {
			Some(replica) if !notice.entries.is_empty() => Some((replica.clone(), notice.entries)),
			Some(_) | None => None,
		};
		let reason = match notice.state {
			Some(replica) if replica.status == VolumeLogReplicaStatus::Deleted => {
				RejectResponse::VolumeLogReplicaDeleted(wire::VolumeLogReplicaDeleted {
					state: replica,
				})
			}
			Some(replica) => RejectResponse::BelowCold(wire::BelowCold {
				cold_write: notice.cold_write,
				state: replica,
			}),
			None => RejectResponse::VolumeNotLoaded,
		};
		observed(
			state,
			key.clone(),
			worker.clone(),
			Ok(StorageWorkerResponse::RejectResponse(reason)),
			Source::Notice,
			effects,
		);
		super::unload::status_observed(state, &key);
		// The observation made the commit known, so its entries can start the apply now.
		if let Some((replica, entries)) = carried {
			super::filesystem::pushed(state, &key, &worker, &replica, entries, effects);
		}
	}
}

/// Counts an observation. A poll that finds a change no notice reported is counted and
/// logged at debug level, at most once a minute.
///
/// Such a poll almost always raced the notice rather than replaced a lost one: the shard
/// had the change marked, held it for `append_notice_delay_ms`, or had the notice in flight
/// when it answered; a refused notice requested the poll; the maintainer had no observation
/// of the member yet; or an older poll reply arrived after a newer notice. A notice whose
/// delivery fails is resent by the shard, so none of these strands a change, and they are
/// routine under load. The `poll_changed` count is the signal to watch, not this line.
fn record_observation(state: &mut MaintainerState, source: Source, changed: bool) {
	let label = match (source, changed) {
		(Source::Notice, _) => "notice",
		(Source::Poll, true) => "poll_changed",
		(Source::Poll, false) => "poll_unchanged",
	};
	crate::metrics::record_status_observation(label);
	if matches!((source, changed), (Source::Poll, true)) {
		let now = state.now_boot_ms;
		if state
			.status_polls
			.unnoticed_change_logged_at
			.is_none_or(|at| now.saturating_sub(at) >= 60_000)
		{
			state.status_polls.unnoticed_change_logged_at = Some(now);
			tracing::debug!(
				"replica status poll found a change no notice reported yet; usually the poll overtook a pending notice"
			);
		}
	}
}


