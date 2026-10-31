//! A grant-scoped, bounded suffix copy. Client matching alone makes a learner vote.

mod progress;
use crate::tablet_maintainer_machine::types::MAX_COLD_BYTES;
use progress::ready_to_report;
pub(crate) use progress::{Checks, Goal, mark, stop_requested, untrack};

use super::{MaintainerPhase, MaintainerState, Operation, cold};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_log_shard_v1::{VolumeLogEntry, VolumeLogReplicaState},
	depot_storage_worker_protocol_v1::{
		self as wire, RejectResponse, StorageWorkerRequest as Request,
		StorageWorkerResponse as Response,
	},
	shared_depot_fs_v1::{FsVolumeRecord, FsVolumeStatus},
	shared_depot_types_v1::{TabletPhase, VolumeLogPosition, VolumeLogReplicaStatus as Status},
};
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::ops::Bound::{Excluded, Unbounded};
use vbare::Encode;

/// Each round observes every loaded volume, then visits keys in order: every loaded volume
/// in the first round under an assignment, and only keys still missing their phase proof in
/// later rounds.
///
/// A round never writes to cold. The learner copies each volume's suffix above the cold
/// position its in-sync members hold, and the log keeps every entry above cold, so no
/// copy needs a fresher cold position. Ordinary tiering and memory pressure keep that
/// suffix bounded and may raise cold while the learner copies; a learner left below the
/// new floor is trimmed up to it before its next read. Lifecycle work a report waits for,
/// such as an unload or a deletion, rides the same tiering passes; a round it keeps from
/// reporting only moves the next throttled pass earlier (see `progress::pass_for_lifecycle`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum CatchUpCycle {
	#[default]
	Idle,
	/// The phase report is in flight or waiting for its retry timer.
	Reporting,
	/// The coordinator accepted the proof; wait for the next manifest.
	Reported,
	/// Waits for the status round of `generation` to observe every loaded volume.
	Preparing {
		generation: u64,
	},
	Copying {
		after: Option<VolumeLogKey>,
	},
	/// The copy visited every key it could, but the tablet was not ready to report. The
	/// round reports on the first event that finds it ready, such as an in-flight open
	/// finishing, and otherwise the CatchUp timer starts the next round.
	Swept,
}

/// The operation registry owns one current copy step; source evidence is never inferred from a tail count.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum CopyStep {
	Load {
		row: FsVolumeRecord,
		request: wire::LoadVolumeRequest,
	},
	/// Retain the exact uncertain mutation and its lifecycle barrier during backoff.
	WaitingToLoad {
		row: FsVolumeRecord,
		request: wire::LoadVolumeRequest,
	},
	Read(SourceRead),
	/// Keep the exact committed payload until the peer definitively accepts or rejects it.
	Append {
		previous: VolumeLogPosition,
		entries: Vec<VolumeLogEntry>,
	},
}

/// Historical progress at a size rejection. Retry only after the offending range can have changed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct BlockedCopy {
	learner: WorkerId,
	cold_volume_seq: u64,
	appended_volume_seq: u64,
}

/// A fixed size error belongs to one volume, not the tablet's serving authority.
fn block(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	target: WorkerId,
	stage: &'static str,
	max_bytes: u32,
) {
	let Some(volume) = state.loaded_volumes.get(&key) else {
		return;
	};
	let cold_volume_seq = volume.row(&key).map_or(0, |row| row.cold.volume_seq);
	let appended_volume_seq = volume
		.members
		.get(&target)
		.map_or(0, |r| r.appended.volume_seq);
	tracing::error!(stage, max_bytes, worker_id = ?target, partition_key_hash = ?depot_types::partition_key_hash(&key.0.volume_key), volume_incarnation = ?key.0.volume_incarnation, "learner catch-up blocked by an incompatible entry limit");
	#[cfg(not(target_arch = "wasm32"))]
	crate::metrics::CATCH_UP_BLOCKED
		.with_label_values(&[stage])
		.inc();
	state.blocked_copies.insert(
		key,
		BlockedCopy {
			learner: target,
			cold_volume_seq,
			appended_volume_seq,
		},
	);
}

/// Immutable source evidence selected before a read; subsequent pages must retain its incarnation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct SourceRead {
	source: WorkerId,
	proof: VolumeLogReplicaState,
	base: VolumeLogPosition,
	/// A previous source page proved the boundary when a full page held only that entry.
	boundary_checked: bool,
}

