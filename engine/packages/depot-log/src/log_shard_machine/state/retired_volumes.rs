//! Deleted log retirement survives Forget until a higher tablet grant fences all
//! loads that the deleting owner could have sent. Ordinary unloads remain reusable.
//!
//! A fence holds only the 16-byte digest of its volume log key and a four-byte slot naming
//! its deleting grant, in one index shared by every tablet, so a lookup is one probe however
//! many tablets deleted volumes. A higher synced grant for a tablet releases that tablet's
//! older slots at once, in time proportional to the slots: a released slot's fences stop
//! counting immediately, and later events remove them a bounded slice of the index at a
//! time. Two keys sharing a 128-bit digest is negligible; if it happened, the later
//! retirement would replace or be ignored in favor of the earlier fence, and an unrelated
//! load sharing a retired digest would be refused.
//!
//! A tablet's fences are released by its next grant on this shard, or by its permanent
//! retirement after a split: a child's listing writes it on the child's members, and a manifest
//! view naming the parent as retired writes it here. A shard removed from a live tablet's
//! replica set keeps that tablet's fences until a later grant of it reaches the shard, at about
//! 20 bytes each and bounded by the deletes it saw while a member.

use super::LogShardState;
use depot_types::VolumeLogKey;
use hashbrown::HashTable;
use rivet_data::generated::{
	depot_log_shard_v1::{ForgetFrame, Frame, RetiredVolumeFrame, TabletGrantFrame},
	depot_storage_worker_protocol_v1::TabletGrant,
};
use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	fmt,
	hash::{Hash, Hasher},
};

/// Index shards, by a digest's leading byte. One growth step rehashes one shard, and one
/// drain step visits one shard: about 4,000 fences at a million.
const INDEX_SHARDS: usize = 256;

/// The fences of a shard.
///
/// Every live slot's epoch is at or above its tablet's synced grant, because a replayed
/// grant releases the slots below it and a replayed Forget below the synced grant is
/// ignored. A digest has at most one index entry; it counts only while its slot is live. A
/// later retirement of the same log by the same tablet moves it to the later epoch.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(from = "StoredRetiredVolumes", into = "StoredRetiredVolumes")]
pub struct RetiredVolumes {
	/// Empty until the first fence, then `INDEX_SHARDS` tables.
	index: Vec<HashTable<Fence>>,
	slots: Vec<Slot>,
	/// Free slot ids, reused last freed first.
	free_slots: Vec<u32>,
	/// Live slots by tablet, then epoch.
	live: BTreeMap<[u8; 16], BTreeMap<u64, u32>>,
	/// Fences in live slots.
	live_fences: usize,
	/// Index entries, live or waiting for the drain. The cap counts these, so memory stays
	/// within it while a release drains.
	entries: usize,
	drain: Drain,
}

#[derive(Clone, Copy)]
struct Fence {
	digest: [u8; 16],
	slot: u32,
}

#[derive(Clone)]
enum Slot {
	/// Fences retired under this grant still refuse loads.
	Live {
		grant: TabletGrantFrame,
		fences: usize,
	},
	/// A higher grant released these fences; their entries wait for the drain.
	Released,
	/// Reusable.
	Free,
}

/// Removal of released fences, one index shard per event.
#[derive(Clone, Default)]
enum Drain {
	#[default]
	Idle,
	/// Visiting shards from `next_shard`. Slots in `round` were released before the round
	/// started and are freed when it ends; slots released since then wait in `next_round`.
	Draining {
		next_shard: usize,
		round: Vec<u32>,
		next_round: Vec<u32>,
	},
}

fn table_hash(digest: &[u8; 16]) -> u64 {
	u64::from_le_bytes(std::array::from_fn(|i| digest[8 + i]))
}

impl RetiredVolumes {
	pub fn len(&self) -> usize {
		self.live_fences
	}

	pub fn is_empty(&self) -> bool {
		self.live_fences == 0
	}

	/// Index entries the cap charges: live fences and released ones not yet drained.
	fn held(&self) -> usize {
		self.entries
	}

