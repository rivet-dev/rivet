//! Every loaded replica on a shard, held in one slab with a hash index into it.
//!
//! A replica already stores its volume log key, so the index holds only a four-byte row id
//! per replica and compares keys through the slab. The key lives once, in its row, and the
//! side maps that track replicas between events key on the row id instead of cloning the
//! key again. A row id is stable for as long as its replica is loaded: removal leaves the
//! slot free, and the next insert reuses the most recently freed slot, so a command that
//! takes a replica out and puts it back in the same transition keeps its row id. Equality,
//! hashing and serialization see the slots and the free list, so a replay that loads and
//! unloads replicas in the same order assigns the same ids.

use std::{
	collections::hash_map::DefaultHasher,
	hash::{BuildHasher, BuildHasherDefault},
	ops::Index,
};

use depot_types::VolumeLogKey;
use hashbrown::HashTable;
use serde::{Deserialize, Serialize};

use super::loaded_volume_replica::LoadedVolumeReplica;

/// A loaded replica's slot in the table. Side maps key on it instead of the volume log key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) struct RowId(u32);

/// The loaded replicas of one shard, keyed by their volume log key.
#[derive(Clone, Serialize, Deserialize)]
#[serde(from = "StoredReplicaTable", into = "StoredReplicaTable")]
pub struct ReplicaTable {
	/// A fixed-key hasher, so a replay hashes exactly as the original run did.
	hasher: BuildHasherDefault<DefaultHasher>,
	index: HashTable<u32>,
	rows: Vec<Option<LoadedVolumeReplica>>,
	/// Freed slots, reused last freed first.
	free: Vec<u32>,
}

#[derive(Serialize, Deserialize)]
struct StoredReplicaTable {
	rows: Vec<Option<LoadedVolumeReplica>>,
	free: Vec<u32>,
}

impl From<StoredReplicaTable> for ReplicaTable {
	fn from(stored: StoredReplicaTable) -> Self {
		let mut table = Self {
			hasher: Default::default(),
			index: HashTable::new(),
			rows: stored.rows,
			free: stored.free,
		};
		for row in 0..table.rows.len() {
			if let Some(replica) = &table.rows[row] {
				let hash = table.hasher.hash_one(&replica.volume_log_key);
				let hasher = &table.hasher;
				let rows = &table.rows;
				table
					.index
					.insert_unique(hash, row as u32, |row| row_hash(hasher, rows, *row));
			}
		}
		table
	}
}

impl From<ReplicaTable> for StoredReplicaTable {
	fn from(table: ReplicaTable) -> Self {
		Self {
			rows: table.rows,
			free: table.free,
		}
	}
}

impl Default for ReplicaTable {
	fn default() -> Self {
		Self {
			hasher: Default::default(),
			index: HashTable::new(),
			rows: Vec::new(),
			free: Vec::new(),
		}
	}
}

fn row_hash(
	hasher: &BuildHasherDefault<DefaultHasher>,
	rows: &[Option<LoadedVolumeReplica>],
	row: u32,
) -> u64 {
	match &rows[row as usize] {
		Some(replica) => hasher.hash_one(&replica.volume_log_key),
		None => 0,
	}
}

impl ReplicaTable {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn len(&self) -> usize {
		self.rows.len() - self.free.len()
	}

	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	fn find(&self, key: &VolumeLogKey) -> Option<u32> {
		let hash = self.hasher.hash_one(key);
		self.index
			.find(hash, |row| {
				self.rows[*row as usize]
					.as_ref()
					.is_some_and(|replica| replica.volume_log_key == *key)
			})
			.copied()
	}

	pub fn contains_key(&self, key: &VolumeLogKey) -> bool {
		self.find(key).is_some()
	}

	/// The row id of a loaded replica.
	pub(crate) fn row_id(&self, key: &VolumeLogKey) -> Option<RowId> {
		self.find(key).map(RowId)
	}

	/// The replica in a row, or `None` once it was removed.
	pub(crate) fn row(&self, id: RowId) -> Option<&LoadedVolumeReplica> {
		self.rows.get(id.0 as usize)?.as_ref()
	}

	pub(crate) fn row_mut(&mut self, id: RowId) -> Option<&mut LoadedVolumeReplica> {
		self.rows.get_mut(id.0 as usize)?.as_mut()
	}

	pub fn get(&self, key: &VolumeLogKey) -> Option<&LoadedVolumeReplica> {
		let row = self.find(key)?;
		self.rows[row as usize].as_ref()
	}

	pub fn get_mut(&mut self, key: &VolumeLogKey) -> Option<&mut LoadedVolumeReplica> {
		let row = self.find(key)?;
		self.rows[row as usize].as_mut()
	}

