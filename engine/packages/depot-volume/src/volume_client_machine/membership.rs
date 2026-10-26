//! Replica membership, voting rules, and retries that keep the same client history.

use super::{
	Effects,
	state::{RecoveryPhase, Replica, VolumeClientPhase, VolumeClientState},
	types::VolumeClientTimer,
};
use rivet_data::generated::{
	depot_log_shard_v1::VolumeLogReplicaState,
	shared_depot_types_v1::{TabletPhase, TabletReplicaSet},
};
use rivet_topology::worker::WorkerId;
use std::collections::{BTreeMap, BTreeSet};

/// Shared membership rules keep all protocol participants on the same voting lists.
pub(super) fn validate(set: &TabletReplicaSet) -> anyhow::Result<()> {
	depot_types::membership::validate(set)
}

/// Sends to both lists during a replacement, counting shared workers only once.
pub(super) fn destinations(set: &TabletReplicaSet) -> BTreeSet<WorkerId> {
	let next = match &set.phase {
		TabletPhase::Stable => &[][..],
		TabletPhase::Learning(p) => p.next.as_slice(),
		TabletPhase::Joint(p) => p.next.as_slice(),
	};
	set.members
		.iter()
		.chain(next)
		.copied()
		.map(WorkerId::from_bytes)
		.collect()
}

/// Learners receive history but cannot contribute recovery or commit evidence.
pub(super) fn voter(set: &TabletReplicaSet, worker: &WorkerId) -> bool {
	set.members.contains(worker.as_bytes())
		|| matches!(&set.phase, TabletPhase::Joint(p) if p.next.contains(worker.as_bytes()))
}

fn learner(set: &TabletReplicaSet, worker: &WorkerId) -> bool {
	matches!(&set.phase, TabletPhase::Learning(p)
		if p.next.contains(worker.as_bytes()) && !set.members.contains(worker.as_bytes()))
}

/// A new Learning generation replaces the replica even when its worker was retained.
pub(super) fn proof_eligible(
	set: &TabletReplicaSet,
	worker: &WorkerId,
	proof: &VolumeLogReplicaState,
) -> bool {
	!learner(set, worker) || proof.learner_replica_set_version >= set.tablet_replica_set_version
}

/// Applies the same phase rule to claiming, confirmation, and close publication.
/// Joint checks each set's majority independently, never a majority of their union.
/// When replacing one of three members, the two shared members alone satisfy both.
pub(super) fn quorum(set: &TabletReplicaSet, accepts: impl Fn(&WorkerId) -> bool) -> bool {
	let majority = |members: &[[u8; 16]]| {
		members
			.iter()
			.filter(|w| accepts(&WorkerId::from_bytes(**w)))
			.count() > members.len() / 2
	};
	majority(&set.members)
		&& match &set.phase {
			TabletPhase::Joint(p) => majority(&p.next),
			TabletPhase::Stable | TabletPhase::Learning(_) => true,
		}
}

/// While the workers `refusing_until` names deny `set` a quorum at `now_boot_ms`, how long
/// until the earliest of their refusals to end frees one. `None` when a quorum is free now,
/// or when the set has no members yet.
pub(super) fn refused_quorum_ms(
	set: &TabletReplicaSet,
	refusing_until: impl Fn(&WorkerId) -> Option<u64>,
	now_boot_ms: u64,
) -> Option<u64> {
	if set.members.is_empty() || quorum(set, |worker| refusing_until(worker).is_none()) {
		return None;
	}
	let mut ends: Vec<u64> = destinations(set)
		.iter()
		.filter_map(|worker| refusing_until(worker))
		.collect();
	ends.sort_unstable();
	let free_at = ends.into_iter().find(|end| {
		quorum(set, |worker| {
			refusing_until(worker).is_none_or(|until| until <= *end)
		})
	})?;
	Some(free_at.saturating_sub(now_boot_ms).max(1))
}

/// The committed prefix is the lower of the two majorities during Joint.
pub(super) fn position(set: &TabletReplicaSet, position: impl Fn(&WorkerId) -> u64) -> u64 {
	let majority = |members: &[[u8; 16]]| {
		let mut positions: Vec<_> = members
			.iter()
			.map(|w| position(&WorkerId::from_bytes(*w)))
			.collect();
		positions.sort_unstable_by(|a, b| b.cmp(a));
		positions[members.len() / 2]
	};
	let old = majority(&set.members);
	match &set.phase {
		TabletPhase::Joint(p) => old.min(majority(&p.next)),
		TabletPhase::Stable | TabletPhase::Learning(_) => old,
	}
}

