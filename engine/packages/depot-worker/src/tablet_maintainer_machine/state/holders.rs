//! Unloading the volumes a gone holder process left open.
//!
//! A claim records the process that holds it (`VolumeLogReplicaState.claimedHolder`). A
//! volume quiet for `holder_check_after_ms` parks under that holder, and the maintainer takes
//! one topology watch per holder process, never one per volume; topology shares that watch
//! with every other tablet on this worker. A live holder keeps its volumes loaded and costs
//! nothing per volume: no timer, no message and no visit. Only the holder's `HolderGone`
//! event walks its parked volumes, and unloads them.
//!
//! The watch is low-urgency cleanup, not fencing. Gone is final for the process it names: an
//! expired or released lease never becomes live again, and a restarted worker is a different
//! process that claims again under a new holder. So the unload needs no second lookup however
//! long it waits for the store; it only rechecks that the volume stayed quiet and still names
//! that holder.
use super::{
	LoadedVolume, MaintainerPhase, MaintainerState,
	unload::{self, UnloadReason},
	volume_keys::KeyId,
};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::MaintainerTimer};
use depot_types::VolumeLogKey;
use rivet_data::generated::shared_depot_types_v1::VolumeHolder;
use rivet_state_machine::IndexedDeadlineQueue;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The process holding `volume`'s newest claim, when that claim recorded one. A newest claim
/// with no recorded holder, such as a maintainer's fencing claim, leaves the holder unknown.
pub(crate) fn holder(volume: &LoadedVolume) -> Option<VolumeHolder> {
	let newest = volume
		.members
		.values()
		.filter_map(|replica| replica.claimed_client.as_ref())
		.max_by_key(|client| client.client_epoch)?;
	volume
		.members
		.values()
		.filter(|replica| replica.claimed_client.as_ref() == Some(newest))
		.find_map(|replica| replica.claimed_holder.clone())
}

/// Holder watches retried per timer delivery, so one delivery's work stays bounded.
const WATCH_RETRIES_PER_TIMER: usize = 256;

/// A holder process, ordered so it can key the holder map and its deadline queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) struct HolderKey {
	pub worker_id: [u8; 16],
	pub worker_incarnation: [u8; 16],
}

impl HolderKey {
	pub(crate) fn of(holder: &VolumeHolder) -> Self {
		Self {
			worker_id: holder.worker_id,
			worker_incarnation: holder.worker_incarnation,
		}
	}

	pub(crate) fn holder(&self) -> VolumeHolder {
		VolumeHolder {
			worker_id: self.worker_id,
			worker_incarnation: self.worker_incarnation,
		}
	}
}

