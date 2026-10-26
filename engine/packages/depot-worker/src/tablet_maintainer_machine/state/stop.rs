//! Stop checks consume only a complete barrier status round begun after admission.

use super::{MaintainerPhase, MaintainerState, poll};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::MaintainerEffect,
	types::{OpId, RequestId},
};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_log_shard_v1::VolumeLogReplicaState,
	depot_storage_worker_protocol_v1::{
		NotReady, StaleTabletGrantEpoch, StaleTabletReplicaSet, StopReadiness,
		StopReadinessResponse as Response,
	},
	shared_depot_fs_v1::FsVolumeStatus,
	shared_depot_types_v1::{TabletPhase, VolumeBlocker, VolumeLogReplicaStatus},
};
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A real stop intent cannot resume this process. Keep its create barrier for the
/// grant's lifetime, including after a reply is cancelled or membership changes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct OwnerQuiescence {
	instance: [u8; 16],
	/// None for a real process stop; only ordinary handoffs can resume at a newer version.
	manifest_version: Option<u64>,
	/// Callers admitted before the barrier may still draw their original incarnation.
	initializers: BTreeSet<RequestId>,
	/// Only mutations already admitted by the barrier can delay it. Later existing
	/// opens retain durable incarnation authority and cannot extend this set.
	loads: BTreeSet<OpId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Checks {
	owner: Option<OwnerQuiescence>,
	/// Each waiting request with the barrier generation that must finish before it is answered.
	pending: BTreeMap<RequestId, (StopReadiness, u64)>,
	observations: BTreeMap<VolumeLogKey, BTreeMap<WorkerId, VolumeLogReplicaState>>,
}

/// Existing opens remain available; only drawing a fresh incarnation is quiesced.
pub(crate) fn may_initialize(state: &MaintainerState, id: RequestId) -> bool {
	state
		.stop_checks
		.owner
		.as_ref()
		.is_none_or(|owner| owner.initializers.contains(&id))
}

/// A load descended from an admitted initializer remains a barrier after its caller expires.
pub(crate) fn load_started(state: &mut MaintainerState, id: OpId, key: &VolumeLogKey) {
	let Some(owner) = &mut state.stop_checks.owner else {
		return;
	};
	if state
		.pending
		.get(&key.0.volume_key)
		.is_some_and(|pending| owner.initializers.contains(&pending.caller.request_id))
	{
		owner.loads.insert(id);
	}
}

pub(crate) fn load_finished(state: &mut MaintainerState, id: OpId) {
	if let Some(owner) = &mut state.stop_checks.owner {
		owner.loads.remove(&id);
	}
}

fn initializing(state: &MaintainerState) -> bool {
	let Some(owner) = &state.stop_checks.owner else {
		return false;
	};
	let waiting_caller = state
		.pending
		.values()
		.any(|pending| owner.initializers.contains(&pending.caller.request_id))
		|| state
			.awaiting_deletion
			.values()
			.any(|caller| owner.initializers.contains(&caller.request_id));
	let unacknowledged_creation = state.loaded_volumes.values().any(|volume| {
		volume
			.creation
			.as_ref()
			.is_some_and(|creation| !creation.acknowledged)
	});
	waiting_caller || unacknowledged_creation || !owner.loads.is_empty()
}

fn quiesce_owner(state: &mut MaintainerState, request: &StopReadiness) -> bool {
	if state.tablet.maintainer != Some(request.worker_id) {
		return true;
	}
	if let Some(owner) = &mut state.stop_checks.owner {
		if owner.instance != request.worker_incarnation {
			return false;
		}
		// A real stop upgrades an ordinary barrier without forgetting admitted work.
		owner.manifest_version = None;
		return true;
	}
	let initializers = state
		.pending
		.values()
		.map(|pending| &pending.caller)
		.chain(state.awaiting_deletion.values())
		.filter(|caller| caller.mode != crate::tablet_maintainer_machine::types::OpenMode::Existing)
		.map(|caller| caller.request_id)
		.collect();
	state.stop_checks.owner = Some(OwnerQuiescence {
		instance: request.worker_incarnation,
		manifest_version: None,
		initializers,
		loads: state
			.operations
			.iter()
			.filter_map(|(id, operation)| {
				matches!(operation, super::Operation::Load { .. }).then_some(*id)
			})
			.collect(),
	});
	true
}

