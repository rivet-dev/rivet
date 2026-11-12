//! Checkpoint commands are nested work in the existing tablet state machine.
//! Administrative I/O is serialized with apply, flush, deletion and split work.
use super::{
	MaintainerPhase, MaintainerState, Operation, filesystem::LatestBarrier, store::StoreWork,
};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::MaintainerEffect,
	types::{MaintainerTimer, OpId, RequestId},
};
use depot_fs_core::checkpoints::CheckpointResult;
use depot_types::VolumeLogKey;
use rivet_data::generated::depot_client_protocol_v2::*;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Pending {
	request: VolumeCheckpointRequest,
	phase: Phase,
	/// The caller's deadline, at which `CheckpointRequest` answers the request unless a
	/// checkpoint entering `Proving` moved that timer earlier. A request waiting for an unload
	/// is admitted again with it.
	deadline_boot_ms: u64,
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
	/// The barrier found a member tail above what the members prove committed, so a commit the
	/// holder acknowledged before the call may lie above it. Entered from `Observing` for a
	/// checkpoint with no minimum; left for `Applying` once `filesystem::tail_proven_through`
	/// proves `frontier`, the highest member tail the barrier saw, or a claim newer than
	/// `claim_epoch`, the newest claim it saw. A live writer proves its tail with its next
	/// appends, and a dead claimant's is proven by the recovery of the volume's holder-gone or
	/// idle unload, which a nonholder checkpoint neither cancels nor holds off. A claimant whose
	/// process lives on but no longer writes, such as a sleeping actor's, proves nothing, so the
	/// checkpoint waits here for half of what was left of its deadline when it entered and is
	/// then refused as `CheckpointError::TailUnproven`, early enough for the refusal to reach a
	/// caller whose transport shares that deadline. A caller that knows the holder is quiet
	/// proves the tail by opening the volume, whose recovery commits a newer claim, or by having
	/// the holder write, and sends the checkpoint again. No I/O is in flight.
	Proving {
		key: VolumeLogKey,
		frontier: u64,
		claim_epoch: u64,
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
	/// checkpoint sees depends on how the unload ends, or it is a `Latest` capture waiting for
	/// the unload's recovery to prove the tail (`spares_unload`). The request waits, under the
	/// caller's deadline, and `resume_after_unload` admits it again with that deadline once no
	/// unload blocks the key and none spares it.
	AwaitingUnload,
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
		// forgets have started, the checkpoint waits for the unload to finish. A `Latest`
		// capture also waits, rather than cancel it, for an unload whose recovery is what
		// proves the volume's tail (see `spares_unload`): that is work on its own volume, under
		// its own deadline. Every other operation needs no proof and cancels the unload. A
		// cancelled unload's idle check waits a full idle period again, as a nonholder
		// checkpoint is not its holder's activity.
		let spared = spares_unload(state, &request);
		if spared || super::lifecycle::unload_blocks_open(state, &request.volume_key) {
			state.pending_checkpoints.insert(
				id,
				Pending {
					request,
					phase: Phase::AwaitingUnload,
					deadline_boot_ms: deadline,
				},
			);
			fx.arm_timer_at(MaintainerTimer::CheckpointRequest(id), deadline);
			return;
		}
		super::unload::cancel_key(state, &request.volume_key, fx);
		if let Some(key) = target(state, &request.volume_key) {
			super::unload::postpone_idle(state, &key);
		}
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
	// Only the holder's own confirmation counts as its activity. A nonholder's request, such as
	// a retried `Latest` checkpoint, must not hold off the holder check whose unload recovers a
	// gone holder's tail, which that checkpoint may be waiting for.
	if let Some(key) = target(state, &request.volume_key) {
		if confirmed_minimum(&request.operation).is_some() {
			state.loaded_volumes.get_mut(&key).unwrap().last_activity = fx.now_boot_ms();
		}
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
	state.pending_checkpoints.insert(
		id,
		Pending {
			request,
			phase,
			deadline_boot_ms: deadline,
		},
	);
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
	// The volume has one claim log, so a minimum names claim log 0 alone.
	if let Some(minimum) = confirmed_minimum(&request.operation)
		&& let Some(minimum_position) = minimum.cut.root_position()
		&& applied
			.as_ref()
			.is_some_and(|p| p.volume_seq < minimum_position.volume_seq)
		&& let Some(key) = key.clone()
	{
		let through = minimum_position.volume_seq;
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
		let op = rivet_data::generated::depot_maintainer_log_v2::MaintainerOp::CheckpointOp(
			rivet_data::generated::depot_maintainer_log_v2::CheckpointOp {
				request: request.clone(),
				created_at_unix_ms: created_at,
				targets: targets(target.clone()),
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
		claims: None,
	});
}

/// The targets of a logged change that follows `target`'s apply: the one target at `target`'s
/// own position, a position its log has already applied, so it cannot hold the change back. A
/// change of a retired incarnation has none.
pub(crate) fn targets(
	target: Option<(
		VolumeLogKey,
		rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
	)>,
) -> Vec<rivet_data::generated::depot_maintainer_log_v2::MaintainerOpTarget> {
	use rivet_data::generated::depot_maintainer_log_v2::MaintainerOpTarget;
	let Some((key, after)) = target else {
		return Vec::new();
	};
	vec![MaintainerOpTarget {
		volume_log_key: key.0,
		after,
	}]
}

/// The cut a logged change with several targets takes its checkpoint at: each target's
/// position by its claim log. `None` for a change with one target or none.
pub(crate) fn targets_cut(
	targets: &[rivet_data::generated::depot_maintainer_log_v2::MaintainerOpTarget],
) -> Option<rivet_data::generated::shared_depot_fs_v2::FsCut> {
	use rivet_data::generated::shared_depot_fs_v2::{FsClaimLogPosition, FsCut};
	if targets.len() < 2 {
		return None;
	}
	let mut logs = targets
		.iter()
		.map(|target| FsClaimLogPosition {
			claim_log: super::claims::base_of(&VolumeLogKey(target.volume_log_key.clone())).1,
			position: target.after.clone(),
		})
		.collect::<Vec<_>>();
	logs.sort_by_key(|log| log.claim_log);
	Some(FsCut { logs })
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
	let Some(rivet_data::generated::depot_maintainer_log_v2::MaintainerOp::CheckpointOp(op)) =
		super::maintainer_log::op(state, token)
	else {
		super::fence(state, fx);
		return;
	};
	let (record, applied) = match op.targets.first() {
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
	let claims = targets_cut(&op.targets);
	fx.execute(MaintainerEffect::CheckpointRequest {
		op_id,
		request: op.request,
		record,
		applied,
		logged: Some(created_at),
		claims,
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
/// A checkpoint request's deadline passed before it finished, or a checkpoint waited out its
/// `Proving` share of it. A checkpoint still proving the volume's tail is refused as
/// `CheckpointError::TailUnproven`, so its caller can tell a tail only its holder or a recovery
/// proves from a maintainer that did not answer; any other request is answered
/// `not_maintainer`, and its caller sends it again.
///
/// Entering `Proving` moves the request's timer to half its remaining deadline. A checkpoint
/// that left `Proving` for another phase before that timer fired has not reached its own
/// deadline, so the timer is armed again at the full deadline instead of answering early.
pub(crate) fn expired(state: &mut MaintainerState, id: RequestId, fx: &mut Effects) {
	let Some(pending) = state.pending_checkpoints.get(&id) else {
		return;
	};
	let proving = matches!(pending.phase, Phase::Proving { .. });
	if !proving && fx.now_boot_ms() < pending.deadline_boot_ms {
		fx.arm_timer_at(
			MaintainerTimer::CheckpointRequest(id),
			pending.deadline_boot_ms,
		);
		return;
	}
	let response = match pending.phase {
		Phase::Proving { .. } => ClientResponse::VolumeCheckpointResponse(
			VolumeCheckpointResponse::CheckpointError(CheckpointError::TailUnproven),
		),
		Phase::Observing { .. }
		| Phase::Applying { .. }
		| Phase::Queued
		| Phase::AwaitingUnload => {
			ClientResponse::VolumeRejectResponse(depot_types::client::not_maintainer())
		}
	};
	state
		.store_queue
		.remove_all(&[StoreWork::Checkpoint { request_id: id }]);
	reply(state, id, response, fx);
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
		if matches!(pending.phase, Phase::AwaitingUnload) {
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
				match super::filesystem::latest_barrier(state, &key, volume.applied.volume_seq) {
					LatestBarrier::Proven { through } => Phase::Applying { key, through },
					LatestBarrier::Unproven {
						frontier,
						claim_epoch,
					} => {
						let now_boot_ms = fx.now_boot_ms();
						let refuse_boot_ms =
							now_boot_ms + pending.deadline_boot_ms.saturating_sub(now_boot_ms) / 2;
						fx.arm_timer_at(MaintainerTimer::CheckpointRequest(id), refuse_boot_ms);
						Phase::Proving {
							key,
							frontier,
							claim_epoch,
						}
					}
				}
			}
			// As in `Observing`, a different incarnation that became the target starts a new
			// barrier over it, and a target no longer loaded sends the request to the store,
			// which answers it, instead of waiting out the deadline.
			Phase::Proving {
				key,
				frontier,
				claim_epoch,
			} => match target(state, &pending.request.volume_key) {
				Some(current) if current != key => Phase::Observing {
					barrier: super::poll::refresh_volume(state, current, fx),
				},
				Some(_) => {
					match super::filesystem::tail_proven_through(state, &key, frontier, claim_epoch)
					{
						Some(through) => Phase::Applying { key, through },
						None => Phase::Proving {
							key,
							frontier,
							claim_epoch,
						},
					}
				}
				None => {
					state
						.pending_checkpoints
						.get_mut(&id)
						.expect("pending")
						.phase = Phase::Queued;
					state
						.store_queue
						.push_back(StoreWork::Checkpoint { request_id: id });
					continue;
				}
			},
			phase @ (Phase::Observing { .. }
			| Phase::Applying { .. }
			| Phase::Queued
			| Phase::AwaitingUnload) => phase,
		};
		let ready = match &phase {
			Phase::Applying { key, through } => state
				.filesystem
				.volumes
				.get(key)
				.is_some_and(|v| v.applied.volume_seq >= *through),
			Phase::Observing { .. }
			| Phase::Proving { .. }
			| Phase::Queued
			| Phase::AwaitingUnload => false,
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

/// Whether `request` waits for the unload of its target instead of cancelling it: a `Latest`
/// capture, the one operation that waits for the volume's tail to be proven, while the unload's
/// recovery is what proves it and the capture's own barrier would read it unproven. The capture
/// is admitted again once the tail is proven or the unload ended.
///
/// This reads the members' last observations without refreshing them, while the capture decides
/// after an `Observing` refresh. A stale "proven" here cancels a recovering unload; the capture
/// then waits in `Proving` until half its deadline and is refused as tail unproven, and the next
/// unload waits a full idle period. That is bounded and rare, so admission does not wait for a
/// refresh it would rarely need.
fn spares_unload(state: &MaintainerState, request: &VolumeCheckpointRequest) -> bool {
	let latest_capture = matches!(
		&request.operation,
		CheckpointOperation::CheckpointCreate(CheckpointCreate { minimum: None, .. })
	);
	latest_capture
		&& target(state, &request.volume_key).is_some_and(|key| {
			let applied = state
				.filesystem
				.volumes
				.get(&key)
				.map_or(0, |volume| volume.applied.volume_seq);
			state.unloading.contains_key(&key)
				&& matches!(
					super::filesystem::latest_barrier(state, &key, applied),
					LatestBarrier::Unproven { .. }
				)
		})
}

/// Admits again every checkpoint waiting for an unload that no longer blocks or spares its key,
/// or every waiting checkpoint outside `Serving`, so it gets the phase's own answer at once.
/// Runs after the unload drive on every event.
pub(crate) fn resume_after_unload(state: &mut MaintainerState, fx: &mut Effects) {
	let serving = state.phase == MaintainerPhase::Serving;
	let released = state
		.pending_checkpoints
		.iter()
		.filter_map(|(id, pending)| match pending.phase {
			Phase::AwaitingUnload
				if !serving
					|| (!super::lifecycle::unload_blocks_open(
						state,
						&pending.request.volume_key,
					) && !spares_unload(state, &pending.request)) =>
			{
				Some((*id, pending.deadline_boot_ms))
			}
			Phase::AwaitingUnload
			| Phase::Observing { .. }
			| Phase::Proving { .. }
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
