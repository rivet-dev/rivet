//! Volume recoveries that unloads and membership changes start. Each runs a recovery-only
//! volume client, which owns every recovery decision.

use super::{MaintainerPhase, MaintainerState, Operation};
use crate::tablet_maintainer_machine::types::REPAIR_TIMEOUT_MS;
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect as Fx, types::*};
use depot_types::VolumeLogKey;
use depot_volume::volume_client_machine::types::RecoveryReport;
use rivet_data::generated::{
	shared_depot_fs_v1::FsVolumeRecord,
	shared_depot_types_v1::{TabletPhase, VolumeLogReplicaStatus as Status},
};
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};

/// Why a recovery started, which decides what its completion reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum RecoveryPurpose {
	Unload,
	Membership {
		grant_epoch: u64,
		replica_set_version: u64,
	},
}

/// Canceled recovery owners keep their concurrency reservation until they join.
pub(crate) fn active_jobs(s: &MaintainerState) -> usize {
	let counts = s.operations.index().counts;
	counts.recover_volume as usize
}

/// Starts one selected recovery under the shared job budget; the caller must hold an eligible closed key.
pub(crate) fn recover(
	s: &mut MaintainerState,
	key: VolumeLogKey,
	row: FsVolumeRecord,
	purpose: RecoveryPurpose,
	fx: &mut Effects,
) {
	// An unfinished create cannot authorize a recovery client. Preserve its
	// retry and retention evidence until the lifecycle resolves it.
	if s.loaded_volumes.get(&key).is_some_and(|volume| {
		volume
			.creation
			.as_ref()
			.is_some_and(|creation| !creation.acknowledged)
	}) {
		return;
	}

	// A recovery client can commit a claim entry even if its owner later times out.
	// Revoke initial-load permission before dispatch, retaining the newest row
	// known to exist in cold storage.
	if let Some(volume) = s.loaded_volumes.get_mut(&key)
		&& let Some(row) = volume.unacknowledged_cold_open.take()
		&& volume
			.stored_row
			.as_ref()
			.is_none_or(|stored| stored.cold.volume_seq < row.cold.volume_seq)
	{
		volume.stored_row = Some((*row).into());
	}
	// The recovery client claims the volume at an epoch this maintainer issues, above every
	// epoch its members hold. Without an epoch to issue, the recovery waits for the next
	// drive, which asks again.
	let floor = s
		.loaded_volumes
		.get(&key)
		.into_iter()
		.flat_map(|volume| volume.members.values())
		.map(super::maintainer_log::epoch_floor)
		.fold(row.cold.client_epoch, u64::max);
	let Some(claim_epoch) = floor
		.checked_add(1)
		.and_then(|min| super::claim_epochs::issue(s, min))
	else {
		tracing::debug!(?key, "recovery waits for a claim epoch");
		return;
	};
	// The stored row can make an unregistered volume eligible for filesystem apply.
	super::filesystem::offer(s, &key);
	super::deletion::observe_loaded(s, &key);
	let required_members = if matches!(purpose, RecoveryPurpose::Membership { .. }) {
		healthy_learner(s, &key)
			.into_iter()
			.collect::<std::collections::BTreeSet<_>>()
	} else {
		Default::default()
	};
	let op_id = s.begin(
		Operation::RecoverVolume {
			key: key.clone(),
			purpose,
			required_members: required_members.clone(),
			cancelled: false,
		},
		fx,
	);
	fx.arm_timer_after(MaintainerTimer::Operation(op_id), REPAIR_TIMEOUT_MS);
	// A healthy discovered learner must finish matching before Learning can advance.
	// Its conditional promotion may outlast the optional-peer window. The original
	// operation deadline already bounds this required work and is never rearmed here.
	fx.execute(Fx::RecoverVolumeRequest {
		op_id,
		request: RecoveryRequest {
			finish_recovery_timeout_ms: if matches!(purpose, RecoveryPurpose::Membership { .. }) {
				Some(if required_members.is_empty() {
					250
				} else {
					REPAIR_TIMEOUT_MS
				})
			} else {
				None
			},
			required_members,
			volume_log_key: key,
			row,
			replica_set: s.tablet.replica_set.clone(),
			timeout_ms: REPAIR_TIMEOUT_MS,
			claim_epoch,
		},
	});
}

fn healthy_learner(s: &MaintainerState, key: &VolumeLogKey) -> Option<WorkerId> {
	if !matches!(s.tablet.replica_set.phase, TabletPhase::Learning(_)) {
		return None;
	}
	super::catch_up::learner(s).filter(|worker| {
		s.listed.contains(worker)
			&& s.loaded_volumes
				.get(key)
				.and_then(|volume| volume.members.get(worker))
				.is_some_and(|replica| {
					replica.status != Status::Deleted
						&& replica.learner_replica_set_version
							>= s.tablet.replica_set.tablet_replica_set_version
				})
	})
}