/// One coherent observation carries the grant as well as the global CAS fence.
pub(crate) fn manifest_observed(
	state: &mut MaintainerState,
	version: u64,
	tablet: rivet_data::generated::depot_tablet_manifest_v1::Tablet,
	effects: &mut Effects,
) {
	if version < state.manifest_version {
		return;
	}
	if version == state.manifest_version && tablet != state.tablet {
		super::fence(state, effects);
		return;
	}
	let same_owner = tablet.id == state.tablet.id
		&& tablet.tablet_grant_epoch == state.tablet.tablet_grant_epoch
		&& tablet.maintainer == state.tablet.maintainer;
	let assignment_changed = tablet != state.tablet;
	super::assignment::update(state, tablet, effects);
	if assignment_changed {
		super::unload::mark_scope(state, super::unload::UnloadScope::All);
	}
	state.manifest_version = version;
	if same_owner
		&& !matches!(
			state.phase,
			MaintainerPhase::Fenced | MaintainerPhase::Stopped
		) && state
		.stop_checks
		.owner
		.as_ref()
		.is_some_and(|owner| owner.manifest_version.is_some_and(|fence| version > fence))
	{
		state.stop_checks.owner = None;
	}
}

/// An imported child can finish recovery before its mandatory ready report can be
/// persisted: that report may be queued behind this same prepared handoff. It has
/// admitted no new work while Opening, and the next owner resumes the exact durable
/// bootstrap source. Keep recovery/replay and the initializer barrier independent.
fn recovered_opening_child(state: &MaintainerState) -> bool {
	state.phase == MaintainerPhase::Opening
		&& state.alarms.reporting_ready
		&& state.alarms.initialization.as_ref().is_some_and(|source| {
			state.tablet.parent_tablet_id == Some(source.parent_id)
				&& state.tablet.hash_range == source.hash_range
		}) && super::maintainer_log::ready(state)
		&& !super::maintainer_log::replay::replaying(state)
		&& super::membership::quorum(&state.tablet.replica_set, |worker| {
			state.listed.contains(worker)
		})
}

/// Ordinary handoff never polls for closed clients or unloads their claims. Its only
/// obligation is to complete creates already admitted before changing the grant.
pub(crate) fn handoff(
	state: &mut MaintainerState,
	id: RequestId,
	request: rivet_data::generated::depot_storage_worker_protocol_v1::StopReadiness,
	stop_intent: bool,
	effects: &mut Effects,
) {
	use rivet_data::generated::depot_storage_worker_protocol_v1::ReadinessPurpose;
	let ReadinessPurpose::Handoff(purpose) = &request.purpose else {
		reply(effects, id, Response::NotServing);
		return;
	};
	let version = purpose.tablet_manifest_version;
	if request.tablet_id != state.tablet.id
		|| request.tablet_grant_epoch != state.tablet.tablet_grant_epoch
		|| request.tablet_replica_set_version != state.tablet.replica_set.tablet_replica_set_version
		|| state.tablet.maintainer != Some(request.worker_id)
		|| version < state.manifest_version
	{
		reply(effects, id, Response::NotServing);
		return;
	}
	if version != state.manifest_version
		|| !(state.phase == MaintainerPhase::Serving
			|| (!stop_intent
				&& (state.phase == MaintainerPhase::Frozen || recovered_opening_child(state))))
	{
		reply(
			effects,
			id,
			Response::NotReady(NotReady {
				blocking_volumes: 0,
				sample: vec![],
			}),
		);
		return;
	}
	if state.stop_checks.owner.as_ref().is_some_and(|owner| {
		owner.instance != request.worker_incarnation
			|| owner
				.manifest_version
				.is_some_and(|captured| captured != version)
	}) {
		reply(effects, id, Response::NotServing);
		return;
	}
	if state.stop_checks.owner.is_none() {
		quiesce_owner(state, &request);
		state
			.stop_checks
			.owner
			.as_mut()
			.expect("current owner")
			.manifest_version = (!stop_intent).then_some(version);
	}
	if stop_intent {
		state
			.stop_checks
			.owner
			.as_mut()
			.expect("current owner")
			.manifest_version = None;
	}

	let response = if initializing(state) {
		Response::NotReady(NotReady {
			blocking_volumes: 0,
			sample: vec![],
		})
	} else {
		state
			.stop_checks
			.owner
			.as_mut()
			.expect("current owner")
			.initializers
			.clear();
		Response::Ready
	};
	reply(effects, id, response);
}

