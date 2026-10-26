//! Loaded replicas ordered by partition-key hash.
//!
//! A tablet owns a hash range, so its replicas form one contiguous run in this order. Listing
//! pages and the listing's exclusion check read that run directly instead of scanning every
//! replica of every tablet on the shard.

use super::LogShardState;
use depot_types::{PartitionKeyHash, PartitionKeyHashRange, VolumeLogKey, partition_key_hash};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, ops::Bound};

/// A replica's position in hash order. `None` sorts before every key of its hash, so it seeks
/// to the first replica at a hash; stored entries always hold a key.
type Position = (PartitionKeyHash, Option<VolumeLogKey>);

/// Every key of `LogShardState::loaded_volume_replicas`, kept in step with it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ReplicaHashes(BTreeSet<Position>);

impl ReplicaHashes {
	/// Accounts one replica being created, replaced or removed.
	pub(crate) fn replaced(&mut self, key: &VolumeLogKey, loaded: bool) {
		let position = (partition_key_hash(&key.0.volume_key), Some(key.clone()));
		if loaded {
			self.0.insert(position);
		} else {
			self.0.remove(&position);
		}
	}

	/// Rebuilds the index from a recovered image.
	pub(crate) fn recovered<'a>(keys: impl Iterator<Item = &'a VolumeLogKey>) -> Self {
		let mut hashes = Self::default();
		for key in keys {
			hashes.replaced(key, true);
		}
		hashes
	}

	/// Replicas in `range` in listing order, starting after `after` when it is set.
	pub(crate) fn listing<'a>(
		&'a self,
		range: &'a PartitionKeyHashRange,
		after: Option<&VolumeLogKey>,
	) -> impl Iterator<Item = &'a VolumeLogKey> + 'a {
		self.positions(range, after)
			.filter_map(|(_, key)| key.as_ref())
	}

	/// The index positions a listing visits: it seeks to the range or cursor and stops at the
	/// range's end, so it never visits a replica of another range.
	fn positions<'a>(
		&'a self,
		range: &'a PartitionKeyHashRange,
		after: Option<&VolumeLogKey>,
	) -> impl Iterator<Item = &'a Position> + 'a {
		let start = match after {
			Some(after) => {
				let hash = partition_key_hash(&after.0.volume_key);
				if hash < range.start {
					Bound::Included((range.start, None))
				} else {
					Bound::Excluded((hash, Some(after.clone())))
				}
			}
			None => Bound::Included((range.start, None)),
		};
		self.0
			.range::<Position, _>((start, Bound::Unbounded))
			.take_while(|(hash, _)| range.end.is_none_or(|end| *hash < end))
	}

	/// The first replica whose hash is at least `hash` and inside `range`.
	fn first_from(
		&self,
		hash: PartitionKeyHash,
		range: &PartitionKeyHashRange,
	) -> Option<&VolumeLogKey> {
		self.0
			.range::<Position, _>((Bound::Included((hash, None)), Bound::Unbounded))
			.next()
			.filter(|(found, _)| range.end.is_none_or(|end| *found < end))
			.and_then(|(_, key)| key.as_ref())
	}
}

/// Whether any loaded replica in `range` is excluded from this worker. Exclusion follows the
/// covering tablet, so one replica per tablet decides it and the check skips to the tablet's
/// end. It costs one lookup per tablet that holds replicas in the range.
pub(crate) fn any_excluded(state: &LogShardState, range: &PartitionKeyHashRange) -> bool {
	excluded_lookups(state, range).0
}

/// `any_excluded` with the number of index lookups it made.
fn excluded_lookups(state: &LogShardState, range: &PartitionKeyHashRange) -> (bool, usize) {
	if state.tablet_manifest.tablet_manifest_version == 0 {
		return (false, 0);
	}
	let mut lookups = 0;
	let mut from = range.start;
	loop {
		lookups += 1;
		let Some(key) = state.replica_hashes.first_from(from, range) else {
			return (false, lookups);
		};
		if super::retirement::excluded(state, key) {
			return (true, lookups);
		}
		// A replica that is not excluded has a covering tablet.
		let hash = partition_key_hash(&key.0.volume_key);
		let Some(tablet) = super::retirement::covering_tablet(state, hash) else {
			return (true, lookups);
		};
		match tablet.hash_range.end {
			Some(end) if end > from => from = end,
			Some(_) | None => return (false, lookups),
		}
	}
}