pub(super) fn learner(state: &MaintainerState) -> Option<WorkerId> {
	let next = match &state.tablet.replica_set.phase {
		TabletPhase::Learning(phase) => &phase.next,
		TabletPhase::Joint(phase) => &phase.next,
		TabletPhase::Stable => return None,
	};
	next.iter()
		.find(|w| !state.tablet.replica_set.members.contains(w))
		.copied()
		.map(WorkerId::from_bytes)
}

/// Each learner has at most one copy step. A load retained for an earlier learner keeps
/// its uncertainty barrier until a definitive reply, so it must not block a new learner.
pub(crate) fn validate_window(state: &MaintainerState) -> anyhow::Result<()> {
	anyhow::ensure!(
		state
			.operations
			.index()
			.catch_up_learners
			.values()
			.all(|ids| ids.len() <= 1),
		"learner copy window exceeded"
	);
	Ok(())
}

/// A failure after discovery requests fresh phase/disk evidence; the learner stays
/// uncovered until a listing answers. One listing or its existing retry owns the refresh.
pub(super) fn refresh_learner(state: &mut MaintainerState, worker: &WorkerId, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving
		|| !matches!(state.tablet.replica_set.phase, TabletPhase::Learning(_))
		|| learner(state).as_ref() != Some(worker)
		|| !state.listed.remove(worker)
	{
		return;
	}
	state.listing_cursors.remove(worker);
	fx.cancel_timer(MaintainerTimer::ListRetry(worker.clone()));
	super::opening::list(state, worker.clone(), None, fx);
}

/// A sent load can recreate a replica. Lifecycle cleanup must not overtake its definitive result.
pub(crate) fn holds_key(state: &MaintainerState, key: &depot_types::VolumeKey) -> bool {
	state
		.operations
		.index()
		.catch_up_holds
		.range(super::incarnations(key))
		.next()
		.is_some()
}

/// Timer-driven rounds share the ordinary status and cold-store budgets.
pub(crate) fn start(state: &mut MaintainerState, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	if state.catch_up == CatchUpCycle::Reporting {
		if state.operations.index().counts.report_catch_up > 0 {
			return;
		}
		state.catch_up = CatchUpCycle::Idle;
	}
	if learner(state).is_none()
		|| !matches!(state.catch_up, CatchUpCycle::Idle | CatchUpCycle::Swept)
	{
		return;
	}
	// Each round starts after a barrier observing every loaded volume, which membership
	// recovery of volumes a slow learner cannot catch up still relies on.
	let generation = super::poll::refresh_all(state, fx);
	state.catch_up = CatchUpCycle::Preparing { generation };
}

/// Lifecycle work owns the key before the copy loop; a copy never resurrects an unloaded volume.
fn lifecycle_eligible(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	state.phase == MaintainerPhase::Serving
		&& state.tablet.replica_set.phase != TabletPhase::Stable
		&& state.loaded_volumes.contains_key(key)
		&& !state.pending.contains_key(&key.0.volume_key)
		&& !state.unloading.contains_key(key)
		&& !super::deletion::holds_key(state, &key.0.volume_key)
		&& !super::forks::holds_key(state, &key.0.volume_key)
		&& !super::repair::holds_key(state, &key.0.volume_key)
		&& !state.operations.index().forget_orphans.contains_key(key)
}

// Only a load to this learner can race installation of its generation. A sent
// load to a failed leaving member keeps its lifecycle barrier without blocking
// catch-up on a different disk.
fn eligible(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	lifecycle_eligible(state, key)
		&& !state
			.operations
			.index()
			.loads
			.get(key)
			.into_iter()
			.flatten()
			.any(|id| {
				matches!(state.operations.get(id), Some(Operation::Load {worker, ..})
				if Some(worker) == learner(state).as_ref())
			})
}



/// Counts one key visited by the copy loop. Test builds only.
fn count_copy_visit() {
	
}

