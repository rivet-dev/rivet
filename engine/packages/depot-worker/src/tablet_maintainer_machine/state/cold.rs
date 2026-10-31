//! Replica observations, shared filesystem apply budgets and trim acknowledgements.

use super::{MaintainerPhase, MaintainerState, Operation};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_log_shard_v1::VolumeLogReplicaState,
	depot_storage_worker_protocol_v1::{self as wire, StorageWorkerRequest, StorageWorkerResponse},
	shared_depot_types_v1::{VolumeLogPosition, VolumeLogReplicaStatus},
};
use rivet_topology::worker::WorkerId;
use std::collections::BTreeMap;

/// Keep observations monotonic within an incarnation. Deleted is terminal for this log key.
pub(crate) fn observe(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	worker: WorkerId,
	observed: VolumeLogReplicaState,
) {
	if super::filesystem::retiring(state, key) {
		return;
	}
	// This observation may hide the members' evidence of a commit, so its floor is raised first.
	super::filesystem::raise_committed_floor(state, key);
	let prior = super::filesystem::known_committed(state, key);
	let Some(volume) = state.loaded_volumes.get_mut(key) else {
		return;
	};
	if volume.members.get(&worker).is_some_and(|old| {
		old.status == VolumeLogReplicaStatus::Deleted
			|| (old.replica_incarnation == observed.replica_incarnation
				&& old.cold.volume_seq > observed.cold.volume_seq)
	}) {
		return;
	}
	volume.members.insert(worker.clone(), observed);
	// An observation that completes a quorum on a newer tail is the volume's activity.
	if super::filesystem::known_committed(state, key) > prior
		&& let Some(volume) = state.loaded_volumes.get_mut(key)
	{
		volume.last_activity = state.now_boot_ms;
	}
	super::filesystem::raise_committed_floor(state, key);
	super::catch_up::mark(state, key);
	super::holders::claim_observed(state, key);
	super::filesystem::source_observed(state, key, &worker);
	super::deletion::observe_loaded(state, key);
	// An observation can complete the in-sync quorum that makes the volume eligible.
	super::filesystem::offer(state, key);
}

/// Replica disk hints may accelerate a flush, but never establish commitment.
pub(crate) fn schedule(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	_observed: &VolumeLogReplicaState,
	hint: Option<wire::ColdWriteStatus>,
	effects: &mut Effects,
) {
	queue_lagging_trims(state, key);
	if hint.is_some_and(|hint| {
		hint.disk_pressure || hint.committed_bytes >= state.config.write_to_cold_above_bytes
	}) {
		super::filesystem::request_flush(state, key, effects);
	}
}
/// Adds a volume whose replicas reported new data to the pass already waiting in the store
/// queue. It joins an upload that runs anyway and never starts one.
pub(crate) fn join_waiting_pass(state: &mut MaintainerState, key: VolumeLogKey) {
	if state.filesystem.volumes.contains_key(&key) {
		state.filesystem.request_flush(&key);
		super::filesystem::queue_flush(state);
	}
}

pub(crate) fn queue_lagging_trims(state: &mut MaintainerState, key: &VolumeLogKey) {
	if let Some(fs) = state.filesystem.volumes.get(key) {
		let cold = fs.record.cold.volume_seq;
		for worker in state.members() {
			if state
				.loaded_volumes
				.get(key)
				.and_then(|v| v.members.get(&worker))
				.is_some_and(|r| r.cold.volume_seq < cold)
			{
				state.trim_ready.insert((key.clone(), worker));
			}
		}
		return;
	}
	// A trim raises a lagging member to a cold position the lost record was to cover, and
	// its log below that position may be the only copy of that data.
	if super::filesystem::is_record_missing(state, key) {
		return;
	}
	let Some(volume) = state.loaded_volumes.get(key) else {
		return;
	};
	let Some(row) = volume.row(key) else {
		return;
	};
	if row.status != rivet_data::generated::shared_depot_fs_v1::FsVolumeStatus::Live
		|| row.cold.volume_seq == 0
	{
		return;
	}
	if state
		.pending_trims
		.get(key)
		.is_some_and(|pending| row.cold.volume_seq >= pending.volume_seq)
	{
		state.pending_trims.remove(key);
	}
	for worker in state.members() {
		if volume
			.members
			.get(&worker)
			.is_some_and(|member| member.cold.volume_seq < row.cold.volume_seq)
		{
			state.trim_ready.insert((key.clone(), worker));
		}
	}
}