	pub fn contains_key(&self, key: &VolumeLogKey) -> bool {
		self.find(&key.digest()).is_some()
	}

	/// The tablets holding live fences.
	pub(crate) fn tablet_ids(&self) -> impl Iterator<Item = &[u8; 16]> {
		self.live.keys()
	}

	fn entry(&self, digest: &[u8; 16]) -> Option<&Fence> {
		self.index
			.get(usize::from(digest[0]))?
			.find(table_hash(digest), |fence| fence.digest == *digest)
	}

	/// The deleting grant whose live fence holds `digest`.
	fn find(&self, digest: &[u8; 16]) -> Option<TabletGrantFrame> {
		let fence = self.entry(digest)?;
		match &self.slots[fence.slot as usize] {
			Slot::Live { grant, .. } => Some(grant.clone()),
			Slot::Released | Slot::Free => None,
		}
	}

	/// Records a fence unless `grants` already supersedes its grant or another live fence
	/// holds the same log. A fence moves only to a later epoch of its own tablet, because
	/// epochs of different tablets are not comparable. A second tablet cannot retire a fenced
	/// log, since the fence refuses that tablet's load; if it ever did, the first fence is
	/// kept, and it still refuses every load from the second tablet. Returns whether the
	/// fence was installed.
	fn retire(
		&mut self,
		grants: &BTreeMap<[u8; 16], u64>,
		digest: [u8; 16],
		grant: &TabletGrantFrame,
	) -> bool {
		let superseded = grants
			.get(&grant.tablet_id)
			.is_some_and(|epoch| *epoch > grant.tablet_grant_epoch);
		if superseded {
			return false;
		}
		if let Some(old) = self.find(&digest) {
			if old.tablet_id != grant.tablet_id {
				// The fence refuses the second tablet's load, so this needs a broken caller or
				// a digest collision. Keep the first fence; it refuses every later load anyway.
				debug_assert!(false, "a second tablet retired a fenced volume log");
				tracing::warn!(
					fenced_tablet = ?old.tablet_id,
					retiring_tablet = ?grant.tablet_id,
					"a second tablet retired a fenced volume log; keeping the first fence"
				);
				return false;
			}
			if old.tablet_grant_epoch > grant.tablet_grant_epoch {
				return false;
			}
		}

		let slot = self.live_slot(grant);
		if self.entry(&digest).is_some_and(|fence| fence.slot == slot) {
			return true;
		}
		if self.index.is_empty() {
			self.index.resize_with(INDEX_SHARDS, HashTable::new);
		}
		let table = &mut self.index[usize::from(digest[0])];
		let previous = match table.find_mut(table_hash(&digest), |fence| fence.digest == digest) {
			Some(fence) => Some(std::mem::replace(&mut fence.slot, slot)),
			None => {
				table.insert_unique(table_hash(&digest), Fence { digest, slot }, |fence| {
					table_hash(&fence.digest)
				});
				self.entries += 1;
				None
			}
		};
		if let Some(previous) = previous {
			self.forget_fence(previous);
		}
		if let Slot::Live { fences, .. } = &mut self.slots[slot as usize] {
			*fences += 1;
		}
		self.live_fences += 1;
		true
	}

	/// The live slot of `grant`, created when absent.
	fn live_slot(&mut self, grant: &TabletGrantFrame) -> u32 {
		if let Some(slot) = self
			.live
			.get(&grant.tablet_id)
			.and_then(|epochs| epochs.get(&grant.tablet_grant_epoch))
		{
			return *slot;
		}
		let created = Slot::Live {
			grant: grant.clone(),
			fences: 0,
		};
		let slot = match self.free_slots.pop() {
			Some(slot) => {
				self.slots[slot as usize] = created;
				slot
			}
			None => {
				self.slots.push(created);
				(self.slots.len() - 1) as u32
			}
		};
		self.live
			.entry(grant.tablet_id)
			.or_default()
			.insert(grant.tablet_grant_epoch, slot);
		slot
	}