fn reply(effects: &mut Effects, request_id: RequestId, response: Response) {
	effects.execute(MaintainerEffect::StopReadinessResponse {
		request_id,
		response,
	});
}

/// Retry hint on a stop-readiness check refused because the maintainer's budget is full.
const OVERLOADED_RETRY_AFTER_MS: u64 = 100;

/// A stop-readiness refusal of a full request budget: nothing was checked, retry later.
pub(crate) fn overloaded() -> Response {
	Response::Overloaded(
		rivet_data::generated::depot_storage_worker_protocol_v1::Overloaded {
			retry_after_ms: OVERLOADED_RETRY_AFTER_MS,
		},
	)
}

pub(crate) fn admit(
	state: &mut MaintainerState,
	id: RequestId,
	request: StopReadiness,
	effects: &mut Effects,
) {
	let response = if request.tablet_id != state.tablet.id
		|| request.tablet_grant_epoch != state.tablet.tablet_grant_epoch
	{
		Some(Response::StaleTabletGrantEpoch(StaleTabletGrantEpoch {
			tablet_grant_epoch: state.tablet.tablet_grant_epoch,
		}))
	} else if request.tablet_replica_set_version
		!= state.tablet.replica_set.tablet_replica_set_version
	{
		Some(Response::StaleTabletReplicaSet(StaleTabletReplicaSet {
			replica_set: state.tablet.replica_set.clone(),
		}))
	} else if !quiesce_owner(state, &request) {
		Some(Response::NotServing)
	} else if state.phase != MaintainerPhase::Serving {
		Some(Response::NotReady(NotReady {
			blocking_volumes: 0,
			sample: vec![],
		}))
	} else if state.request_count() >= state.config.max_requests as usize {
		Some(overloaded())
	} else if state.loaded_volumes.is_empty() && !initializing(state) {
		if let Some(owner) = &mut state.stop_checks.owner {
			owner.initializers.clear();
		}
		Some(Response::Ready)
	} else {
		None
	};
	if let Some(response) = response {
		reply(effects, id, response);
		return;
	}
	super::catch_up::stop_requested(state, &request.worker_id);
	let generation = poll::refresh_all(state, effects);
	state.stop_checks.pending.insert(id, (request, generation));
	// The round may already have finished inside `refresh_all` when nothing is loaded
	// for it to visit.
	if poll::refreshed(state, generation) {
		completed(state, effects);
	}
}

/// A barrier round began; observations from earlier rounds no longer count.
pub(crate) fn started(state: &mut MaintainerState) {
	state.stop_checks.observations.clear();
}

pub(crate) fn observe(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	worker: &WorkerId,
	observed: &VolumeLogReplicaState,
) {
	if !state.stop_checks.pending.is_empty() {
		state
			.stop_checks
			.observations
			.entry(key.clone())
			.or_default()
			.insert(worker.clone(), observed.clone());
	}
}

