//! Replica-set fences survive restart and establish the old-majority promotion barrier.

use super::{LogShardPhase, LogShardState, tablet_grants};
use crate::log_shard_machine::types::TabletManifestEntry;
use rivet_data::generated::{
	depot_log_shard_v1::{Frame, TabletReplicaSetFrame},
	depot_storage_worker_protocol_v1::RejectResponse,
	shared_depot_types_v1::TabletReplicaSet,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Never lowers a disk fence, including when older covered segments are replayed.
pub(crate) fn replay(
	sets: &mut BTreeMap<[u8; 16], TabletReplicaSet>,
	frame: &TabletReplicaSetFrame,
) {
	if sets.get(&frame.tablet_id).is_none_or(|old| {
		old.tablet_replica_set_version < frame.replica_set.tablet_replica_set_version
	}) {
		sets.insert(frame.tablet_id, frame.replica_set.clone());
	}
}

/// Retains routing information with the version so stale-client replies can use a recovered fence.
fn from_view(tablet: &TabletManifestEntry) -> TabletReplicaSet {
	TabletReplicaSet {
		tablet_replica_set_version: tablet.tablet_replica_set_version,
		members: tablet.members.iter().map(|w| *w.as_bytes()).collect(),
		disks: tablet.disks.clone(),
		phase: tablet.phase.clone(),
	}
}

/// A tablet's effective replica set, borrowed from wherever it is recorded, so admission can
/// compare versions without copying the member and disk lists.
#[derive(Clone, Copy)]
pub(crate) enum EffectiveSet<'a> {
	Manifest(&'a TabletManifestEntry),
	Recorded(&'a TabletReplicaSet),
}

impl<'a> EffectiveSet<'a> {
	pub(crate) fn version(self) -> u64 {
		match self {
			EffectiveSet::Manifest(tablet) => tablet.tablet_replica_set_version,
			EffectiveSet::Recorded(set) => set.tablet_replica_set_version,
		}
	}

	pub(crate) fn to_set(self) -> TabletReplicaSet {
		match self {
			EffectiveSet::Manifest(tablet) => from_view(tablet),
			EffectiveSet::Recorded(set) => set.clone(),
		}
	}
}

/// Admission sees the highest disk, pending, or manifest version. Replies still wait for sync.
pub(crate) fn effective_ref<'a>(
	state: &'a LogShardState,
	tablet: &'a TabletManifestEntry,
) -> EffectiveSet<'a> {
	let mut set = EffectiveSet::Manifest(tablet);
	if let Some(disk) = state.tablet_replica_sets.get(&tablet.tablet_id)
		&& disk.tablet_replica_set_version > set.version()
	{
		set = EffectiveSet::Recorded(disk);
	}
	if let Some(pending) = state.memory_batches.pending_replica_set(&tablet.tablet_id)
		&& pending.tablet_replica_set_version > set.version()
	{
		set = EffectiveSet::Recorded(pending);
	}
	set
}

/// The owned form of [`effective_ref`].
pub(crate) fn effective(state: &LogShardState, tablet: &TabletManifestEntry) -> TabletReplicaSet {
	effective_ref(state, tablet).to_set()
}

/// Returns a missing disk fence once; an already pending frame shares the same ordered barrier.
pub(crate) fn frames(state: &LogShardState, tablet: &TabletManifestEntry) -> Vec<Frame> {
	let set = effective_ref(state, tablet);
	let installed = state
		.tablet_replica_sets
		.get(&tablet.tablet_id)
		.map_or(0, |r| r.tablet_replica_set_version);
	let pending = state
		.memory_batches
		.pending_replica_set(&tablet.tablet_id)
		.is_some_and(|pending| pending.tablet_replica_set_version >= set.version());
	if installed >= set.version() || pending {
		Vec::new()
	} else {
		vec![Frame::TabletReplicaSetFrame(TabletReplicaSetFrame {
			tablet_id: tablet.tablet_id,
			replica_set: set.to_set(),
		})]
	}
}

/// A listing requests a version barrier but cannot invent a future manifest assignment.
pub(crate) fn listing_frames(
	state: &LogShardState,
	tablet_id: &[u8; 16],
	version: u64,
) -> Result<Vec<Frame>, RejectResponse> {
	let tablet = state
		.tablet_manifest
		.tablets
		.get(tablet_id)
		.ok_or(RejectResponse::NotServing)?;
	if effective_ref(state, tablet).version() < version {
		return Err(RejectResponse::NotServing);
	}
	Ok(frames(state, tablet))
}

/// Progress of the replica-set fence scan. A fence can be missing only after a newer
/// manifest raised a tablet's version, after recovery replaced the disk fences, or after
/// a full write buffer stopped an earlier scan. Admission reserves the writing tablet's
/// own fence and a disk write never lowers one, so a completed scan stays valid across
/// writes until one of those happens. Rescanning every manifest tablet after each disk
/// write would make every write cost one pass over the cluster's tablets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum ReplicaSetScan {
	/// Every manifest tablet's effective replica set is on disk or in a pending frame.
	Settled,
	/// A newer view or recovery left tablets to examine, or a full write buffer cut the scan short.
	Pending,
}

/// Queues the missing fences within the write budget, then settles the scan. A full
/// buffer leaves it pending, and every event ends with this dispatch, so the event that
/// frees the buffer, such as a completed disk write, retries it.
pub(crate) fn dispatch(state: &mut LogShardState) {
	if !matches!(
		state.phase,
		LogShardPhase::Serving | LogShardPhase::Draining
	) || state.replica_set_scan == ReplicaSetScan::Settled
	{
		return;
	}
	let tablet_ids: Vec<_> = state.tablet_manifest.tablets.keys().copied().collect();
	for tablet_id in tablet_ids {
		let frames = frames(state, &state.tablet_manifest.tablets[&tablet_id]);
		if frames.is_empty() {
			continue;
		}
		let Ok(bytes) = tablet_grants::reserve(state, &frames) else {
			return;
		};
		// The batch tracks its disk size per frame, so frames enter only through it.
		if state
			.memory_batches
			.filling
			.push_frames(frames, state.config.max_batch_bytes)
			.is_err()
		{
			return;
		}
		state.memory_batches.filling.encoded_bytes += bytes;
	}
	state.replica_set_scan = ReplicaSetScan::Settled;
}

/// The newest replica set this shard knows for a tablet, from its manifest view or disk fence.
pub(crate) fn known(state: &LogShardState, tablet_id: &[u8; 16]) -> Option<TabletReplicaSet> {
	state
		.tablet_manifest
		.tablets
		.get(tablet_id)
		.map(|tablet| effective(state, tablet))
		.or_else(|| state.tablet_replica_sets.get(tablet_id).cloned())
}

/// Forget removes replica generations, but never the tablet fence. A later ordinary
/// reopen must not let an old learner request erase a now-counting replica. The
/// current Learning generation remains eligible only on a non-counting learner.
pub(crate) fn learner_floor(state: &LogShardState, tablet_id: &[u8; 16]) -> u64 {
	let Some(set) = known(state, tablet_id) else {
		return 0;
	};
	if matches!(&set.phase, rivet_data::generated::shared_depot_types_v1::TabletPhase::Learning(phase)
		if !set.members.contains(state.worker_id.as_bytes()) && phase.next.contains(state.worker_id.as_bytes()))
	{
		set.tablet_replica_set_version.saturating_sub(1)
	} else {
		set.tablet_replica_set_version
	}
}


