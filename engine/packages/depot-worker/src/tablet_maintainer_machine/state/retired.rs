//! Assigned retired parents forget their maintainer log replicas once every split child
//! imported the log, then wait for clone pins before deleting their store and reporting.
//! Shared snapshots can keep the store pinned without needing the parent's log replicas.

use super::{MaintainerPhase, MaintainerState, Operation, store::StoreWork};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use rivet_data::generated::{
	depot_storage_worker_protocol_v1::{
		self as wire, RejectResponse, StorageWorkerRequest, StorageWorkerResponse,
	},
	depot_tablet_coordinator_protocol_v1::{RetiredTabletDeleted, TabletCoordinatorResponse},
	depot_tablet_manifest_v1::RetiredTablet,
};
use rivet_env::capabilities::slatedb_result::DeleteDatabaseResult;
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Cleanup {
	pub retired: RetiredTablet,
	step: Step,
	/// Holders discovered during sent deletion/report I/O wait for that attempt to finish.
	discovered: BTreeSet<WorkerId>,
	/// A failed forget of the parent's log was logged; later failures are debug until the
	/// log is forgotten.
	forget_warned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Step {
	Delete,
	/// The parent's log replicas are forgotten one member at a time once the manifest
	/// records every child's import. Late discovery retains the physical cleanup milestone
	/// to resume, including a store already deleted or a report already accepted.
	ForgetLog {
		members: BTreeMap<WorkerId, LogReplica>,
		then: AfterForget,
	},
	Report,
	Running,
	WaitingDelete,
	/// A forget failed; the retry alarm makes the log's remaining members eligible again.
	WaitingForget {
		members: BTreeMap<WorkerId, LogReplica>,
		then: AfterForget,
	},
	WaitingReport,
	Complete,
	/// The retired record was withdrawn after store deletion. Finish current member
	/// listings and any revealed log replicas, then drop this local obligation.
	AwaitListings,
}

/// Late discovery resumes the physical cleanup milestone already reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum AfterForget {
	Delete,
	Report,
	Complete,
	AwaitListings,
}

