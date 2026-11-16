//! Peer reference administration is nested in the existing tablet owner.
//! Latest captures one finite source frontier; exact receipts remain authoritative
//! after deletion or restoration of the logical source.
use super::{
	MaintainerPhase, MaintainerState, Operation, filesystem::LatestBarrier, store::StoreWork,
};
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
use rivet_data::generated::depot_client_protocol_v2::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Pending {
	request: FsForkReferenceRequest,
	phase: Phase,
	/// The caller's deadline, at which `ForkReferenceRequest` answers the request unless an
	/// acquire entering `Proving` moved that timer earlier.
	deadline_boot_ms: u64,
}

impl Pending {
	pub(crate) fn spec(&self) -> &rivet_data::generated::shared_depot_fs_v2::FsForkSpec {
		self.request.spec()
	}

	pub(crate) fn request(&self) -> &FsForkReferenceRequest {
		&self.request
	}
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Phase {
	/// Read the immutable receipt before any mutable lifecycle or replica barrier.
	CheckingReceipt,
	/// A volume barrier over the source volume precedes selection of a loaded Latest source.
	Observing { barrier: super::poll::VolumeBarrier },
	/// The barrier found a member tail above what the members prove committed, so an entry
	/// the source acknowledged before the call may lie above `known_committed`. Entered from
	/// `Observing`; left for `Applying` once the members prove committed through `frontier`,
	/// the highest member tail the barrier saw, or prove a claim newer than `claim_epoch`, the
	/// newest claim the barrier saw, which a recovery commits only above every committed entry.
	/// A live writer proves its own tail as its appends reach a quorum. A dead claimant's tail
	/// is proven by the recovery of the parent's holder-gone or idle unload, which a waiting
	/// acquire does not hold off: acquires leave the parent's activity alone, and
	/// `keep_loaded` spares an unload that still has a tail to recover. A claimant whose
	/// process lives on but no longer writes, such as a sleeping actor's, proves nothing, so
	/// an acquire waits here for half of what was left of its deadline when it entered and is
	/// then refused as `TailUnproven`, early enough for the refusal to reach a caller whose
	/// transport shares that deadline. A caller that knows the claimant is quiet proves the
	/// tail by opening the parent, whose recovery commits a newer claim.
	Proving {
		key: VolumeLogKey,
		frontier: u64,
		claim_epoch: u64,
	},
	/// Later appends cannot extend the selected finite apply barrier.
	Applying { key: VolumeLogKey, through: u64 },
	/// Metadata work is queued or dispatched; the operation separately owns I/O.
	Queued,
	/// A group fork acquire of a parent this tablet does not hold registered asks the store,
	/// with the group's fork children the cache knows, for the pin the parent needs at its
	/// stored cut. Entered from `Queued` at dispatch, once the acquire passed every gate a
	/// logged acquire passes; left for `Pinning` when the store returns a pin to log, for
	/// `QueuedPinned` when a pin already covers the fork or none is needed, or answered when
	/// the store answers the acquire, as busy while no pin can stand yet.
	PlanningPin,
	/// The idle parent's pin is logged at `token` as a `ForkPinOp`, which the replay queue
	/// applies to the store once it commits. Left for `QueuedPinned` once the op is applied,
	/// so no receipt or reply names a pin that is not durable in the log.
	Pinning {
		token: super::maintainer_log::LogToken,
	},
	/// Queued like `Queued`, after planning found the parent pinned through the fork: the
	/// acquire is logged and applied without planning again.
	QueuedPinned,
	/// A group fork acquire whose parent is not pinned through the fork yet: its pin, the
	/// parent's registration or a read of the group is on its way. Entered at dispatch from
	/// `resume`, the phase it was dispatched in, or with `resume` `Queued` when an idle
	/// parent's planned pin found another op of the parent's name in the replay queue, such
	/// as a concurrent acquire's pin; the request is listed in
	/// `pin_waiting_acquires`. Left for `resume`, queued for dispatch again, when `cause`
	/// allows it, or at the latest `retry_ms` after it parked; answered busy at its deadline, or
	/// `TailUnproven` at a recheck that finds its parent's pin waiting for an unproven tail.
	/// The caller's fork job waits for this reply instead of sleeping a retry delay.
	WaitingPin { resume: Box<Phase>, cause: PinWait },
}

/// What a group fork acquire in `Phase::WaitingPin` waits for, which decides what wakes it
/// besides its recheck timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum PinWait {
	/// The parent's pin, its registration, a read of the group or the tablet's row mark: any
	/// pin of the parent applying, the parent registering, a group read completing or the row
	/// mark landing wakes it.
	Progress,
	/// An idle parent's planned pin found another op of the parent's name in the replay queue.
	/// Only a pin of the parent applying or its recheck wakes it: anything else would send it
	/// to the store to plan the same pin and park it again.
	ParentOp,
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
	if let FsForkReferenceRequest::FsForkAcquire(acquire) = request
		&& let Some(key) = target(state, request)
	{
		match &acquire.spec.selector {
			// A group fork acquire of a parent being deleted waits for the delete, which logs
			// the parent's fork pin before it drains; once it is done the store answers from the
			// deleted incarnation's pin. It never pins at the same cut a second time.
			FsForkSelector::FsForkGroup(_) => {
				if state.deleting.contains_key(&key) {
					return Err(depot_types::client::overloaded());
				}
			}
			FsForkSelector::FsForkLatest
			| FsForkSelector::FsForkNamed(_)
			| FsForkSelector::FsForkAtTime(_) => {
				if state.deleting.contains_key(&key) || state.loaded_volumes[&key].is_deleted() {
					return Err(VolumeRejectResponse::VolumeDeleted);
				}
			}
		}
	}
	Ok(())
}
/// A parent waiting to be put away stays loaded for an acquire, as it does for an open,
/// unless its unload already started forgetting replicas. The unload waits for ordinary
/// tiering, so turning the acquire away would hold it off for up to the tiering age. The
/// cancelled unload's idle check waits a full idle period, as an acquire keeps the parent in
/// use without counting as its holder's activity.
///
/// An unload of a parent whose members hold a tail they do not prove committed is spared:
/// its recovery is what proves that tail, which a `Latest` acquire waits for, so cancelling
/// it would leave the acquire waiting on itself. The acquire is answered busy until the
/// recovery proved the tail, and the next one cancels the unload.
fn keep_loaded(state: &mut MaintainerState, request: &FsForkReferenceRequest, fx: &mut Effects) {
	if !matches!(request, FsForkReferenceRequest::FsForkAcquire(_)) {
		return;
	}
	let Some(key) = target(state, request) else {
		return;
	};
	if !state.unloading.contains_key(&key)
		|| super::lifecycle::unload_blocks_open(state, request.volume_key())
		|| super::filesystem::tail_unproven(state, &key)
	{
		return;
	}
	let volume_key = request.volume_key().clone();
	super::unload::cancel_key(state, &volume_key, fx);
	super::unload::postpone_idle(state, &key);
}

