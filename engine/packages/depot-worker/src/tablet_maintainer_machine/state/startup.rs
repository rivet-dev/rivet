//! One finite replica-recovery barrier per joining process, under this tablet grant.
//!
//! Admission freezes the volume inventory and proven committed floor. Fresh InSync read
//! quorums bound the remaining history in writer-recovery order (epoch, then sequence).
//! Retries may lower that bound but never raise it: subsequent writes and opens belong
//! to ordinary storage lifecycle, not startup work. A later writer may supersede an
//! abandoned suffix, but neither hot nor cold completion can bypass the committed floor.
//! Cold recovery runs even without that quorum. Matching a stale replica starts only with
//! fresh quorum evidence and permission still requires a later observation of the target.

use super::{
	LoadedVolume, MaintainerPhase, MaintainerState, filesystem, membership, poll, repair, stop,
	unload,
};
use crate::tablet_maintainer_machine::{
	Effects,
	types::{
		RequestId, STARTUP_READINESS_SAMPLE_LIMIT, STARTUP_READINESS_WARN_AFTER_MS,
		STARTUP_READINESS_WARN_INTERVAL_MS,
	},
};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_log_shard_v1::VolumeLogReplicaState,
	depot_replica_readiness_v1::ReplicaReadiness,
	depot_storage_worker_protocol_v1::{NotReady, StopReadinessResponse as Response},
	shared_depot_fs_v2::FsVolumeStatus,
	shared_depot_types_v1::{
		TabletPhase, VolumeBlocker, VolumeLogPosition, VolumeLogReplicaStatus,
	},
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
	/// A fresh round may try matching again after this backoff.
	next_repair_at_boot_ms: u64,
}

