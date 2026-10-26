//! Reloads a member missing from a loaded volume so the volume returns to its full replica set.
//!
//! An open loads every member but answers once a quorum is in sync, so a member whose load
//! failed or timed out after that, or a member that later reports the volume as not loaded,
//! leaves the volume serving on fewer replicas than its tablet has. Without this state
//! machine only a client reopen would load the member again.
//!
//! Work is event-driven: an absent member is reported by the failed open load, the open's
//! completion or a status observation, and each tracked member always has exactly one of a
//! place in the ready queue, an in-flight load or an armed retry timer. Eligibility is
//! rechecked whenever an entry is revisited, so an entry for a volume that unloaded, began
//! deleting or reopened is dropped lazily rather than by a sweep.
//!
//! The load creates the replica in `Repairing` status at the cold position of an in-sync
//! member, fenced by that member's claimed client. It never votes until the serving client
//! matches its suffix and marks it in sync through the client's own repair.

use super::{MaintainerPhase, MaintainerState, Operation, lifecycle::ReplicaOpenStep};
use crate::tablet_maintainer_machine::types::MEMBER_REPAIR_MAX_BACKOFF_MS;
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect as Fx, types::*};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_storage_worker_protocol_v1::{
		self as wire, StorageWorkerRequest as Request, StorageWorkerResponse as Response,
	},
	shared_depot_fs_v1::FsVolumeStatus,
	shared_depot_types_v1::VolumeLogReplicaStatus as Status,
};
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// Members being reloaded, with the queue of those waiting for a load slot.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct MemberRepairs {
	entries: BTreeMap<(VolumeLogKey, WorkerId), MemberRepair>,
	/// Entries in `Ready`, oldest first. A load completion starts the next one.
	ready: VecDeque<(VolumeLogKey, WorkerId)>,
	/// Entries in `Loading`, bounded by `max_member_repair_loads`.
	loading: u32,
}

/// The repair of one missing member. `attempt` counts failed loads and sets the backoff.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum MemberRepair {
	/// Queued in `ready` until a load slot frees.
	Ready { attempt: u32 },
	/// A `LoadVolumeRequest` is in flight as operation `op_id`.
	Loading { attempt: u32, op_id: OpId },
	/// The last load failed; `MemberRepairRetry` requeues it.
	Waiting { attempt: u32 },
}

/// The in-sync member a repair load copies its fence and cold position from, or `None`
/// when the maintainer must not reload `worker` for `key` now.
fn eligible<'a>(
	state: &'a MaintainerState,
	key: &VolumeLogKey,
	worker: &WorkerId,
) -> Option<&'a rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState> {
	// Repair runs while the replica set changes too, so a member an open could not reload, or
	// one a cancelled close forgot after the open answered, does not stay missing until the
	// change ends. A Learning-only destination is not a voter yet; catch-up fills it.
	if state.phase != MaintainerPhase::Serving
		|| state.disk_incarnation(worker).is_none()
		|| !state.members().contains(worker)
		|| !super::membership::voter(&state.tablet.replica_set, worker)
	{
		return None;
	}
	let volume = state.loaded_volumes.get(key)?;
	// An open loads every member itself, and unloads, deletions, operator repairs and
	// learner copies own the volume's replicas while they run.
	if volume.members.contains_key(worker)
		|| volume.unacknowledged_cold_open.is_some()
		|| volume
			.creation
			.as_ref()
			.is_some_and(|creation| !creation.acknowledged)
		|| state.pending.contains_key(&key.0.volume_key)
		|| state.unloading.contains_key(key)
		// A forget of this member still in flight, such as one of a cancelled close, could
		// remove the replica this load creates. Its answer reports the member again.
		|| state
			.operations
			.index()
			.forgets
			.contains_key(&(key.clone(), worker.clone()))
		|| state.deleting.contains_key(key)
		|| super::repair::holds_key(state, &key.0.volume_key)
		|| super::catch_up::holds_key(state, &key.0.volume_key)
	{
		return None;
	}
	// Only a volume a quorum serves is missing a member; a minority is orphan cleanup's.
	let in_sync = |w: &WorkerId| {
		volume
			.members
			.get(w)
			.is_some_and(|replica| replica.status == Status::InSync)
	};
	if !super::membership::quorum(&state.tablet.replica_set, in_sync) {
		return None;
	}
	volume
		.row(key)
		.filter(|row| row.status == FsVolumeStatus::Live)?;
	volume
		.members
		.values()
		.filter(|replica| replica.status == Status::InSync)
		.max_by_key(|replica| {
			(
				replica.claimed_client.as_ref().map(|c| c.client_epoch),
				replica.cold.volume_seq,
			)
		})
}