/// Called only after all status requests in the round have completed or timed out.
/// Volumes admitted during the round must also have fresh observations or block it.
pub(crate) fn completed(state: &mut MaintainerState, effects: &mut Effects) {
	let done: Vec<_> = state
		.stop_checks
		.pending
		.iter()
		.filter(|(_, (_, generation))| poll::refreshed(state, *generation))
		.map(|(id, _)| *id)
		.collect();
	for id in done {
		let (request, _) = state
			.stop_checks
			.pending
			.remove(&id)
			.expect("pending check");
		let required =
			depot_types::stop::required_voters(&state.tablet.replica_set, &request.worker_id);
		let mut blocking_volumes = 0;
		let mut sample = Vec::new();
		let mut released = Vec::new();
		for (key, volume) in &state.loaded_volumes {
			if super::unload::closed_through(state, key).is_some() {
				continue;
			}
			// An incarnation whose stored record is lost serves nobody and is never unloaded
			// or repaired by this maintainer, so a member of it that no client will bring in
			// sync must not hold a worker's stop; the stopping worker's disk keeps its replica.
			if super::filesystem::is_record_missing(state, key) {
				continue;
			}
			let observations = state.stop_checks.observations.get(key);
			let mut blocked = false;
			for worker in &required {
				let observed =
					observations.and_then(|members| members.get(&WorkerId::from_bytes(*worker)));
				if observed.is_some_and(|r| {
					r.status == VolumeLogReplicaStatus::InSync
						|| caught_up_to_cold(state, key, volume, observations, *worker, r)
				}) {
					continue;
				}
				blocked = true;
				if sample.len() < 16 {
					sample.push(VolumeBlocker {
						volume_log_key: key.0.clone(),
						member: *worker,
						status: observed.map(|r| r.status.clone()),
						appended_seq: observed.map_or(0, |r| r.appended.volume_seq),
						committed_seq: observed
							.map_or(0, |r| super::filesystem::committed_on(state, key, r)),
					});
				}
			}
			blocking_volumes += u64::from(blocked);
			if !volume
				.creation
				.as_ref()
				.is_some_and(|creation| !creation.acknowledged)
				&& claimed_by(volume, &request.worker_id, &request.worker_incarnation)
				&& super::membership::quorum(&state.tablet.replica_set, |worker| {
					observations
						.and_then(|members| members.get(worker))
						.is_some_and(|replica| {
							replica.claimed_client.is_some()
								&& replica.claimed_holder.as_ref().is_some_and(|holder| {
									holder.worker_id == request.worker_id
										&& holder.worker_incarnation == request.worker_incarnation
								})
						})
				}) && let Some(client) = volume
				.members
				.values()
				.filter_map(|replica| replica.claimed_client.as_ref())
				.max_by_key(|client| client.client_epoch)
			{
				released.push((key.clone(), client.clone()));
			}
		}
		for (key, client) in released {
			unload_released(
				state,
				key,
				client,
				rivet_data::generated::shared_depot_types_v1::VolumeHolder {
					worker_id: request.worker_id,
					worker_incarnation: request.worker_incarnation,
				},
			);
		}
		let ready = blocking_volumes == 0 && !initializing(state);
		if ready && let Some(owner) = &mut state.stop_checks.owner {
			owner.initializers.clear();
		}
		reply(
			effects,
			id,
			if ready {
				Response::Ready
			} else {
				Response::NotReady(NotReady {
					blocking_volumes,
					sample,
				})
			},
		);
	}
	state.stop_checks.observations.clear();
}

