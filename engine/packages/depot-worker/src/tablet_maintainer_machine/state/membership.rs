//! Replica destinations and quorum evidence for one fixed tablet assignment.

use rivet_data::generated::shared_depot_types_v1::{TabletPhase, TabletReplicaSet};
use rivet_topology::worker::WorkerId;
use std::collections::BTreeSet;

/// Every replacement changes one worker; duplicate or disjoint lists cannot define a quorum.
pub(super) fn validate(set: &TabletReplicaSet) -> anyhow::Result<()> {
	depot_types::membership::validate(set)?;
	let destinations = destinations(set);
	anyhow::ensure!(
		set.disks.len() == destinations.len()
			&& set
				.disks
				.iter()
				.map(|disk| disk.worker_id)
				.collect::<BTreeSet<_>>()
				.len() == destinations.len()
			&& destinations
				.iter()
				.all(|worker| set
					.disks
					.iter()
					.any(|disk| disk.worker_id == *worker.as_bytes()
						&& disk.disk_incarnation != [0; 16])),
		"every member and learner requires one admitted disk identity"
	);
	Ok(())
}

/// Discovery and lifecycle requests visit every destination, including the learner.
/// Preserve manifest member order, then append the joining worker once.
pub(super) fn destinations(set: &TabletReplicaSet) -> Vec<WorkerId> {
	let next = match &set.phase {
		TabletPhase::Stable => &[][..],
		TabletPhase::Learning(p) => p.next.as_slice(),
		TabletPhase::Joint(p) => p.next.as_slice(),
	};
	set.members
		.iter()
		.chain(next.iter().filter(|w| !set.members.contains(w)))
		.copied()
		.map(WorkerId::from_bytes)
		.collect()
}

/// The workers that form the replica set once the current phase completes: `next` while
/// reconfiguring, otherwise the members.
pub(super) fn incoming(set: &TabletReplicaSet) -> Vec<WorkerId> {
	let workers = match &set.phase {
		TabletPhase::Stable => set.members.as_slice(),
		TabletPhase::Learning(p) => p.next.as_slice(),
		TabletPhase::Joint(p) => p.next.as_slice(),
	};
	workers.iter().copied().map(WorkerId::from_bytes).collect()
}

/// A Learning destination is filled by catch-up, not classified as a damaged voter.
pub(super) fn voter(set: &TabletReplicaSet, worker: &WorkerId) -> bool {
	set.members.contains(worker.as_bytes())
		|| matches!(&set.phase, TabletPhase::Joint(p) if p.next.contains(worker.as_bytes()))
}

/// Learning counts only old members; Joint needs a majority of both lists.
/// Use correlated member identities, never the number of replies from the union.
pub(super) fn quorum(set: &TabletReplicaSet, accepts: impl Fn(&WorkerId) -> bool) -> bool {
	let majority = |members: &[[u8; 16]]| {
		members
			.iter()
			.filter(|w| accepts(&WorkerId::from_bytes(**w)))
			.count() > members.len() / 2
	};
	majority(&set.members)
		&& match &set.phase {
			TabletPhase::Stable | TabletPhase::Learning(_) => true,
			TabletPhase::Joint(p) => majority(&p.next),
		}
}
