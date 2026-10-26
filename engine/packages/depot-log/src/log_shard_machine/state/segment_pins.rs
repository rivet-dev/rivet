//! Counts of retained replica entries per disk batch. Segment collection asks whether
//! any retained entry lives in a segment's batch range; keeping these counts current as
//! entries gain a batch location or leave a replica answers that per segment instead of
//! scanning every entry of every loaded replica on each event.

use super::loaded_volume_replica::{LoadedVolumeReplica, ReplicaEntry};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// Retained entries by the log sequence of the batch that holds them on disk. Entries
/// still waiting for their batch have no location and pin nothing here.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SegmentPins {
	entries_by_batch_log_seq: BTreeMap<u64, u64>,
}

impl SegmentPins {
	/// Counts every located entry of the given replicas, for recovery and validation.
	pub fn recompute(replicas: &super::replica_table::ReplicaTable) -> Self {
		let mut pins = Self::default();
		for replica in replicas.values() {
			pins.add_replica(replica);
		}
		pins
	}

	/// Records that one retained entry now lives in `batch_log_seq`.
	pub fn add(&mut self, batch_log_seq: u64) {
		*self
			.entries_by_batch_log_seq
			.entry(batch_log_seq)
			.or_insert(0) += 1;
	}

	/// Replaces one replica's entries with another version of the same replica. Either
	/// side may be absent for a replica that is created or removed.
	pub fn replace(
		&mut self,
		old: Option<&LoadedVolumeReplica>,
		new: Option<&LoadedVolumeReplica>,
	) {
		match (old, new) {
			(Some(old), Some(new)) => self.replace_entries(&old.entries, &new.entries),
			(Some(old), None) => self.remove_replica(old),
			(None, Some(new)) => self.add_replica(new),
			(None, None) => {}
		}
	}

	/// Applies only the difference between two versions of one replica's entries, so an
	/// append costs time for what it changed rather than for every retained entry.
	///
	/// A version changes its entries only by trimming a prefix, dropping a suffix and
	/// appending, so entries are contiguous by volume sequence and only a suffix of the
	/// range both versions hold can differ. Walking that range down from its end, the first
	/// entry at the same log position and the same disk batch in both versions ends the
	/// walk: log positions identify entries, so it and every entry below it are the same
	/// entries in the same batches. Entries without a disk batch never end the walk. Versions
	/// whose entries are not contiguous are recounted in full.
	fn replace_entries(&mut self, old: &VecDeque<ReplicaEntry>, new: &VecDeque<ReplicaEntry>) {
		let (Some(old_first), Some(new_first)) = (contiguous_first(old), contiguous_first(new))
		else {
			self.remove_entries(old.iter());
			self.add_entries(new.iter());
			return;
		};
		let old_last = old_first + old.len() as u64 - 1;
		let new_last = new_first + new.len() as u64 - 1;
		let shared_first = old_first.max(new_first);
		let shared_last = old_last.min(new_last);
		if shared_first > shared_last {
			self.remove_entries(old.iter());
			self.add_entries(new.iter());
			return;
		}

		// Entries outside the shared range belong to one version only.
		self.remove_entries(outside(old, old_first, shared_first, shared_last));
		self.add_entries(outside(new, new_first, shared_first, shared_last));

		// Within the shared range, only a suffix can differ.
		for volume_seq in (shared_first..=shared_last).rev() {
			let old_entry = &old[(volume_seq - old_first) as usize];
			let new_entry = &new[(volume_seq - new_first) as usize];
			if old_entry.client_epoch == new_entry.client_epoch
				&& old_entry.batch_log_seq.is_some()
				&& old_entry.batch_log_seq == new_entry.batch_log_seq
			{
				break;
			}
			self.remove_entries(std::iter::once(old_entry));
			self.add_entries(std::iter::once(new_entry));
		}
	}

	/// The lowest batch sequence any retained entry lives in.
	pub fn oldest(&self) -> Option<u64> {
		self.entries_by_batch_log_seq.keys().next().copied()
	}

	/// Whether any retained entry lives in a batch at or above `first` and below `end`,
	/// with no upper bound when `end` is none.
	pub fn any_in(&self, first: u64, end: Option<u64>) -> bool {
		match end {
			Some(end) if end <= first => false,
			Some(end) => self
				.entries_by_batch_log_seq
				.range(first..end)
				.next()
				.is_some(),
			None => self
				.entries_by_batch_log_seq
				.range(first..)
				.next()
				.is_some(),
		}
	}

	fn add_replica(&mut self, replica: &LoadedVolumeReplica) {
		self.add_entries(replica.entries.iter());
	}

	fn remove_replica(&mut self, replica: &LoadedVolumeReplica) {
		self.remove_entries(replica.entries.iter());
	}

	fn add_entries<'a>(&mut self, entries: impl Iterator<Item = &'a ReplicaEntry>) {
		for batch_log_seq in entries.filter_map(|entry| entry.batch_log_seq) {
			self.add(batch_log_seq);
		}
	}

	fn remove_entries<'a>(&mut self, entries: impl Iterator<Item = &'a ReplicaEntry>) {
		for batch_log_seq in entries.filter_map(|entry| entry.batch_log_seq) {
			if let Some(count) = self.entries_by_batch_log_seq.get_mut(&batch_log_seq) {
				*count -= 1;
				if *count == 0 {
					self.entries_by_batch_log_seq.remove(&batch_log_seq);
				}
			}
		}
	}
}

/// Contiguous entries starting at `first` whose volume sequence is outside
/// `shared_first..=shared_last`, which lies within them.
fn outside(
	entries: &VecDeque<ReplicaEntry>,
	first: u64,
	shared_first: u64,
	shared_last: u64,
) -> impl Iterator<Item = &ReplicaEntry> {
	let below = (shared_first - first) as usize;
	let above = (shared_last - first) as usize + 1;
	entries.range(..below).chain(entries.range(above..))
}

/// The volume sequence of the first entry when the entries are contiguous by volume
/// sequence, which is how a replica keeps them; none when empty or not contiguous.
fn contiguous_first(entries: &VecDeque<ReplicaEntry>) -> Option<u64> {
	let first = entries.front()?.volume_seq;
	let last = entries.back()?.volume_seq;
	if last.checked_sub(first)? + 1 == entries.len() as u64 {
		Some(first)
	} else {
		None
	}
}
