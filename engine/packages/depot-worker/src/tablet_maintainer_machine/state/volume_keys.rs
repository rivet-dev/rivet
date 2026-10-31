//! Loaded volume keys interned once, so the per-volume queues and sets key on a small id.
//!
//! A `VolumeLogKey` is about eighty bytes inline, and a quiet volume sits in several
//! queues and sets at once: the idle check queue, the status poll queue, the retention
//! queue and its holder's parked set. Keying those on a `KeyId` keeps one copy of the key
//! here instead of one per container. The id is a slot plus the slot's epoch, so an id left
//! behind in a queue after its volume unloaded resolves to nothing instead of to whatever
//! volume reuses the slot; queue entries are hints that reread the volume anyway.

use depot_types::VolumeLogKey;
use hashbrown::HashTable;
use serde::{Deserialize, Serialize};
use std::{
	collections::hash_map::DefaultHasher,
	hash::{BuildHasher, BuildHasherDefault},
};

/// An interned key. Equal to another id only for the same slot in the same lifetime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) struct KeyId {
	slot: u32,
	/// Goes up each time the slot is reused, so an id of an earlier occupant is stale.
	slot_epoch: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Slot {
	slot_epoch: u32,
	key: Option<VolumeLogKey>,
}

/// The interned keys of one maintainer. Equality, hashing and encoding see the slots and
/// the free list; the hash index is rebuilt when the keys are decoded.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(from = "StoredVolumeKeys", into = "StoredVolumeKeys")]
pub(crate) struct VolumeKeys {
	slots: Vec<Slot>,
	/// Freed slots, reused last freed first.
	free: Vec<u32>,
	/// A fixed-key hasher, so a replay hashes exactly as the original run did.
	hasher: BuildHasherDefault<DefaultHasher>,
	index: HashTable<u32>,
}

#[derive(Serialize, Deserialize)]
struct StoredVolumeKeys {
	slots: Vec<Slot>,
	free: Vec<u32>,
}

impl From<StoredVolumeKeys> for VolumeKeys {
	fn from(stored: StoredVolumeKeys) -> Self {
		let mut keys = Self {
			slots: stored.slots,
			free: stored.free,
			hasher: Default::default(),
			index: HashTable::new(),
		};
		for slot in 0..keys.slots.len() {
			if let Some(key) = &keys.slots[slot].key {
				let hash = keys.hasher.hash_one(key);
				let hasher = &keys.hasher;
				let slots = &keys.slots;
				keys.index
					.insert_unique(hash, slot as u32, |slot| slot_hash(hasher, slots, *slot));
			}
		}
		keys
	}
}

impl From<VolumeKeys> for StoredVolumeKeys {
	fn from(keys: VolumeKeys) -> Self {
		Self {
			slots: keys.slots,
			free: keys.free,
		}
	}
}

impl VolumeKeys {
	

	fn slot_of(&self, key: &VolumeLogKey) -> Option<u32> {
		let hash = self.hasher.hash_one(key);
		self.index
			.find(hash, |slot| {
				self.slots[*slot as usize].key.as_ref() == Some(key)
			})
			.copied()
	}

	/// The id of an interned key.
	pub(crate) fn find(&self, key: &VolumeLogKey) -> Option<KeyId> {
		let slot = self.slot_of(key)?;
		Some(KeyId {
			slot,
			slot_epoch: self.slots[slot as usize].slot_epoch,
		})
	}

	/// The key an id names, or `None` for an id whose key was released since.
	pub(crate) fn key(&self, id: KeyId) -> Option<&VolumeLogKey> {
		let slot = self.slots.get(id.slot as usize)?;
		if slot.slot_epoch != id.slot_epoch {
			return None;
		}
		slot.key.as_ref()
	}

	/// Interns `key`, detached from the message it arrived in, or returns its existing id.
	pub(crate) fn intern(&mut self, key: &VolumeLogKey) -> KeyId {
		if let Some(id) = self.find(key) {
			return id;
		}
		let hash = self.hasher.hash_one(key);
		let slot = match self.free.pop() {
			Some(slot) => {
				let reused = &mut self.slots[slot as usize];
				reused.slot_epoch = reused.slot_epoch.wrapping_add(1);
				reused.key = Some(key.detach());
				slot
			}
			None => {
				let slot = u32::try_from(self.slots.len()).unwrap_or(u32::MAX);
				self.slots.push(Slot {
					slot_epoch: 0,
					key: Some(key.detach()),
				});
				slot
			}
		};
		let hasher = &self.hasher;
		let slots = &self.slots;
		self.index
			.insert_unique(hash, slot, |slot| slot_hash(hasher, slots, *slot));
		KeyId {
			slot,
			slot_epoch: self.slots[slot as usize].slot_epoch,
		}
	}

	/// Frees `key`'s slot. Ids handed out for it no longer resolve.
	pub(crate) fn release(&mut self, key: &VolumeLogKey) {
		let hash = self.hasher.hash_one(key);
		let slots = &self.slots;
		let Ok(entry) = self
			.index
			.find_entry(hash, |slot| slots[*slot as usize].key.as_ref() == Some(key))
		else {
			return;
		};
		let (slot, _) = entry.remove();
		self.slots[slot as usize].key = None;
		self.free.push(slot);
	}
}

/// Rehashes an indexed slot when the hash index grows. It must hash the key exactly as
/// lookups do: hashing the slot's `Option` instead files every moved entry under a hash no
/// lookup computes, so its key stops being found and is interned again in a second slot.
/// Every indexed slot holds a key, because `release` drops the index entry first.
fn slot_hash(hasher: &BuildHasherDefault<DefaultHasher>, slots: &[Slot], slot: u32) -> u64 {
	match &slots[slot as usize].key {
		Some(key) => hasher.hash_one(key),
		None => 0,
	}
}

impl std::fmt::Debug for VolumeKeys {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_map()
			.entries(self.slots.iter().enumerate().filter_map(|(slot, entry)| {
				entry.key.as_ref().map(|key| {
					(
						KeyId {
							slot: slot as u32,
							slot_epoch: entry.slot_epoch,
						},
						key,
					)
				})
			}))
			.finish()
	}
}

impl PartialEq for VolumeKeys {
	fn eq(&self, other: &Self) -> bool {
		self.slots == other.slots && self.free == other.free
	}
}

impl Eq for VolumeKeys {}

impl std::hash::Hash for VolumeKeys {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.slots.hash(state);
		self.free.hash(state);
	}
}


