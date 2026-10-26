//! Deadline-ordered key queues that state machine cores hold in their state.
//!
//! Queue entries are check times, not authority. A core pops due keys, re-derives each
//! key's real deadline from its own authoritative fields, and reschedules keys that are
//! not due yet. Both queues are plain ordered collections, so they are deterministic,
//! hashable and serializable like the rest of a core's state.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Keys ordered by `(at, key)`, for callers that already store each key's scheduled time on
/// a per-key record.
///
/// The caller's record is the one authority for the scheduled time and passes it back to
/// `replace`, `schedule_earliest` and `cancel`, so the queue keeps a single copy of each key.
/// Passing a time that does not match the queued entry leaves that entry in place.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeadlineQueue<K: Ord> {
	entries: BTreeSet<(u64, K)>,
}

impl<K: Ord> Default for DeadlineQueue<K> {
	fn default() -> Self {
		Self {
			entries: BTreeSet::new(),
		}
	}
}

impl<K: Ord + Clone> DeadlineQueue<K> {
	pub fn new() -> Self {
		Self::default()
	}

	/// Queues `key` at `at`. The caller must not have it queued at another time.
	pub fn schedule(&mut self, key: K, at: u64) {
		self.entries.insert((at, key));
	}

	/// Moves `key` from `old_at`, if it was queued, to `new_at`.
	pub fn replace(&mut self, key: K, old_at: Option<u64>, new_at: u64) {
		if let Some(old_at) = old_at {
			self.entries.remove(&(old_at, key.clone()));
		}
		self.entries.insert((new_at, key));
	}

	/// Keeps the earlier of the queued time and `at`, and returns the time that remains
	/// queued, which the caller stores on its record.
	pub fn schedule_earliest(&mut self, key: K, old_at: Option<u64>, at: u64) -> u64 {
		match old_at {
			Some(old_at) if old_at <= at => old_at,
			Some(_) | None => {
				self.replace(key, old_at, at);
				at
			}
		}
	}

	/// Removes `key` queued at `at`, returning whether it was queued.
	pub fn cancel(&mut self, key: &K, at: u64) -> bool {
		self.entries.remove(&(at, key.clone()))
	}

	pub fn contains(&self, key: &K, at: u64) -> bool {
		self.entries.contains(&(at, key.clone()))
	}

	/// The earliest queued time, which is where the owner arms its head timer.
	pub fn next_at(&self) -> Option<u64> {
		self.entries.first().map(|(at, _)| *at)
	}

	/// Removes and returns up to `limit` keys due at `now`, in `(at, key)` order. Due keys
	/// past the limit stay queued, so the owner's re-armed head timer fires again at once.
	pub fn pop_due(&mut self, now: u64, limit: usize) -> Vec<(u64, K)> {
		let mut due = Vec::new();
		while due.len() < limit {
			let Some(entry) = self.entries.first() else {
				break;
			};
			if entry.0 > now {
				break;
			}
			let entry = self.entries.pop_first().expect("first entry exists");
			due.push(entry);
		}
		due
	}

	pub fn len(&self) -> usize {
		self.entries.len()
	}

	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}

	/// Queued entries in `(at, key)` order, for debug validation against owner records.
	pub fn iter(&self) -> impl DoubleEndedIterator<Item = &(u64, K)> {
		self.entries.iter()
	}
}

/// A [`DeadlineQueue`] that also indexes each key's scheduled time, for callers with no
/// per-key record to hold it. Each key is queued at most once.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IndexedDeadlineQueue<K: Ord> {
	queue: DeadlineQueue<K>,
	scheduled_at: BTreeMap<K, u64>,
}

impl<K: Ord> Default for IndexedDeadlineQueue<K> {
	fn default() -> Self {
		Self {
			queue: DeadlineQueue::default(),
			scheduled_at: BTreeMap::new(),
		}
	}
}

impl<K: Ord + Clone> IndexedDeadlineQueue<K> {
	pub fn new() -> Self {
		Self::default()
	}

	/// Queues `key` at `at`, replacing any earlier or later queued time.
	pub fn schedule(&mut self, key: K, at: u64) {
		let old_at = self.scheduled_at.insert(key.clone(), at);
		self.queue.replace(key, old_at, at);
	}

	/// Keeps the earlier of the queued time and `at`, and returns the time that remains.
	pub fn schedule_earliest(&mut self, key: K, at: u64) -> u64 {
		let old_at = self.scheduled_at.get(&key).copied();
		let at = self.queue.schedule_earliest(key.clone(), old_at, at);
		self.scheduled_at.insert(key, at);
		at
	}

	/// Removes `key`, returning the time it was queued at.
	pub fn cancel(&mut self, key: &K) -> Option<u64> {
		let at = self.scheduled_at.remove(key)?;
		self.queue.cancel(key, at);
		Some(at)
	}

	pub fn scheduled_at(&self, key: &K) -> Option<u64> {
		self.scheduled_at.get(key).copied()
	}

	pub fn next_at(&self) -> Option<u64> {
		self.queue.next_at()
	}

	/// Removes and returns up to `limit` due keys in `(at, key)` order.
	pub fn pop_due(&mut self, now: u64, limit: usize) -> Vec<(u64, K)> {
		let due = self.queue.pop_due(now, limit);
		for (_, key) in &due {
			self.scheduled_at.remove(key);
		}
		due
	}

	pub fn len(&self) -> usize {
		self.scheduled_at.len()
	}

	pub fn is_empty(&self) -> bool {
		self.scheduled_at.is_empty()
	}

	pub fn iter(&self) -> impl DoubleEndedIterator<Item = &(u64, K)> {
		self.queue.iter()
	}

	/// Checks that the index and the deadline order hold exactly the same entries.
	pub fn debug_validate(&self) -> anyhow::Result<()> {
		anyhow::ensure!(
			self.queue.len() == self.scheduled_at.len(),
			"deadline index holds {} keys but the queue holds {}",
			self.scheduled_at.len(),
			self.queue.len()
		);
		for (key, at) in &self.scheduled_at {
			anyhow::ensure!(
				self.queue.contains(key, *at),
				"indexed deadline missing from the queue"
			);
		}
		Ok(())
	}
}
