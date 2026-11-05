//! One finite replica-recovery barrier per joining process, under this tablet grant.
//!
//! Admission freezes the volume inventory and proven committed floor. Fresh InSync read
//! quorums bound the remaining history in writer-recovery order (epoch, then sequence).
//! Retries may lower that bound but never raise it: subsequent writes and opens belong
//! to ordinary storage lifecycle, not startup work. A later writer may supersede an
//! abandoned suffix, but neither hot nor cold completion can bypass the committed floor.
//! Recovery runs even without that quorum; permission still requires fresh target evidence.

use super::{MaintainerPhase, MaintainerState, filesystem, membership, poll, stop};
use crate::tablet_maintainer_machine::{Effects, types::RequestId};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_log_shard_v1::VolumeLogReplicaState,
	depot_replica_readiness_v1::ReplicaReadiness,
	depot_storage_worker_protocol_v1::{NotReady, StopReadinessResponse as Response},
	shared_depot_types_v1::{VolumeLogPosition, VolumeLogReplicaStatus},
};
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Goal {
	/// Proven committed data at admission; both sequence and epoch must be covered.
	committed: VolumeLogPosition,
	/// Least upper bound from a fresh voting read quorum, or none before one answers.
	/// Unlike committed data, an uncommitted suffix may be discarded by a later writer.
	bound: Option<VolumeLogPosition>,
}

