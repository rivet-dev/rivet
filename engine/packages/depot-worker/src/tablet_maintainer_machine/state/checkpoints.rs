//! Checkpoint commands are nested work in the existing tablet state machine.
//! Administrative I/O is serialized with apply, flush, deletion and split work.
use super::{MaintainerPhase, MaintainerState, Operation, store::StoreWork};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::MaintainerEffect,
	types::{MaintainerTimer, OpId, RequestId},
};
use depot_fs_core::checkpoints::CheckpointResult;
use depot_types::VolumeLogKey;
use rivet_data::generated::depot_client_protocol_v1::*;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Pending {
	request: VolumeCheckpointRequest,
	phase: Phase,
}

impl Pending {
	/// The volume the checkpoint request names.
	pub(crate) fn volume_key(&self) -> &depot_types::VolumeKey {
		&self.request.volume_key
	}
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Phase {
	/// A volume barrier refreshes every member's view of the target volume before the
	/// nonholder's committed target is fixed.
	Observing {
		barrier: super::poll::VolumeBarrier,
	},
	/// Waits for the volume's apply to reach `through`: the committed frontier a new
	/// checkpoint captured, or the position of the holder's logged op a confirmation names.
	/// Later appends cannot extend this finite apply barrier.
	Applying {
		key: VolumeLogKey,
		through: u64,
	},
	Queued,
	/// The target incarnation's unload has started forgetting replicas, so what the
	/// checkpoint sees depends on how the unload ends. The request waits, under the caller's
	/// deadline, and `resume_after_unload` admits it again once no unload blocks the key.
	/// `deadline` is the caller's, kept to re-admit with the same bound.
	AwaitingUnload {
		deadline: u64,
	},
}

/// Why `check` refuses a request.
enum Refusal {
	/// Returned to the caller as is.
	Reject(VolumeRejectResponse),
	/// The target incarnation is unloading. Admission cancels or waits for the unload;
	/// any later check answers `Overloaded`, as before.
	Unloading,
}

impl Refusal {
	fn into_reject(self) -> VolumeRejectResponse {
		match self {
			Refusal::Reject(reject) => reject,
			Refusal::Unloading => depot_types::client::overloaded(),
		}
	}
}

/// The position of the holder's logged op that a delete or retention change confirms.
pub(crate) fn logged_minimum(operation: &CheckpointOperation) -> Option<&CheckpointPosition> {
	match operation {
		CheckpointOperation::CheckpointDelete(op) => op.minimum.as_ref(),
		CheckpointOperation::CheckpointSetRetention(op) => op.minimum.as_ref(),
		CheckpointOperation::CheckpointCreate(_)
		| CheckpointOperation::CheckpointInspect(_)
		| CheckpointOperation::CheckpointList(_) => None,
	}
}

/// The position of a holder's logged checkpoint op that this request confirms after apply.
fn confirmed_minimum(operation: &CheckpointOperation) -> Option<&CheckpointPosition> {
	match operation {
		CheckpointOperation::CheckpointCreate(op) => op.minimum.as_ref(),
		CheckpointOperation::CheckpointDelete(_)
		| CheckpointOperation::CheckpointSetRetention(_) => logged_minimum(operation),
		CheckpointOperation::CheckpointInspect(_) | CheckpointOperation::CheckpointList(_) => None,
	}
}

fn target(state: &MaintainerState, key: &depot_types::VolumeKey) -> Option<VolumeLogKey> {
	state
		.loaded_volumes
		.range(super::incarnations(key))
		.map(|(k, _)| k)
		.find(|k| !super::deletion::is_replaced(state, k))
		.cloned()
}
fn check(state: &MaintainerState, request: &VolumeCheckpointRequest) -> Result<(), Refusal> {
	use Refusal::Reject;
	match state.phase {
		MaintainerPhase::Frozen => return Err(Reject(VolumeRejectResponse::TabletSplitting)),
		MaintainerPhase::Serving => {}
		_ => return Err(Reject(depot_types::client::not_maintainer())),
	}
	if !state
		.tablet
		.hash_range
		.contains(depot_types::partition_key_hash(&request.volume_key))
	{
		return Err(Reject(depot_types::client::not_maintainer()));
	}
	if super::forks::quiescing(&state.forks, &request.volume_key)
		|| state.pending.contains_key(&request.volume_key)
		|| super::deletion::ambiguous(state, &request.volume_key)
		|| super::maintainer_log::replay::pending_for(state, &request.volume_key)
	{
		return Err(Reject(depot_types::client::overloaded()));
	}
	// A name whose current incarnation has no stored record takes no checkpoint operation,
	// a listing included: a logged change would wait for an apply that never comes, and the
	// name is unavailable until that incarnation is deleted.
	if let Some(incarnation) = super::filesystem::record_missing(state, &request.volume_key) {
		return Err(Reject(depot_types::client::record_missing(incarnation)));
	}
	// Listing and exact flag removal also address retained incarnations after
	// logical deletion. Their ordered store step revalidates the durable rows.
	if matches!(
		request.operation,
		CheckpointOperation::CheckpointList(_) | CheckpointOperation::CheckpointDelete(_)
	) {
		return Ok(());
	}
	if let Some(key) = target(state, &request.volume_key) {
		if state.unloading.contains_key(&key) {
			return Err(Refusal::Unloading);
		}
		if state.deleting.contains_key(&key) || state.loaded_volumes[&key].is_deleted() {
			return Err(Reject(VolumeRejectResponse::VolumeDeleted));
		}
		if state.loaded_volumes[&key]
			.creation
			.as_ref()
			.is_some_and(|c| !c.acknowledged)
		{
			return Err(Reject(depot_types::client::overloaded()));
		}
	}
	Ok(())
}
/// Whether `request` would log an op on an incarnation whose drain is held. A drain applies a
/// tail that a single replica may hold, and a logged op records the volume's applied position
/// as committed. A restore abandoned after a crash leaves the incarnation live, so an op
/// logged inside that tail would outlast it as false commit evidence. Such changes wait until
/// the drain is gone. Only admission and dispatch ask: a change already logged is answered
/// with its result.
fn waits_for_drain(state: &MaintainerState, request: &VolumeCheckpointRequest) -> bool {
	needs_log(&request.operation)
		&& target(state, &request.volume_key).is_some_and(|key| {
			change_targets_live(&request.operation, &key) && super::drain::held(state, &key)
		})
}

fn reply(state: &mut MaintainerState, id: RequestId, response: ClientResponse, fx: &mut Effects) {
	state.pending_checkpoints.remove(&id);
	fx.cancel_timer(MaintainerTimer::CheckpointRequest(id));
	fx.execute(MaintainerEffect::Reply {
		request_id: id,
		response,
	});
}
pub(crate) fn admit(
	state: &mut MaintainerState,
	id: RequestId,
	request: VolumeCheckpointRequest,
	deadline: u64,
	fx: &mut Effects,
) {
	if state.pending_checkpoints.contains_key(&id) {
		return;
	}
	if !depot_fs_core::checkpoints::valid_operation(&request.operation) {
		reply(
			state,
			id,
			ClientResponse::VolumeCheckpointResponse(VolumeCheckpointResponse::CheckpointError(
				CheckpointError::Invalid,
			)),
			fx,
		);
		return;
	}
	if deadline <= fx.now_boot_ms()
		|| state.request_count() >= state.config.max_requests as usize
		|| request.volume_key.byte_len() > state.config.tuning().max_volume_key_bytes as usize
	{
		reply(
			state,
			id,
			ClientResponse::VolumeRejectResponse(depot_types::client::overloaded()),
			fx,
		);
		return;
	}
	let mut refusal = check(state, &request).err();
	if matches!(refusal, Some(Refusal::Unloading)) {
		// Admitted like an open of the key: an unload that has not started forgetting
		// replicas is cancelled, so the volume stays loaded and serves the checkpoint. Once
		// forgets have started, the checkpoint waits for the unload to finish.
		if super::lifecycle::unload_blocks_open(state, &request.volume_key) {
			state.pending_checkpoints.insert(
				id,
				Pending {
					request,
					phase: Phase::AwaitingUnload { deadline },
				},
			);
			fx.arm_timer_at(MaintainerTimer::CheckpointRequest(id), deadline);
			return;
		}
		super::unload::cancel_key(state, &request.volume_key, fx);
		refusal = check(state, &request).err();
	}
	if refusal.is_none() && waits_for_drain(state, &request) {
		refusal = Some(Refusal::Reject(depot_types::client::overloaded()));
	}
	if let Some(refusal) = refusal {
		reply(
			state,
			id,
			ClientResponse::VolumeRejectResponse(refusal.into_reject()),
			fx,
		);
		return;
	}
	if let Some(key) = target(state, &request.volume_key) {
		state.loaded_volumes.get_mut(&key).unwrap().last_activity = fx.now_boot_ms();
		super::filesystem::load(state, key);
	}
	let observed = match &request.operation {
		CheckpointOperation::CheckpointCreate(CheckpointCreate { minimum: None, .. }) => {
			target(state, &request.volume_key)
		}
		_ => None,
	};
	let phase = if let Some(key) = observed {
		Phase::Observing {
			barrier: super::poll::refresh_volume(state, key, fx),
		}
	} else {
		state
			.store_queue
			.push_back(StoreWork::Checkpoint { request_id: id });
		Phase::Queued
	};
	state
		.pending_checkpoints
		.insert(id, Pending { request, phase });
	fx.arm_timer_at(MaintainerTimer::CheckpointRequest(id), deadline);
}
pub(crate) fn dispatch(state: &mut MaintainerState, id: RequestId, fx: &mut Effects) {
	let Some(pending) = state.pending_checkpoints.get(&id).cloned() else {
		return;
	};
	let request = pending.request;
	let mut refusal = check(state, &request).err();
	if refusal.is_none() && waits_for_drain(state, &request) {
		refusal = Some(Refusal::Reject(depot_types::client::overloaded()));
	}
	if let Some(refusal) = refusal {
		reply(
			state,
			id,
			ClientResponse::VolumeRejectResponse(refusal.into_reject()),
			fx,
		);
		return;
	}
	let key = target(state, &request.volume_key);
	let volume = key.as_ref().and_then(|k| state.filesystem.volumes.get(k));
	let record = volume.map(|v| v.record.to_record());
	let applied = volume.map(|v| v.applied.clone());
	// A confirmation of a holder's logged op is answered once the apply passes that op, not
	// refused as lagging for the holder to ask again. The op's position is committed, so it is
	// also the commit hint that starts the apply. The request keeps its deadline timer, and
	// `drive` queues it again once the volume has applied that far.
	if let Some(minimum) = confirmed_minimum(&request.operation)
		&& applied
			.as_ref()
			.is_some_and(|p| p.volume_seq < minimum.position.volume_seq)
		&& let Some(key) = key.clone()
	{
		let through = minimum.position.volume_seq;
		let same_incarnation = minimum.volume_incarnation == key.0.volume_incarnation;
		super::filesystem::committed(state, key.clone(), through, fx);
		if same_incarnation {
			state
				.pending_checkpoints
				.get_mut(&id)
				.expect("pending")
				.phase = Phase::Applying { key, through };
			return;
		}
	}
	let releases = matches!(request.operation, CheckpointOperation::CheckpointDelete(_));
	// A change without a holder has no volume log entry, so its durable record is a
	// maintainer log op. The operation keeps its volume's footprint while the op commits,
	// which holds the volume's apply at `applied`, the position the op follows.
	if needs_log(&request.operation) {
		if !super::maintainer_log::admits(state) {
			reply(
				state,
				id,
				ClientResponse::VolumeRejectResponse(depot_types::client::overloaded()),
				fx,
			);
			return;
		}
		let target = key
			.clone()
			.zip(applied.clone())
			.filter(|(key, _)| change_targets_live(&request.operation, key));
		let created_at = fx.clocks().unix_ms;
		let op = rivet_data::generated::depot_maintainer_log_v1::MaintainerOp::CheckpointOp(
			rivet_data::generated::depot_maintainer_log_v1::CheckpointOp {
				request: request.clone(),
				created_at_unix_ms: created_at,
				target: target.clone().map(|(key, after)| {
					rivet_data::generated::depot_maintainer_log_v1::MaintainerOpTarget {
						volume_log_key: key.0,
						after,
					}
				}),
			},
		);
		let op_id = state.begin(
			Operation::Checkpoint {
				request_id: id,
				volume_key: request.volume_key.clone(),
				releases,
				log: None,
			},
			fx,
		);
		let token = super::maintainer_log::submit(
			state,
			op,
			target,
			super::maintainer_log::Waiter::Store(op_id),
		);
		state.operations.update(&op_id, |operation| {
			if let Operation::Checkpoint { log, .. } = operation {
				*log = Some((token, created_at));
			}
		});
		return;
	}
	let op_id = state.begin(
		Operation::Checkpoint {
			request_id: id,
			volume_key: request.volume_key.clone(),
			releases,
			log: None,
		},
		fx,
	);
	fx.execute(MaintainerEffect::CheckpointRequest {
		op_id,
		request,
		record,
		applied,
		logged: None,
	});
}

/// Whether a checkpoint request changes rows with no holder's log entry behind it: a
/// create, delete or retention change without a logged minimum.
fn needs_log(operation: &CheckpointOperation) -> bool {
	match operation {
		CheckpointOperation::CheckpointCreate(op) => op.minimum.is_none(),
		CheckpointOperation::CheckpointDelete(op) => op.minimum.is_none(),
		CheckpointOperation::CheckpointSetRetention(op) => op.minimum.is_none(),
		CheckpointOperation::CheckpointInspect(_) | CheckpointOperation::CheckpointList(_) => false,
	}
}

/// Whether a change acts on the live incarnation `key`, whose log the op follows. A delete
/// of a retired incarnation's checkpoint follows no volume log.
fn change_targets_live(operation: &CheckpointOperation, key: &VolumeLogKey) -> bool {
	match operation {
		CheckpointOperation::CheckpointDelete(op) => {
			op.volume_incarnation == key.0.volume_incarnation
		}
		CheckpointOperation::CheckpointCreate(_)
		| CheckpointOperation::CheckpointSetRetention(_)
		| CheckpointOperation::CheckpointInspect(_)
		| CheckpointOperation::CheckpointList(_) => true,
	}
}

/// The maintainer log op of checkpoint operation `op_id` committed at `token`: apply the
/// change, at the position the volume's apply is held at.
pub(crate) fn logged(
	state: &mut MaintainerState,
	op_id: OpId,
	token: super::maintainer_log::LogToken,
	fx: &mut Effects,
) {
	let Some(Operation::Checkpoint {
		log: Some((_, created_at)),
		..
	}) = state.operations.get(&op_id).cloned()
	else {
		return;
	};
	let Some(rivet_data::generated::depot_maintainer_log_v1::MaintainerOp::CheckpointOp(op)) =
		super::maintainer_log::op(state, token)
	else {
		super::fence(state, fx);
		return;
	};
	let (record, applied) = match op.target.as_ref() {
		Some(target) => {
			let key = VolumeLogKey(target.volume_log_key.clone());
			let volume = state.filesystem.volumes.get(&key);
			(
				volume.map(|v| v.record.to_record()),
				Some(target.after.clone()),
			)
		}
		None => {
			let key = target(state, &op.request.volume_key);
			let volume = key.as_ref().and_then(|k| state.filesystem.volumes.get(k));
			(
				volume.map(|v| v.record.to_record()),
				volume.map(|v| v.applied.clone()),
			)
		}
	};
	fx.execute(MaintainerEffect::CheckpointRequest {
		op_id,
		request: op.request,
		record,
		applied,
		logged: Some(created_at),
	});
}
pub(crate) fn completed(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<CheckpointResult, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(Operation::Checkpoint { .. })
	) {
		return;
	}
	let Some(Operation::Checkpoint {
		request_id, log, ..
	}) = state.finish(id, fx)
	else {
		unreachable!()
	};
	let Ok(result) = result else {
		super::fence(state, fx);
		return;
	};
	if let Some((token, _)) = log {
		super::maintainer_log::applied(state, token, fx);
	}
	// Even an expired caller leaves a durable result that must update our view.
	if let Some(record) = &result.record {
		super::filesystem::record_flushed(state, record, fx);
	}
	let Some(pending) = state.pending_checkpoints.get(&request_id) else {
		return;
	};
	let response = match check(state, &pending.request) {
		Ok(()) => ClientResponse::VolumeCheckpointResponse(result.response),
		Err(refusal) => ClientResponse::VolumeRejectResponse(refusal.into_reject()),
	};
	reply(state, request_id, response, fx);
}
pub(crate) fn expired(state: &mut MaintainerState, id: RequestId, fx: &mut Effects) {
	if !state.pending_checkpoints.contains_key(&id) {
		return;
	}
	state
		.store_queue
		.remove_all(&[StoreWork::Checkpoint { request_id: id }]);
	reply(
		state,
		id,
		ClientResponse::VolumeRejectResponse(depot_types::client::not_maintainer()),
		fx,
	);
}