/// One page per volume per round bounds memory and prevents one busy volume starving the others.
pub(crate) fn drive(state: &mut MaintainerState, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	if let CatchUpCycle::Preparing { generation } = state.catch_up
		&& super::poll::refreshed(state, generation)
	{
		state.catch_up = CatchUpCycle::Copying { after: None };
	}
	progress::capture(state);
	if progress::pending(state) && state.catch_up != CatchUpCycle::Reporting {
		fx.arm_timer_after(MaintainerTimer::CatchUp, state.config.tuning().retry_ms);
	}
	if matches!(
		state.catch_up,
		CatchUpCycle::Copying { .. } | CatchUpCycle::Swept
	) && ready_to_report(state)
	{
		finish_round(state, fx);
		return;
	}
	// The learner check comes first: it is cheap, and a stable tablet has no copy work.
	let copying = state.operations.index().counts.catch_up > 0;
	let Some(target) = learner(state) else {
		if state.tablet.replica_set.phase != TabletPhase::Stable
			&& !matches!(
				state.catch_up,
				CatchUpCycle::Reporting | CatchUpCycle::Reported
			) {
			if ready_to_report(state) {
				finish_round(state, fx);
			} else {
				progress::match_members(state, fx);
			}
		}
		if !copying {
			state.blocked_copies.clear();
		}
		return;
	};
	// Only a copy step to this learner serializes the copy loop. A retained load to an
	// earlier learner keeps its own key barrier but must not block this learner.
	let copying_to_target = state
		.operations
		.index()
		.catch_up_learners
		.contains_key(&target);
	if copying_to_target {
		// A failed extra learner may retain a sent load indefinitely. Its
		// uncertainty barrier does not prevent matching a retained voter.
		progress::match_members(state, fx);
		return;
	}
	// A block owns no I/O or payload. Drop it with the volume, a different
	// learner, or progress that can remove the oversized range from the copy.
	state.blocked_copies.retain(|key, blocked| {
		blocked.learner == target
			&& state.loaded_volumes.get(key).is_some_and(|volume| {
				volume
					.row(key)
					.is_none_or(|row| row.cold.volume_seq <= blocked.cold_volume_seq)
					&& volume
						.members
						.get(&target)
						.is_none_or(|r| r.appended.volume_seq <= blocked.appended_volume_seq)
			})
	});
	// Skipped keys share one budget per drive, so one event never walks every loaded
	// volume. An exhausted budget yields through an immediate timer and resumes at the
	// cursor.
	let mut visits = 0;
	loop {
		let CatchUpCycle::Copying { after } = &state.catch_up else {
			return;
		};
		// Reuse the cold read reservation, including encoded entries while Append is in flight.
		if cold::bytes(state).saturating_add(cold::read_budget(state)) > MAX_COLD_BYTES {
			return;
		}
		if visits == progress::CHECK_BUDGET {
			fx.arm_timer_after(MaintainerTimer::CatchUp, 0);
			return;
		}
		let Some(key) = progress::next_copy_key(state, after.as_ref()) else {
			progress::mark_copy_swept(state);
			finish_round(state, fx);
			return;
		};
		visits += 1;
		count_copy_visit();
		// Lifecycle work can retain a trim indefinitely. Skip its key instead of
		// blocking unrelated volumes behind a floor this round cannot use.
		if !eligible(state, &key) || state.blocked_copies.contains_key(&key) {
			state.catch_up = CatchUpCycle::Copying { after: Some(key) };
			continue;
		}

		// Wait for an InSync member to publish the cold floor of the last completed pass.
		// Keep the cursor before this key so the trim reply resumes it without a new round.
		if state.pending_trims.contains_key(&key) {
			return;
		}
		state.catch_up = CatchUpCycle::Copying {
			after: Some(key.clone()),
		};
		let volume = &state.loaded_volumes[&key];
		if !super::membership::quorum(&state.tablet.replica_set, |w| {
			volume
				.members
				.get(w)
				.is_some_and(|r| r.status == Status::InSync)
		}) {
			continue;
		}
		let Some(row) = volume
			.row(&key)
			.filter(|row| row.status == FsVolumeStatus::Live)
		else {
			continue;
		};
		let learning_version = match state.tablet.replica_set.phase {
			TabletPhase::Learning(_) => Some(state.tablet.replica_set.tablet_replica_set_version),
			_ => None,
		};
		if !volume.members.contains_key(&target)
			|| learning_version.is_some_and(|version| {
				volume
					.members
					.get(&target)
					.is_some_and(|replica| replica.learner_replica_set_version < version)
			}) {
			// Joint never starts a new learner generation; missing replicas need client repair.
			let Some(learning_version) = learning_version else {
				continue;
			};
			// Copy the fence and cold position from one observation. The learner's
			// acknowledgements cannot replace the members that fence an older client.
			let Some(source) = volume
				.members
				.values()
				.filter(|replica| replica.status == Status::InSync)
				.max_by_key(|replica| replica.cold.volume_seq)
			else {
				continue;
			};
			let mut row = row;
			row.cold = source.cold.clone();
			let request = wire::LoadVolumeRequest {
				creation: volume.creation_evidence(state.config.write_versions.maintainer_log),
				open_claim: None,
				learner_replica_set_version: Some(learning_version),
				disk_incarnation: state.disk_incarnation(&target).expect("current learner"),
				claimed_client: source.claimed_client.clone(),
				claimed_holder: None,
				header: header(state, &key),
				cold: row.cold.clone(),
				status: Status::Repairing,
				initial_entries: Vec::new(),
			};
			send(
				state,
				key,
				target,
				CopyStep::Load {
					row,
					request: request.clone(),
				},
				Request::LoadVolumeRequest(request),
				fx,
			);
			return;
		}
		if read_next(state, key, target.clone(), fx) {
			return;
		}
	}
}