/// A new destination has no claim or matching evidence until its correlated reply arrives.
/// Its append limit starts at the configured maximum until its shard reports a smaller one.
pub(super) fn unobserved(max_append_bytes: u32) -> Replica {
	Replica {
		append_limit_bytes: max_append_bytes,
		repair: None,
		repair_attempt: 0,
		repair_avoid: None,
		matched: false,
		claim: None,
		pending_claim: None,
		incarnation: [0; 16],
		acked: 0,
		sent: 0,
		in_flight: BTreeMap::new(),
		refused: None,
		repair_floor: None,
		repair_goal: None,
		blocked: None,
	}
}

/// Replaces only newer configuration evidence. Retries preserve client and entry identities;
/// all previous operation IDs are retired before any new-configuration reply can count.
/// The coordinator must carry client fences across promotion; claiming again during an uncommitted
/// claim entry cannot substitute for that control-plane guarantee.
pub(super) fn adopt(s: &mut VolumeClientState, set: TabletReplicaSet, fx: &mut Effects) -> bool {
	if set.tablet_replica_set_version <= s.replica_set.tablet_replica_set_version
		|| validate(&set).is_err()
	{
		return false;
	}

	// Recovery until the claim entry commits needs a new quorum proof, at the same client epoch.
	let claim_again = matches!(
		s.phase,
		VolumeClientPhase::Recovering(
			RecoveryPhase::Claiming
				| RecoveryPhase::Replaying { .. }
				| RecoveryPhase::Matching
				| RecoveryPhase::CommittingClaimEntry { .. }
		)
	);
	let goal = if claim_again {
		None
	} else {
		super::repair::goal(s)
	};
	if let VolumeClientPhase::Recovering(RecoveryPhase::Replaying { op_id, .. }) = s.phase {
		fx.cancel_timer(VolumeClientTimer::Control(op_id));
	}
	s.late_ack_floor = s.next_op;
	for (worker, replica) in &mut s.replicas {
		for op in replica.in_flight.keys() {
			fx.cancel_timer(VolumeClientTimer::Append(*op));
		}
		replica.in_flight.clear();
		if let Some(op) = replica.pending_claim.take() {
			fx.cancel_timer(VolumeClientTimer::Control(op));
		}
		if let Some(step) = replica.repair.take() {
			fx.cancel_timer(VolumeClientTimer::Control(step.op_id()));
		}
		// Previously committed history stays committed, but uncommitted writes need fresh votes.
		replica.acked = replica.acked.min(s.commit_publishing.committed);
		replica.sent = replica.acked;
		let new_learner = learner(&set, worker);
		if claim_again || new_learner {
			replica.repair_attempt = 0;
			replica.claim = None;
			replica.matched = false;
		}
		if new_learner {
			replica.incarnation = [0; 16];
			// A new learner's copy starts over, so an earlier copy's floor pins nothing.
			replica.repair_floor = None;
		}
		// The old version's operations and matching proof cannot authorize this version.
		replica.repair_goal = if replica.matched { None } else { goal };
	}
	for barrier in s.barriers.values_mut() {
		barrier.confirmed.clear();
	}

	// Keep matching histories for retained members, and discover new peers independently.
	let destinations = destinations(&set);
	s.loading_members
		.retain(|worker| destinations.contains(worker));
	s.replicas.retain(|worker, _| destinations.contains(worker));
	let max_append_bytes = s.config.max_append_bytes;
	for worker in destinations {
		s.replicas.entry(worker).or_insert_with(|| {
			let mut replica = unobserved(max_append_bytes);
			replica.repair_goal = goal;
			replica
		});
	}
	s.replica_set = std::sync::Arc::new(set);
	if claim_again {
		s.claim_entry = None;
		s.claim_quorum.clear();
		s.phase = VolumeClientPhase::Recovering(RecoveryPhase::Claiming);
		for worker in s.replicas.keys().cloned().collect::<Vec<_>>() {
			super::recovery::claim(s, worker, fx);
		}
	}
	true
}