/// The in-sync member a reload of `worker` for `key` would copy, with its cold position, when
/// the maintainer may reload it now.
pub(crate) fn reload_source(
	state: &MaintainerState,
	key: &VolumeLogKey,
	worker: &WorkerId,
) -> Option<(
	WorkerId,
	rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
)> {
	let source = eligible(state, key, worker)?;
	let volume = state.loaded_volumes.get(key)?;
	volume
		.members
		.iter()
		.find(|(_, replica)| *replica == source)
		.map(|(member, replica)| (member.clone(), replica.cold.clone()))
}

/// The members of `key` whose reload waits for its retry deadline.
pub(crate) fn waiting_members(state: &MaintainerState, key: &VolumeLogKey) -> Vec<WorkerId> {
	state
		.member_repairs
		.entries
		.iter()
		.filter(|((entry, _), repair)| {
			entry == key && matches!(repair, MemberRepair::Waiting { .. })
		})
		.map(|((_, worker), _)| worker.clone())
		.collect()
}

/// Reports that `worker` holds no replica of the loaded volume `key`. Starts tracking its
/// repair when the volume is eligible and the member is not already tracked.
pub(crate) fn absent(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	worker: &WorkerId,
	fx: &mut Effects,
) {
	let id = (key.clone(), worker.clone());
	if state.config.tuning().max_member_repair_loads == 0
		|| state.member_repairs.entries.contains_key(&id)
		|| eligible(state, key, worker).is_none()
	{
		return;
	}
	state
		.member_repairs
		.entries
		.insert(id.clone(), MemberRepair::Ready { attempt: 0 });
	state.member_repairs.ready.push_back(id);
	start_ready(state, fx);
}

/// Reports every member absent from `key` after its open answered, except those an open
/// load still reaches.
pub(crate) fn open_completed(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	for worker in state.members() {
		if !loading_member(state, key, &worker) {
			absent(state, key, &worker, fx);
		}
	}
}

/// Whether an open load of `worker` for `key` is still in flight.
fn loading_member(state: &MaintainerState, key: &VolumeLogKey, worker: &WorkerId) -> bool {
	state.operations.index().loads.get(key).is_some_and(|ops| {
		ops.iter().any(|op| {
			matches!(
				state.operations.get(op),
				Some(Operation::Load { worker: w, .. }) if w == worker
			)
		})
	})
}

/// Whether a repair load of `key` is in flight. An unload waits for it, so the load
/// cannot recreate a replica behind the unload's forget.
pub(crate) fn loading(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	state
		.member_repairs
		.entries
		.range(super::operations::workers_of(key))
		.any(|(_, repair)| matches!(repair, MemberRepair::Loading { .. }))
}

/// Starts queued loads while slots are free, dropping entries that are no longer eligible.
fn start_ready(state: &mut MaintainerState, fx: &mut Effects) {
	while state.member_repairs.loading < state.config.tuning().max_member_repair_loads {
		let Some(id) = state.member_repairs.ready.pop_front() else {
			return;
		};
		let Some(MemberRepair::Ready { attempt }) = state.member_repairs.entries.get(&id).cloned()
		else {
			continue;
		};
		let (key, worker) = id.clone();
		let Some(source) = eligible(state, &key, &worker).cloned() else {
			state.member_repairs.entries.remove(&id);
			continue;
		};
		let Some(disk_incarnation) = state.disk_incarnation(&worker) else {
			state.member_repairs.entries.remove(&id);
			continue;
		};
		let request = Request::LoadVolumeRequest(wire::LoadVolumeRequest {
			creation: state.loaded_volumes.get(&key).and_then(|volume| {
				volume.creation_evidence(state.config.write_versions.maintainer_log)
			}),
			open_claim: None,
			learner_replica_set_version: None,
			disk_incarnation,
			claimed_client: source.claimed_client.clone(),
			claimed_holder: source.claimed_holder.clone(),
			header: wire::ReplicaRequestHeader::MaintainerRequestHeader(
				wire::MaintainerRequestHeader {
					volume_log_key: key.0.clone(),
					tablet_grant: state.grant(),
				},
			),
			cold: source.cold.clone(),
			status: Status::Repairing,
			initial_entries: Vec::new(),
		});
		let op_id = state.begin(
			Operation::Load {
				step: ReplicaOpenStep::MissingMember,
				worker: worker.clone(),
				key,
			},
			fx,
		);
		state
			.member_repairs
			.entries
			.insert(id, MemberRepair::Loading { attempt, op_id });
		state.member_repairs.loading += 1;
		crate::metrics::record_member_repair("started");
		fx.execute(Fx::ReplicaRequest {
			op_id,
			worker_id: worker,
			request,
		});
	}
	if !state.member_repairs.ready.is_empty() {
		tracing::debug!(
			queued = state.member_repairs.ready.len(),
			limit = state.config.tuning().max_member_repair_loads,
			"missing member reloads wait for a load slot"
		);
	}
}