/// Writer recovery selects the newest history by epoch before considering its length.
/// A newer appended epoch preserves prior commits; a newer claim alone proves no history.
fn history_order(position: &VolumeLogPosition) -> (u64, u64) {
	(position.client_epoch, position.volume_seq)
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Session {
	instance: [u8; 16],
	disk: [u8; 16],
	/// Empty is completed, retained for idempotent retry until this assignment ends.
	remaining: BTreeMap<VolumeLogKey, Goal>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) struct Checks {
	/// At most one session per original voter (at most three), never per HTTP probe.
	sessions: BTreeMap<WorkerId, Session>,
	pending: BTreeMap<RequestId, (WorkerId, u64)>,
	/// Only replies to this barrier's reads count; notices and older targeted polls
	/// can drive recovery but cannot substitute for fresh startup evidence.
	pub(super) observations: BTreeMap<VolumeLogKey, BTreeMap<WorkerId, VolumeLogReplicaState>>,
	/// An authoritative absence differs from an unanswered or NotServing read.
	pub(super) absent: BTreeMap<VolumeLogKey, BTreeSet<WorkerId>>,
}

fn tracking(state: &MaintainerState, generation: u64, key: &VolumeLogKey) -> bool {
	state
		.stop_checks
		.startup
		.pending
		.values()
		.any(|(_, requested)| *requested <= generation)
		&& state
			.stop_checks
			.startup
			.sessions
			.values()
			.any(|session| session.remaining.contains_key(key))
}

pub(super) fn observe(
	state: &mut MaintainerState,
	generation: u64,
	key: &VolumeLogKey,
	worker: &WorkerId,
	replica: &VolumeLogReplicaState,
) {
	if tracking(state, generation, key) {
		state
			.stop_checks
			.startup
			.observations
			.entry(key.clone())
			.or_default()
			.insert(worker.clone(), replica.clone());
	}
}

pub(super) fn absent(
	state: &mut MaintainerState,
	generation: u64,
	key: &VolumeLogKey,
	worker: &WorkerId,
) {
	if tracking(state, generation, key) {
		state
			.stop_checks
			.startup
			.absent
			.entry(key.clone())
			.or_default()
			.insert(worker.clone());
	}
}

pub(super) fn pending(state: &MaintainerState) -> usize {
	state.stop_checks.startup.pending.len()
}

pub(crate) fn admit(
	state: &mut MaintainerState,
	id: RequestId,
	request: ReplicaReadiness,
	fx: &mut Effects,
) {
	let worker = WorkerId::from_bytes(request.worker_id);
	if state.phase != MaintainerPhase::Serving
		|| request.tablet_id != state.tablet.id
		|| request.tablet_grant_epoch != state.tablet.tablet_grant_epoch
		|| request.tablet_replica_set_version != state.tablet.replica_set.tablet_replica_set_version
		|| !state
			.tablet
			.replica_set
			.members
			.contains(&request.worker_id)
		|| state.disk_incarnation(&worker) != Some(request.disk_incarnation)
		|| !state
			.tablet
			.replica_set
			.members
			.iter()
			.all(|worker| state.listed.contains(&WorkerId::from_bytes(*worker)))
	{
		stop::reply(fx, id, Response::NotServing);
		return;
	}
	if state.request_count() >= state.config.max_requests as usize {
		stop::reply(fx, id, stop::overloaded());
		return;
	}
	let reuse = state
		.stop_checks
		.startup
		.sessions
		.get(&worker)
		.is_some_and(|session| {
			session.instance == request.worker_incarnation
				&& session.disk == request.disk_incarnation
		});
	if !reuse {
		let remaining = state
			.loaded_volumes
			.keys()
			.map(|key| {
				(
					key.clone(),
					Goal {
						committed: filesystem::committed_prefix(state, key),
						bound: None,
					},
				)
			})
			.collect();
		state.stop_checks.startup.sessions.insert(
			worker.clone(),
			Session {
				instance: request.worker_incarnation,
				disk: request.disk_incarnation,
				remaining,
			},
		);
	}
	// A caller retry replaces its old waiter but preserves the finite session. This
	// bounds abandoned requests as well as recovery state when a transport times out.
	let replaced = state
		.stop_checks
		.startup
		.pending
		.iter()
		.filter(|(_, (target, _))| target == &worker)
		.map(|(id, _)| *id)
		.collect::<Vec<_>>();
	let mut in_flight = None;
	for old in replaced {
		let (_, generation) = state
			.stop_checks
			.startup
			.pending
			.remove(&old)
			.expect("startup waiter");
		if reuse {
			in_flight = Some(generation);
		}
		stop::reply(fx, old, Response::NotServing);
	}
	if state.stop_checks.startup.sessions[&worker]
		.remaining
		.is_empty()
	{
		stop::reply(fx, id, Response::Ready);
		return;
	}
	// A transport retry of the same session joins its existing barrier. Requiring
	// a newer round on every retry would starve a valid round longer than the RPC timeout.
	let generation = in_flight.unwrap_or_else(|| poll::refresh_all(state, fx));
	state
		.stop_checks
		.startup
		.pending
		.insert(id, (worker, generation));
	if poll::refreshed(state, generation) {
		stop::completed(state, fx);
	}
}

pub(super) fn completed(state: &mut MaintainerState, fx: &mut Effects) {
	let done = state
		.stop_checks
		.startup
		.pending
		.iter()
		.filter(|(_, (_, generation))| poll::refreshed(state, *generation))
		.map(|(id, (worker, _))| (*id, worker.clone()))
		.collect::<Vec<_>>();
	for (id, worker) in done {
		state.stop_checks.startup.pending.remove(&id);
		let mut session = state
			.stop_checks
			.startup
			.sessions
			.remove(&worker)
			.expect("startup session");
		session.remaining.retain(|key, goal| {
			let Some(volume) = state.loaded_volumes.get(key) else {
				return false;
			};
			if super::unload::closed_through(state, key).is_some()
				|| filesystem::is_record_missing(state, key)
			{
				return false;
			}
			let observations = state.stop_checks.startup.observations.get(key);
			if let Some(bound) = observed_bound(state, key)
				&& goal
					.bound
					.as_ref()
					.is_none_or(|previous| history_order(&bound) < history_order(previous))
			{
				// Every read quorum independently bounds the admission history. A
				// returning quorum can disprove an abandoned suffix; later appends
				// cannot turn a finite startup barrier into a moving target.
				goal.bound = Some(bound);
			}
			let observed = observations.and_then(|answers| answers.get(&worker));
			// Rebuilt nonvoters do not intersect historical committing quorums.
			// Cold coverage therefore needs every original voter freshly accounted
			// for: an unanswered peer can hide a hot suffix newer than its listing.
			let cold_observed = state.tablet.replica_set.members.iter().all(|member| {
				let member = WorkerId::from_bytes(*member);
				observations.is_some_and(|answers| answers.contains_key(&member))
					|| state
						.stop_checks
						.startup
						.absent
						.get(key)
						.is_some_and(|absent| absent.contains(&member))
			});
			let ready = observed.is_some_and(|replica| {
				if replica.appended.volume_seq < goal.committed.volume_seq
					|| replica.appended.client_epoch < goal.committed.client_epoch
				{
					return false;
				}
				let hot = !replica.lost_entries
					&& replica.status == VolumeLogReplicaStatus::InSync
					&& goal.bound.as_ref().is_some_and(|bound| {
						history_order(&replica.appended) >= history_order(bound)
					});
				hot || cold_observed
					&& stop::covered_by_cold(
						state,
						key,
						volume,
						observations,
						*worker.as_bytes(),
						replica,
					)
			});
			!ready
		});
		// Recovery does not require permission or a read quorum. In particular, a lone
		// intact cold replica must be able to load its record and reconstruct the joiner.
		for key in session.remaining.keys() {
			stop::recover_replica(state, key, &worker, fx);
		}
		let blocking_volumes = session.remaining.len() as u64;
		state.stop_checks.startup.sessions.insert(worker, session);
		stop::reply(
			fx,
			id,
			if blocking_volumes == 0 {
				Response::Ready
			} else {
				Response::NotReady(NotReady {
					blocking_volumes,
					sample: vec![],
				})
			},
		);
	}
}

/// A voting read quorum intersects every committing quorum. Its newest history therefore
/// bounds prior commits, including those whose other copy is unavailable. As in volume
/// client recovery, compare epoch before sequence: a later writer preserves committed
/// history but can discard an older writer's longer uncommitted tail. Prefer the least
/// bound over all available quorums. Rebuilt nonvoters require the separate cold proof.
fn observed_bound(state: &MaintainerState, key: &VolumeLogKey) -> Option<VolumeLogPosition> {
	let answers = state.stop_checks.startup.observations.get(key)?;
	let mut answers = answers
		.iter()
		.filter(|(worker, replica)| {
			// A rebuilt Repairing replica may omit acknowledgements of its previous
			// incarnation even with lost_entries=false. Only a separate cold proof
			// can count it; its tail cannot bound committed hot history.
			membership::voter(&state.tablet.replica_set, worker)
				&& !replica.lost_entries
				&& replica.status == VolumeLogReplicaStatus::InSync
		})
		.collect::<Vec<_>>();
	answers.sort_by_key(|(_, replica)| history_order(&replica.appended));
	let mut bounded = BTreeSet::new();
	for (worker, replica) in answers {
		bounded.insert(worker);
		if membership::quorum(&state.tablet.replica_set, |worker| bounded.contains(worker)) {
			return Some(replica.appended.clone());
		}
	}
	None
}

pub(super) fn cancel(state: &mut MaintainerState, fx: &mut Effects) {
	for (id, _) in std::mem::take(&mut state.stop_checks.startup.pending) {
		stop::reply(fx, id, Response::NotServing);
	}
	state.stop_checks.startup.sessions.clear();
	state.stop_checks.startup.observations.clear();
	state.stop_checks.startup.absent.clear();
}
