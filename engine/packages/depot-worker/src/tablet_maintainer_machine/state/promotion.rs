//! Joint promotion first closes old-only commits, then observes a fresh prefix on both majorities.

use super::{MaintainerPhase, MaintainerState, Operation, cold, opening};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use depot_types::{VolumeLogKey, listing_order};
use rivet_data::generated::{
	depot_storage_worker_protocol_v1 as wire,
	shared_depot_types_v1::{TabletPhase, VolumeLogPosition},
};
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Promotion {
	#[default]
	Inactive,
	/// Acknowledgements certify the Joint version on disk, not merely a manifest observation.
	Fencing {
		acknowledged: BTreeSet<WorkerId>,
		retrying: BTreeSet<WorkerId>,
	},
	/// Every page in this round was requested after the old-majority disk barrier.
	Observing {
		retrying: BTreeSet<WorkerId>,
		listed: BTreeSet<WorkerId>,
		cursors: BTreeMap<WorkerId, Option<VolumeLogKey>>,
		/// Old-member appended tails bound even client commits not yet announced to a replica.
		frontiers: BTreeMap<WorkerId, BTreeMap<VolumeLogKey, VolumeLogPosition>>,
	},
}

impl Promotion {
	pub(crate) fn for_phase(phase: &TabletPhase) -> Self {
		if matches!(phase, TabletPhase::Joint(_)) {
			Self::Fencing {
				acknowledged: BTreeSet::new(),
				retrying: BTreeSet::new(),
			}
		} else {
			Self::Inactive
		}
	}
}

fn majority(members: &[[u8; 16]], seen: &BTreeSet<WorkerId>) -> bool {
	members
		.iter()
		.filter(|w| seen.contains(&WorkerId::from_bytes(**w)))
		.count()
		> members.len() / 2
}

/// Cancels only obsolete read attempts; mutations retain their original uncertainty barriers.
pub(crate) fn reset(state: &mut MaintainerState, fx: &mut Effects) {
	let old: Vec<_> = state
		.operations
		.index()
		.promotion_lists
		.values()
		.flatten()
		.copied()
		.collect();
	for id in old {
		state.finish(id, fx);
	}
	state.promotion = Promotion::for_phase(&state.tablet.replica_set.phase);
}

/// Shares the bounded status window. An unreachable old member does not prevent either majority.
pub(crate) fn drive(state: &mut MaintainerState, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving
		|| !matches!(state.tablet.replica_set.phase, TabletPhase::Joint(_))
	{
		return;
	}
	let counts = state.operations.index().counts;
	let active = (counts.promotion_list + counts.poll) as usize;
	let mut available = (state.config.tuning().max_status_requests as usize).saturating_sub(active);
	let members = state.members();
	for worker in members {
		if available == 0 {
			break;
		}
		if match &state.promotion {
			Promotion::Fencing { retrying, .. } | Promotion::Observing { retrying, .. } => {
				retrying.contains(&worker)
			}
			Promotion::Inactive => true,
		} {
			continue;
		}
		if state
			.operations
			.index()
			.promotion_lists
			.contains_key(&worker)
		{
			continue;
		}
		let (collecting, after) = match &state.promotion {
			Promotion::Fencing { acknowledged, .. } if !acknowledged.contains(&worker) => {
				(false, None)
			}
			Promotion::Observing {
				listed, cursors, ..
			} if !listed.contains(&worker) => (true, cursors.get(&worker).cloned().flatten()),
			_ => continue,
		};
		let op_id = state.begin(
			Operation::PromotionList {
				worker: worker.clone(),
				after: after.clone(),
				collecting,
			},
			fx,
		);
		fx.execute(MaintainerEffect::ReplicaRequest {
			op_id,
			worker_id: worker,
			request: wire::StorageWorkerRequest::ListTabletReplicasRequest(
				wire::ListTabletReplicasRequest {
					tablet_replica_set_version: state.tablet.replica_set.tablet_replica_set_version,
					tablet_grant: state.grant(),
					hash_range: state.tablet.hash_range,
					retired_parent: state.tablet.parent_tablet_id,
					after: after.map(Into::into),
					max_replicas: if collecting {
						state.config.tuning().listing_page_size
					} else {
						0
					},
				},
			),
		});
		available -= 1;
	}
}

