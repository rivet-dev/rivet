//! A restore publishes a new incarnation over the one it replaces without writing to cold.
//!
//! Its drain claims the replaced incarnation's replicas, which fences every writer, and applies the
//! final tail. The restore then checks the stored record, quiesces the replaced incarnation, logs
//! its publication as a maintainer log op, and publishes once that op commits. Nothing is uploaded
//! for it: the replaced incarnation's log stays live and untrimmed until an ordinary pass covers
//! the publication, so after a crash its replicas still replay it, and the op's replay publishes
//! again over it. The publication writes the replaced incarnation's own record Deleted, and that
//! incarnation then runs an ordinary delete.
use super::*;
use depot_types::VolumeLogKey;

/// A restored source can keep applying until its fixed image has been copied.
/// After that boundary no new old-incarnation mutation may start.
pub(crate) fn quiescing(forks: &Forks, key: &VolumeKey) -> bool {
	forks.jobs.get(key).is_some_and(|job| {
		job.spec.restore.is_some()
			&& matches!(
				job.phase,
				Phase::Queued(StoreAction::RestoreTarget | StoreAction::Publish { .. })
					| Phase::Storing(LocalStep::RestoreTarget | LocalStep::Publish)
					| Phase::Logging(_)
			)
	})
}

/// Stops the replaced incarnation's apply, unload and cold work before its record is checked.
pub(super) fn quiesce(state: &mut MaintainerState, key: &VolumeKey, fx: &mut Effects) {
	let old = state.forks.jobs[key].spec.restore.expect("restore target");
	let old = VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
		volume_key: key.clone(),
		volume_incarnation: old,
	});
	super::super::deletion::quiesce(state, &old, fx);
	super::super::unload::cancel_key(state, key, fx);
	state.pending_rows.remove(&old);
	state.pending_trims.remove(&old);
}

/// Already dispatched operations finish under their original ownership. The
/// store FIFO separately orders apply/flush/checkpoint work before target lookup.
pub(super) fn quiet(state: &MaintainerState, key: &VolumeKey) -> bool {
	!super::super::repair::holds_key(state, key)
		&& !super::super::catch_up::holds_key(state, key)
		&& !state.pending.contains_key(key)
		&& !replica_lifecycle_in_flight(state, key)
}

/// Whether a replica load or forget of any incarnation of `key` is in flight.
fn replica_lifecycle_in_flight(state: &MaintainerState, key: &VolumeKey) -> bool {
	let index = state.operations.index();
	let incarnations = super::super::incarnations(key);
	let forgets = (
		incarnations.start().clone(),
		rivet_topology::worker::WorkerId::from_bytes([0; 16]),
	)
		..=(
			incarnations.end().clone(),
			rivet_topology::worker::WorkerId::from_bytes([0xff; 16]),
		);
	index.loads.range(incarnations.clone()).next().is_some()
		|| index.forget_orphans.range(incarnations).next().is_some()
		|| index.forgets.range(forgets).next().is_some()
}

/// Queues a restore's target check once the replaced incarnation's replica work drained.
pub(super) fn drive(state: &mut MaintainerState) {
	// It runs after every event; with no fork job it has nothing to do.
	if state.forks.jobs.is_empty() {
		return;
	}
	let waiting: Vec<_> = state
		.forks
		.jobs
		.iter()
		.filter(|(_, job)| matches!(job.phase, Phase::Queued(StoreAction::RestoreTarget)))
		.map(|(key, _)| key.clone())
		.collect();
	for key in waiting {
		if quiet(state, &key)
			&& !state
				.store_queue
				.iter()
				.any(|work| matches!(work, StoreWork::Fork { volume_key } if volume_key == &key))
		{
			state
				.store_queue
				.push_back(StoreWork::Fork { volume_key: key });
		}
	}
}
