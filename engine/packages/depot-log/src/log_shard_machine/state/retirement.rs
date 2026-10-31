//! Removed workers collect their own replicas from ordered, complete manifest views.

use super::{LogShardPhase, LogShardState, tablet_grants};
use crate::log_shard_machine::types::{TabletManifestEntry, TabletManifestView};
use depot_types::hash_range_index::{HashRangeIndex, HashRangeLookup};
use depot_types::{PartitionKeyHash, VolumeLogKey, partition_key_hash};
use rivet_data::generated::{
	depot_log_shard_v1::{ForgetFrame, Frame, TabletGrantFrame},
	shared_depot_types_v1::TabletPhase,
};
use serde::{Deserialize, Serialize};

/// Progress of the removed-member scan. Exclusion depends only on the routing view and
/// this worker's identity, and admission rejects excluded keys, so a completed scan stays
/// valid until a newer view is installed, and keys a pending scan already passed never need
/// another look under the same view. Each view therefore costs one pass over the replicas,
/// however many events it takes to queue the forgets.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum RetirementScan {
	/// Every excluded replica already has a forget frame queued or written.
	Settled,
	/// A newer view, recovery, or a full write buffer left replicas to examine, starting after
	/// `after` in key order.
	Pending { after: Option<VolumeLogKey> },
}

/// An older view must never undo re-addition. Equal versions are immutable snapshots.
pub(crate) fn install(state: &mut LogShardState, manifest: TabletManifestView) {
	if manifest.tablet_manifest_version > state.tablet_manifest.tablet_manifest_version {
		let maintainers = state
			.tablet_manifest
			.tablets
			.iter()
			.map(|(id, tablet)| (*id, tablet.maintainer.clone()))
			.collect();
		set_manifest(state, manifest);
		super::status_notices::maintainers_changed(state, &maintainers);
		state.retirement_scan = RetirementScan::Pending { after: None };
		state.replica_set_scan = super::tablet_replica_sets::ReplicaSetScan::Pending;
	}
}

/// Replaces the routing view and its range index together.
pub(crate) fn set_manifest(state: &mut LogShardState, manifest: TabletManifestView) {
	state.tablet_ranges = HashRangeIndex::new(
		manifest
			.tablets
			.iter()
			.map(|(id, tablet)| (*id, tablet.hash_range)),
	);
	state.tablet_manifest = manifest;
}

/// The routing view's tablet covering `hash`, found through the range index on the hot path.
pub(crate) fn covering_tablet(
	state: &LogShardState,
	hash: PartitionKeyHash,
) -> Option<&TabletManifestEntry> {
	match state.tablet_ranges.lookup(hash) {
		HashRangeLookup::Covered(id) => state.tablet_manifest.tablets.get(id),
		HashRangeLookup::Uncovered => None,
		HashRangeLookup::Unindexed => state
			.tablet_manifest
			.tablets
			.values()
			.find(|tablet| tablet.hash_range.contains(hash)),
	}
}

/// Splits route by the current ranges, not by the tablet that originally loaded the replica.
pub(crate) fn excluded(state: &LogShardState, key: &VolumeLogKey) -> bool {
	if state.tablet_manifest.tablet_manifest_version == 0 {
		return false;
	}
	let hash = partition_key_hash(&key.0.volume_key);
	let Some(tablet) = covering_tablet(state, hash) else {
		return true;
	};
	if tablet.members.contains(&state.worker_id) {
		return false;
	}
	match &tablet.phase {
		TabletPhase::Stable => true,
		TabletPhase::Learning(phase) => !phase.next.contains(state.worker_id.as_bytes()),
		TabletPhase::Joint(phase) => !phase.next.contains(state.worker_id.as_bytes()),
	}
}

/// Queues bounded forget frames behind earlier accepted writes. Replicas remain in memory
/// until those frames sync, so checkpoints and replies never publish an unsynced absence.
/// A full buffer leaves the scan pending, so it resumes after the next disk barrier without
/// retaining a second work queue.
pub(crate) fn dispatch(state: &mut LogShardState) {
	if !matches!(
		state.phase,
		LogShardPhase::Serving | LogShardPhase::Draining
	) {
		return;
	}
	// A fence synced after the scan settled can name an already retired parent.
	if state.parent_retirement_due && retire_parent_grants(state) {
		state.parent_retirement_due = false;
	}
	if state.retirement_scan == RetirementScan::Settled {
		return;
	}
	let RetirementScan::Pending { mut after } = state.retirement_scan.clone() else {
		return;
	};
	if !retire_parent_grants(state) {
		return;
	}
	// Retirement is rare, so one pass collects the excluded replicas above the cursor in
	// key order instead of keeping the loaded replicas sorted for it.
	let mut pending = state
		.loaded_volume_replicas
		.keys()
		.filter(|key| after.as_ref().is_none_or(|after| *key > after))
		.filter(|key| excluded(state, key) && !state.memory_batches.forgetting(key))
		.cloned()
		.collect::<Vec<VolumeLogKey>>();
	pending.sort();
	let mut pending = pending.into_iter();
	loop {
		let next = pending.next();
		let Some(key) = next else {
			state.retirement_scan = RetirementScan::Settled;
			return;
		};

		let frame = Frame::ForgetFrame(ForgetFrame {
			retirement: None,
			volume_log_key: key.clone().into(),
		});
		let Ok(bytes) = tablet_grants::reserve(state, std::slice::from_ref(&frame)) else {
			state.retirement_scan = RetirementScan::Pending { after };
			return;
		};
		// The batch tracks its disk size per frame, so frames enter only through it.
		if state
			.memory_batches
			.filling
			.push_frames(vec![frame], state.config.max_batch_bytes)
			.is_err()
		{
			state.retirement_scan = RetirementScan::Pending { after };
			return;
		}
		state.memory_batches.filling.encoded_bytes += bytes;
		after = Some(key);
	}
}

/// Queues the permanent retirement grant for every split parent the routing view names as
/// retired while this shard still holds its fences, so they are released on a shard outside
/// every child's replica set. Retirement only refuses the parent's later requests, which a
/// child's listing already does on its members; cleanup of a retired parent runs under its
/// owner's own grant. Returns false when the write buffer is full, leaving the scan pending.
fn retire_parent_grants(state: &mut LogShardState) -> bool {
	let parents: Vec<[u8; 16]> = state
		.retired_volumes
		.tablet_ids()
		.filter(|tablet_id| state.tablet_manifest.retired_tablets.contains(*tablet_id))
		.filter(|tablet_id| {
			tablet_grants::effective_epoch(state, tablet_id)
				!= tablet_grants::RETIRED_TABLET_GRANT_EPOCH
		})
		.copied()
		.collect();
	for tablet_id in parents {
		let frame = Frame::TabletGrantFrame(TabletGrantFrame {
			tablet_id,
			tablet_grant_epoch: tablet_grants::RETIRED_TABLET_GRANT_EPOCH,
		});
		let Ok(bytes) = tablet_grants::reserve(state, std::slice::from_ref(&frame)) else {
			return false;
		};
		if state
			.memory_batches
			.filling
			.push_frames(vec![frame], state.config.max_batch_bytes)
			.is_err()
		{
			return false;
		}
		state.memory_batches.filling.encoded_bytes += bytes;
	}
	true
}


