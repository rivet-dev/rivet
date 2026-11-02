//! Watches on other worker processes, for low-urgency cleanup only.
//!
//! A watch resolves once one worker process is gone: its lease expired, it released its
//! heartbeat, a newer process of the same worker claimed the heartbeat, or the heartbeat was
//! garbage collected. A gone process never comes back, so a fired watch is final.
//!
//! The state machine keeps one watch per watched process however many subscribers hold it;
//! the `Topology` handle coalesces subscribers and sends `WatchRequest` for the first and
//! `UnwatchRequest` after the last one drops. Detection depends on the watched worker's role:
//!
//! ```text
//! storage worker    every discovery listing already reads its heartbeat: judged there, no I/O
//! stateless worker  never listed: one exact-key heartbeat read per process when the watch
//!                   starts and then per `watch.poll_interval_ms`, shared by every subscriber
//! ```
//!
//! Detection is deliberately slow and is never the evidence that makes a fence safe. Callers
//! use a watch to clean up after a crashed process, such as unloading the volumes it left
//! open, with actions that stay correct even if the process were alive; anything that decides
//! ownership or correctness must read fresh lease evidence instead.
use super::types::*;
use super::{effect::*, state::*};
use crate::heartbeat::{Observation, Read};
use crate::worker::{WorkerId, WorkerInstanceId, WorkerRole};
use crate::worker_lease::{WorkerLease, judge};
use rivet_state_machine::IndexedDeadlineQueue;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Polls started per `Timer::WatchPoll` delivery. Due processes past it stay queued and the
/// timer fires again at once, so one delivery's work stays bounded.
const POLLS_PER_TIMER: usize = 256;

/// One worker process: the worker ID and the incarnation of the process that ran under it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WatchedProcess {
	pub worker_id: WorkerId,
	pub instance_id: WorkerInstanceId,
}

/// How a watched process is found gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Detection {
	/// A storage worker. Each discovery listing judges it, with no extra request.
	Listing,
	/// A stateless worker, or one not listed yet. `Timer::WatchPoll` reads its heartbeat by
	/// exact key once per interval. `reading` is the generation of the read in flight, if any;
	/// a response with another generation is stale and ignored.
	Polling { reading: Option<u64> },
}

/// One watched process, shared by every subscriber of the `Topology` handle.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Watch {
	/// The handle's subscription for this process. An `UnwatchRequest` for an older
	/// subscription is ignored, so a quick unwatch and rewatch never drops the new watch.
	pub subscription: u64,
	pub detection: Detection,
	/// Only a listing issued after this instant judges the watch. An older listing may predate
	/// the watched process: it can still show the previous process of the worker, or no
	/// heartbeat at all, while the watched one is alive. Set when the watch starts and when a
	/// poll moves it to listing detection.
	pub judged_after_boot_ms: u64,
}

/// Why a watched process counts as gone. Each one is final for that process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GoneReason {
	/// Its heartbeat is older than the takeover bound.
	Expired,
	/// It released its heartbeat at a clean shutdown.
	Released,
	/// A newer process of the same worker claimed the heartbeat.
	Replaced,
	/// No heartbeat exists in either role. Garbage collection removes only long-expired ones.
	Missing,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WatchState {
	/// One entry per watched process.
	pub watches: BTreeMap<WatchedProcess, Watch>,
	/// The `Listing` subset of `watches`, so a listing visits only storage watches.
	pub(super) listed: BTreeSet<WatchedProcess>,
	/// The next poll of each `Polling` watch.
	pub(super) polls: IndexedDeadlineQueue<WatchedProcess>,
	pub(super) next_generation: u64,
	/// Poll reads in flight, at most `watch.poll_concurrency`. A due poll above it waits in
	/// the queue for a response to free a read, so every due process is read in turn.
	pub(super) reading: usize,
}