/// Whether `worker`'s `Repairing` replica of `key`, observed in this round, held every
/// acknowledged entry when the round observed it, so a stop may count it as a survivor.
///
/// This is the proof an open uses to promote a caught-up member (see
/// `lifecycle::promote_caught_up`): a quorum of this round's answers holds nothing above the
/// stored cold row, no observed member holds or knows a commit above it, and the replica ends
/// exactly at that row without lost entries. An acknowledged entry above the row would be on one
/// of that quorum, so every entry acknowledged by then is in the cold store and on this replica.
/// Only its status keeps it from voting, and the next open of the volume promotes it.
///
/// The round fences no client, as for an in-sync survivor: a live client may still commit above
/// the row afterwards on its in-sync members and must bring this member in sync itself before it
/// can commit without the stopping worker.
///
/// Without this, nothing would release the stop when the client that should promote the member is
/// gone: a client that closed before repairing it leaves the close to an unload that waits for
/// tiering, and a new grant does not know about that close. The volume then parks under the
/// client's still-live process and never goes idle.
fn caught_up_to_cold(
	state: &MaintainerState,
	key: &VolumeLogKey,
	volume: &super::LoadedVolume,
	observations: Option<&BTreeMap<WorkerId, VolumeLogReplicaState>>,
	worker: [u8; 16],
	replica: &VolumeLogReplicaState,
) -> bool {
	// Mirror the phases in which an open may promote: Learning keeps the old voting quorum, while
	// a Joint configuration needs the opener's own claim, so a stop during one keeps waiting.
	let replica_set = &state.tablet.replica_set;
	let current_voter = match &replica_set.phase {
		TabletPhase::Stable => true,
		TabletPhase::Learning(_) => replica_set.members.contains(&worker),
		TabletPhase::Joint(_) => false,
	};
	let Some(row) = volume.stored_row.as_deref() else {
		return false;
	};
	if !current_voter
		|| state
			.disk_incarnation(&WorkerId::from_bytes(worker))
			.is_none()
		|| replica.status != VolumeLogReplicaStatus::Repairing
		|| replica.lost_entries
		|| replica.appended != row.cold
		|| replica.cold != row.cold
		|| row.status != FsVolumeStatus::Live
		|| row.volume_incarnation != key.0.volume_incarnation
		|| super::filesystem::known_committed(state, key) > row.cold.volume_seq
	{
		return false;
	}

	let within_row = |member: &VolumeLogReplicaState| {
		member.status != VolumeLogReplicaStatus::Deleted
			&& member.appended.volume_seq <= row.cold.volume_seq
			&& member.known_committed_volume_seq <= row.cold.volume_seq
	};
	let observed_suffix = volume
		.members
		.values()
		.chain(
			observations
				.into_iter()
				.flat_map(|members| members.values()),
		)
		.any(|member| !within_row(member));
	let answered_within_row = super::membership::quorum(replica_set, |member| {
		observations
			.and_then(|members| members.get(member))
			.is_some_and(|answer| !answer.lost_entries && within_row(answer))
	});
	answered_within_row && !observed_suffix
}

/// Whether the newest claim on `volume` belongs to the stopping process. A later process of
/// the same worker is a different holder that may have the volume open.
fn claimed_by(
	volume: &super::LoadedVolume,
	worker: &[u8; 16],
	worker_incarnation: &[u8; 16],
) -> bool {
	super::holders::holder(volume).is_some_and(|holder| {
		holder.worker_id == *worker && holder.worker_incarnation == *worker_incarnation
	})
}

/// Replays the joined process's close, including volumes on tablets where it stores no replica.
/// Fresh quorum observations identify the released process. The captured claim is checked
/// atomically by conditional forget, so a concurrent replacement writer is never adopted.
/// This hint can finish fully cold history; it never takes over an unpublished suffix.

fn unload_released(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	client: rivet_data::generated::shared_depot_types_v1::ClientIdentity,
	holder: rivet_data::generated::shared_depot_types_v1::VolumeHolder,
) {
	// A frozen tablet refuses closes, so it must not replay one either.
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	let Some(volume) = state.loaded_volumes.get(&key) else {
		return;
	};
	if volume.is_deleted()
		|| state.pending.contains_key(&key.0.volume_key)
		|| state.unloading.contains_key(&key)
		|| state.deleting.contains_key(&key)
	{
		return;
	}
	tracing::info!(
		volume = ?key,
		"unloading a volume the stopping worker released, so its stop is not blocked"
	);
	super::unload::begin(
		state,
		key,
		super::unload::UnloadReason::Released { client, holder },
	);
}

/// Assignment changes and fencing invalidate the whole observation round.
pub(crate) fn cancel(state: &mut MaintainerState, effects: &mut Effects) {
	for (id, _) in std::mem::take(&mut state.stop_checks.pending) {
		reply(effects, id, Response::NotServing);
	}
	state.stop_checks.observations.clear();
}

pub(crate) fn pending(state: &MaintainerState) -> usize {
	state.stop_checks.pending.len()
}