/// Retry a whole bounded round until the current phase's committed prefix is covered.
///
/// A round that is not ready waits as `Swept` until its retry timer. Readiness can depend on
/// lifecycle work that is only briefly in flight, such as an open of a volume reopened every
/// few milliseconds, so every event in that wait checks it again instead of only the next
/// round's end.
fn finish_round(state: &mut MaintainerState, fx: &mut Effects) {
	if !ready_to_report(state) {
		progress::match_members(state, fx);
		progress::pass_for_lifecycle(state, fx);
		state.catch_up = CatchUpCycle::Swept;
		fx.arm_timer_after(MaintainerTimer::CatchUp, state.config.tuning().retry_ms);
		return;
	}
	let tablet_replica_set_version = state.tablet.replica_set.tablet_replica_set_version;
	let learner = learner(state);
	tracing::info!(
		tablet_id = %uuid::Uuid::from_bytes(state.tablet.id),
		tablet_replica_set_version,
		learning = matches!(state.tablet.replica_set.phase, TabletPhase::Learning(_)),
		loaded_volumes = state.loaded_volumes.len(),
		learner = ?learner,
		"tablet maintainer reported a membership phase covered"
	);
	state.catch_up = CatchUpCycle::Reporting;
	let op_id = state.begin(
		Operation::ReportCatchUp {
			tablet_replica_set_version,
		},
		fx,
	);
	fx.execute(MaintainerEffect::ReportCatchUpRequest {
		op_id,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::CatchUpReport {
			tablet_id: state.tablet.id,
			tablet_grant_epoch: state.tablet.tablet_grant_epoch,
			tablet_replica_set_version,
		},
	});
}

pub(crate) fn recovered_goal(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	through: VolumeLogPosition,
	fx: &mut Effects,
) {
	progress::recovered(state, key, through, fx);
}

/// A membership recovery of `key` finished, whatever its result. See
/// [`progress::membership_recovery_finished`].
pub(crate) fn membership_recovery_finished(state: &mut MaintainerState, key: &VolumeLogKey) {
	progress::membership_recovery_finished(state, key);
}

/// An accepted report waits for the next manifest; failure repeats observation under the same phase.
pub(crate) fn reported(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<
		rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse,
		String,
	>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&op_id),
		Some(Operation::ReportCatchUp { .. })
	) {
		return;
	}
	let Some(Operation::ReportCatchUp {
		tablet_replica_set_version,
	}) = state.finish(op_id, fx)
	else {
		unreachable!()
	};
	if state.phase != MaintainerPhase::Serving
		|| tablet_replica_set_version != state.tablet.replica_set.tablet_replica_set_version
	{
		return;
	}
	if matches!(result, Ok(rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse::Accepted(_))) {
		state.catch_up = CatchUpCycle::Reported;
	} else {
		// Keep unrelated drives from bypassing backoff. The CatchUp timer releases
		// this state after the correlated report operation has finished.
		state.catch_up = CatchUpCycle::Reporting;
		fx.arm_timer_after(MaintainerTimer::CatchUp, state.config.tuning().retry_ms);
	}
}