/// **WatchRequest:** starts watching `process`, or adopts a new subscription for it.
pub(super) fn handle_watch_request(
	state: &mut State,
	process: WatchedProcess,
	subscription: u64,
	now_boot_ms: u64,
) {
	if let Some(watch) = state.watches.watches.get_mut(&process) {
		watch.subscription = subscription;
		return;
	}

	// A worker the current view lists is a storage worker, judged by the listings that start
	// from now on; any other is polled. The view itself may predate the watched process, so it
	// judges nothing.
	let listed = state
		.discovery
		.entries
		.iter()
		.flatten()
		.any(|entry| entry.worker_id == process.worker_id && entry.role == WorkerRole::Storage);
	let detection = if listed {
		state.watches.listed.insert(process.clone());
		Detection::Listing
	} else {
		// The first read is due at once. A caller usually watches a process it has not heard
		// from in a while, and a crashed storage worker has already left the listed view, so
		// waiting a full interval first would only delay the cleanup the watch exists for.
		state.watches.polls.schedule(process.clone(), now_boot_ms);
		Detection::Polling { reading: None }
	};
	state.watches.watches.insert(
		process,
		Watch {
			subscription,
			detection,
			judged_after_boot_ms: now_boot_ms,
		},
	);
}

/// **UnwatchRequest:** removes the watch when it still carries `subscription`.
pub(super) fn handle_unwatch_request(
	state: &mut State,
	process: WatchedProcess,
	subscription: u64,
) {
	let current = state
		.watches
		.watches
		.get(&process)
		.is_some_and(|watch| watch.subscription == subscription);
	if current {
		remove(state, &process);
	}
}

/// **Timer::WatchPoll:** reads each due stateless process's heartbeat once.
pub(super) fn polls_due(state: &mut State, now_boot_ms: u64, out: &mut Effects) {
	let free = state
		.config
		.watch
		.poll_concurrency
		.saturating_sub(state.watches.reading);
	let due = state
		.watches
		.polls
		.pop_due(now_boot_ms, POLLS_PER_TIMER.min(free));
	let interval_ms = state.config.watch.poll_interval_ms;
	let deadline_boot_ms = now_boot_ms.saturating_add(state.config.lease.io_timeout_ms);
	for (_, process) in due {
		state.watches.next_generation = state
			.watches
			.next_generation
			.checked_add(1)
			.expect("watch generation exhausted");
		let generation = state.watches.next_generation;
		let Some(watch) = state.watches.watches.get_mut(&process) else {
			continue;
		};
		if let Detection::Polling { reading: Some(_) } = watch.detection {
			state
				.watches
				.polls
				.schedule(process, now_boot_ms.saturating_add(interval_ms));
			continue;
		}
		// A process has at most one read in flight; the executor always answers it, by its
		// deadline at the latest.
		watch.detection = Detection::Polling {
			reading: Some(generation),
		};
		state.watches.reading += 1;
		state
			.watches
			.polls
			.schedule(process.clone(), now_boot_ms.saturating_add(interval_ms));
		out.execute(Effect::ReadWatchedHeartbeatRequest {
			process,
			generation,
			deadline_boot_ms,
		});
	}
}

/// **ReadWatchedHeartbeatResponse:** judges a polled process from its heartbeat.
pub(super) fn handle_read_watched_heartbeat_response(
	state: &mut State,
	process: WatchedProcess,
	generation: u64,
	result: Read,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	let current = state.watches.watches.get(&process).is_some_and(|watch| {
		watch.detection
			== Detection::Polling {
				reading: Some(generation),
			}
	});
	if !current {
		return;
	}
	if let Some(watch) = state.watches.watches.get_mut(&process) {
		watch.detection = Detection::Polling { reading: None };
	}
	state.watches.reading -= 1;

	let reason = match &result {
		Read::Found(observation) => {
			// A storage worker is judged by every listing from now on, with no more polls.
			if observation.role == WorkerRole::Storage {
				if let Some(watch) = state.watches.watches.get_mut(&process) {
					watch.detection = Detection::Listing;
					watch.judged_after_boot_ms = now_boot_ms;
				}
				state.watches.polls.cancel(&process);
				state.watches.listed.insert(process.clone());
			}
			gone_reason(state, &process, Some(observation), now_boot_ms)
		}
		Read::Missing => gone_reason(state, &process, None, now_boot_ms),
		Read::Unknown => None,
	};
	if let Some(reason) = reason {
		fire(state, &process, reason, out);
	}
}