	/// Uncounts a fence that left `slot`, freeing a live slot that no fence names any more.
	fn forget_fence(&mut self, slot: u32) {
		let Slot::Live { grant, fences } = &mut self.slots[slot as usize] else {
			return;
		};
		*fences -= 1;
		self.live_fences -= 1;
		if *fences == 0 {
			let grant = grant.clone();
			self.unlink(&grant);
			self.slots[slot as usize] = Slot::Free;
			self.free_slots.push(slot);
		}
	}

	fn unlink(&mut self, grant: &TabletGrantFrame) {
		if let Some(epochs) = self.live.get_mut(&grant.tablet_id) {
			epochs.remove(&grant.tablet_grant_epoch);
			if epochs.is_empty() {
				self.live.remove(&grant.tablet_id);
			}
		}
	}

	/// Releases every slot below a synced grant. Its fences stop counting now; the drain
	/// removes their entries over later events.
	fn release(&mut self, grant: &TabletGrantFrame) {
		let Some(epochs) = self.live.get_mut(&grant.tablet_id) else {
			return;
		};
		let kept = epochs.split_off(&grant.tablet_grant_epoch);
		let released = std::mem::replace(epochs, kept);
		if epochs.is_empty() {
			self.live.remove(&grant.tablet_id);
		}
		for slot in released.into_values() {
			if let Slot::Live { fences, .. } = self.slots[slot as usize] {
				self.live_fences -= fences;
			}
			self.slots[slot as usize] = Slot::Released;
			match &mut self.drain {
				Drain::Idle => {
					self.drain = Drain::Draining {
						next_shard: 0,
						round: vec![slot],
						next_round: Vec::new(),
					};
				}
				Drain::Draining { next_round, .. } => next_round.push(slot),
			}
		}
	}

	/// Removes the released fences of one index shard. Runs after every event while a
	/// release is draining and does nothing otherwise.
	pub(crate) fn drain_step(&mut self) {
		let Drain::Draining {
			next_shard,
			round,
			next_round,
		} = &mut self.drain
		else {
			return;
		};
		if let Some(table) = self.index.get_mut(*next_shard) {
			let slots = &self.slots;
			let before = table.len();
			table.retain(|fence| matches!(slots[fence.slot as usize], Slot::Live { .. }));
			self.entries -= before - table.len();
			// Return the memory of a mostly released shard, keeping room for the survivors.
			if table.len() < table.capacity() / 4 {
				table.shrink_to(table.len(), |fence| table_hash(&fence.digest));
			}
		}
		*next_shard += 1;
		if *next_shard < self.index.len() {
			return;
		}

		// Every shard was visited after these slots were released, so none is named.
		for slot in std::mem::take(round) {
			self.slots[slot as usize] = Slot::Free;
			self.free_slots.push(slot);
		}
		let waiting = std::mem::take(next_round);
		self.drain = if waiting.is_empty() {
			Drain::Idle
		} else {
			Drain::Draining {
				next_shard: 0,
				round: waiting,
				next_round: Vec::new(),
			}
		};
	}

	/// Live fences as (digest, grant), ordered by digest.
	fn sorted(&self) -> Vec<RetiredVolumeFrame> {
		let mut frames: Vec<RetiredVolumeFrame> = self
			.index
			.iter()
			.flat_map(|table| table.iter())
			.filter_map(|fence| match &self.slots[fence.slot as usize] {
				Slot::Live { grant, .. } => Some(RetiredVolumeFrame {
					volume_log_key_digest: fence.digest,
					retirement: grant.clone(),
				}),
				Slot::Released | Slot::Free => None,
			})
			.collect();
		frames.sort_unstable_by_key(|frame| frame.volume_log_key_digest);
		frames
	}
}

/// Equality, hashing and serialization see the live fences, not slots, layout or drain.
impl PartialEq for RetiredVolumes {
	fn eq(&self, other: &Self) -> bool {
		self.len() == other.len() && self.sorted() == other.sorted()
	}
}