/// Cancel local report attempts when the assignment changes; remote fencing rejects stale proofs.
pub(crate) fn cancel_report(state: &mut MaintainerState, fx: &mut Effects) {
	let ids: Vec<_> = state
		.operations
		.iter()
		.filter_map(|(id, op)| matches!(op, Operation::ReportCatchUp { .. }).then_some(*id))
		.collect();
	for op_id in ids {
		state.finish(op_id, fx);
		fx.execute(MaintainerEffect::CancelCatchUpReportRequest { op_id });
	}
}

/// Only a current InSync source may supply committed entries. A lower learner cold floor is trimmed first.
fn read_next(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	target: WorkerId,
	fx: &mut Effects,
) -> bool {
	if !eligible(state, &key) {
		return false;
	}
	let volume = &state.loaded_volumes[&key];
	let Some(row) = volume
		.row(&key)
		.filter(|row| row.status == FsVolumeStatus::Live)
	else {
		return false;
	};
	let Some(replica) = volume.members.get(&target) else {
		return false;
	};
	if replica.status == Status::Deleted {
		return false;
	}
	if replica.cold.volume_seq < row.cold.volume_seq {
		cold::trim(state, key, target, fx);
		return false;
	}
	let base = replica.appended.clone();
	let source = volume
		.members
		.read_source(
			state.config.local_worker.as_ref(),
			|worker, r| {
				*worker != target
					&& r.status == Status::InSync
					&& r.cold.volume_seq <= base.volume_seq
					&& super::filesystem::committed_on(state, &key, r) > base.volume_seq
			},
			|candidates| {
				candidates.max_by_key(|(_, r)| {
					(
						super::filesystem::committed_on(state, &key, r),
						r.cold.volume_seq,
					)
				})
			},
		)
		.map(|(worker, r)| (worker.clone(), r.clone()));
	let Some((source, proof)) = source else {
		return false;
	};
	let boundary_checked = base == proof.cold;
	// An unequal epoch at the cold boundary is divergence, never permission to splice histories.
	if base.volume_seq == proof.cold.volume_seq && !boundary_checked {
		return false;
	}
	read_page(
		state,
		key,
		target,
		SourceRead {
			source,
			proof,
			base,
			boundary_checked,
		},
		fx,
	);
	true
}

/// Request the boundary once, then continue after it if the page had no room for new entries.
fn read_page(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	target: WorkerId,
	read: SourceRead,
	fx: &mut Effects,
) {
	let first = if read.boundary_checked {
		read.base.volume_seq + 1
	} else {
		read.base.volume_seq
	};
	let source = read.source.clone();
	let request = Request::ReadVolumeLogEntriesRequest(wire::ReadVolumeLogEntriesRequest {
		include_deleted: false,
		volume_log_key: key.0.clone(),
		first_volume_seq: first,
		max_bytes: state.config.tuning().max_cold_read_bytes,
	});
	let op_id = state.begin(
		Operation::CatchUp {
			key,
			learner: target,
			step: CopyStep::Read(read),
		},
		fx,
	);
	fx.execute(MaintainerEffect::ReplicaRequest {
		op_id,
		worker_id: source,
		request,
	});
}

fn header(state: &MaintainerState, key: &VolumeLogKey) -> wire::ReplicaRequestHeader {
	wire::ReplicaRequestHeader::MaintainerRequestHeader(wire::MaintainerRequestHeader {
		volume_log_key: key.0.clone(),
		tablet_grant: state.grant(),
	})
}

fn send(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	target: WorkerId,
	step: CopyStep,
	request: Request,
	fx: &mut Effects,
) {
	let op_id = state.begin(
		Operation::CatchUp {
			key,
			learner: target.clone(),
			step,
		},
		fx,
	);
	fx.execute(MaintainerEffect::ReplicaRequest {
		op_id,
		worker_id: target,
		request,
	});
}

/// Whether an uncertain load to `target` still needs its retry and lifecycle barrier.
///
/// Counting, promotion and lifecycle work visit only the current destinations, so a
/// worker outside them is never counted. A late delivery can create only the stale
/// learner replica that worker could already hold had the load arrived before it left.
/// Its shard rejects or retires it once its manifest view excludes the worker, and its
/// older learner version keeps it from counting. If the worker rejoins, a loaded volume
/// is reloaded at the new version and any other stale replica lags like a member and is
/// trimmed or repaired.
fn load_retained(state: &MaintainerState, target: &WorkerId) -> bool {
	state.members().contains(target)
}