/// Judges every storage watch against a successful listing, before discovery filters it.
/// `entries` is sorted by worker ID.
pub(super) fn listing_observed(
	state: &mut State,
	entries: &[Observation],
	issued_boot_ms: u64,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	if state.watches.listed.is_empty() {
		return;
	}
	let mut gone = Vec::new();
	for process in &state.watches.listed {
		let judged = state
			.watches
			.watches
			.get(process)
			.is_some_and(|watch| issued_boot_ms > watch.judged_after_boot_ms);
		if !judged {
			continue;
		}
		let entry = entries
			.binary_search_by(|entry| entry.worker_id.cmp(&process.worker_id))
			.ok()
			.map(|index| &entries[index]);
		if let Some(reason) = gone_reason(state, process, entry, now_boot_ms) {
			gone.push((process.clone(), reason));
		}
	}
	for (process, reason) in gone {
		fire(state, &process, reason, out);
	}
}

/// Whether `observation`, the watched worker's heartbeat or `None` when it has none, shows
/// `process` gone. An expiry judgement needs store time; without it the process is kept.
fn gone_reason(
	state: &State,
	process: &WatchedProcess,
	observation: Option<&Observation>,
	now_boot_ms: u64,
) -> Option<GoneReason> {
	let Some(observation) = observation else {
		return Some(GoneReason::Missing);
	};
	if WorkerInstanceId::from_bytes(observation.heartbeat.instance_id) != process.instance_id {
		return Some(GoneReason::Replaced);
	}
	let store_now = state
		.discovery
		.store_clock
		.as_ref()
		.map(|clock| clock.lower_bound(now_boot_ms));
	match judge(
		observation.heartbeat.state,
		observation.modified_unix_ms,
		store_now,
		&state.config.lease,
	) {
		WorkerLease::Expired => Some(GoneReason::Expired),
		WorkerLease::Released => Some(GoneReason::Released),
		WorkerLease::Live | WorkerLease::Unknown => None,
	}
}

/// Resolves a gone process's watch for every subscriber and forgets it.
fn fire(state: &mut State, process: &WatchedProcess, reason: GoneReason, out: &mut Effects) {
	let Some(watch) = remove(state, process) else {
		return;
	};
	out.execute(Effect::WatchedProcessGone {
		process: process.clone(),
		subscription: watch.subscription,
		reason,
	});
}

fn remove(state: &mut State, process: &WatchedProcess) -> Option<Watch> {
	let watch = state.watches.watches.remove(process)?;
	if let Detection::Polling { reading: Some(_) } = watch.detection {
		state.watches.reading -= 1;
	}
	state.watches.listed.remove(process);
	state.watches.polls.cancel(process);
	Some(watch)
}

/// Keeps `Timer::WatchPoll` at the head of the poll queue while a read is free. With every
/// read in flight the timer waits for a response, which arms it again.
pub(super) fn arm(state: &State, out: &mut Effects) {
	let free = state.watches.reading < state.config.watch.poll_concurrency;
	match state.watches.polls.next_at() {
		Some(at) if free => out.arm_timer_at(Timer::WatchPoll, at),
		Some(_) | None => out.cancel_timer(Timer::WatchPoll),
	}
}

/// Drops every watch when topology stops. Subscribers are never told a process is gone on
/// account of this worker stopping; their handles simply never resolve.
pub(super) fn stop(state: &mut State, out: &mut Effects) {
	state.watches = WatchState::default();
	out.cancel_timer(Timer::WatchPoll);
}

/// Every polling watch has one queued poll, and `listed` is exactly the listing watches.
pub(super) fn validate(state: &State) -> anyhow::Result<()> {
	for (process, watch) in &state.watches.watches {
		match watch.detection {
			Detection::Listing => anyhow::ensure!(
				state.watches.listed.contains(process)
					&& state.watches.polls.scheduled_at(process).is_none(),
				"listing watch is not tracked as listed"
			),
			Detection::Polling { .. } => anyhow::ensure!(
				!state.watches.listed.contains(process)
					&& state.watches.polls.scheduled_at(process).is_some(),
				"polling watch has no scheduled poll"
			),
		}
	}
	anyhow::ensure!(
		state.watches.listed.len() + state.watches.polls.len() == state.watches.watches.len(),
		"watch indexes do not match the watches"
	);
	let reading = state
		.watches
		.watches
		.values()
		.filter(|watch| matches!(watch.detection, Detection::Polling { reading: Some(_) }))
		.count();
	anyhow::ensure!(
		reading == state.watches.reading && reading <= state.config.watch.poll_concurrency,
		"watch reads in flight do not match the watches"
	);
	Ok(())
}