/// Only version-fenced pages from the admitted disk contribute to either proof.
pub(crate) fn completed(
	state: &mut MaintainerState,
	worker: WorkerId,
	after: Option<VolumeLogKey>,
	collecting: bool,
	result: Result<wire::StorageWorkerResponse, String>,
	fx: &mut Effects,
) {
	let Ok(wire::StorageWorkerResponse::ListTabletReplicasResponse(page)) = result else {
		retry_later(state, worker, fx);
		return;
	};
	if state.disk_incarnation(&worker) != Some(page.disk_incarnation)
		|| page.tablet_replica_set_version != state.tablet.replica_set.tablet_replica_set_version
	{
		retry_later(state, worker, fx);
		return;
	}
	if !collecting {
		let Promotion::Fencing { acknowledged, .. } = &mut state.promotion else {
			return;
		};
		acknowledged.insert(worker);
		if !majority(&state.tablet.replica_set.members, acknowledged) {
			return;
		}
		// Discard outstanding barrier reads. No pre-barrier page can become final evidence.
		reset(state, fx);
		state.promotion = Promotion::Observing {
			retrying: BTreeSet::new(),
			listed: BTreeSet::new(),
			cursors: BTreeMap::new(),
			frontiers: BTreeMap::new(),
		};
		return;
	}
	if !matches!(state.promotion, Promotion::Observing { .. }) {
		return;
	}
	let mut cursor = after;
	if page.replicas.len() > state.config.tuning().listing_page_size as usize {
		super::fence(state, fx);
		return;
	}
	let count = page.replicas.len();
	for frame in &page.replicas {
		let key = VolumeLogKey::from(frame.volume_log_key.clone());
		let hash = depot_types::partition_key_hash(&key.0.volume_key);
		if cursor
			.as_ref()
			.is_some_and(|old| listing_order(old) >= listing_order(&key))
			|| !state.tablet.hash_range.contains(hash)
			|| key.0.volume_key.byte_len() > state.config.tuning().max_volume_key_bytes as usize
		{
			super::fence(state, fx);
			return;
		}
		cursor = Some(key.clone());
		if let Promotion::Observing { frontiers, .. } = &mut state.promotion
			&& state.tablet.replica_set.members.contains(worker.as_bytes())
		{
			let frontier = frontiers
				.entry(worker.clone())
				.or_default()
				.entry(key.clone())
				.or_insert_with(|| frame.state.appended.clone());
			if (
				frame.state.appended.volume_seq,
				frame.state.appended.client_epoch,
			) > (frontier.volume_seq, frontier.client_epoch)
			{
				*frontier = frame.state.appended.clone();
			}
		}
	}
	// Reuse bounded discovery for unknown keys and refresh observations already held.
	opening::listed(
		state,
		worker.clone(),
		None,
		Ok(wire::StorageWorkerResponse::ListTabletReplicasResponse(
			page.clone(),
		)),
		fx,
	);
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	for frame in page.replicas {
		cold::observe(
			state,
			&VolumeLogKey::from(frame.volume_log_key),
			worker.clone(),
			frame.state,
		);
	}
	if let Promotion::Observing {
		listed, cursors, ..
	} = &mut state.promotion
	{
		if count < state.config.tuning().listing_page_size as usize {
			listed.insert(worker.clone());
			cursors.remove(&worker);
		} else {
			cursors.insert(worker, cursor);
		}
	}
}

/// Both sets have installed Joint; the fresh old majority bounds every possible old-only commit.
pub(crate) fn observed(state: &MaintainerState) -> bool {
	let TabletPhase::Joint(phase) = &state.tablet.replica_set.phase else {
		return false;
	};
	let Promotion::Observing { listed, .. } = &state.promotion else {
		return false;
	};
	majority(&state.tablet.replica_set.members, listed) && majority(&phase.next, listed)
}

pub(crate) fn frontier_position(state: &MaintainerState, key: &VolumeLogKey) -> VolumeLogPosition {
	match &state.promotion {
		Promotion::Observing {
			frontiers, listed, ..
		} => frontiers
			.iter()
			.filter(|(worker, _)| listed.contains(*worker))
			.filter_map(|(_, logs)| logs.get(key))
			.max_by_key(|position| (position.volume_seq, position.client_epoch))
			.cloned(),
		_ => None,
	}
	.unwrap_or(VolumeLogPosition {
		volume_seq: 0,
		client_epoch: 0,
	})
}

/// Failed reads back off without occupying a transport slot.
fn retry_later(state: &mut MaintainerState, worker: WorkerId, fx: &mut Effects) {
	match &mut state.promotion {
		Promotion::Fencing { retrying, .. } | Promotion::Observing { retrying, .. } => {
			retrying.insert(worker.clone());
			fx.arm_timer_after(
				MaintainerTimer::PromotionRetry(worker),
				state.config.tuning().retry_ms,
			);
		}
		Promotion::Inactive => {}
	}
}

/// A stale timer only releases a wait; dispatch still validates the current Joint assignment.
pub(crate) fn retry(state: &mut MaintainerState, worker: WorkerId) {
	match &mut state.promotion {
		Promotion::Fencing { retrying, .. } | Promotion::Observing { retrying, .. } => {
			retrying.remove(&worker);
		}
		Promotion::Inactive => {}
	}
}

/// Final quorum evidence uses only members that completed this post-barrier listing.
pub(crate) fn member_observed(state: &MaintainerState, worker: &[u8; 16]) -> bool {
	match &state.promotion {
		Promotion::Observing { listed, .. } => listed.contains(&WorkerId::from_bytes(*worker)),
		_ => false,
	}
}