/// A retry retains its original grant, disk identity, cold position and copied client fence.
/// A target that left the replica set is dropped instead, releasing the volume's barrier.
pub(crate) fn retry_load(state: &mut MaintainerState, op_id: OpId, fx: &mut Effects) {
	let Some(Operation::CatchUp {
		key,
		learner,
		step: CopyStep::WaitingToLoad { row, request },
	}) = state.finish(op_id, fx)
	else {
		return;
	};
	if !load_retained(state, &learner) {
		tracing::debug!(worker_id = ?learner, "dropping uncertain catch-up load to a worker that left the replica set");
		return;
	}
	send(
		state,
		key,
		learner,
		CopyStep::Load {
			row,
			request: request.clone(),
		},
		Request::LoadVolumeRequest(request),
		fx,
	);
}

/// Old operation IDs are filtered by the dispatcher. Copy replies are hints, not voting or promotion proofs.
pub(crate) fn completed(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	target: WorkerId,
	step: CopyStep,
	result: Result<Response, String>,
	fx: &mut Effects,
) {
	if !matches!(&step, CopyStep::Read(_))
		&& matches!(
			&result,
			Err(_) | Ok(Response::RejectResponse(RejectResponse::NotServing))
		) {
		refresh_learner(state, &target, fx);
	}

	match step {
		CopyStep::WaitingToLoad { .. } => unreachable!("a waiting load has no response"),
		CopyStep::Load { row, request } => {
			let response = match result {
				Ok(Response::ReplicaStateResponse(response)) => response,
				Err(_) => {
					if !load_retained(state, &target) {
						tracing::debug!(worker_id = ?target, "dropping uncertain catch-up load to a worker that left the replica set");
						return;
					}
					let op_id = state.begin(
						Operation::CatchUp {
							key,
							learner: target,
							step: CopyStep::WaitingToLoad { row, request },
						},
						fx,
					);
					fx.arm_timer_after(
						MaintainerTimer::Operation(op_id),
						state.config.tuning().retry_ms,
					);
					return;
				}
				Ok(response) => {
					observe(state, key, target, Ok(response));
					return;
				}
			};
			cold::observe(state, &key, target.clone(), response.state);
			if learner(state).as_ref() == Some(&target) {
				read_next(state, key, target, fx);
			}
		}
		CopyStep::Read(source) => {
			if learner(state).as_ref() == Some(&target) {
				read(state, key, target, source, result, fx);
			}
		}
		CopyStep::Append {
			previous,
			mut entries,
		} => {
			if !eligible(state, &key) || learner(state).as_ref() != Some(&target) {
				return;
			}
			if let Ok(Response::RejectResponse(RejectResponse::RequestTooLarge(reject))) = &result {
				// A definitive size rejection applied nothing. Retry a smaller exact
				// prefix; the next round fetches the remainder from the same boundary.
				if entries.len() > 1 {
					entries.truncate(entries.len().div_ceil(2));
					append(state, key, target, previous, entries, fx);
				} else {
					block(state, key, target, "append", reject.max_bytes);
				}
			} else {
				observe(state, key, target, result);
			}
		}
	}
}