/// Reserve both the retained filesystem page and its executor copy.
/// Buffered pages and read reservations share the budget to avoid a dispatch deadlock.
pub(crate) fn bytes(state: &MaintainerState) -> u64 {
	let index = state.operations.index();
	super::filesystem::queued_bytes(state)
		.saturating_add(u64::from(index.read_reservations).saturating_mul(read_budget(state)))
		.saturating_add(index.apply_bytes)
}

pub(super) fn read_budget(state: &MaintainerState) -> u64 {
	2 * (u64::from(state.config.tuning().max_cold_read_bytes)
		+ u64::from(state.config.tuning().max_volume_key_bytes)
		+ 64)
}

/// Whether a trim of one volume on one member is due now.
enum TrimDue {
	/// Send a trim to this cold position, with the claim it carries.
	Send(
		VolumeLogPosition,
		Option<rivet_data::generated::shared_depot_types_v1::ClientIdentity>,
	),
	/// An in-sync member already holds the published floor, which acknowledges the
	/// volume's pending trim exactly as a trim reply would.
	Acknowledged,
	/// No trim is due: the maintainer does not serve, the member backs off or already has a
	/// trim in flight, nothing is published, or a repairing member is already at the floor.
	NotDue,
}

/// Decides the trim of `key` on `worker` without changing state, so a pass over the ready
/// set can decide every entry before it acts.
fn trim_item(state: &MaintainerState, key: &VolumeLogKey, worker: &WorkerId) -> TrimDue {
	// A trim of an incarnation whose stored record is lost may drop the only copy of the data
	// that record covered; see `filesystem::record_missing`.
	if state.phase != MaintainerPhase::Serving
		|| super::filesystem::is_record_missing(state, key)
		|| state.trim_backoff.contains(&(key.clone(), worker.clone()))
		|| state
			.operations
			.index()
			.trims
			.contains_key(&(key.clone(), worker.clone()))
	{
		return TrimDue::NotDue;
	}
	let published = state
		.filesystem
		.volumes
		.get(key)
		.map(|v| v.record.cold.clone())
		.or_else(|| {
			state
				.loaded_volumes
				.get(key)
				.and_then(|v| v.row(key))
				.map(|r| r.cold)
		});
	let cold = state
		.pending_trims
		.get(key)
		.cloned()
		.into_iter()
		.chain(published)
		.max_by_key(|p| p.volume_seq);
	let Some(cold) = cold else {
		return TrimDue::NotDue;
	};
	let member = state
		.loaded_volumes
		.get(key)
		.and_then(|volume| volume.members.get(worker));
	let Some(member) = member else {
		return TrimDue::NotDue;
	};
	if member.cold.volume_seq >= cold.volume_seq {
		// This member already acknowledged the published floor. In particular an
		// empty record has no entries to trim, so no later trim reply can arrive.
		// Repairing replicas cannot certify that the retained history is complete.
		return match member.status == VolumeLogReplicaStatus::InSync {
			true => TrimDue::Acknowledged,
			false => TrimDue::NotDue,
		};
	}
	// A release hint must not assert a replacement writer by copying a claim into the trim.
	let released = state.unloading.get(key).is_some_and(|unload| {
		matches!(unload.reason, super::unload::UnloadReason::Released { .. })
	});
	let claimed_client = if released {
		None
	} else {
		state
			.loaded_volumes
			.get(key)
			.and_then(super::LoadedVolume::trim_claim)
	};
	TrimDue::Send(cold, claimed_client)
}