/// A pending open keeps exclusivity until the correlated recovery or missing-load completion.
pub(crate) fn holds_key(s: &MaintainerState, key: &depot_types::VolumeKey) -> bool {
	s.operations
		.index()
		.repair_holds
		.range(super::incarnations(key))
		.next()
		.is_some()
}

/// Keep the operation reservation until the executor confirms that the child owner has joined.
pub(crate) fn cancel_job(s: &mut MaintainerState, id: OpId, fx: &mut Effects) {
	if let Some(Operation::RecoverVolume {
		cancelled: false, ..
	}) = s.operations.get(&id)
	{
		s.operations.update(&id, |operation| {
			if let Operation::RecoverVolume { cancelled, .. } = operation {
				*cancelled = true;
			}
		});
		fx.cancel_timer(MaintainerTimer::Operation(id));
		fx.execute(Fx::CancelRecoveryRequest { op_id: id });
	}
}

/// Let a current required learner finish before reopening; cancel other recovery owners and join them.
pub(crate) fn cancel_key(s: &mut MaintainerState, key: &depot_types::VolumeKey, fx: &mut Effects) {
	cancel_holds(s, key, false, fx);
}

/// A reopen that waited `repair_open_wait_ms` cancels even a required learner's recovery:
/// the serving client matches the learner instead, and an actor wake stops waiting.
pub(crate) fn preempt_key(s: &mut MaintainerState, key: &depot_types::VolumeKey, fx: &mut Effects) {
	cancel_holds(s, key, true, fx);
}

fn cancel_holds(
	s: &mut MaintainerState,
	key: &depot_types::VolumeKey,
	preempt_required: bool,
	fx: &mut Effects,
) {
	let ids = s
		.operations
		.index()
		.repair_holds
		.range(super::incarnations(key))
		.flat_map(|(_, ids)| ids.iter().copied())
		.collect::<Vec<_>>();
	for id in ids {
		// A reopen must not at once preempt the bounded recovery that fences its previous
		// holder and matches its captured required learner. It waits for this same owner to
		// join, up to `repair_open_wait_ms`, without extending the operation or caller deadline.
		if !preempt_required
			&& matches!(s.operations.get(&id), Some(Operation::RecoverVolume {
			purpose: RecoveryPurpose::Membership { grant_epoch, replica_set_version },
			required_members,
			cancelled: false,
			..
		}) if *grant_epoch == s.tablet.tablet_grant_epoch
			&& *replica_set_version == s.tablet.replica_set.tablet_replica_set_version
			&& !required_members.is_empty())
		{
			continue;
		}
		// A load is idempotent and bounded by its existing operation deadline. Let it
		// report normally so a reopen cannot overlap the missing-replica decision.
		if matches!(s.operations.get(&id), Some(Operation::RecoverVolume { .. })) {
			cancel_job(s, id, fx);
		}
	}
}

/// Count only the current job, then release its key regardless of success or cancellation.
pub(crate) fn completed(
	s: &mut MaintainerState,
	id: OpId,
	result: Result<RecoveryReport, String>,
	fx: &mut Effects,
) {
	if !matches!(s.operations.get(&id), Some(Operation::RecoverVolume { .. })) {
		return;
	}
	let Some(Operation::RecoverVolume {
		key,
		purpose,
		cancelled,
		..
	}) = s.finish(id, fx)
	else {
		unreachable!()
	};
	if let RecoveryPurpose::Membership {
		grant_epoch,
		replica_set_version,
	} = &purpose
		&& *grant_epoch == s.tablet.tablet_grant_epoch
		&& *replica_set_version == s.tablet.replica_set.tablet_replica_set_version
	{
		if !cancelled {
			if let Some(worker) = super::catch_up::learner(s) {
				let incomplete = match &result {
					Ok(report) => report.incomplete.contains(&worker),
					Err(_) => true,
				};
				if incomplete {
					super::catch_up::refresh_learner(s, &worker, fx);
				}
			}
			if let Ok(report) = &result {
				super::catch_up::recovered_goal(s, &key, report.through.clone(), fx);
			}
		}
		// Cancellation cannot undo a sent claim entry. Once this owner joins, even a
		// timed-out recovery gives the next writer a fresh holder allowance.
		super::catch_up::membership_recovery_finished(s, &key);
	}
	if result.is_ok() {
		super::poll::request(s, key.clone(), fx);
	}
	released(s, key.clone(), fx);
	super::unload::recovery_finished(s, &key, fx);
}

/// Every terminal path hands the key back to its waiting opener or its background unload.
fn released(s: &mut MaintainerState, key: VolumeLogKey, fx: &mut Effects) {
	if s.phase != MaintainerPhase::Serving {
		return;
	}
	super::lifecycle::resume_after_repair(s, &key.0.volume_key, fx);
	if !s.pending.contains_key(&key.0.volume_key)
		&& !holds_key(s, &key.0.volume_key)
		&& s.loaded_volumes
			.get(&key)
			.is_some_and(|v| v.idle(s.now_boot_ms, s.config.idle_unload_after_ms))
	{
		super::unload::begin(s, key, super::unload::UnloadReason::Requested);
	}
}