/// Applies the reply or timeout of repair load `op_id`. A loaded replica becomes a member
/// observation; a failure waits a bounded backoff before the member is tried again.
pub(crate) fn loaded(
	state: &mut MaintainerState,
	op_id: OpId,
	key: VolumeLogKey,
	worker: WorkerId,
	result: Result<Response, String>,
	fx: &mut Effects,
) {
	let id = (key.clone(), worker.clone());
	let attempt = match state.member_repairs.entries.get(&id) {
		Some(MemberRepair::Loading {
			attempt,
			op_id: current,
		}) if *current == op_id => *attempt,
		Some(
			MemberRepair::Loading { .. }
			| MemberRepair::Ready { .. }
			| MemberRepair::Waiting { .. },
		)
		| None => return,
	};
	state.member_repairs.loading -= 1;

	let replica = match result {
		Ok(Response::ReplicaStateResponse(response))
			if response.state.status != Status::Deleted =>
		{
			Some(response.state)
		}
		_ => None,
	};
	match replica {
		Some(replica) => {
			state.member_repairs.entries.remove(&id);
			crate::metrics::record_member_repair("loaded");
			if let Some(volume) = state.loaded_volumes.get_mut(&key) {
				volume.members.insert(worker, replica);
			}
			super::deletion::observe_loaded(state, &key);
			super::filesystem::member_observed(state, &key);
		}
		None if eligible(state, &key, &worker).is_some() => {
			crate::metrics::record_member_repair("failed");
			let attempt = attempt.saturating_add(1);
			let delay = state
				.config
				.tuning()
				.retry_ms
				.saturating_mul(1u64 << attempt.min(16))
				.min(MEMBER_REPAIR_MAX_BACKOFF_MS);
			state
				.member_repairs
				.entries
				.insert(id, MemberRepair::Waiting { attempt });
			fx.arm_timer_after(MaintainerTimer::MemberRepairRetry(key, worker), delay);
		}
		None => {
			state.member_repairs.entries.remove(&id);
		}
	}
	start_ready(state, fx);
}

/// Requeues a member whose retry backoff ended.
pub(crate) fn retry(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	worker: WorkerId,
	fx: &mut Effects,
) {
	let id = (key, worker);
	let Some(MemberRepair::Waiting { attempt }) = state.member_repairs.entries.get(&id).cloned()
	else {
		return;
	};
	state
		.member_repairs
		.entries
		.insert(id.clone(), MemberRepair::Ready { attempt });
	state.member_repairs.ready.push_back(id);
	start_ready(state, fx);
}

/// Forgets every repair when the maintainer stops serving. In-flight loads complete into
/// nothing; a replica one of them created is found by orphan cleanup or the next open.
pub(crate) fn clear(state: &mut MaintainerState, fx: &mut Effects) {
	for ((key, worker), repair) in std::mem::take(&mut state.member_repairs.entries) {
		match repair {
			MemberRepair::Waiting { .. } => {
				fx.cancel_timer(MaintainerTimer::MemberRepairRetry(key, worker))
			}
			MemberRepair::Ready { .. } | MemberRepair::Loading { .. } => {}
		}
	}
	state.member_repairs = MemberRepairs::default();
}