impl AfterForget {
	fn step(self) -> Step {
		match self {
			Self::Delete => Step::Delete,
			Self::Report => Step::Report,
			Self::Complete => Step::Complete,
			Self::AwaitListings => Step::AwaitListings,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum LogReplica {
	/// The forget has not been sent, or failed and waits for the retry alarm.
	Pending,
	/// The member answered that it holds no replica of the parent's log any more.
	Forgotten,
}

/// The identity of a retired record, apart from the child imports the coordinator adds over
/// time.
fn same_parent(a: &RetiredTablet, b: &RetiredTablet) -> bool {
	a.partition_key_hash_start == b.partition_key_hash_start
		&& a.tablet_id == b.tablet_id
		&& a.checkpoint_id == b.checkpoint_id
		&& a.children == b.children
}

/// Whether every split child logged the parent's ops into its own log, so no child reads
/// the parent's log replicas again.
fn log_imported_by_every_child(retired: &RetiredTablet) -> bool {
	retired
		.children
		.iter()
		.all(|child| retired.log_imported_by.contains(child))
}

/// The members that may hold the parent's log: those listed holding it while this tablet
/// opened, and the current replica set.
fn log_members(state: &MaintainerState, retired: &RetiredTablet) -> BTreeMap<WorkerId, LogReplica> {
	let key = super::maintainer_log::log_key(retired.tablet_id, retired.partition_key_hash_start);
	state
		.maintainer_log
		.foreign
		.get(&key)
		.into_iter()
		.flatten()
		.cloned()
		.chain(state.members())
		.map(|worker| (worker, LogReplica::Pending))
		.collect()
}

/// A returning member can reveal a parent's replica after initial cleanup captured its
/// holders. Keep that work even while physical deletion or its report is in flight.
pub(crate) fn discovered(
	state: &mut MaintainerState,
	key: &depot_types::VolumeLogKey,
	worker: WorkerId,
) {
	let Some(cleanup) = state.retired.get_mut(&key.0.volume_incarnation) else {
		return;
	};
	if *key
		!= super::maintainer_log::log_key(
			cleanup.retired.tablet_id,
			cleanup.retired.partition_key_hash_start,
		) {
		return;
	}
	match &mut cleanup.step {
		Step::ForgetLog { members, .. } | Step::WaitingForget { members, .. } => {
			members.entry(worker).or_insert(LogReplica::Pending);
		}
		_ => {
			cleanup.discovered.insert(worker);
		}
	}
}

fn active(state: &MaintainerState) -> bool {
	matches!(
		state.phase,
		MaintainerPhase::Serving | MaintainerPhase::Frozen
	)
}

/// The supervisor derives cleanup from the live range containing each first hash.
pub(crate) fn update(
	state: &mut MaintainerState,
	retired: Vec<RetiredTablet>,
	effects: &mut Effects,
) {
	if matches!(
		state.phase,
		MaintainerPhase::Draining | MaintainerPhase::Stopped | MaintainerPhase::Fenced
	) {
		return;
	}
	let mut next = std::collections::BTreeMap::new();
	for parent in retired {
		if !state
			.tablet
			.hash_range
			.contains(parent.partition_key_hash_start)
			|| parent.tablet_id == state.tablet.id
			|| parent.tablet_id == [0; 16]
			|| parent.checkpoint_id == [0; 16]
			|| next.insert(parent.tablet_id, parent.clone()).is_some()
			|| state
				.retired
				.get(&parent.tablet_id)
				.is_some_and(|old| !same_parent(&old.retired, &parent))
		{
			super::fence(state, effects);
			return;
		}
	}

	// Sent storage I/O keeps its operation until completion, even if no longer assigned.
	// A report may be canceled, since repeating it is idempotent at the coordinator.
	let withdrawn = state
		.retired
		.keys()
		.filter(|id| !next.contains_key(*id))
		.copied()
		.collect::<Vec<_>>();
	for id in withdrawn {
		let reports = state
			.operations
			.iter()
			.filter_map(|(op, value)| {
				matches!(value, Operation::ReportRetired { tablet_id } if *tablet_id == id)
					.then_some(*op)
			})
			.collect::<Vec<_>>();
		let cleanup = state.retired.get_mut(&id).expect("withdrawn parent");
		// A committed report can reach the supervisor before its RPC reply. Once
		// physical deletion finished, stop reporting but retain owed log cleanup and
		// unfinished listings under this maintainer's unchanged grant.
		let keep_logs = match &mut cleanup.step {
			Step::ForgetLog { then, .. } | Step::WaitingForget { then, .. }
				if *then != AfterForget::Delete =>
			{
				*then = AfterForget::AwaitListings;
				true
			}
			Step::Running if !reports.is_empty() => {
				cleanup.step = Step::AwaitListings;
				true
			}
			Step::Report | Step::WaitingReport | Step::Complete | Step::AwaitListings => {
				cleanup.step = Step::AwaitListings;
				true
			}
			_ => false,
		};
		// Repeated manifest observations must not cancel a still-owed forget retry.
		if !keep_logs || !matches!(cleanup.step, Step::WaitingForget { .. }) {
			effects.cancel_timer(MaintainerTimer::RetiredRetry(id));
		}
		if !keep_logs {
			state.retired.remove(&id);
		}
		for op_id in reports {
			state.finish(op_id, effects);
			effects.execute(MaintainerEffect::CancelRetiredReportRequest { op_id });
		}
	}
	for (id, retired) in next {
		// The coordinator adds child imports to the record over time.
		if let Some(cleanup) = state.retired.get_mut(&id) {
			cleanup.retired = retired;
			// A temporarily withdrawn assignment can return; its store is already
			// deleted, so resume only the coordinator report after any owed forgets.
			match &mut cleanup.step {
				Step::AwaitListings => cleanup.step = Step::Report,
				Step::ForgetLog { then, .. } | Step::WaitingForget { then, .. }
					if *then == AfterForget::AwaitListings =>
				{
					*then = AfterForget::Report
				}
				_ => {}
			}
			continue;
		}
		let members = log_members(state, &retired);
		state.retired.insert(
			id,
			Cleanup {
				retired,
				step: Step::ForgetLog {
					members,
					then: AfterForget::Delete,
				},
				discovered: BTreeSet::new(),
				forget_warned: false,
			},
		);
	}
}

/// Whether `cleanup` may send the next forget of the parent's log.
fn forget_due(cleanup: &Cleanup) -> bool {
	matches!(cleanup.step, Step::ForgetLog { .. }) && log_imported_by_every_child(&cleanup.retired)
}

fn queue_delete(state: &mut MaintainerState, id: [u8; 16]) {
	let work = StoreWork::DeleteRetired { tablet_id: id };
	if !state.store_queue.contains(&work) {
		state.store_queue.push_back(work);
	}
}

/// One cleanup operation at a time bounds storage and coordinator work independently of manifest size.
pub(crate) fn drive(state: &mut MaintainerState, effects: &mut Effects) {
	if !active(state) || state.operations.index().counts.retired > 0 {
		return;
	}
	let id = loop {
		let Some((&id, cleanup)) = state.retired.iter().find(|(_, cleanup)| {
			matches!(cleanup.step, Step::Delete)
			// A membership change must finish discovering holders before the report can
			// remove the durable cleanup assignment. Excluded members need not answer.
			|| (matches!(cleanup.step, Step::Report | Step::AwaitListings)
				&& state.members().iter().all(|worker| state.listed.contains(worker)))
			|| forget_due(cleanup)
			|| (!cleanup.discovered.is_empty() && cleanup.step != Step::Running)
		}) else {
			return;
		};
		if cleanup.step == Step::AwaitListings && cleanup.discovered.is_empty() {
			state.retired.remove(&id);
			effects.cancel_timer(MaintainerTimer::RetiredRetry(id));
			continue;
		}
		break id;
	};
	let cleanup = state.retired.get(&id).expect("selected parent");
	if !cleanup.discovered.is_empty() {
		let cleanup = state.retired.get_mut(&id).expect("selected parent");
		let then = match cleanup.step {
			Step::Delete | Step::WaitingDelete => AfterForget::Delete,
			Step::Report | Step::WaitingReport => AfterForget::Report,
			Step::Complete => AfterForget::Complete,
			Step::AwaitListings => AfterForget::AwaitListings,
			Step::ForgetLog { .. } | Step::WaitingForget { .. } | Step::Running => {
				unreachable!("discovery joins an active forget directly")
			}
		};
		cleanup.step = Step::ForgetLog {
			members: std::mem::take(&mut cleanup.discovered)
				.into_iter()
				.map(|worker| (worker, LogReplica::Pending))
				.collect(),
			then,
		};
		effects.cancel_timer(MaintainerTimer::RetiredRetry(id));
	}
	let cleanup = state.retired.get(&id).expect("selected parent");
	if let Step::ForgetLog { members, then } = &cleanup.step {
		let Some(worker) = members
			.iter()
			.find(|(_, member)| **member == LogReplica::Pending)
			.map(|(worker, _)| worker.clone())
		else {
			let next = then.step();
			let delete = next == Step::Delete;
			state.retired.get_mut(&id).expect("selected parent").step = next;
			if delete {
				queue_delete(state, id);
			}
			return;
		};
		let key = super::maintainer_log::log_key(
			cleanup.retired.tablet_id,
			cleanup.retired.partition_key_hash_start,
		);
		let request = StorageWorkerRequest::ForgetVolumeRequest(wire::ForgetVolumeRequest {
			observation: None,
			header: wire::ReplicaRequestHeader::MaintainerRequestHeader(
				wire::MaintainerRequestHeader {
					volume_log_key: key.0,
					tablet_grant: state.grant(),
				},
			),
			expected_appended: None,
			expected_claim: None,
		});
		tracing::debug!(
			tablet_id = ?uuid::Uuid::from_bytes(id),
			?worker,
			"forgetting a retired parent's maintainer log replica"
		);
		let op_id = state.begin(
			Operation::ForgetRetiredLog {
				tablet_id: id,
				worker: worker.clone(),
			},
			effects,
		);
		effects.execute(MaintainerEffect::ReplicaRequest {
			op_id,
			worker_id: worker,
			request,
		});
		return;
	}
	if cleanup.step == Step::Report {
		state.retired.get_mut(&id).expect("selected parent").step = Step::Running;
		let op_id = state.begin(Operation::ReportRetired { tablet_id: id }, effects);
		effects.execute(MaintainerEffect::ReportRetiredRequest {
			op_id,
			request: RetiredTabletDeleted {
				tablet_id: id,
				maintainer: state.tablet.maintainer.expect("assigned maintainer"),
				tablet_grant_epoch: state.tablet.tablet_grant_epoch,
			},
		});
	} else {
		queue_delete(state, id);
	}
}

/// The exact checkpoint remains fixed across deletion failures and maintainer moves.
pub(crate) fn dispatch(state: &mut MaintainerState, id: [u8; 16], effects: &mut Effects) {
	// Assignments may enqueue another parent while earlier cleanup waits for the store.
	// Recheck the shared deletion/report window when queued work actually starts.
	if !active(state) || state.operations.index().counts.retired > 0 {
		return;
	}
	let Some(cleanup) = state
		.retired
		.get_mut(&id)
		.filter(|c| c.step == Step::Delete)
	else {
		return;
	};
	cleanup.step = Step::Running;
	let retired = cleanup.retired.clone();
	let op_id = state.begin(Operation::DeleteRetired { tablet_id: id }, effects);
	effects.execute(MaintainerEffect::DeleteRetiredRequest { op_id, retired });
}

/// Pinned parents yield the store until the next interval; only completed deletion permits reporting.
pub(crate) fn deleted(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<DeleteDatabaseResult, String>,
	effects: &mut Effects,
) {
	if !active(state)
		|| !matches!(
			state.operations.get(&op_id),
			Some(Operation::DeleteRetired { .. })
		) {
		return;
	}
	let Some(Operation::DeleteRetired { tablet_id }) = state.finish(op_id, effects) else {
		unreachable!()
	};
	let Some(cleanup) = state
		.retired
		.get_mut(&tablet_id)
		.filter(|c| c.step == Step::Running)
	else {
		return;
	};
	if result == Ok(DeleteDatabaseResult::Deleted) {
		cleanup.step = Step::Report;
	} else {
		cleanup.step = Step::WaitingDelete;
		effects.arm_timer_after(
			MaintainerTimer::RetiredRetry(tablet_id),
			state.config.write_to_cold_after_ms,
		);
	}
}

/// One member answered a forget of the parent's log, or the request timed out. A member
/// that holds no replica is done; any other answer waits for the retry alarm, which re-sends
/// the same forget. Every member done resumes the retained physical cleanup milestone.
pub(crate) fn forgotten(
	state: &mut MaintainerState,
	tablet_id: [u8; 16],
	worker: WorkerId,
	result: Result<StorageWorkerResponse, String>,
	effects: &mut Effects,
) {
	if !active(state) {
		return;
	}
	let Some(cleanup) = state.retired.get_mut(&tablet_id) else {
		return;
	};
	let Step::ForgetLog { members, then } = &mut cleanup.step else {
		return;
	};
	tracing::debug!(
		tablet_id = ?uuid::Uuid::from_bytes(tablet_id),
		?worker,
		?result,
		"a retired parent's maintainer log replica forget answered"
	);
	// A member that held the replica forgets it; one that never held it is done too.
	let held = matches!(
		result,
		Ok(StorageWorkerResponse::ReplicaStateResponse(_))
			| Ok(StorageWorkerResponse::RejectResponse(
				RejectResponse::VolumeLogReplicaDeleted(_)
			))
	);
	let forgotten = held
		|| matches!(
			result,
			Ok(StorageWorkerResponse::RejectResponse(
				RejectResponse::VolumeNotLoaded
			))
		);
	if forgotten {
		if held {
			crate::metrics::RETIRED_LOG_REPLICAS_FORGOTTEN.inc();
		}
		members.insert(worker, LogReplica::Forgotten);
		if members
			.values()
			.all(|member| *member == LogReplica::Forgotten)
		{
			cleanup.step = then.step();
		}
		return;
	}
	if cleanup.forget_warned {
		tracing::debug!(tablet_id = ?uuid::Uuid::from_bytes(tablet_id), ?worker, ?result, "forgetting a retired parent's maintainer log replica failed again");
	} else {
		cleanup.forget_warned = true;
		tracing::warn!(
			tablet_id = ?uuid::Uuid::from_bytes(tablet_id),
			?worker,
			?result,
			"forgetting a retired parent's maintainer log replica failed; retrying after the interval"
		);
	}
	let members = std::mem::take(members);
	cleanup.step = Step::WaitingForget {
		members,
		then: *then,
	};
	effects.arm_timer_after(
		MaintainerTimer::RetiredRetry(tablet_id),
		state.config.tuning().retry_ms,
	);
}

/// A lost acceptance repeats the parent identity; it never reopens a deleted store.
pub(crate) fn reported(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<TabletCoordinatorResponse, String>,
	effects: &mut Effects,
) {
	if !active(state)
		|| !matches!(
			state.operations.get(&op_id),
			Some(Operation::ReportRetired { .. })
		) {
		return;
	}
	let Some(Operation::ReportRetired { tablet_id }) = state.finish(op_id, effects) else {
		unreachable!()
	};
	let Some(cleanup) = state
		.retired
		.get_mut(&tablet_id)
		.filter(|c| c.step == Step::Running)
	else {
		return;
	};
	if matches!(result, Ok(TabletCoordinatorResponse::Accepted(_))) {
		cleanup.step = Step::Complete;
	} else {
		cleanup.step = Step::WaitingReport;
		effects.arm_timer_after(
			MaintainerTimer::RetiredRetry(tablet_id),
			state.config.tuning().retry_ms,
		);
	}
}

pub(crate) fn report_timeout(state: &mut MaintainerState, op_id: OpId, effects: &mut Effects) {
	effects.execute(MaintainerEffect::CancelRetiredReportRequest { op_id });
	reported(
		state,
		op_id,
		Err("retired report timed out".into()),
		effects,
	);
}

/// Obsolete timers cannot resurrect a withdrawn assignment or duplicate an active operation.
pub(crate) fn retry(state: &mut MaintainerState, id: [u8; 16]) {
	if !active(state) {
		return;
	}
	let current = state.members();
	if let Some(cleanup) = state.retired.get_mut(&id) {
		cleanup.step = match std::mem::replace(&mut cleanup.step, Step::Complete) {
			Step::WaitingDelete => Step::Delete,
			// A member that left the replica set forgets excluded logs on its own, or is
			// gone for good; waiting for its answer would hold the cleanup forever.
			Step::WaitingForget { mut members, then } => {
				members.retain(|worker, member| {
					*member == LogReplica::Forgotten || current.contains(worker)
				});
				if members
					.values()
					.all(|member| *member == LogReplica::Forgotten)
				{
					then.step()
				} else {
					Step::ForgetLog { members, then }
				}
			}
			Step::WaitingReport => Step::Report,
			other @ (Step::Delete
			| Step::ForgetLog { .. }
			| Step::Report
			| Step::Running
			| Step::Complete
			| Step::AwaitListings) => other,
		};
	}
}

/// Stop retrying immediately; the ordered executor still joins any sent deletion before close.
pub(crate) fn cancel(state: &mut MaintainerState, effects: &mut Effects) {
	for id in std::mem::take(&mut state.retired).into_keys() {
		effects.cancel_timer(MaintainerTimer::RetiredRetry(id));
	}
}
