//! Peer reference administration is nested in the existing tablet owner.
//! Latest captures one finite source frontier; exact receipts remain authoritative
//! after deletion or restoration of the logical source.
use super::{MaintainerPhase, MaintainerState, Operation, store::StoreWork};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::MaintainerEffect,
	types::{MaintainerTimer, OpId, RequestId},
};
use depot_fs_core::{
	FsForkError, FsForkSelector,
	forks::{ReferenceBarrier, ReferenceResult},
};
use depot_types::VolumeLogKey;
use rivet_data::generated::depot_client_protocol_v1::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Pending {
	request: FsForkReferenceRequest,
	phase: Phase,
}

impl Pending {
	pub(crate) fn spec(&self) -> &rivet_data::generated::shared_depot_fs_v1::FsForkSpec {
		self.request.spec()
	}
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Phase {
	/// Read the immutable receipt before any mutable lifecycle or replica barrier.
	CheckingReceipt,
	/// A volume barrier over the source volume precedes selection of a loaded Latest source.
	Observing { barrier: super::poll::VolumeBarrier },
	/// Later appends cannot extend the selected finite apply barrier.
	Applying { key: VolumeLogKey, through: u64 },
	/// Metadata work is queued or dispatched; the operation separately owns I/O.
	Queued,
}
fn target(state: &MaintainerState, request: &FsForkReferenceRequest) -> Option<VolumeLogKey> {
	state
		.loaded_volumes
		.range(super::incarnations(request.volume_key()))
		.map(|(key, _)| key)
		.find(|key| !super::deletion::is_replaced(state, key))
		.cloned()
}
fn check(
	state: &MaintainerState,
	request: &FsForkReferenceRequest,
) -> Result<(), VolumeRejectResponse> {
	match state.phase {
		MaintainerPhase::Serving => {}
		MaintainerPhase::Frozen => return Err(VolumeRejectResponse::TabletSplitting),
		_ => return Err(depot_types::client::not_maintainer()),
	}
	if !state
		.tablet
		.hash_range
		.contains(depot_types::partition_key_hash(request.volume_key()))
	{
		return Err(depot_types::client::not_maintainer());
	}
	// In particular, do not reject a deleted/replaced source here: its exact
	// Held or Released receipt must be read before current-volume validation.
	Ok(())
}
fn source_ready(
	state: &MaintainerState,
	request: &FsForkReferenceRequest,
) -> Result<(), VolumeRejectResponse> {
	// Recovered maintainer log ops of the parent may hold its references or checkpoints.
	if super::maintainer_log::replay::pending_for(state, request.volume_key()) {
		return Err(depot_types::client::overloaded());
	}
	// Busy lifecycle work may still have an unacknowledged creation in RAM.
	// Return a transient result; never turn it into a terminal negative receipt.
	if matches!(request, FsForkReferenceRequest::FsForkAcquire(_))
		&& (super::forks::quiescing(&state.forks, request.volume_key())
			|| state.pending.contains_key(request.volume_key())
			|| super::deletion::ambiguous(state, request.volume_key())
			|| target(state, request).is_some_and(|key| {
				state.unloading.contains_key(&key)
					|| state.loaded_volumes[&key]
						.creation
						.as_ref()
						.is_some_and(|c| !c.acknowledged)
			})) {
		return Err(depot_types::client::overloaded());
	}
	if matches!(request, FsForkReferenceRequest::FsForkAcquire(_))
		&& target(state, request).is_some_and(|key| {
			state.deleting.contains_key(&key) || state.loaded_volumes[&key].is_deleted()
		}) {
		return Err(VolumeRejectResponse::VolumeDeleted);
	}
	Ok(())
}
/// A parent waiting to be put away stays loaded for an acquire, as it does for an open,
/// unless its unload already started forgetting replicas. The unload waits for ordinary
/// tiering, so turning the acquire away would hold it off for up to the tiering age.
fn keep_loaded(state: &mut MaintainerState, request: &FsForkReferenceRequest, fx: &mut Effects) {
	if matches!(request, FsForkReferenceRequest::FsForkAcquire(_))
		&& target(state, request).is_some_and(|key| state.unloading.contains_key(&key))
		&& !super::lifecycle::unload_blocks_open(state, request.volume_key())
	{
		let volume_key = request.volume_key().clone();
		super::unload::cancel_key(state, &volume_key, fx);
	}
}

fn reply(state: &mut MaintainerState, id: RequestId, response: ClientResponse, fx: &mut Effects) {
	state.pending_fork_references.remove(&id);
	fx.cancel_timer(MaintainerTimer::ForkReferenceRequest(id));
	fx.execute(MaintainerEffect::Reply {
		request_id: id,
		response,
	});
}
pub(crate) fn admit(
	state: &mut MaintainerState,
	id: RequestId,
	request: FsForkReferenceRequest,
	deadline: u64,
	fx: &mut Effects,
) {
	if state.pending_fork_references.contains_key(&id) {
		return;
	}
	if request.region() != state.config.local_region
		|| !depot_fs_core::forks::valid_spec(request.spec(), request.spec().destination_region)
	{
		reply(
			state,
			id,
			ClientResponse::FsForkReferenceResponse(FsForkReferenceResponse::FsForkError(
				FsForkError::Invalid,
			)),
			fx,
		);
		return;
	}
	let spec = request.spec();
	let error = if deadline <= fx.now_boot_ms()
		|| state.request_count() >= state.config.max_requests as usize
		|| spec.parent_volume_key.byte_len() > state.config.tuning().max_volume_key_bytes as usize
		|| spec.destination_volume_key.byte_len()
			> state.config.tuning().max_volume_key_bytes as usize
	{
		Some(depot_types::client::overloaded())
	} else {
		check(state, &request).err()
	};
	if let Some(error) = error {
		reply(state, id, ClientResponse::VolumeRejectResponse(error), fx);
		return;
	}
	if let Some(key) = target(state, &request) {
		state.loaded_volumes.get_mut(&key).unwrap().last_activity = state.now_boot_ms;
	}
	let phase = if matches!(request, FsForkReferenceRequest::FsForkAcquire(_)) {
		Phase::CheckingReceipt
	} else {
		Phase::Queued
	};
	state
		.store_queue
		.push_back(StoreWork::ForkReference { request_id: id });
	state
		.pending_fork_references
		.insert(id, Pending { request, phase });
	fx.arm_timer_at(MaintainerTimer::ForkReferenceRequest(id), deadline);
}
pub(crate) fn dispatch(state: &mut MaintainerState, id: RequestId, fx: &mut Effects) {
	let Some(pending) = state.pending_fork_references.get(&id).cloned() else {
		return;
	};
	let barrier = if pending.phase == Phase::CheckingReceipt {
		ReferenceBarrier::Lookup
	} else {
		ReferenceBarrier::Passed
	};
	let request = pending.request;
	if let Err(error) = check(state, &request) {
		reply(state, id, ClientResponse::VolumeRejectResponse(error), fx);
		return;
	}
	keep_loaded(state, &request, fx);
	if barrier == ReferenceBarrier::Passed
		&& let Err(error) = source_ready(state, &request)
	{
		reply(state, id, ClientResponse::VolumeRejectResponse(error), fx);
		return;
	}
	// A logged acquire or release records the parent's applied position as committed, which
	// a held drain's tail is not; see `checkpoints::waits_for_drain`. A receipt lookup logs
	// nothing and goes on.
	if barrier == ReferenceBarrier::Passed
		&& changes_references(&request)
		&& target(state, &request).is_some_and(|key| super::drain::held(state, &key))
	{
		reply(
			state,
			id,
			ClientResponse::VolumeRejectResponse(depot_types::client::overloaded()),
			fx,
		);
		return;
	}
	let key = target(state, &request);
	let volume = key
		.as_ref()
		.and_then(|key| state.filesystem.volumes.get(key));
	let record = volume.map(|v| v.record.to_record());
	let applied = volume.map(|v| v.applied.clone());
	if barrier == ReferenceBarrier::Passed
		&& matches!(request, FsForkReferenceRequest::FsForkAcquire(_))
		&& let Some(minimum) = &request.spec().minimum
		&& applied
			.as_ref()
			.is_some_and(|p| p.volume_seq < minimum.position.volume_seq)
		&& let Some(key) = key.clone()
		&& key.0.volume_incarnation == minimum.incarnation
	{
		super::filesystem::committed(state, key, minimum.position.volume_seq, fx);
	}
	let volumes = fork_volumes(state, request.spec());
	let releases = matches!(request, FsForkReferenceRequest::FsForkRelease(_));
	// An acquire or release changes the parent's rows with no entry of the parent's own log
	// behind it, so its durable record is a maintainer log op. The operation keeps the
	// parent's footprint while the op commits, which holds the parent's apply at `applied`:
	// the position a `Latest` acquire pins, as an implicit checkpoint.
	if barrier == ReferenceBarrier::Passed && changes_references(&request) {
		// An acquire above the parent's applied position waits for apply without logging:
		// a logged op would hold the parent's apply below the minimum it waits for.
		let lagging = matches!(request, FsForkReferenceRequest::FsForkAcquire(_))
			&& request.spec().minimum.as_ref().is_some_and(|minimum| {
				key.as_ref()
					.is_some_and(|key| key.0.volume_incarnation == minimum.incarnation)
					&& applied
						.as_ref()
						.is_some_and(|applied| applied.volume_seq < minimum.position.volume_seq)
			});
		if lagging {
			reply(
				state,
				id,
				ClientResponse::FsForkReferenceResponse(FsForkReferenceResponse::FsForkError(
					FsForkError::Lagging,
				)),
				fx,
			);
			return;
		}
		if !super::maintainer_log::admits(state) {
			reply(
				state,
				id,
				ClientResponse::VolumeRejectResponse(depot_types::client::overloaded()),
				fx,
			);
			return;
		}
		let target = key.clone().zip(applied.clone());
		let created_at = fx.clocks().unix_ms;
		let op = rivet_data::generated::depot_maintainer_log_v1::MaintainerOp::ForkReferenceOp(
			rivet_data::generated::depot_maintainer_log_v1::ForkReferenceOp {
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
			Operation::ForkReference {
				request_id: id,
				volumes,
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
			if let Operation::ForkReference { log, .. } = operation {
				*log = Some((token, created_at));
			}
		});
		return;
	}
	let op_id = state.begin(
		Operation::ForkReference {
			request_id: id,
			volumes,
			releases,
			log: None,
		},
		fx,
	);
	let loaded_parent_incarnations =
		super::loaded_incarnations(state, &request.spec().parent_volume_key);
	fx.execute(MaintainerEffect::ForkReferenceRequest {
		op_id,
		request,
		barrier,
		region: state.config.local_region,
		record,
		applied,
		logged: None,
		loaded_parent_incarnations,
	});
}

/// Whether a reference request changes the parent's references: an acquire or a release.
/// A check only reads.
fn changes_references(request: &FsForkReferenceRequest) -> bool {
	match request {
		FsForkReferenceRequest::FsForkAcquire(_) | FsForkReferenceRequest::FsForkRelease(_) => true,
		FsForkReferenceRequest::FsForkCheck(_) => false,
	}
}

/// The maintainer log op of reference operation `op_id` committed at `token`: apply the
/// change at the position the parent's apply is held at.
pub(crate) fn logged(
	state: &mut MaintainerState,
	op_id: OpId,
	token: super::maintainer_log::LogToken,
	fx: &mut Effects,
) {
	let Some(Operation::ForkReference {
		log: Some((_, created_at)),
		..
	}) = state.operations.get(&op_id).cloned()
	else {
		return;
	};
	let Some(rivet_data::generated::depot_maintainer_log_v1::MaintainerOp::ForkReferenceOp(op)) =
		super::maintainer_log::op(state, token)
	else {
		super::fence(state, fx);
		return;
	};
	let (record, applied) = match &op.target {
		Some(target) => {
			let key = VolumeLogKey(target.volume_log_key.clone());
			(
				state
					.filesystem
					.volumes
					.get(&key)
					.map(|volume| volume.record.to_record()),
				Some(target.after.clone()),
			)
		}
		None => (None, None),
	};
	let loaded_parent_incarnations =
		super::loaded_incarnations(state, &op.request.spec().parent_volume_key);
	fx.execute(MaintainerEffect::ForkReferenceRequest {
		op_id,
		request: op.request,
		barrier: ReferenceBarrier::Passed,
		region: state.config.local_region,
		record,
		applied,
		logged: Some(created_at),
		loaded_parent_incarnations,
	});
}
pub(crate) fn completed(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<ReferenceResult, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(Operation::ForkReference { .. })
	) {
		return;
	}
	let Some(Operation::ForkReference {
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
	let (response, record) = match result {
		ReferenceResult::SourceProgressRequired => {
			let Some(pending) = state.pending_fork_references.get(&request_id).cloned() else {
				return;
			};
			if pending.phase != Phase::CheckingReceipt
				|| !matches!(pending.request, FsForkReferenceRequest::FsForkAcquire(_))
			{
				super::fence(state, fx);
				return;
			}
			keep_loaded(state, &pending.request, fx);
			if let Err(error) = source_ready(state, &pending.request) {
				reply(
					state,
					request_id,
					ClientResponse::VolumeRejectResponse(error),
					fx,
				);
				return;
			}
			let key = target(state, &pending.request);
			if let Some(key) = &key {
				super::filesystem::load(state, key.clone());
			}
			let spec = pending.request.spec();
			let observed = key.filter(|key| {
				matches!(spec.selector, FsForkSelector::FsForkLatest)
					&& spec.minimum.is_none()
					&& !state.loaded_volumes[key].is_deleted()
					&& !state.deleting.contains_key(key)
			});
			let phase = if let Some(key) = observed {
				Phase::Observing {
					barrier: super::poll::refresh_volume(state, key, fx),
				}
			} else {
				state
					.store_queue
					.push_back(StoreWork::ForkReference { request_id });
				Phase::Queued
			};
			state
				.pending_fork_references
				.get_mut(&request_id)
				.expect("pending")
				.phase = phase;
			return;
		}
		ReferenceResult::Complete { response, record } => (*response, record),
	};
	if let Some(record) = &record {
		super::filesystem::record_flushed(state, record, fx);
	}
	let Some(pending) = state.pending_fork_references.get(&request_id) else {
		return;
	};
	// A receipt means the hold and its pending check are written, so the check gets its
	// deadline in this transition rather than waiting for a discovery pass to find it.
	let acquired = if let (
		FsForkReferenceRequest::FsForkAcquire(acquire),
		FsForkReferenceResponse::FsForkReceipt(_),
	) = (&pending.request, &response)
	{
		Some(acquire.spec.clone())
	} else {
		None
	};
	if let Some(spec) = acquired {
		super::fork_cleanup::acquired(state, &spec);
	}
	let Some(pending) = state.pending_fork_references.get(&request_id) else {
		return;
	};
	let response = match check(state, &pending.request) {
		Ok(()) => ClientResponse::FsForkReferenceResponse(response),
		Err(error) => ClientResponse::VolumeRejectResponse(error),
	};
	reply(state, request_id, response, fx);
}
pub(crate) fn expired(state: &mut MaintainerState, id: RequestId, fx: &mut Effects) {
	if !state.pending_fork_references.contains_key(&id) {
		return;
	}
	state
		.store_queue
		.remove_all(&[StoreWork::ForkReference { request_id: id }]);
	reply(
		state,
		id,
		ClientResponse::VolumeRejectResponse(depot_types::client::not_maintainer()),
		fx,
	);
}
pub(crate) fn drive(state: &mut MaintainerState, fx: &mut Effects) {
	let ids = state
		.pending_fork_references
		.keys()
		.copied()
		.collect::<Vec<_>>();
	for id in ids {
		let pending = state.pending_fork_references[&id].clone();
		if let Err(error) = check(state, &pending.request) {
			reply(state, id, ClientResponse::VolumeRejectResponse(error), fx);
			continue;
		}
		if pending.phase != Phase::CheckingReceipt
			&& let Err(error) = source_ready(state, &pending.request)
		{
			reply(state, id, ClientResponse::VolumeRejectResponse(error), fx);
			continue;
		}
		let phase = match pending.phase {
			Phase::Observing { barrier } if super::poll::volume_refreshed(state, &barrier) => {
				let key = target(state, &pending.request);
				match key {
					// A different incarnation became the source while the barrier ran,
					// and none of its members has been read since the request.
					Some(key) if !state.loaded_volumes[&key].is_deleted() && key != barrier.key => {
						Phase::Observing {
							barrier: super::poll::refresh_volume(state, key, fx),
						}
					}
					Some(key) if !state.loaded_volumes[&key].is_deleted() => {
						let Some(volume) = state.filesystem.volumes.get(&key) else {
							continue;
						};
						let through = super::filesystem::latest_committed(
							state,
							&key,
							volume.applied.volume_seq,
						);
						Phase::Applying { key, through }
					}
					_ => {
						state
							.store_queue
							.push_back(StoreWork::ForkReference { request_id: id });
						Phase::Queued
					}
				}
			}
			phase => phase,
		};
		let ready = match &phase {
			Phase::Applying { key, through } => state
				.filesystem
				.volumes
				.get(key)
				.is_none_or(|v| v.applied.volume_seq >= *through),
			_ => false,
		};
		state
			.pending_fork_references
			.get_mut(&id)
			.expect("pending reference")
			.phase = if ready {
			state
				.store_queue
				.push_back(StoreWork::ForkReference { request_id: id });
			Phase::Queued
		} else {
			phase
		};
	}
}



/// The spec's parent and destination volumes that live in this tablet, deduplicated.
pub(crate) fn fork_volumes(
	state: &MaintainerState,
	spec: &rivet_data::generated::shared_depot_fs_v1::FsForkSpec,
) -> Vec<depot_types::VolumeKey> {
	let mut volumes = Vec::new();
	for key in [&spec.parent_volume_key, &spec.destination_volume_key] {
		if super::in_tablet(state, key) && !volumes.contains(key) {
			volumes.push(key.clone());
		}
	}
	volumes
}