/// Verify the source incarnation, committed limit and shared boundary before forwarding any bytes.
fn read(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	target: WorkerId,
	read: SourceRead,
	result: Result<Response, String>,
	fx: &mut Effects,
) {
	let SourceRead {
		source,
		proof,
		base,
		boundary_checked,
	} = read;
	if !eligible(state, &key) {
		return;
	}
	let Ok(Response::ReadVolumeLogEntriesResponse(mut page)) = result else {
		if let Ok(Response::RejectResponse(RejectResponse::RequestTooLarge(reject))) = &result {
			// The shard reports this only when even the first entry cannot fit.
			// Repeating the same read would otherwise stall every catch-up round.
			block(state, key, target, "read", reject.max_bytes);
			return;
		}
		observe(state, key, source, result);
		return;
	};
	if page.state.replica_incarnation != proof.replica_incarnation
		|| page.state.status != Status::InSync
	{
		cold::observe(state, &key, source, page.state);
		return;
	}
	let bytes = page
		.entries
		.iter()
		.try_fold(0usize, |sum, entry| sum.checked_add(entry.encoded_len()));
	if bytes.is_none_or(|n| n > state.config.tuning().max_cold_read_bytes as usize) {
		super::fence(state, fx);
		return;
	}
	let committed = super::filesystem::committed_on(state, &key, &proof)
		.min(super::filesystem::committed_on(state, &key, &page.state));
	let source_cold = page.state.cold.volume_seq;
	cold::observe(state, &key, source.clone(), page.state);
	if committed <= base.volume_seq || source_cold > base.volume_seq {
		return;
	}

	// Include the source's boundary entry unless cold already proves its epoch. A
	// numeric sequence match alone cannot establish that two suffixes belong together.
	if !boundary_checked {
		let Some(anchor) = page.entries.first() else {
			return;
		};
		if anchor.volume_seq != base.volume_seq || anchor.client_epoch != base.client_epoch {
			return;
		}
		page.entries.remove(0);
		if page.entries.is_empty() {
			read_page(
				state,
				key,
				target,
				SourceRead {
					source,
					proof,
					base,
					boundary_checked: true,
				},
				fx,
			);
			return;
		}
	}
	let mut previous = base.clone();
	let count = page
		.entries
		.iter()
		.take_while(|entry| entry.volume_seq <= committed)
		.count();
	page.entries.truncate(count);
	for entry in &page.entries {
		if previous.volume_seq.checked_add(1) != Some(entry.volume_seq)
			|| entry.client_epoch < previous.client_epoch
		{
			super::fence(state, fx);
			return;
		}
		previous = VolumeLogPosition {
			volume_seq: entry.volume_seq,
			client_epoch: entry.client_epoch,
		};
	}
	if page.entries.is_empty() {
		return;
	}
	append(state, key, target, base, page.entries, fx);
}

/// Retain one page for definitive size-rejection retries, within the two-copy read reservation.
fn append(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	target: WorkerId,
	previous: VolumeLogPosition,
	entries: Vec<VolumeLogEntry>,
	fx: &mut Effects,
) {
	let committed_volume_seq = entries
		.last()
		.expect("nonempty committed prefix")
		.volume_seq;
	let request = Request::AppendRequest(wire::AppendRequest {
		header: header(state, &key),
		previous: previous.clone(),
		entries: entries.clone(),
		committed_volume_seq,
	});
	send(
		state,
		key,
		target,
		CopyStep::Append { previous, entries },
		request,
		fx,
	);
}

/// A concurrent trim, match or deletion may win. Refresh evidence and retry a later round without truncating.
fn observe(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	worker: WorkerId,
	result: Result<Response, String>,
) {
	let observed = match result {
		Ok(Response::ReplicaStateResponse(response)) => Some(response.state),
		Ok(Response::RejectResponse(reason)) => match reason {
			RejectResponse::Diverged(r) => Some(r.state),
			RejectResponse::VolumeSeqGap(r) => Some(r.state),
			RejectResponse::BelowCold(r) => Some(r.state),
			RejectResponse::VolumeLogReplicaDeleted(r) => Some(r.state),
			RejectResponse::VolumeNotLoaded => {
				if let Some(volume) = state.loaded_volumes.get_mut(&key) {
					volume.members.remove(&worker);
				}
				progress::mark(state, &key);
				None
			}
			RejectResponse::StaleClientEpoch(_)
			| RejectResponse::ClientConflict(_)
			| RejectResponse::NotClaimed(_)
			| RejectResponse::AppendedMismatch(_)
			| RejectResponse::VolumeLogReplicaIncarnationMismatch(_)
			| RejectResponse::StaleTabletReplicaSet(_)
			| RejectResponse::StaleTabletGrantEpoch(_)
			| RejectResponse::Overloaded(_)
			| RejectResponse::NotServing
			| RejectResponse::RequestTooLarge(_)
			| RejectResponse::TooManyLoadedVolumes(_)
			| RejectResponse::DiskIncarnationMismatch(_)
			| RejectResponse::DiskFull(_)
			| RejectResponse::UnsupportedPayloadVersion(_) => None,
		},
		Ok(
			Response::ReadVolumeLogEntriesResponse(_)
			| Response::ListTabletReplicasResponse(_)
			| Response::GetDiskIncarnationResponse(_)
			| Response::ReadVolumeLogStatusResponse(_)
			| Response::ForgetVolumesResponse(_)
			| Response::TrimVolumesResponse(_)
			| Response::StorageWorkerBatchResponse(_),
		)
		| Err(_) => None,
	};
	if let Some(replica) = observed {
		cold::observe(state, &key, worker, replica);
	}
}