/// Quiet held volumes, parked under the process their newest claim names, with one topology
/// watch per holder process. A volume parks here at its holder check and leaves the idle check
/// queue: while its holder is live it is never idle-unloaded and costs nothing.
///
/// `by_holder` and `parked` are exact: a newer claim moves a volume to its new holder, and
/// removing a loaded volume drops it. A holder is watched exactly while it has a parked volume
/// here; `finish_event` reconciles the holders an event touched with `watched`, so the watch
/// effects follow the sets without a scan.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct HeldVolumes {
	/// Parked volumes by their interned key id.
	pub(crate) by_holder: BTreeMap<HolderKey, BTreeSet<KeyId>>,
	/// The holder each parked volume sits under and when it parked, so a claim by another
	/// process or a removal moves the volume out of its holder's set without a scan. Any
	/// activity since the park, such as an open by a process whose claim the members have not
	/// reported yet, keeps the volume out of a holder-gone unload.
	pub(crate) parked: BTreeMap<KeyId, Parked>,
	/// Holders with a `WatchHolderRequest` this maintainer has not ended by `UnwatchHolder`.
	/// A refused watch stays here until its retry or its unwatch, so the executor always gets
	/// an `UnwatchHolder` for a holder it was asked to watch.
	pub(crate) watched: BTreeMap<HolderKey, WatchSlot>,
	pub(crate) next_watch_id: u64,
	/// Holders whose parked set or watch changed during the current event.
	pub(crate) dirty: BTreeSet<HolderKey>,
	/// Holders whose watch topology refused, retried after `holder_check_after_ms`.
	pub(crate) watch_retries: IndexedDeadlineQueue<HolderKey>,
	/// The time `MaintainerTimer::HolderWatchRetry` is armed for.
	pub(crate) armed_at: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct WatchSlot {
	/// The current request's id; a `HolderWatchRefused` of any other is stale.
	pub watch_id: u64,
	/// Topology refused this watch; a retry is scheduled while the holder has parked volumes.
	pub refused: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Parked {
	pub holder: HolderKey,
	pub parked_at: u64,
}

/// Whether `key` is parked under the process its newest claim names.
pub(crate) fn parked(state: &MaintainerState, key: &VolumeLogKey, volume: &LoadedVolume) -> bool {
	let Some(id) = state.volume_keys.find(key) else {
		return false;
	};
	match (state.held_volumes.parked.get(&id), holder(volume)) {
		(Some(parked), Some(holder)) => parked.holder == HolderKey::of(&holder),
		(Some(_), None) | (None, Some(_)) | (None, None) => false,
	}
}

/// Moves a parked volume to the process a newer observed claim names: out of the old
/// holder's set, whose watch ends when it holds nothing else here, and under the new holder,
/// which is watched when it is new here. A newest claim naming no holder returns the volume to
/// the idle rule instead.
pub(crate) fn claim_observed(state: &mut MaintainerState, key: &VolumeLogKey) {
	let Some(id) = state.volume_keys.find(key) else {
		return;
	};
	let Some(parked) = state.held_volumes.parked.get(&id).copied() else {
		return;
	};
	let Some(volume) = state.loaded_volumes.get(key) else {
		return;
	};
	let current = holder(volume);
	if current
		.as_ref()
		.is_some_and(|holder| HolderKey::of(holder) == parked.holder)
	{
		return;
	}
	unpark(&mut state.held_volumes, id);
	match current {
		Some(holder) => park(state, id, &holder),
		None => super::unload::track_idle(state, key),
	}
}

/// Parks a quiet volume under `holder`. The first volume of a holder here starts its watch.
pub(crate) fn park(state: &mut MaintainerState, id: KeyId, holder: &VolumeHolder) {
	let holder = HolderKey::of(holder);
	let parked_at = state.now_boot_ms;
	let held = &mut state.held_volumes;
	if let Some(previous) = held.parked.get(&id).copied()
		&& previous.holder != holder
	{
		unpark(held, id);
	}
	held.by_holder.entry(holder).or_default().insert(id);
	held.parked.insert(id, Parked { holder, parked_at });
	held.dirty.insert(holder);
}

/// Removes `id` from its holder's set, if it is parked. Called for a newer claim and for every
/// removal of a loaded volume, since key ids are reused once released.
pub(crate) fn unpark(held: &mut HeldVolumes, id: KeyId) {
	let Some(parked) = held.parked.remove(&id) else {
		return;
	};
	if let Some(ids) = held.by_holder.get_mut(&parked.holder) {
		ids.remove(&id);
		if ids.is_empty() {
			held.by_holder.remove(&parked.holder);
		}
	}
	held.dirty.insert(parked.holder);
}

/// **HolderGone:** the holder's topology watch resolved, so that process is gone for good.
///
/// Unloads each volume parked under it that still names it, was quiet for a full holder
/// interval before it parked and since, and is not busy; the rest return to the idle rule. Quiet matters because a new process may
/// have opened the volume under a claim the members have not reported yet. Outside `Serving`
/// nothing unloads and every parked volume returns to the idle rule, which parks it again
/// later. This is the only place that visits a holder's parked volumes.
pub(crate) fn gone(state: &mut MaintainerState, holder: VolumeHolder) {
	let holder_key = HolderKey::of(&holder);
	state.held_volumes.dirty.insert(holder_key);
	state.held_volumes.watch_retries.cancel(&holder_key);
	let Some(parked) = state.held_volumes.by_holder.remove(&holder_key) else {
		return;
	};
	crate::metrics::record_idle_visits("holder_gone", parked.len());
	let serving = state.phase == MaintainerPhase::Serving;
	for id in parked {
		let Some(Parked { parked_at, .. }) = state.held_volumes.parked.remove(&id) else {
			continue;
		};
		let Some(key) = state.volume_keys.key(id).cloned() else {
			continue;
		};
		let Some(volume) = state.loaded_volumes.get(&key) else {
			continue;
		};
		let still_held = holder_of(volume) == Some(holder_key);
		let quiet = quiet_through(state, volume.last_activity, parked_at);
		let free = !volume.is_deleted()
			&& !state.pending.contains_key(&key.0.volume_key)
			&& !state.unloading.contains_key(&key);
		if serving && still_held && quiet && free {
			unload::begin(
				state,
				key,
				UnloadReason::HolderGone {
					holder: holder.clone(),
					quiet_since: parked_at,
				},
			);
		} else {
			unload::track_idle(state, &key);
		}
	}
}

/// Whether a volume last active at `last_activity` was quiet for a full holder interval before
/// it parked at `parked_at` and has stayed quiet since. A volume that a newer claim moved
/// under another holder parks at once, so this is what keeps an open whose claim the members
/// have not reported yet from being fenced by an older holder's gone unload.
fn quiet_through(state: &MaintainerState, last_activity: u64, parked_at: u64) -> bool {
	last_activity.saturating_add(state.config.tuning().holder_check_after_ms) <= parked_at
}

fn holder_of(volume: &LoadedVolume) -> Option<HolderKey> {
	holder(volume).as_ref().map(HolderKey::of)
}

/// **HolderWatchRefused:** topology could not take watch `watch_id`, for example at its watch
/// cap. While the holder still has a parked volume here it is watched again after
/// `holder_check_after_ms`, a long backoff, since a full watch table frees up slowly. A refusal
/// of an older watch than the current one is ignored.
pub(crate) fn watch_refused(state: &mut MaintainerState, holder: VolumeHolder, watch_id: u64) {
	let holder = HolderKey::of(&holder);
	let held = &mut state.held_volumes;
	let Some(slot) = held
		.watched
		.get_mut(&holder)
		.filter(|slot| slot.watch_id == watch_id)
	else {
		return;
	};
	slot.refused = true;
	held.dirty.insert(holder);
	if held.by_holder.contains_key(&holder) {
		let at = state
			.now_boot_ms
			.saturating_add(state.config.tuning().holder_check_after_ms);
		held.watch_retries.schedule(holder, at);
	}
}

/// **Timer HolderWatchRetry:** due refused holders are watched again by `finish_event`.
pub(crate) fn retries_due(state: &mut MaintainerState) {
	state.held_volumes.armed_at = None;
	let due = state
		.held_volumes
		.watch_retries
		.pop_due(state.now_boot_ms, WATCH_RETRIES_PER_TIMER);
	for (_, holder) in due {
		state.held_volumes.dirty.insert(holder);
	}
}

/// After every event: starts a watch for each touched holder that now has parked volumes and
/// none, and ends it for each that has none left. Visits only the holders the event touched.
pub(crate) fn finish_event(state: &mut MaintainerState, fx: &mut Effects) {
	let dirty = std::mem::take(&mut state.held_volumes.dirty);
	for holder in dirty {
		let held = &mut state.held_volumes;
		let parked = held.by_holder.contains_key(&holder);
		let watched = held.watched.get(&holder).copied();
		let retrying = held.watch_retries.scheduled_at(&holder).is_some();
		let needs_watch = match watched {
			None => true,
			Some(slot) => slot.refused && !retrying,
		};
		if parked && needs_watch {
			held.next_watch_id += 1;
			let watch_id = held.next_watch_id;
			held.watched.insert(
				holder,
				WatchSlot {
					watch_id,
					refused: false,
				},
			);
			fx.execute(MaintainerEffect::WatchHolderRequest {
				holder: holder.holder(),
				watch_id,
			});
		} else if !parked {
			held.watch_retries.cancel(&holder);
			if watched.is_some() {
				held.watched.remove(&holder);
				fx.execute(MaintainerEffect::UnwatchHolder {
					holder: holder.holder(),
				});
			}
		}
	}
	let next_at = state.held_volumes.watch_retries.next_at();
	super::arm_queue_head(
		&mut state.held_volumes.armed_at,
		next_at,
		&state.phase,
		state.now_boot_ms,
		MaintainerTimer::HolderWatchRetry,
		fx,
	);
}

/// Ends every holder watch when the maintainer stops serving its volumes.
pub(crate) fn stop(state: &mut MaintainerState, fx: &mut Effects) {
	let held = std::mem::take(&mut state.held_volumes);
	for holder in held.watched.into_keys() {
		fx.execute(MaintainerEffect::UnwatchHolder {
			holder: holder.holder(),
		});
	}
	fx.cancel_timer(MaintainerTimer::HolderWatchRetry);
}

/// Rechecks a holder-gone unload as it reaches the store: it proceeds only while the volume
/// stayed quiet since it parked and still names the gone holder, and otherwise falls back to
/// the idle rule. Returns whether the unload may read its row now.
pub(crate) fn ready(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	checked: &VolumeHolder,
	quiet_since: u64,
	fx: &mut Effects,
) -> bool {
	let still_gone = state.loaded_volumes.get(key).is_some_and(|volume| {
		quiet_through(state, volume.last_activity, quiet_since)
			&& holder(volume).as_ref() == Some(checked)
	});
	if !still_gone {
		unload::cancel(state, key, fx);
	}
	still_gone
}


