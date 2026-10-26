//! Replica-set changes retain the grant, open cold store and outstanding mutation barriers.

use super::{MaintainerPhase, MaintainerState, Operation, catch_up, membership, opening, repair};
use crate::tablet_maintainer_machine::{Effects, types::MaintainerTimer};
use rivet_data::generated::{depot_tablet_manifest_v1::Tablet, shared_depot_types_v1::TabletPhase};

/// Membership may change within one grant. Ownership, range and split changes require
/// their own lifecycle and cannot silently authorize an existing store owner.
pub(crate) fn update(state: &mut MaintainerState, tablet: Tablet, effects: &mut Effects) {
	if matches!(
		state.phase,
		MaintainerPhase::Fenced | MaintainerPhase::Stopped | MaintainerPhase::Draining
	) {
		return;
	}
	let current = &state.tablet;
	if tablet.id != current.id
		|| tablet.tablet_grant_epoch != current.tablet_grant_epoch
		|| tablet.maintainer != current.maintainer
		|| tablet.hash_range.start != current.hash_range.start
		|| tablet.hash_range.end != current.hash_range.end
		|| tablet.parent_tablet_id != current.parent_tablet_id
		|| tablet.parent_range_start != current.parent_range_start
		|| tablet.split != current.split
		|| membership::validate(&tablet.replica_set).is_err()
		|| current.replica_set.disks.iter().any(|old| {
			tablet.replica_set.disks.iter().any(|new| {
				old.worker_id == new.worker_id && old.disk_incarnation != new.disk_incarnation
			})
		}) {
		super::fence(state, effects);
		return;
	}
	let version = tablet.replica_set.tablet_replica_set_version;
	let current_version = current.replica_set.tablet_replica_set_version;
	if version < current_version {
		return;
	}
	if version == current_version {
		if tablet.replica_set != current.replica_set {
			super::fence(state, effects);
		}
		return;
	}

	// An operator repair claim's client captured the old voting configuration too, and a
	// learner's catch-up would claim over it; every claim ends.
	super::operator_repair::claims::replica_set_changed(state, effects);

	// A recovery client captured its original voting configuration. Cancel and
	// join it before releasing its key; a queued reopen must not overtake it.
	let recovering = state
		.operations
		.iter()
		.filter_map(|(id, operation)| {
			matches!(operation, Operation::RecoverVolume { .. }).then_some(*id)
		})
		.collect::<Vec<_>>();
	for id in recovering {
		repair::cancel_job(state, id, effects);
	}

	// A Joint cold open's claim and proof belong to its assignment. Retire the
	// caller while retaining every already-sent mutation's completion barrier.
	super::lifecycle::assignment_updated(state, effects);

	// Keep sent loads and their deadlines: cancellation cannot prove they did
	// not create a replica. Their correlated replies release the existing barriers.
	catch_up::cancel_report(state, effects);
	super::stop::cancel(state, effects);
	let previous = state.members();
	let replaced = membership::destinations(&tablet.replica_set) != previous;
	state.tablet = tablet;
	if replaced && state.phase == MaintainerPhase::Serving {
		super::unload::members_changed(state, &previous, effects);
	}
	if state.phase == MaintainerPhase::Serving {
		super::filesystem::mark_all(state);
	}
	effects.execute(
		crate::tablet_maintainer_machine::effect::MaintainerEffect::RefreshCommandReplicaSets,
	);
	if replaced {
		super::pending_work::members_replaced(state, effects);
	}
	// The maintainer log follows the replica set: new destinations get its replica.
	super::maintainer_log::assignment_updated(state);
	// New members have never been observed, so every volume returns to the base interval.
	super::poll::reset_all(state);
	super::promotion::reset(state, effects);
	state.catch_up = catch_up::CatchUpCycle::Idle;
	state.membership_goals.clear();
	state.membership_checks = Default::default();
	state.blocked_copies.clear();
	effects.cancel_timer(MaintainerTimer::CatchUp);
	// Startup still establishes the cold-store fence before raising any shard
	// grant. The open completion will list the newly installed destinations.
	if state
		.operations
		.values()
		.any(|op| matches!(op, Operation::OpenCold))
	{
		return;
	}
	let members = state.members();
	// Listings certify one exact phase. Cancel obsolete read attempts before re-listing;
	// sent loads retain their existing lifecycle/uncertainty barriers.
	let lists: Vec<_> = state
		.operations
		.iter()
		.filter_map(|(id, operation)| matches!(operation, Operation::List { .. }).then_some(*id))
		.collect();
	for id in lists {
		state.finish(id, effects);
		effects.execute(
			crate::tablet_maintainer_machine::effect::MaintainerEffect::CancelReplicaRequest {
				op_id: id,
			},
		);
	}
	state.listed.clear();
	state.listing_cursors.clear();
	state
		.listing_cursors
		.retain(|worker, _| members.contains(worker));
	for worker in members {
		if !state.listed.contains(&worker)
			&& !state.operations.values().any(
				|op| matches!(op, Operation::List { worker: pending, .. } if pending == &worker),
			) {
			opening::list(state, worker, None, effects);
		}
	}
	if matches!(
		state.tablet.replica_set.phase,
		TabletPhase::Learning(_) | TabletPhase::Joint(_)
	) {
		effects.arm_timer_after(MaintainerTimer::CatchUp, state.config.tuning().retry_ms);
	}
	// A replica set that returned to `Stable` with every member listed can prove the
	// minorities that replacement had to retain.
	opening::collect_orphans(state, "stable", opening::OrphanScan::Every, effects);
}


