//! Voters whose continued availability is necessary when one configured disk stops.

use rivet_data::generated::shared_depot_types_v1::{TabletPhase, TabletReplicaSet};
use std::collections::BTreeSet;

/// A stop without membership reduction must leave all remaining voters available.
/// Joint requires this independently for both sets. A Learning learner does
/// not vote. The overlapping voters also cover any set that does not contain W.
pub fn required_voters(set: &TabletReplicaSet, worker: &[u8; 16]) -> BTreeSet<[u8; 16]> {
	let mut required = BTreeSet::new();
	let mut add = |members: &Vec<[u8; 16]>| {
		if members.contains(worker) {
			required.extend(members.iter().filter(|id| *id != worker).copied());
		}
	};
	add(&set.members);
	if let TabletPhase::Joint(phase) = &set.phase {
		add(&phase.next);
	}
	required
}

/// Whether stopping `worker` leaves a voting set that contains it without a majority of its
/// members. A single-member set is exempt: no stop of its only member keeps it available,
/// and a cluster with one storage worker must still be able to stop.
pub fn loses_majority(set: &TabletReplicaSet, worker: &[u8; 16]) -> bool {
	let loses = |members: &Vec<[u8; 16]>| {
		members.len() >= 2 && members.contains(worker) && (members.len() - 1) * 2 <= members.len()
	};
	if loses(&set.members) {
		return true;
	}
	if let TabletPhase::Joint(phase) = &set.phase {
		return loses(&phase.next);
	}
	false
}