/// Sends the due trims of `keys` on `worker`: one `TrimRequest` for a single volume, or one
/// `TrimVolumesRequest` under one grant check for several, as a pass publishes many volumes
/// at once.
fn dispatch_trims(
	state: &mut MaintainerState,
	worker: WorkerId,
	keys: Vec<VolumeLogKey>,
	effects: &mut Effects,
) {
	let mut items = Vec::new();
	for key in keys {
		match trim_item(state, &key, &worker) {
			TrimDue::Send(cold, claimed_client) => items.push((key, cold, claimed_client)),
			TrimDue::Acknowledged => {
				state.pending_trims.remove(&key);
			}
			TrimDue::NotDue => {}
		}
	}
	match items.len() {
		0 => {}
		1 => {
			let (key, cold, claimed_client) = items.pop().expect("one trim");
			let request = StorageWorkerRequest::TrimRequest(wire::TrimRequest {
				claimed_client,
				header: wire::ReplicaRequestHeader::MaintainerRequestHeader(
					wire::MaintainerRequestHeader {
						volume_log_key: key.0.clone(),
						tablet_grant: state.grant(),
					},
				),
				cold: cold.clone(),
			});
			let op_id = state.begin(
				Operation::Trim {
					key,
					worker: worker.clone(),
					cold,
				},
				effects,
			);
			effects.execute(MaintainerEffect::ReplicaRequest {
				op_id,
				worker_id: worker,
				request,
			});
		}
		_ => {
			let request = StorageWorkerRequest::TrimVolumesRequest(wire::TrimVolumesRequest {
				tablet_grant: state.grant(),
				volumes: items
					.iter()
					.map(|(key, cold, claimed_client)| wire::TrimVolume {
						volume_log_key: key.0.clone(),
						cold: cold.clone(),
						claimed_client: claimed_client.clone(),
					})
					.collect(),
			});
			let op_id = state.begin(
				Operation::TrimBatch {
					worker: worker.clone(),
					trims: items
						.into_iter()
						.map(|(key, cold, _)| (key, cold))
						.collect(),
				},
				effects,
			);
			effects.execute(MaintainerEffect::ReplicaRequest {
				op_id,
				worker_id: worker,
				request,
			});
		}
	}
}

/// Completes a batched trim: each volume's result is handled exactly as its own trim's
/// reply. A failed request, or a reply that does not answer every volume, fails each trim,
/// which backs it off; a whole-batch rejection applies to each volume.
pub(crate) fn trimmed_batch(
	state: &mut MaintainerState,
	worker: WorkerId,
	trims: Vec<(VolumeLogKey, VolumeLogPosition)>,
	result: Result<StorageWorkerResponse, String>,
	effects: &mut Effects,
) {
	let results: Vec<Result<StorageWorkerResponse, String>> = match result {
		Ok(StorageWorkerResponse::TrimVolumesResponse(reply))
			if reply.results.len() == trims.len() =>
		{
			reply
				.results
				.into_iter()
				.map(|result| match result {
					wire::ForgetVolumeResult::ReplicaStateResponse(response) => {
						Ok(StorageWorkerResponse::ReplicaStateResponse(response))
					}
					wire::ForgetVolumeResult::RejectResponse(reason) => {
						Ok(StorageWorkerResponse::RejectResponse(reason))
					}
				})
				.collect()
		}
		Ok(StorageWorkerResponse::RejectResponse(reason)) => trims
			.iter()
			.map(|_| Ok(StorageWorkerResponse::RejectResponse(reason.clone())))
			.collect(),
		Ok(_) => trims
			.iter()
			.map(|_| Err("batched trim answered with an unexpected reply".to_owned()))
			.collect(),
		Err(error) => trims.iter().map(|_| Err(error.clone())).collect(),
	};
	for ((key, cold), result) in trims.into_iter().zip(results) {
		trimmed(state, key, worker.clone(), cold, result, effects);
	}
}