/// Whether `position` holds the admitted committed floor. Sequence and epoch are compared
/// separately because the floor must cover both.
fn covers_floor(position: &VolumeLogPosition, committed: &VolumeLogPosition) -> bool {
	position.volume_seq >= committed.volume_seq && position.client_epoch >= committed.client_epoch
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
	/// Gives an existing holder a quiet interval to match the returning member itself.
	started_at_boot_ms: u64,
	next_warning_at_boot_ms: u64,
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
						next_repair_at_boot_ms: 0,
					},
				)
			})
			.collect();
		state.stop_checks.startup.sessions.insert(
			worker.clone(),
			Session {
				instance: request.worker_incarnation,
				disk: request.disk_incarnation,
				started_at_boot_ms: state.now_boot_ms,
				next_warning_at_boot_ms: state
					.now_boot_ms
					.saturating_add(STARTUP_READINESS_WARN_AFTER_MS),
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
	let mut repairs = Vec::new();
	for (id, worker) in done {
		state.stop_checks.startup.pending.remove(&id);
		let mut session = state
			.stop_checks
			.startup
			.sessions
			.remove(&worker)
			.expect("startup session");
		let warn = state.now_boot_ms >= session.next_warning_at_boot_ms;
		let mut sample = Vec::new();
		session.remaining.retain(|key, goal| {
			let Some(volume) = state.loaded_volumes.get(key) else {
				return false;
			};
			if unload::closed_through(state, key).is_some()
				|| filesystem::is_record_missing(state, key)
			{
				return false;
			}
			let observations = state.stop_checks.startup.observations.get(key);
			let fresh_bound = observed_bound(state, key);
			if let Some(bound) = fresh_bound.as_ref()
				&& goal
					.bound
					.as_ref()
					.is_none_or(|previous| history_order(bound) < history_order(previous))
			{
				// Every read quorum independently bounds the admission history. A
				// returning quorum can disprove an abandoned suffix; later appends
				// cannot turn a finite startup barrier into a moving target.
				goal.bound = Some(bound.clone());
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
			if cold_observed && minority_leftover(state, key, volume, goal, &worker) {
				return false;
			}
			let ready = observed.is_some_and(|replica| {
				if !covers_floor(&replica.appended, &goal.committed) {
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
			if ready {
				return false;
			}
			if sample.len() < STARTUP_READINESS_SAMPLE_LIMIT {
				sample.push(VolumeBlocker {
					volume_log_key: key.0.clone(),
					member: *worker.as_bytes(),
					status: observed.map(|replica| replica.status.clone()),
					appended_seq: observed.map_or(0, |replica| replica.appended.volume_seq),
					committed_seq: goal.committed.volume_seq,
				});
				if warn {
					log_blocker(state, key, goal, &worker, volume, cold_observed);
				}
			}

			let needs_repair = state.now_boot_ms >= goal.next_repair_at_boot_ms
				&& observed.is_some_and(|replica| {
					// An interrupted repair can leave its destination nonvoting.
					// Only intact voters counted toward the source quorum above.
					membership::voter(&state.tablet.replica_set, &worker)
						&& match replica.status {
							VolumeLogReplicaStatus::Repairing => true,
							VolumeLogReplicaStatus::InSync => !replica.lost_entries,
							VolumeLogReplicaStatus::Deleted => false,
						}
				}) && fresh_bound.as_ref().is_some_and(|bound| {
				// Readiness uses a frozen floor. A new repair claim must also
				// account for commits learned while this round was in flight.
				covers_floor(bound, &goal.committed)
					&& covers_floor(bound, &filesystem::committed_prefix(state, key))
			});
			// An intact cold replica can load its record and reconstruct an absent joiner
			// even before a quorum answers. Log matching requires a fresh voting quorum.
			stop::recover_replica(state, key, &worker, fx);
			if needs_repair {
				repairs.push((
					goal.next_repair_at_boot_ms,
					key.clone(),
					worker.clone(),
					session.started_at_boot_ms,
				));
			}
			true
		});
		let blocking_volumes = session.remaining.len() as u64;
		if warn && blocking_volumes > 0 {
			session.next_warning_at_boot_ms = state
				.now_boot_ms
				.saturating_add(STARTUP_READINESS_WARN_INTERVAL_MS);
		}
		state.stop_checks.startup.sessions.insert(worker, session);
		stop::reply(
			fx,
			id,
			if blocking_volumes == 0 {
				Response::Ready
			} else {
				Response::NotReady(NotReady {
					blocking_volumes,
					sample,
				})
			},
		);
	}
	// Give volumes that have never tried, or have waited the longest since their
	// last attempt, first use of the available recovery slots.
	repairs.sort_unstable();
	for (_, key, worker, started_at_boot_ms) in repairs {
		if repair::active_jobs(state) >= state.config.tuning().max_repairs as usize {
			break;
		}
		repair_stale(state, &key, &worker, started_at_boot_ms, fx);
	}
}

/// Report this round's observations, including authoritative absences, without adding wire fields.
fn log_blocker(
	state: &MaintainerState,
	key: &VolumeLogKey,
	goal: &Goal,
	worker: &WorkerId,
	volume: &LoadedVolume,
	cold_observed: bool,
) {
	let checks = &state.stop_checks.startup;
	let absent = checks.absent.get(key);
	let observations = checks.observations.get(key);
	let target = observations.and_then(|answers| answers.get(worker));
	let hot_blocker = match target {
		None if absent.is_some_and(|members| members.contains(worker)) => "target_absent",
		None => "target_unobserved",
		Some(replica) if !covers_floor(&replica.appended, &goal.committed) => "behind_committed",
		Some(replica) if replica.lost_entries => "lost_entries",
		Some(replica) if replica.status != VolumeLogReplicaStatus::InSync => "not_in_sync",
		Some(_) if goal.bound.is_none() => "no_bound",
		Some(_) => "behind_bound",
	};
	let observed = observations
		.into_iter()
		.flatten()
		.map(|(member, replica)| {
			(
				member,
				&replica.status,
				&replica.appended,
				&replica.cold,
				replica.lost_entries,
			)
		})
		.collect::<Vec<_>>();
	tracing::warn!(
		tablet_id = %uuid::Uuid::from_bytes(state.tablet.id),
		tablet_grant_epoch = state.tablet.tablet_grant_epoch,
		joining_worker = %worker,
		volume = ?key,
		hot_blocker,
		committed = ?goal.committed,
		quorum_bound = ?goal.bound,
		all_voters_observed = cold_observed,
		replica_observations = ?observed,
		absent_members = ?absent,
		stored_cold = ?volume.stored_row.as_ref().map(|row| &row.cold),
		applied = ?state.filesystem.volumes.get(key).map(|volume| &volume.applied),
		"volume is blocking storage startup readiness"
	);
}

/// Match one stale voter through the ordinary quorum recovery protocol. A current close or
/// a proved startup claim can bypass the holder quiet interval.
fn repair_stale(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	worker: &WorkerId,
	started_at_boot_ms: u64,
	fx: &mut Effects,
) {
	let name = &key.0.volume_key;
	let now = state.now_boot_ms;
	let index = state.operations.index();
	if state.phase != MaintainerPhase::Serving
		|| !matches!(state.tablet.replica_set.phase, TabletPhase::Stable)
		|| state.pending.contains_key(name)
		|| super::deletion::holds_key(state, name)
		|| super::forks::holds_key(state, name)
		|| super::maintainer_log::replay::pending_for(state, name)
		|| repair::holds_key(state, name)
		|| super::operator_repair::claims::holds(state, key)
		|| super::member_repair::loading(state, key)
		|| index.loads.contains_key(key)
		|| index.forget_orphans.contains_key(key)
		|| unload::forgetting(state, key)
		|| unload::close_notices_pending(state, key)
	{
		return;
	}
	let volume = &state.loaded_volumes[key];
	if volume.unacknowledged_cold_open.is_some()
		|| volume
			.creation
			.as_ref()
			.is_some_and(|creation| !creation.acknowledged)
	{
		return;
	}
	let unloading = state.unloading.get(key);
	if unloading.is_some_and(|unload| unload.recovery_backoff) {
		return;
	}
	let Some(answers) = state.stop_checks.startup.observations.get(key) else {
		return;
	};
	let fast = fast_recovery_proof(state, key, volume, answers);
	// A holderless snapshot may predate a newer writer or a replica rebuild. Base the
	// fallback on the recorded activity and session start, which status reads cannot rewind.
	let tuning = state.config.tuning();
	let quiet_ms = tuning.holder_check_after_ms.max(tuning.repair_quiet_ms);
	if !fast && (!volume.idle(now, quiet_ms) || now < started_at_boot_ms.saturating_add(quiet_ms)) {
		return;
	}
	let Some(mut row) = volume
		.row(key)
		.filter(|row| row.status == FsVolumeStatus::Live)
	else {
		return;
	};
	// The store can see a staged row, and even a durable row can be ahead of trim.
	// The replay baseline is the newest cold image already adopted by a fresh intact
	// voter. This also accounts for any source whose hot prefix has been trimmed.
	let mut cold_positions = answers
		.iter()
		.filter(|(worker, replica)| intact_voter(state, worker, replica))
		.map(|(_, replica)| &replica.cold);
	let Some(cold) = cold_positions.clone().max_by_key(|cold| cold.volume_seq) else {
		return;
	};
	if cold_positions.any(|position| position.volume_seq == cold.volume_seq && position != cold) {
		return;
	}
	row.cold = cold.clone();
	row.recent_writes
		.retain(|receipt| receipt.volume_seq <= cold.volume_seq);
	repair::recover(
		state,
		key.clone(),
		row,
		repair::RecoveryPurpose::Startup {
			worker: *worker.as_bytes(),
			replica_set_version: state.tablet.replica_set.tablet_replica_set_version,
		},
		fx,
	);
}

/// The close or startup claim must belong to this grant. A writer granted a newer
/// claim by this maintainer may still be installing it, even if no status read shows it yet.
fn fast_recovery_proof(
	state: &MaintainerState,
	key: &VolumeLogKey,
	volume: &LoadedVolume,
	answers: &BTreeMap<WorkerId, VolumeLogReplicaState>,
) -> bool {
	let issued = super::claim_epochs::get(state, key);
	let closed = state
		.unloading
		.get(key)
		.filter(|unload| unload.row_check == unload::RowCheck::Checked)
		.and_then(|unload| unload.requested_client.as_ref())
		.filter(|client| Some(client.client_epoch) == issued.open);
	// Different replicas can hold different attempts. Every newer application grant
	// clears the interval, so no in-range claim belongs to an application writer.
	let repair_claim = |epoch| {
		issued
			.startup_repairs
			.is_some_and(|epochs| epochs.contains(epoch))
	};
	let latest_repair = issued.startup_repairs.map(|epochs| epochs.latest);
	let Some(ceiling) = closed.map(|client| client.client_epoch).max(latest_repair) else {
		return false;
	};

	let mut repair_identities = BTreeMap::new();
	for replica in answers.values().chain(volume.members.values()) {
		if super::maintainer_log::epoch_floor(replica) > ceiling {
			return false;
		}
		let Some(client) = replica.claimed_client.as_ref() else {
			continue;
		};
		if closed
			.is_some_and(|closed| closed.client_epoch == client.client_epoch && closed != client)
		{
			return false;
		}
		if repair_claim(client.client_epoch)
			&& (replica.claimed_holder.is_some()
				|| repair_identities
					.insert(client.client_epoch, client)
					.is_some_and(|previous| previous != client))
		{
			return false;
		}
	}

	membership::quorum(&state.tablet.replica_set, |worker| {
		answers.get(worker).is_some_and(|replica| {
			intact_voter(state, worker, replica)
				&& replica.claimed_client.as_ref().is_some_and(|client| {
					closed == Some(client) || repair_claim(client.client_epoch)
				})
		})
	})
}

/// Give other blocked volumes a turn after the recovery owner releases this key.
pub(super) fn repair_finished(state: &mut MaintainerState, key: &VolumeLogKey) {
	let tuning = state.config.tuning();
	let retry_at = state
		.now_boot_ms
		.saturating_add(tuning.operation_timeout_ms)
		.saturating_add(tuning.retry_ms);
	for session in state.stop_checks.startup.sessions.values_mut() {
		if let Some(goal) = session.remaining.get_mut(key) {
			goal.next_repair_at_boot_ms = retry_at;
		}
	}
}

/// Whether the joiner's replica of a volume is a leftover that startup does not wait for.
/// That holds when this round finds the replica, in a stable replica set, on fewer voters
/// than a quorum, and either the replica or the volume's live cold row covers the committed
/// floor admitted for the volume. The caller must already know that every member answered
/// this round or reported the volume absent, because an unanswered member could hide another
/// replica. A volume that a create, a cold open or a pending open is working on keeps waiting
/// for the startup proofs, because that work may still be placing the replicas a quorum will
/// store.
///
/// While the replica set is stable, every path that removes a voter's replica of a live
/// volume requires that the cold row covers the replica or that no quorum stores the volume.
/// Losing the disk is the one exception, and it is why the committed floor still applies. A
/// floor that neither the joiner nor cold covers means that acknowledged data was lost, and
/// the worker must stay unready so that the loss is noticed. When no quorum stores the volume
/// hot, the joiner's copy cannot be behind an acknowledged entry that another voter serves,
/// and a later open or orphan cleanup reconciles the leftover. Neither startup proof can
/// release such a volume, because both need a quorum of answers. While the replica set
/// changes, a missing current quorum does not prove that an older configuration never
/// acknowledged an entry, so the volume keeps blocking.
fn minority_leftover(
	state: &MaintainerState,
	key: &VolumeLogKey,
	volume: &LoadedVolume,
	goal: &Goal,
	joiner: &WorkerId,
) -> bool {
	let Some(answers) = state.stop_checks.startup.observations.get(key) else {
		return false;
	};
	let stores = |worker: &WorkerId| {
		answers
			.get(worker)
			.is_some_and(|replica| replica.status != VolumeLogReplicaStatus::Deleted)
	};
	let Some(replica) = answers.get(joiner).filter(|_| stores(joiner)) else {
		return false;
	};
	let lifecycle_in_flight = volume.creation.is_some()
		|| volume.unacknowledged_cold_open.is_some()
		|| state.pending.contains_key(&key.0.volume_key);
	if lifecycle_in_flight
		|| !matches!(state.tablet.replica_set.phase, TabletPhase::Stable)
		|| membership::quorum(&state.tablet.replica_set, stores)
	{
		return false;
	}

	let cold_covers = volume.stored_row.as_deref().is_some_and(|row| {
		row.status == FsVolumeStatus::Live
			&& row.volume_incarnation == key.0.volume_incarnation
			&& covers_floor(&row.cold, &goal.committed)
	});
	covers_floor(&replica.appended, &goal.committed) || cold_covers
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
			intact_voter(state, worker, replica)
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

fn intact_voter(
	state: &MaintainerState,
	worker: &WorkerId,
	replica: &VolumeLogReplicaState,
) -> bool {
	membership::voter(&state.tablet.replica_set, worker)
		&& !replica.lost_entries
		&& replica.status == VolumeLogReplicaStatus::InSync
}

pub(super) fn cancel(state: &mut MaintainerState, fx: &mut Effects) {
	for (id, _) in std::mem::take(&mut state.stop_checks.startup.pending) {
		stop::reply(fx, id, Response::NotServing);
	}
	state.stop_checks.startup.sessions.clear();
	state.stop_checks.startup.observations.clear();
	state.stop_checks.startup.absent.clear();
}
