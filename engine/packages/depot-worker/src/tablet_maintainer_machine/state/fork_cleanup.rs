//! Bounded abandoned-hold checks and outgoing releases owned by the maintainer.
//!
//! Each check or release whose durable row is known to this maintainer and is not yet
//! settled has a deadline in `due`: a check at the end of its grace period, and a check
//! or release whose attempt ended without a definitive answer after a retry delay. The
//! deadline is armed in the transition that learns of the row (the acquire that wrote
//! it, or the discovery pass that found it), so no later scan is needed to drive it.
use super::{MaintainerPhase, MaintainerState, Operation, store::StoreWork};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::MaintainerEffect,
	types::{MaintainerTimer, OpId},
};
use depot_fs_core::{FsForkPresence, FsForkSpec, forks::cleanup::*};
use rivet_data::generated::depot_client_protocol_v1::*;
use rivet_state_machine::IndexedDeadlineQueue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Due checks and releases visited per timer delivery. Due entries past it stay queued
/// and the timer fires again at once, so one delivery's work stays bounded.
const DUE_PER_TIMER: usize = 256;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Cleanup {
	jobs: BTreeMap<Work, Phase>,
	/// When each known, unsettled check or release is next inspected, in boot time. An
	/// entry is a check time, not authority: the inspection rereads the durable row and
	/// its creation time, and a settled row reports `Progress::Idle`.
	pub(crate) due: IndexedDeadlineQueue<Work>,
	/// The time `MaintainerTimer::ForkCleanupDue` is armed for.
	pub(crate) due_armed_at: Option<u64>,
}

