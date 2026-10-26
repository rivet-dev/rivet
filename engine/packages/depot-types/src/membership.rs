//! Replica configuration rules shared by the coordinator, maintainer and volume client.

use rivet_data::generated::shared_depot_types_v1::{TabletPhase, TabletReplicaSet};
use std::collections::BTreeSet;

/// Small clusters use every worker, up to the ordinary three-replica target.
pub fn valid_members(members: &[[u8; 16]]) -> bool {
	(1..=3).contains(&members.len())
		&& members.iter().collect::<BTreeSet<_>>().len() == members.len()
}

/// One transition may add, remove, or replace one member. The retained member
/// keeps the maintainer in both configurations; quorum sizes never follow liveness.
pub fn validate(set: &TabletReplicaSet) -> anyhow::Result<()> {
	anyhow::ensure!(
		valid_members(&set.members),
		"requires one to three distinct replicas"
	);
	let next = match &set.phase {
		TabletPhase::Stable => return Ok(()),
		TabletPhase::Learning(phase) => &phase.next,
		TabletPhase::Joint(phase) => &phase.next,
	};
	anyhow::ensure!(
		valid_members(next),
		"requires one to three distinct next replicas"
	);
	let retained = next.iter().filter(|id| set.members.contains(id)).count();
	let added = next.len() - retained;
	let removed = set.members.len() - retained;
	anyhow::ensure!(
		retained > 0 && added <= 1 && removed <= 1 && added + removed > 0,
		"membership transition must change one member and retain a maintainer"
	);
	Ok(())
}