/// Captures one finite committed frontier after refreshed replica observations.
/// Waiting here leaves the store queue free for the apply work that satisfies it.
pub(crate) fn drive(state: &mut MaintainerState, fx: &mut Effects) {
	let ids = state
		.pending_checkpoints
		.keys()
		.copied()
		.collect::<Vec<_>>();
	for id in ids {
		let pending = state.pending_checkpoints[&id].clone();
		// A request waiting for an unload is admitted again by `resume_after_unload`.
		if matches!(pending.phase, Phase::AwaitingUnload { .. }) {
			continue;
		}
		if let Err(refusal) = check(state, &pending.request) {
			reply(
				state,
				id,
				ClientResponse::VolumeRejectResponse(refusal.into_reject()),
				fx,
			);
			continue;
		}
		let phase = match pending.phase {
			Phase::Observing { barrier } if super::poll::volume_refreshed(state, &barrier) => {
				let Some(key) = target(state, &pending.request.volume_key) else {
					state
						.pending_checkpoints
						.get_mut(&id)
						.expect("pending")
						.phase = Phase::Queued;
					state
						.store_queue
						.push_back(StoreWork::Checkpoint { request_id: id });
					continue;
				};
				// A different incarnation became the target while the barrier ran, and
				// none of its members has been read since the request.
				if key != barrier.key {
					state
						.pending_checkpoints
						.get_mut(&id)
						.expect("pending")
						.phase = Phase::Observing {
						barrier: super::poll::refresh_volume(state, key, fx),
					};
					continue;
				}
				let Some(volume) = state.filesystem.volumes.get(&key) else {
					continue;
				};
				let through =
					super::filesystem::latest_committed(state, &key, volume.applied.volume_seq);
				Phase::Applying { key, through }
			}
			phase => phase,
		};
		let ready = match &phase {
			Phase::Applying { key, through } => state
				.filesystem
				.volumes
				.get(key)
				.is_some_and(|v| v.applied.volume_seq >= *through),
			_ => false,
		};
		state
			.pending_checkpoints
			.get_mut(&id)
			.expect("pending")
			.phase = if ready {
			state
				.store_queue
				.push_back(StoreWork::Checkpoint { request_id: id });
			Phase::Queued
		} else {
			phase
		};
	}
}

/// Admits again every checkpoint waiting for an unload that no longer blocks its key, or every
/// waiting checkpoint outside `Serving`, so it gets the phase's own answer at once. Runs after
/// the unload drive on every event.
pub(crate) fn resume_after_unload(state: &mut MaintainerState, fx: &mut Effects) {
	let serving = state.phase == MaintainerPhase::Serving;
	let released = state
		.pending_checkpoints
		.iter()
		.filter_map(|(id, pending)| match pending.phase {
			Phase::AwaitingUnload { deadline }
				if !serving
					|| !super::lifecycle::unload_blocks_open(
						state,
						&pending.request.volume_key,
					) =>
			{
				Some((*id, deadline))
			}
			Phase::AwaitingUnload { .. }
			| Phase::Observing { .. }
			| Phase::Applying { .. }
			| Phase::Queued => None,
		})
		.collect::<Vec<_>>();
	for (id, deadline) in released {
		let Some(pending) = state.pending_checkpoints.remove(&id) else {
			continue;
		};
		fx.cancel_timer(MaintainerTimer::CheckpointRequest(id));
		admit(state, id, pending.request, deadline, fx);
	}
}