fn reply(state: &mut MaintainerState, id: RequestId, response: ClientResponse, fx: &mut Effects) {
	if let Some(pending) = state.pending_fork_references.remove(&id) {
		if matches!(pending.phase, Phase::WaitingPin { .. }) {
			state
				.pin_waiting_acquires
				.remove(&(pending.request.volume_key().clone(), id));
		}
		super::groups::acquire_ended(state, &pending.request);
	}
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
	if let FsForkReferenceRequest::FsForkAcquire(acquire) = &request
		&& let FsForkSelector::FsForkGroup(group) = &acquire.spec.selector
		&& let Some(key) = target(state, &request)
	{
		super::groups::observe_fork(state, &key, group.fork_number, fx);
	}
	let phase = if matches!(request, FsForkReferenceRequest::FsForkAcquire(_)) {
		Phase::CheckingReceipt
	} else {
		Phase::Queued
	};
	state
		.store_queue
		.push_back(StoreWork::ForkReference { request_id: id });
	super::groups::acquire_admitted(state, &request);
	state.pending_fork_references.insert(
		id,
		Pending {
			request,
			phase,
			deadline_boot_ms: deadline,
		},
	);
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
	// A grouped parent's reference rows carry its group's fork numbers, which only a store at
	// the group fork's row version can hold. Until the tablet's pass raises it, an acquire or
	// release is refused before it runs, and the fork job sends it again.
	if barrier == ReferenceBarrier::Passed
		&& changes_references(&request)
		&& depot_types::keys::partition::forkable_group(
			&request.spec().parent_volume_key.partition_key,
		)
		.is_some()
		&& !super::groups::rows_ready(state, fx)
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
	// The parent has one claim log, so a minimum names claim log 0 alone.
	if barrier == ReferenceBarrier::Passed
		&& matches!(request, FsForkReferenceRequest::FsForkAcquire(_))
		&& let Some(minimum) = &request.spec().minimum
		&& let Some(minimum_position) = minimum.position.root_position()
		&& applied
			.as_ref()
			.is_some_and(|p| p.volume_seq < minimum_position.volume_seq)
		&& let Some(key) = key.clone()
		&& key.0.volume_incarnation == minimum.incarnation
	{
		super::filesystem::committed(state, key, minimum_position.volume_seq, fx);
	}
	// A group fork acquire names a fork number the parent may not have pinned yet: the
	// cache learns the fork now, and the pin is logged before the acquire is, so the store
	// finds it. Until the pin is in the store the acquire waits for it; see
	// `Phase::WaitingPin`.
	if barrier == ReferenceBarrier::Passed
		&& let FsForkReferenceRequest::FsForkAcquire(acquire) = &request
		&& let FsForkSelector::FsForkGroup(group) = &acquire.spec.selector
		&& super::groups::acquire_fork(
			state,
			key.as_ref(),
			&acquire.spec.parent_volume_key,
			group.fork_number,
			fx,
		) == super::groups::AcquireGate::Busy
	{
		wait_for_pin(state, id, PinWait::Progress, fx);
		return;
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
					&& minimum.position.root_position().is_some_and(|minimum| {
						applied
							.as_ref()
							.is_some_and(|applied| applied.volume_seq < minimum.volume_seq)
					})
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
		// A parent this tablet does not hold registered has no apply path to pin it, so its
		// pin is planned by the store, logged and applied before the acquire is logged.
		if pending.phase == Phase::Queued && plans_idle_pin(&request, record.as_ref()) {
			plan_idle_pin(state, id, request, fx);
			return;
		}
		let target = key.clone().zip(applied.clone());
		let created_at = fx.clocks().unix_ms;
		let op = rivet_data::generated::depot_maintainer_log_v2::MaintainerOp::ForkReferenceOp(
			rivet_data::generated::depot_maintainer_log_v2::ForkReferenceOp {
				request: request.clone(),
				created_at_unix_ms: created_at,
				targets: target
					.clone()
					.map(|(key, after)| {
						rivet_data::generated::depot_maintainer_log_v2::MaintainerOpTarget {
							volume_log_key: key.0,
							after,
						}
					})
					.into_iter()
					.collect(),
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
		group_children: None,
	});
}

/// Parks group fork acquire `id` until its parent's pin can stand; see `Phase::WaitingPin`.
/// The request's timer fires `retry_ms` from now, or at its deadline if that is sooner, so an
/// acquire whose progress no notification reports is still dispatched again.
fn wait_for_pin(state: &mut MaintainerState, id: RequestId, cause: PinWait, fx: &mut Effects) {
	let retry_ms = state.config.tuning().retry_ms;
	let Some(pending) = state.pending_fork_references.get_mut(&id) else {
		return;
	};
	let parent = pending.request.volume_key().clone();
	let recheck_at = fx
		.now_boot_ms()
		.saturating_add(retry_ms)
		.min(pending.deadline_boot_ms);
	let resume = std::mem::replace(&mut pending.phase, Phase::Queued);
	pending.phase = Phase::WaitingPin {
		resume: Box::new(resume),
		cause,
	};
	state.pin_waiting_acquires.insert((parent, id));
	crate::metrics::GROUP_ACQUIRE_PIN_WAITS.inc();
	fx.arm_timer_at(MaintainerTimer::ForkReferenceRequest(id), recheck_at);
}

/// Dispatches again the group fork acquires waiting for a pin of `parent`'s volume, or of every
/// parent when `parent` is `None`. Called when a parent's pin applies or the parent registers
/// (`Some`), and when a group read completes or the tablet's row mark lands (`None`); the
/// `None` sweep skips acquires waiting for another op of their parent's name
/// (`PinWait::ParentOp`), which only their parent's pin or their recheck wakes. An acquire
/// still not pinned parks again at dispatch.
pub(crate) fn pins_progressed(
	state: &mut MaintainerState,
	parent: Option<&depot_types::VolumeKey>,
	fx: &mut Effects,
) {
	let woken: Vec<(depot_types::VolumeKey, RequestId)> = match parent {
		Some(parent) => state
			.pin_waiting_acquires
			.range((parent.clone(), RequestId(0))..=(parent.clone(), RequestId(u64::MAX)))
			.cloned()
			.collect(),
		None => state
			.pin_waiting_acquires
			.iter()
			.filter(|(_, id)| {
				!state
					.pending_fork_references
					.get(id)
					.is_some_and(|pending| {
						matches!(
							pending.phase,
							Phase::WaitingPin {
								cause: PinWait::ParentOp,
								..
							}
						)
					})
			})
			.cloned()
			.collect(),
	};
	for (parent, id) in woken {
		state.pin_waiting_acquires.remove(&(parent, id));
		resume_waiting(state, id, fx);
	}
}

/// Returns acquire `id` from `WaitingPin` to the phase it parked in and queues it for
/// dispatch, re-arming its timer at its deadline.
fn resume_waiting(state: &mut MaintainerState, id: RequestId, fx: &mut Effects) {
	let Some(pending) = state.pending_fork_references.get_mut(&id) else {
		return;
	};
	let Phase::WaitingPin { resume, .. } = &pending.phase else {
		return;
	};
	pending.phase = (**resume).clone();
	let deadline = pending.deadline_boot_ms;
	fx.arm_timer_at(MaintainerTimer::ForkReferenceRequest(id), deadline);
	state
		.store_queue
		.push_back(StoreWork::ForkReference { request_id: id });
}

/// Whether `request` is a group fork acquire of a parent the tablet does not hold
/// registered, which no apply path pins: `record` is the registered parent's record.
fn plans_idle_pin(
	request: &FsForkReferenceRequest,
	record: Option<&depot_fs_core::FsVolumeRecord>,
) -> bool {
	let FsForkReferenceRequest::FsForkAcquire(acquire) = request else {
		return false;
	};
	record.is_none() && matches!(acquire.spec.selector, FsForkSelector::FsForkGroup(_))
}

/// Sends an idle parent's group fork acquire to the store as a receipt lookup that carries
/// the group's fork children, which plans the parent's pin; see `Phase::PlanningPin`.
fn plan_idle_pin(
	state: &mut MaintainerState,
	id: RequestId,
	request: FsForkReferenceRequest,
	fx: &mut Effects,
) {
	let group_children = super::groups::known_children(state, &request);
	let volumes = fork_volumes(state, request.spec());
	let op_id = state.begin(
		Operation::ForkReference {
			request_id: id,
			volumes,
			releases: false,
			log: None,
		},
		fx,
	);
	state
		.pending_fork_references
		.get_mut(&id)
		.expect("pending reference")
		.phase = Phase::PlanningPin;
	let loaded_parent_incarnations =
		super::loaded_incarnations(state, &request.spec().parent_volume_key);
	fx.execute(MaintainerEffect::ForkReferenceRequest {
		op_id,
		request,
		barrier: ReferenceBarrier::Lookup,
		region: state.config.local_region,
		record: None,
		applied: None,
		logged: None,
		loaded_parent_incarnations,
		group_children,
	});
}

/// The store planned the pin an idle parent needs before acquire `request_id` is logged:
/// the pin is logged as a `ForkPinOp` and applied through the replay queue, and the acquire
/// waits in `Pinning` until it is applied. While another op of the parent's name waits in the
/// replay queue, such as a concurrent acquire's pin, the acquire waits for that op in
/// `WaitingPin` instead and plans again once a pin of the parent applies (or at its recheck),
/// so two pins of one fork are never logged and a burst of acquires of one idle parent is not
/// answered busy.
fn idle_pin_planned(
	state: &mut MaintainerState,
	request_id: RequestId,
	pin: rivet_data::generated::depot_maintainer_log_v2::ForkPinOp,
	fx: &mut Effects,
) {
	let key = VolumeLogKey(pin.volume_log_key.clone());
	if !super::maintainer_log::admits(state) {
		reply(
			state,
			request_id,
			ClientResponse::VolumeRejectResponse(depot_types::client::overloaded()),
			fx,
		);
		return;
	}
	if super::maintainer_log::replay::name_has_ops(state, &key.0.volume_key) {
		let Some(pending) = state.pending_fork_references.get_mut(&request_id) else {
			return;
		};
		pending.phase = Phase::Queued;
		wait_for_pin(state, request_id, PinWait::ParentOp, fx);
		return;
	}
	let token = super::maintainer_log::replay::submit_live(
		state,
		rivet_data::generated::depot_maintainer_log_v2::MaintainerOp::ForkPinOp(pin),
		key,
	);
	crate::metrics::GROUP_FORK_PINS
		.with_label_values(&["idle"])
		.inc();
	state
		.pending_fork_references
		.get_mut(&request_id)
		.expect("pending reference")
		.phase = Phase::Pinning { token };
}

/// The store needs the children of the forks from `first_fork` to plan the idle parent's pin
/// of acquire `request_id`: the group cache reads them, and the acquire plans again once they
/// are known, at once when the cache already holds them.
fn idle_pin_children_needed(
	state: &mut MaintainerState,
	request_id: RequestId,
	first_fork: u64,
	fx: &mut Effects,
) {
	let Some(pending) = state.pending_fork_references.get_mut(&request_id) else {
		return;
	};
	pending.phase = Phase::Queued;
	let spec = pending.spec().clone();
	let (Some(group), FsForkSelector::FsForkGroup(fork)) = (
		depot_types::keys::partition::forkable_group(&spec.parent_volume_key.partition_key),
		&spec.selector,
	) else {
		super::fence(state, fx);
		return;
	};
	if super::groups::idle_pin_children_needed(state, group, first_fork, fork.fork_number, fx) {
		state
			.store_queue
			.push_back(StoreWork::ForkReference { request_id });
	} else {
		wait_for_pin(state, request_id, PinWait::Progress, fx);
	}
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
	let Some(rivet_data::generated::depot_maintainer_log_v2::MaintainerOp::ForkReferenceOp(op)) =
		super::maintainer_log::op(state, token)
	else {
		super::fence(state, fx);
		return;
	};
	let (record, applied) = match op.targets.first() {
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
		group_children: None,
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
	let planning = state
		.pending_fork_references
		.get(&request_id)
		.map(|pending| pending.phase == Phase::PlanningPin);
	let (response, record) = match result {
		ReferenceResult::NeedsIdlePin(pin) => {
			match planning {
				Some(true) => idle_pin_planned(state, request_id, *pin, fx),
				// The request expired while the store planned; nothing was written.
				None => {}
				Some(false) => super::fence(state, fx),
			}
			return;
		}
		ReferenceResult::NeedsGroupChildren { first_fork } => {
			match planning {
				Some(true) => idle_pin_children_needed(state, request_id, first_fork, fx),
				// The request expired while the store planned; nothing was written.
				None => {}
				Some(false) => super::fence(state, fx),
			}
			return;
		}
		ReferenceResult::SourceProgressRequired if planning == Some(true) => {
			// A pin covers the fork or none is needed: the acquire is logged next.
			state
				.pending_fork_references
				.get_mut(&request_id)
				.expect("pending reference")
				.phase = Phase::QueuedPinned;
			state
				.store_queue
				.push_back(StoreWork::ForkReference { request_id });
			return;
		}
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
/// A reference request's deadline passed before it finished, or an acquire waited out its
/// `Proving` share of it. An acquire still proving its parent's tail is refused as
/// `TailUnproven`, so its fork job and the job's callers can tell a tail only a recovery proves
/// from a maintainer that did not answer; any other request is answered `not_maintainer`, and
/// its caller sends it again.
///
/// Entering `Proving` moves the request's timer to half its remaining deadline. An acquire
/// that left `Proving` for another phase before that timer fired has not reached its own
/// deadline, so the timer is armed again at the full deadline instead of answering early.
///
/// A group acquire parked in `WaitingPin` while its parent's pin waits for an unproven tail
/// (see `groups::pin_waits_for_tail`) is refused as `TailUnproven` at its first recheck,
/// `retry_ms` after it parked, rather than busy at its deadline.
pub(crate) fn expired(state: &mut MaintainerState, id: RequestId, fx: &mut Effects) {
	let Some(pending) = state.pending_fork_references.get(&id) else {
		return;
	};
	let proving = matches!(pending.phase, Phase::Proving { .. });
	// A parked acquire whose parent's pin still waits at its recheck for a tail nothing proved
	// is refused as `TailUnproven`: only the parent's holder writing, a recovery or a proof its
	// caller asks for resolves that, so the caller learns why instead of busy at its deadline.
	if matches!(pending.phase, Phase::WaitingPin { .. })
		&& target(state, &pending.request)
			.is_some_and(|key| super::groups::pin_waits_for_tail(state, &key))
	{
		state
			.store_queue
			.remove_all(&[StoreWork::ForkReference { request_id: id }]);
		reply(
			state,
			id,
			ClientResponse::FsForkReferenceResponse(FsForkReferenceResponse::FsForkError(
				FsForkError::TailUnproven,
			)),
			fx,
		);
		return;
	}
	// A parked acquire's recheck: it is dispatched again, which parks it again while its
	// parent is still not pinned.
	if matches!(pending.phase, Phase::WaitingPin { .. })
		&& fx.now_boot_ms() < pending.deadline_boot_ms
	{
		let parent = pending.request.volume_key().clone();
		state.pin_waiting_acquires.remove(&(parent, id));
		resume_waiting(state, id, fx);
		return;
	}
	if !proving && fx.now_boot_ms() < pending.deadline_boot_ms {
		fx.arm_timer_at(
			MaintainerTimer::ForkReferenceRequest(id),
			pending.deadline_boot_ms,
		);
		return;
	}
	let response = match pending.phase {
		Phase::Proving { .. } => ClientResponse::FsForkReferenceResponse(
			FsForkReferenceResponse::FsForkError(FsForkError::TailUnproven),
		),
		Phase::WaitingPin { .. } => ClientResponse::FsForkReferenceResponse(
			FsForkReferenceResponse::FsForkError(FsForkError::Busy),
		),
		Phase::CheckingReceipt
		| Phase::Observing { .. }
		| Phase::Applying { .. }
		| Phase::Queued
		| Phase::PlanningPin
		| Phase::Pinning { .. }
		| Phase::QueuedPinned => {
			ClientResponse::VolumeRejectResponse(depot_types::client::not_maintainer())
		}
	};
	state
		.store_queue
		.remove_all(&[StoreWork::ForkReference { request_id: id }]);
	reply(state, id, response, fx);
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
						match super::filesystem::latest_barrier(
							state,
							&key,
							volume.applied.volume_seq,
						) {
							LatestBarrier::Unproven {
								frontier,
								claim_epoch,
							} => {
								let now_boot_ms = fx.now_boot_ms();
								let refuse_boot_ms = now_boot_ms
									+ pending.deadline_boot_ms.saturating_sub(now_boot_ms) / 2;
								fx.arm_timer_at(
									MaintainerTimer::ForkReferenceRequest(id),
									refuse_boot_ms,
								);
								Phase::Proving {
									key,
									frontier,
									claim_epoch,
								}
							}
							LatestBarrier::Proven { through } => Phase::Applying { key, through },
						}
					}
					_ => {
						state
							.store_queue
							.push_back(StoreWork::ForkReference { request_id: id });
						Phase::Queued
					}
				}
			}
			Phase::Proving {
				key,
				frontier,
				claim_epoch,
			} => proving(state, id, &pending.request, key, frontier, claim_epoch, fx),
			// The pin op is applied: the acquire is logged next and finds the pin.
			Phase::Pinning { token }
				if !super::maintainer_log::replay::pending_token(state, token) =>
			{
				state
					.store_queue
					.push_back(StoreWork::ForkReference { request_id: id });
				Phase::QueuedPinned
			}
			phase @ (Phase::CheckingReceipt
			| Phase::Observing { .. }
			| Phase::Applying { .. }
			| Phase::Queued
			| Phase::PlanningPin
			| Phase::Pinning { .. }
			| Phase::QueuedPinned
			| Phase::WaitingPin { .. }) => phase,
		};
		let ready = match &phase {
			Phase::Applying { key, through } => state
				.filesystem
				.volumes
				.get(key)
				.is_none_or(|v| v.applied.volume_seq >= *through),
			Phase::CheckingReceipt
			| Phase::Observing { .. }
			| Phase::Proving { .. }
			| Phase::Queued
			| Phase::PlanningPin
			| Phase::Pinning { .. }
			| Phase::QueuedPinned
			| Phase::WaitingPin { .. } => false,
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

/// The next phase of an acquire proving `key`'s tail: `Applying` through what the members
/// prove committed once `tail_proven_through` allows it, else still `Proving`. As in
/// `Observing`, a different live incarnation that became the source starts a new barrier over
/// it, and a source no longer loaded live sends the request to the store, which answers it.
fn proving(
	state: &mut MaintainerState,
	id: RequestId,
	request: &FsForkReferenceRequest,
	key: VolumeLogKey,
	frontier: u64,
	claim_epoch: u64,
	fx: &mut Effects,
) -> Phase {
	match target(state, request) {
		Some(current) if !state.loaded_volumes[&current].is_deleted() && current != key => {
			return Phase::Observing {
				barrier: super::poll::refresh_volume(state, current, fx),
			};
		}
		Some(current) if !state.loaded_volumes[&current].is_deleted() => {}
		_ => {
			state
				.store_queue
				.push_back(StoreWork::ForkReference { request_id: id });
			return Phase::Queued;
		}
	}
	match super::filesystem::tail_proven_through(state, &key, frontier, claim_epoch) {
		Some(through) => Phase::Applying { key, through },
		None => Phase::Proving {
			key,
			frontier,
			claim_epoch,
		},
	}
}

/// The spec's parent and destination volumes that live in this tablet, deduplicated.
pub(crate) fn fork_volumes(
	state: &MaintainerState,
	spec: &rivet_data::generated::shared_depot_fs_v2::FsForkSpec,
) -> Vec<depot_types::VolumeKey> {
	let mut volumes = Vec::new();
	for key in [&spec.parent_volume_key, &spec.destination_volume_key] {
		if super::in_tablet(state, key) && !volumes.contains(key) {
			volumes.push(key.clone());
		}
	}
	volumes
}