/// Only an in-sync member's disk acknowledgement advances the published cold position.
pub(crate) fn trimmed(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	worker: WorkerId,
	cold: VolumeLogPosition,
	result: Result<StorageWorkerResponse, String>,
	effects: &mut Effects,
) {
	if matches!(
		result,
		Ok(StorageWorkerResponse::RejectResponse(
			wire::RejectResponse::VolumeNotLoaded
		))
	) {
		if let Some(volume) = state.loaded_volumes.get_mut(&key) {
			volume.members.remove(&worker);
			super::catch_up::mark(state, &key);
		}
		return;
	}
	let Ok(StorageWorkerResponse::ReplicaStateResponse(reply)) = result else {
		back_off_trim(state, key, worker, effects);
		return;
	};
	let acknowledged = reply.state.cold.volume_seq >= cold.volume_seq;
	if acknowledged
		&& reply.state.status == VolumeLogReplicaStatus::InSync
		&& state
			.pending_trims
			.get(&key)
			.is_some_and(|pending| reply.state.cold.volume_seq >= pending.volume_seq)
	{
		state.pending_trims.remove(&key);
	}
	observe(state, &key, worker.clone(), reply.state);
	if !acknowledged {
		back_off_trim(state, key, worker, effects);
	}
}

/// Holds the member's trims until `TrimRetry` fires. A failed trim marks its volume, and
/// the drives that queue trims would otherwise resend it in the same instant.
fn back_off_trim(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	worker: WorkerId,
	effects: &mut Effects,
) {
	effects.arm_timer_after(
		MaintainerTimer::TrimRetry(key.clone(), worker.clone()),
		state.config.tuning().retry_ms,
	);
	state.trim_backoff.insert((key, worker));
}

/// Ends a member's trim backoff when its `TrimRetry` fires and queues the trim again.
pub(crate) fn retry_trim(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	worker: WorkerId,
	effects: &mut Effects,
) {
	state.trim_backoff.remove(&(key.clone(), worker.clone()));
	trim(state, key, worker, effects);
}

/// Deduplicates member work while a bounded transport window drains it.
pub(crate) fn trim(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	worker: WorkerId,
	_effects: &mut Effects,
) {
	state.trim_ready.insert((key, worker));
}

/// Trim does not need a new disk read; it uses only completed cold-pass evidence. Ready
/// trims of one member go out together, up to `max_forget_batch_volumes` per request, in one
/// pass over the ready set.
pub(crate) fn drive_trims(state: &mut MaintainerState, effects: &mut Effects) {
	if state.phase != MaintainerPhase::Serving || state.trim_ready.is_empty() {
		return;
	}
	let max_batch = state.config.max_forget_batch_volumes.max(1) as usize;
	let index = state.operations.index();
	let mut budget = (state.config.tuning().max_trim_requests as usize)
		.saturating_sub(index.counts.trim as usize);
	let mut taken = Vec::new();
	let mut acknowledged = Vec::new();
	let mut groups = BTreeMap::<WorkerId, Vec<VolumeLogKey>>::new();
	// Entries skipped here are bounded by in-flight trims and by the entries of volumes
	// whose deletion is ambiguous, and each costs a map lookup. An entry with no trim due
	// leaves the ready set without using the window, as a single trim's dispatch would.
	for entry in &state.trim_ready {
		if budget == 0 {
			break;
		}
		if super::deletion::ambiguous(state, &entry.0.0.volume_key)
			|| index.trims.contains_key(entry)
		{
			continue;
		}
		let group = groups.entry(entry.1.clone()).or_default();
		if group.len() >= max_batch {
			continue;
		}
		taken.push(entry.clone());
		match trim_item(state, &entry.0, &entry.1) {
			TrimDue::Send(..) => {
				group.push(entry.0.clone());
				budget -= 1;
			}
			TrimDue::Acknowledged => acknowledged.push(entry.0.clone()),
			TrimDue::NotDue => {}
		}
	}
	for entry in &taken {
		state.trim_ready.remove(entry);
	}
	for key in &acknowledged {
		state.pending_trims.remove(key);
	}
	for (worker, keys) in groups {
		dispatch_trims(state, worker, keys, effects);
	}
}