impl Cleanup {
	/// Cleanup jobs holding a slot of the background cold-read window.
	pub(crate) fn job_count(&self) -> usize {
		self.jobs.len()
	}
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Phase {
	/// One bounded local step is waiting for the shared store FIFO.
	Queued(Action),
	/// The local operation must complete before this identity can advance.
	Storing,
	/// Retain the exact body while an independently owned regional attempt runs.
	Peer(FsForkSpec),
}
/// Inspects a check or release found by the discovery pass. One that cannot start now
/// because the cleanup jobs are full keeps a deadline, so the pass never has to find it
/// again.
pub(crate) fn discover(state: &mut MaintainerState, work: Work) -> Result<(), ()> {
	if !state.tablet.hash_range.contains(work.scope.hash) {
		return Err(());
	}
	if state.phase != MaintainerPhase::Serving || state.fork_cleanup.jobs.contains_key(&work) {
		return Ok(());
	}
	// A materialization's release waits for the pass that covers its copied pages; that
	// pass's completion discovers it again.
	if work.kind == Kind::Release && super::materialize::uncovered(state, &work.scope) {
		return Ok(());
	}
	if state.fork_cleanup.jobs.len() >= state.config.max_background_cold_reads as usize {
		let at = state
			.now_boot_ms
			.saturating_add(state.config.tuning().retry_ms);
		schedule(state, work, at);
		return Ok(());
	}
	queue(state, work, Action::Inspect);
	Ok(())
}

/// Keeps the earlier of `work`'s queued inspection time and `at`.
fn schedule(state: &mut MaintainerState, work: Work, at: u64) {
	state.fork_cleanup.due.schedule_earliest(work, at);
}

/// An acquire on this tablet wrote the pending check of `spec`'s fork, so the check is
/// due when its grace period ends.
pub(crate) fn acquired(state: &mut MaintainerState, spec: &FsForkSpec) {
	let work = Work {
		scope: depot_fs_core::keys::slatedb::Scope::new(
			&spec.parent_volume_key,
			uuid::Uuid::from_bytes(spec.operation_id),
		),
		kind: Kind::Check,
	};
	if !state.tablet.hash_range.contains(work.scope.hash) {
		return;
	}
	let at = state
		.now_boot_ms
		.saturating_add(state.config.fork_check_grace_ms);
	schedule(state, work, at);
}

/// Runs the due inspections. A check or release whose job is already running is left to
/// that job, which queues the next deadline itself if it ends unsettled.
pub(crate) fn due(state: &mut MaintainerState, _fx: &mut Effects) {
	state.fork_cleanup.due_armed_at = None;
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	let now = state.now_boot_ms;
	for (_, work) in state.fork_cleanup.due.pop_due(now, DUE_PER_TIMER) {
		if discover(state, work).is_err() {
			// The tablet's range shrank past this row; the child owning it discovers it.
			continue;
		}
	}
}

/// Keeps `MaintainerTimer::ForkCleanupDue` armed at the head of the due queue.
pub(crate) fn arm_due(state: &mut MaintainerState, fx: &mut Effects) {
	let next_at = state.fork_cleanup.due.next_at();
	super::arm_queue_head(
		&mut state.fork_cleanup.due_armed_at,
		next_at,
		&state.phase,
		state.now_boot_ms,
		MaintainerTimer::ForkCleanupDue,
		fx,
	);
}
fn queue(state: &mut MaintainerState, work: Work, action: Action) {
	state
		.fork_cleanup
		.jobs
		.insert(work.clone(), Phase::Queued(action));
	state.store_queue.push_back(StoreWork::ForkCleanup { work });
}
/// Queues every cleanup job waiting for a store slot again, once replay of the maintainer
/// log ended.
/// Offers the queued cleanups of `volume_key`'s incarnations again once its replay ended.
pub(crate) fn requeue_queued_of(state: &mut MaintainerState, volume_key: &depot_types::VolumeKey) {
	let probe = depot_fs_core::keys::slatedb::Scope::new(volume_key, uuid::Uuid::nil());
	let queued = state
		.fork_cleanup
		.jobs
		.iter()
		.filter(|(work, phase)| {
			matches!(phase, Phase::Queued(_))
				&& work.scope.hash == probe.hash
				&& work.scope.name == probe.name
		})
		.map(|(work, _)| work.clone())
		.collect::<Vec<_>>();
	for work in queued {
		let item = StoreWork::ForkCleanup { work };
		if !state.store_queue.contains(&item) {
			state.store_queue.push_back(item);
		}
	}
}

pub(crate) fn requeue_queued(state: &mut MaintainerState) {
	let queued = state
		.fork_cleanup
		.jobs
		.iter()
		.filter(|(_, phase)| matches!(phase, Phase::Queued(_)))
		.map(|(work, _)| work.clone())
		.collect::<Vec<_>>();
	for work in queued {
		let item = StoreWork::ForkCleanup { work };
		if !state.store_queue.contains(&item) {
			state.store_queue.push_back(item);
		}
	}
}

pub(crate) fn dispatch(state: &mut MaintainerState, work: Work, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving {
		state.fork_cleanup.jobs.remove(&work);
		return;
	}
	let Some(Phase::Queued(action)) = state.fork_cleanup.jobs.get(&work) else {
		return;
	};
	// Cleanup decides by the store's rows, which recovered maintainer log ops of its volume
	// may still change. The job stays queued and is offered again when that replay ends.
	if super::maintainer_log::replay::pending_for_scope(state, &work.scope) {
		return;
	}
	let action = action.clone();
	// Only a completed check releases a hold, and the release may schedule the parent's
	// sweep.
	let loaded_parent_incarnations = match &action {
		Action::Checked { spec, .. } => super::loaded_incarnations(state, &spec.parent_volume_key),
		Action::Inspect | Action::Released(_) => Vec::new(),
	};
	state.fork_cleanup.jobs.insert(work.clone(), Phase::Storing);
	let op_id = state.begin(Operation::ForkCleanupStore { work: work.clone() }, fx);
	fx.execute(MaintainerEffect::ForkCleanupRequest {
		op_id,
		work,
		region: state.config.local_region,
		action,
		loaded_parent_incarnations,
	});
}
pub(crate) fn stored(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<Progress, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(Operation::ForkCleanupStore { .. })
	) {
		return;
	}
	let Some(Operation::ForkCleanupStore { work }) = state.finish(id, fx) else {
		unreachable!()
	};
	if state.fork_cleanup.jobs.get(&work) != Some(&Phase::Storing) {
		super::fence(state, fx);
		return;
	}
	let Ok(result) = result else {
		super::fence(state, fx);
		return;
	};
	let (spec, region, request) = match result {
		Progress::Idle => {
			state.fork_cleanup.jobs.remove(&work);
			return;
		}
		Progress::Check { spec, created_at } if work.kind == Kind::Check => {
			// Clock rollback conservatively retains the hold. The deadline only
			// schedules the next inspection, which checks the elapsed age again; it is
			// capped at one grace period so a wall clock that moved back cannot defer it
			// further.
			let elapsed = i128::from(fx.clocks().unix_ms) - i128::from(created_at);
			let grace = i128::from(state.config.fork_check_grace_ms);
			if elapsed < grace {
				state.fork_cleanup.jobs.remove(&work);
				let remaining = u64::try_from((grace - elapsed).min(grace)).unwrap_or(0);
				let at = state.now_boot_ms.saturating_add(remaining);
				schedule(state, work, at);
				return;
			}
			let request = FsForkReferenceRequest::FsForkCheck(FsForkCheck { spec: spec.clone() });
			let region = spec.destination_region;
			(spec, region, request)
		}
		Progress::Release(spec) if work.kind == Kind::Release => {
			let request =
				FsForkReferenceRequest::FsForkRelease(FsForkRelease { spec: spec.clone() });
			let region = spec.parent_region;
			(spec, region, request)
		}
		_ => {
			super::fence(state, fx);
			return;
		}
	};
	if !work.matches(&spec, state.config.local_region) {
		super::fence(state, fx);
		return;
	}
	state
		.fork_cleanup
		.jobs
		.insert(work.clone(), Phase::Peer(spec));
	let op_id = state.begin(Operation::ForkCleanupPeer { work }, fx);
	fx.execute(MaintainerEffect::ForkPeerRequest {
		op_id,
		region,
		request: ClientRequest::FsForkReferenceRequest(request),
		deadline_boot_ms: fx
			.now_boot_ms()
			.saturating_add(state.config.tuning().operation_timeout_ms),
	});
}
pub(crate) fn peer(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<ClientResponse, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(Operation::ForkCleanupPeer { .. })
	) {
		return;
	}
	let Some(Operation::ForkCleanupPeer { work }) = state.finish(id, fx) else {
		unreachable!()
	};
	let Some(Phase::Peer(spec)) = state.fork_cleanup.jobs.remove(&work) else {
		super::fence(state, fx);
		return;
	};
	let action = match (work.kind, result) {
		(
			Kind::Check,
			Ok(ClientResponse::FsForkReferenceResponse(FsForkReferenceResponse::FsForkPresence(
				presence,
			))),
		) if presence != FsForkPresence::Preparing => Action::Checked { spec, presence },
		(
			Kind::Release,
			Ok(ClientResponse::FsForkReferenceResponse(FsForkReferenceResponse::FsForkReleased)),
		) => Action::Released(spec),
		// Preparing is not absence. A timeout or routing rejection cannot release
		// anything either; the retained row authorizes another attempt, which a
		// check repeats after another grace period and a release after a retry delay.
		(kind, _) => {
			let delay = match kind {
				Kind::Check => state.config.fork_check_grace_ms,
				Kind::Release => state.config.tuning().retry_ms,
			};
			let at = state.now_boot_ms.saturating_add(delay);
			schedule(state, work, at);
			return;
		}
	};
	queue(state, work, action);
}


