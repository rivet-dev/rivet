//! Ordered index from partition key hash to the range that covers it.

use crate::{PartitionKeyHash, PartitionKeyHashRange};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Finds the range covering a hash in O(log n) by keeping ranges ordered by start.
///
/// A routing view's ranges never overlap, so the only candidate is the range with the
/// greatest start at or below the hash. A set that does overlap is not indexed, and every
/// lookup answers [`HashRangeLookup::Unindexed`] so the caller keeps its linear search and
/// its own tie-breaking order. Empty ranges cover no hash and are left out.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HashRangeIndex<K> {
	by_start: BTreeMap<PartitionKeyHash, (K, PartitionKeyHashRange)>,
	overlapping: bool,
}

/// The answer to one [`HashRangeIndex::lookup`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashRangeLookup<'a, K> {
	/// The indexed range that covers the hash.
	Covered(&'a K),
	/// No indexed range covers the hash.
	Uncovered,
	/// The ranges overlap, so the index cannot answer.
	Unindexed,
}

impl<K> Default for HashRangeIndex<K> {
	fn default() -> Self {
		Self {
			by_start: BTreeMap::new(),
			overlapping: false,
		}
	}
}

impl<K> HashRangeIndex<K> {
	pub fn new(ranges: impl IntoIterator<Item = (K, PartitionKeyHashRange)>) -> Self {
		let mut by_start = BTreeMap::new();
		let mut overlapping = false;
		for (key, range) in ranges {
			if !range.is_valid() {
				continue;
			}
			if by_start.insert(range.start, (key, range)).is_some() {
				overlapping = true;
			}
		}

		// Sorted by start, a set is disjoint exactly when each range ends at or before the
		// next one starts.
		let mut previous_end: Option<Option<PartitionKeyHash>> = None;
		for (start, (_, range)) in &by_start {
			if let Some(end) = previous_end {
				if end.is_none_or(|end| end > *start) {
					overlapping = true;
				}
			}
			previous_end = Some(range.end);
		}

		if overlapping {
			by_start.clear();
		}
		Self {
			by_start,
			overlapping,
		}
	}

	pub fn lookup(&self, hash: PartitionKeyHash) -> HashRangeLookup<'_, K> {
		if self.overlapping {
			return HashRangeLookup::Unindexed;
		}
		match self.by_start.range(..=hash).next_back() {
			Some((_, (key, range))) if range.contains(hash) => HashRangeLookup::Covered(key),
			Some(_) | None => HashRangeLookup::Uncovered,
		}
	}
}