impl Eq for RetiredVolumes {}

impl Hash for RetiredVolumes {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.sorted().hash(state);
	}
}

impl fmt::Debug for RetiredVolumes {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("RetiredVolumes")
			.field("live_fences", &self.live_fences)
			.field("live_tablets", &self.live.len())
			.finish()
	}
}

#[derive(Serialize, Deserialize)]
struct StoredRetiredVolumes {
	fences: Vec<RetiredVolumeFrame>,
}

impl From<RetiredVolumes> for StoredRetiredVolumes {
	fn from(retired: RetiredVolumes) -> Self {
		Self {
			fences: retired.sorted(),
		}
	}
}

impl From<StoredRetiredVolumes> for RetiredVolumes {
	fn from(stored: StoredRetiredVolumes) -> Self {
		let mut retired = RetiredVolumes::default();
		for frame in &stored.fences {
			restore(&mut retired, &BTreeMap::new(), frame);
		}
		retired
	}
}

/// Replay only synced facts, shared by disk completion and startup recovery. Adds every
/// key whose retirement it installs to `changed`. A released fence is not added: a checkpoint
/// lists only fences its own grants do not supersede, so dropping one needs no record.
pub(crate) fn replay(
	retired: &mut RetiredVolumes,
	grants: &BTreeMap<[u8; 16], u64>,
	frame: &Frame,
	changed: &mut BTreeSet<VolumeLogKey>,
) {
	match frame {
		Frame::ForgetFrame(ForgetFrame {
			volume_log_key,
			retirement: Some(grant),
		}) => {
			let key = VolumeLogKey(volume_log_key.clone());
			if retired.retire(grants, key.digest(), grant) {
				changed.insert(key);
			}
		}
		Frame::TabletGrantFrame(grant) => retired.release(grant),
		_ => {}
	}
}

/// Installs one checkpointed fence under the checkpoint's own grants.
pub(crate) fn restore(
	retired: &mut RetiredVolumes,
	grants: &BTreeMap<[u8; 16], u64>,
	frame: &RetiredVolumeFrame,
) {
	retired.retire(grants, frame.volume_log_key_digest, &frame.retirement);
}

/// A pending Forget already blocks every command to that log. After sync this
/// smaller fact keeps the old grant from reconstructing a fresh voting replica.
pub(crate) fn permits_load(state: &LogShardState, key: &VolumeLogKey, grant: &TabletGrant) -> bool {
	if state.retired_volumes.is_empty() {
		return true;
	}
	state.retired_volumes.find(&key.digest()).is_none_or(|old| {
		old.tablet_id == grant.tablet_id && grant.tablet_grant_epoch > old.tablet_grant_epoch
	})
}

/// Include unflushed retirement frames in the bounded admission count. Released fences
/// still waiting for the drain count too, so the ledger never holds more entries than the
/// cap. A full ledger leaves the deleted replica intact until a higher grant permits cleanup
/// and the drain removes the released entries.
pub(crate) fn has_capacity(state: &LogShardState, key: &VolumeLogKey) -> bool {
	if state.retired_volumes.contains_key(key) {
		return true;
	}
	let pending = state.memory_batches.pending_retirement_forgets();
	state.retired_volumes.held().saturating_add(pending) < state.config.max_loaded_volumes as usize
}

/// Every live fence, ordered by digest as a merged checkpoint chain orders them, so equal
/// ledgers encode equally.
pub fn checkpoint(retired: &RetiredVolumes) -> Vec<RetiredVolumeFrame> {
	retired.sorted()
}

/// The fence a checkpoint delta lists for a changed log, if one holds it.
pub(crate) fn checkpoint_frame(
	retired: &RetiredVolumes,
	key: &VolumeLogKey,
) -> Option<RetiredVolumeFrame> {
	let volume_log_key_digest = key.digest();
	retired
		.find(&volume_log_key_digest)
		.map(|retirement| RetiredVolumeFrame {
			volume_log_key_digest,
			retirement,
		})
}


