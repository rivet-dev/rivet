//! Snapshot reads and atomic batches; the memory store retains every row version.

use crate::{Error, FsCheckpoint, FsDentry, FsFormat, FsInode, Result};
use std::cmp::Reverse;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowKey {
	Format,
	Inode(u64),
	Dentry(u64, Vec<u8>),
	Link(u64, u64, Vec<u8>),
	Xattr(u64, Vec<u8>),
	Kv(u64, Vec<u8>),
	Orphan(u64),
	Chunk(u64, u64),
}

impl RowKey {
	fn sort_fields(&self) -> (u8, u64, u8, u64, &[u8]) {
		match self {
			Self::Format => (0, 0, 6, 0, &[]),
			Self::Inode(ino) => (0, *ino, 0, 0, &[]),
			Self::Dentry(ino, name) => (0, *ino, 1, 0, name),
			Self::Link(ino, parent, name) => (0, *ino, 2, *parent, name),
			Self::Xattr(ino, name) => (0, *ino, 3, 0, name),
			Self::Kv(ino, key) => (0, *ino, 4, 0, key),
			Self::Orphan(ino) => (0, 0, 5, *ino, &[]),
			Self::Chunk(ino, chunk) => (1, *ino, 0, *chunk, &[]),
		}
	}
}

impl Ord for RowKey {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.sort_fields().cmp(&other.sort_fields())
	}
}

impl PartialOrd for RowKey {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
	Format(FsFormat),
	Inode(FsInode),
	Dentry(FsDentry),
	Bytes(bytes::Bytes),
	Unit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowRange {
	All,
	MetadataAll,
	ChunksAll,
	Metadata(u64),
	Chunks(u64),
	ChunkSuffix {
		ino: u64,
		first: u64,
	},
	Directory(u64),
	Xattrs(u64),
	Kv {
		ino: u64,
		start: Vec<u8>,
		end: Option<Vec<u8>>,
	},
	Orphans,
}

/// A pure rule can suspend on a cache miss. Its owner loads this bounded request
/// through an injected snapshot, then evaluates the same rule against that snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadRequest {
	GetInode(u64),
	Get(RowKey),
	Scan {
		range: RowRange,
		after: Option<RowKey>,
		limit: usize,
		as_of: Option<u64>,
	},
}

impl RowRange {
	pub fn contains(&self, key: &RowKey) -> bool {
		match (self, key) {
			(Self::All, _) => true,
			(Self::MetadataAll, key) => !matches!(key, RowKey::Chunk(..)),
			(Self::ChunksAll, RowKey::Chunk(..)) => true,
			(
				Self::Metadata(i),
				RowKey::Inode(j)
				| RowKey::Dentry(j, _)
				| RowKey::Link(j, _, _)
				| RowKey::Xattr(j, _)
				| RowKey::Kv(j, _),
			) => i == j,
			(Self::Chunks(i), RowKey::Chunk(j, _)) => i == j,
			(Self::ChunkSuffix { ino, first }, RowKey::Chunk(i, index)) => {
				ino == i && index >= first
			}
			(Self::Directory(i), RowKey::Dentry(j, _)) => i == j,
			(Self::Xattrs(i), RowKey::Xattr(j, _)) => i == j,
			(Self::Kv { ino, start, end }, RowKey::Kv(i, key)) => {
				ino == i && key >= start && end.as_ref().is_none_or(|end| key < end)
			}
			(Self::Orphans, RowKey::Orphan(_)) => true,
			_ => false,
		}
	}
}

/// Scan cursors are exclusive, with the same snapshot applied to every page.
pub trait RowStore {
	fn get(&self, key: &RowKey, as_of: Option<u64>) -> Result<Option<Row>>;
	fn scan(
		&self,
		range: &RowRange,
		after: Option<&RowKey>,
		limit: usize,
		as_of: Option<u64>,
	) -> Result<Vec<(RowKey, Row)>>;
}

/// `None` is a persisted Deleted version, not a physical key deletion.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Batch {
	pub rows: BTreeMap<RowKey, Option<Row>>,
	pub checkpoints: BTreeMap<(u64, Vec<u8>), FsCheckpoint>,
	/// Administrative ops (see `txn::administrative`) with their entry's volume seq, in log
	/// order. The store applies them after the rows of the same entry.
	pub admin: Vec<(u64, crate::FsOp)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryStore {
	pub versions: BTreeMap<(RowKey, Reverse<u64>), Option<Row>>,
	pub checkpoints: BTreeMap<(u64, Vec<u8>), FsCheckpoint>,
}

impl MemoryStore {
	/// Distinguishes an overlay tombstone from a row absent from this overlay.
	pub fn version(&self, key: &RowKey, as_of: Option<u64>) -> Option<(u64, &Option<Row>)> {
		self.versions
			.range((key.clone(), Reverse(as_of.unwrap_or(u64::MAX)))..)
			.next()
			.filter(|((found, _), _)| found == key)
			.map(|((_, Reverse(seq)), value)| (*seq, value))
	}

	pub fn write(&mut self, seq: u64, batch: Batch) -> Result<()> {
		// Conflicting replay is a corruption error and must leave the whole batch untouched.
		for (key, value) in &batch.rows {
			if self
				.versions
				.get(&(key.clone(), Reverse(seq)))
				.is_some_and(|old| old != value)
			{
				return Err(Error::Corrupt);
			}
		}
		for (key, value) in &batch.checkpoints {
			if self.checkpoints.get(key).is_some_and(|old| old != value) {
				return Err(Error::Corrupt);
			}
		}
		self.versions.extend(
			batch
				.rows
				.into_iter()
				.map(|(key, value)| ((key, Reverse(seq)), value)),
		);
		self.checkpoints.extend(batch.checkpoints);
		Ok(())
	}
}

impl RowStore for MemoryStore {
	fn get(&self, key: &RowKey, as_of: Option<u64>) -> Result<Option<Row>> {
		Ok(self
			.versions
			.range((key.clone(), Reverse(as_of.unwrap_or(u64::MAX)))..)
			.next()
			.filter(|((found, _), _)| found == key)
			.and_then(|(_, value)| value.clone()))
	}

	fn scan(
		&self,
		range: &RowRange,
		after: Option<&RowKey>,
		limit: usize,
		as_of: Option<u64>,
	) -> Result<Vec<(RowKey, Row)>> {
		let mut result = Vec::new();
		let mut previous = None;
		for ((key, Reverse(seq)), value) in &self.versions {
			if result.len() >= limit {
				break;
			}
			if *seq > as_of.unwrap_or(u64::MAX)
				|| after.is_some_and(|after| key <= after)
				|| !range.contains(key)
				|| previous == Some(key)
			{
				continue;
			}
			previous = Some(key);
			if let Some(value) = value {
				result.push((key.clone(), value.clone()));
			}
		}
		Ok(result)
	}
}

pub fn inode(store: &impl RowStore, ino: u64, as_of: Option<u64>) -> Result<FsInode> {
	match store.get(&RowKey::Inode(ino), as_of)? {
		Some(Row::Inode(value)) => Ok(value),
		None => Err(Error::NotFound),
		_ => Err(Error::Corrupt),
	}
}

/// No default on reads: a missing or unsupported persisted format is corruption.
pub fn format(store: &impl RowStore) -> Result<FsFormat> {
	match store.get(&RowKey::Format, None)? {
		Some(Row::Format(value)) if value.inline_max_bytes <= crate::CHUNK_BYTES as u64 => {
			Ok(value)
		}
		_ => Err(Error::Corrupt),
	}
}