	/// Stores `replica` under its own key, returning its row id and the replica it replaced.
	/// The key argument must equal the replica's key; it is accepted for call-site symmetry
	/// with maps and checked in debug builds.
	pub(crate) fn insert(
		&mut self,
		key: VolumeLogKey,
		replica: LoadedVolumeReplica,
	) -> (RowId, Option<LoadedVolumeReplica>) {
		debug_assert!(
			key == replica.volume_log_key,
			"replica stored under another key"
		);
		drop(key);

		if let Some(row) = self.find(&replica.volume_log_key) {
			let replaced = self.rows[row as usize].replace(replica);
			return (RowId(row), replaced);
		}

		let hash = self.hasher.hash_one(&replica.volume_log_key);
		let row = match self.free.pop() {
			Some(row) => {
				self.rows[row as usize] = Some(replica);
				row
			}
			None => {
				let row = u32::try_from(self.rows.len()).unwrap_or(u32::MAX);
				self.rows.push(Some(replica));
				row
			}
		};
		let hasher = &self.hasher;
		let rows = &self.rows;
		self.index
			.insert_unique(hash, row, |row| row_hash(hasher, rows, *row));
		(RowId(row), None)
	}

	/// Removes a replica, freeing its row for the next insert.
	pub fn remove(&mut self, key: &VolumeLogKey) -> Option<LoadedVolumeReplica> {
		let hash = self.hasher.hash_one(key);
		let rows = &self.rows;
		let entry = self
			.index
			.find_entry(hash, |row| {
				rows[*row as usize]
					.as_ref()
					.is_some_and(|replica| replica.volume_log_key == *key)
			})
			.ok()?;
		let (row, _) = entry.remove();
		let replica = self.rows[row as usize].take();
		self.free.push(row);
		replica
	}

	pub fn remove_entry(
		&mut self,
		key: &VolumeLogKey,
	) -> Option<(VolumeLogKey, LoadedVolumeReplica)> {
		let replica = self.remove(key)?;
		Some((replica.volume_log_key.clone(), replica))
	}

	pub fn keys(&self) -> impl Iterator<Item = &VolumeLogKey> {
		self.values().map(|replica| &replica.volume_log_key)
	}

	pub fn values(&self) -> impl Iterator<Item = &LoadedVolumeReplica> {
		self.rows.iter().filter_map(Option::as_ref)
	}

	pub fn values_mut(&mut self) -> impl Iterator<Item = &mut LoadedVolumeReplica> {
		self.rows.iter_mut().filter_map(Option::as_mut)
	}

	pub fn iter(&self) -> impl Iterator<Item = (&VolumeLogKey, &LoadedVolumeReplica)> {
		self.values()
			.map(|replica| (&replica.volume_log_key, replica))
	}

	/// Every loaded replica with its row id, in row order.
	pub(crate) fn rows(&self) -> impl Iterator<Item = (RowId, &LoadedVolumeReplica)> {
		self.rows.iter().enumerate().filter_map(|(row, replica)| {
			replica.as_ref().map(|replica| (RowId(row as u32), replica))
		})
	}
}

impl<'a> IntoIterator for &'a ReplicaTable {
	type Item = (&'a VolumeLogKey, &'a LoadedVolumeReplica);
	type IntoIter = std::iter::FilterMap<
		std::slice::Iter<'a, Option<LoadedVolumeReplica>>,
		fn(&'a Option<LoadedVolumeReplica>) -> Option<(&'a VolumeLogKey, &'a LoadedVolumeReplica)>,
	>;

	fn into_iter(self) -> Self::IntoIter {
		fn pair(
			row: &Option<LoadedVolumeReplica>,
		) -> Option<(&VolumeLogKey, &LoadedVolumeReplica)> {
			row.as_ref()
				.map(|replica| (&replica.volume_log_key, replica))
		}
		self.rows.iter().filter_map(pair)
	}
}

impl Index<&VolumeLogKey> for ReplicaTable {
	type Output = LoadedVolumeReplica;

	fn index(&self, key: &VolumeLogKey) -> &Self::Output {
		match self.get(key) {
			Some(replica) => replica,
			None => panic!("replica not loaded"),
		}
	}
}

impl FromIterator<LoadedVolumeReplica> for ReplicaTable {
	fn from_iter<I: IntoIterator<Item = LoadedVolumeReplica>>(replicas: I) -> Self {
		let mut table = Self::new();
		for replica in replicas {
			table.insert(replica.volume_log_key.clone(), replica);
		}
		table
	}
}

impl std::fmt::Debug for ReplicaTable {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_map()
			.entries(self.rows().map(|(id, replica)| (id, replica)))
			.finish()
	}
}

impl PartialEq for ReplicaTable {
	fn eq(&self, other: &Self) -> bool {
		self.rows == other.rows && self.free == other.free
	}
}

impl Eq for ReplicaTable {}

impl std::hash::Hash for ReplicaTable {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.rows.hash(state);
		self.free.hash(state);
	}
}
