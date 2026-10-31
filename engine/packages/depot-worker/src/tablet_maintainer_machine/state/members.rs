//! A loaded volume's member replica states, keyed by worker.

use rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState;
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};

/// A loaded volume's member states in worker order, with the map operations the maintainer
/// uses. A tablet has a handful of members, so a sorted slice sized exactly to them replaces
/// a `BTreeMap`, whose smallest node holds eleven entries: with three members this cuts the
/// allocation from about 1.3 KiB to 360 bytes per loaded volume. Iteration order, equality,
/// hashing and encoding match the map's.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Members(Box<[(WorkerId, VolumeLogReplicaState)]>);

impl Members {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn len(&self) -> usize {
		self.0.len()
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	fn position(&self, worker: &WorkerId) -> Result<usize, usize> {
		self.0.binary_search_by(|(member, _)| member.cmp(worker))
	}

	pub fn get(&self, worker: &WorkerId) -> Option<&VolumeLogReplicaState> {
		self.position(worker).ok().map(|index| &self.0[index].1)
	}

	pub fn get_mut(&mut self, worker: &WorkerId) -> Option<&mut VolumeLogReplicaState> {
		self.position(worker).ok().map(|index| &mut self.0[index].1)
	}

	pub fn contains_key(&self, worker: &WorkerId) -> bool {
		self.position(worker).is_ok()
	}

	/// Sets `worker`'s state and returns the state it replaced.
	pub fn insert(
		&mut self,
		worker: WorkerId,
		state: VolumeLogReplicaState,
	) -> Option<VolumeLogReplicaState> {
		match self.position(&worker) {
			Ok(index) => Some(std::mem::replace(&mut self.0[index].1, state)),
			Err(index) => {
				let mut entries = std::mem::take(&mut self.0).into_vec();
				entries.insert(index, (worker, state));
				self.0 = entries.into_boxed_slice();
				None
			}
		}
	}

	/// Returns `worker`'s state, inserting `state` first if the worker has none.
	pub fn get_or_insert(
		&mut self,
		worker: WorkerId,
		state: VolumeLogReplicaState,
	) -> &mut VolumeLogReplicaState {
		let index = match self.position(&worker) {
			Ok(index) => index,
			Err(index) => {
				let mut entries = std::mem::take(&mut self.0).into_vec();
				entries.insert(index, (worker, state));
				self.0 = entries.into_boxed_slice();
				index
			}
		};
		&mut self.0[index].1
	}

	pub fn remove(&mut self, worker: &WorkerId) -> Option<VolumeLogReplicaState> {
		let index = self.position(worker).ok()?;
		let mut entries = std::mem::take(&mut self.0).into_vec();
		let (_, state) = entries.remove(index);
		self.0 = entries.into_boxed_slice();
		Some(state)
	}

	pub fn iter(
		&self,
	) -> impl DoubleEndedIterator<Item = (&WorkerId, &VolumeLogReplicaState)> + Clone {
		self.0.iter().map(|(worker, state)| (worker, state))
	}

	/// Picks the replica to read from among the members that pass `eligible`. The member on
	/// `local`, the maintainer's own worker, wins when it is eligible, so the read stays in
	/// process; otherwise `fallback` chooses among the eligible members in worker order. Any
	/// eligible replica serves the same entries, so the choice only changes where bytes travel.
	pub fn read_source<'a>(
		&'a self,
		local: Option<&WorkerId>,
		eligible: impl Fn(&WorkerId, &VolumeLogReplicaState) -> bool,
		fallback: impl FnOnce(
			&mut dyn Iterator<Item = (&'a WorkerId, &'a VolumeLogReplicaState)>,
		) -> Option<(&'a WorkerId, &'a VolumeLogReplicaState)>,
	) -> Option<(&'a WorkerId, &'a VolumeLogReplicaState)> {
		if let Some(index) = local.and_then(|local| self.position(local).ok()) {
			let (worker, state) = &self.0[index];
			if eligible(worker, state) {
				return Some((worker, state));
			}
		}
		let mut candidates = self
			.iter()
			.filter(|(worker, state)| eligible(worker, state));
		fallback(&mut candidates)
	}

	pub fn iter_mut(
		&mut self,
	) -> impl DoubleEndedIterator<Item = (&WorkerId, &mut VolumeLogReplicaState)> {
		self.0.iter_mut().map(|(worker, state)| (&*worker, state))
	}

	pub fn keys(&self) -> impl DoubleEndedIterator<Item = &WorkerId> + Clone {
		self.0.iter().map(|(worker, _)| worker)
	}

	pub fn values(&self) -> impl DoubleEndedIterator<Item = &VolumeLogReplicaState> + Clone {
		self.0.iter().map(|(_, state)| state)
	}

	pub fn values_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut VolumeLogReplicaState> {
		self.0.iter_mut().map(|(_, state)| state)
	}
}

impl Members {
	/// Keeps only the members `keep` accepts.
	pub fn retain(&mut self, mut keep: impl FnMut(&WorkerId, &mut VolumeLogReplicaState) -> bool) {
		let mut entries = std::mem::take(&mut self.0).into_vec();
		entries.retain_mut(|(worker, state)| keep(worker, state));
		self.0 = entries.into_boxed_slice();
	}
}

impl<const N: usize> From<[(WorkerId, VolumeLogReplicaState); N]> for Members {
	fn from(entries: [(WorkerId, VolumeLogReplicaState); N]) -> Self {
		entries.into_iter().collect()
	}
}

impl std::ops::Index<&WorkerId> for Members {
	type Output = VolumeLogReplicaState;

	fn index(&self, worker: &WorkerId) -> &VolumeLogReplicaState {
		self.get(worker).expect("member of the loaded volume")
	}
}

impl FromIterator<(WorkerId, VolumeLogReplicaState)> for Members {
	fn from_iter<I: IntoIterator<Item = (WorkerId, VolumeLogReplicaState)>>(entries: I) -> Self {
		let mut members = Self::default();
		members.extend(entries);
		members
	}
}

impl Extend<(WorkerId, VolumeLogReplicaState)> for Members {
	fn extend<I: IntoIterator<Item = (WorkerId, VolumeLogReplicaState)>>(&mut self, entries: I) {
		let mut sorted = std::mem::take(&mut self.0).into_vec();
		for (worker, state) in entries {
			match sorted.binary_search_by(|(member, _)| member.cmp(&worker)) {
				Ok(index) => sorted[index].1 = state,
				Err(index) => sorted.insert(index, (worker, state)),
			}
		}
		self.0 = sorted.into_boxed_slice();
	}
}

impl<'a> IntoIterator for &'a Members {
	type Item = (&'a WorkerId, &'a VolumeLogReplicaState);
	type IntoIter = std::iter::Map<
		std::slice::Iter<'a, (WorkerId, VolumeLogReplicaState)>,
		fn(&'a (WorkerId, VolumeLogReplicaState)) -> (&'a WorkerId, &'a VolumeLogReplicaState),
	>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.iter().map(|(worker, state)| (worker, state))
	}
}

impl IntoIterator for Members {
	type Item = (WorkerId, VolumeLogReplicaState);
	type IntoIter = std::vec::IntoIter<(WorkerId, VolumeLogReplicaState)>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_vec().into_iter()
	}
}

impl Serialize for Members {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.collect_map(self.iter())
	}
}

impl<'de> Deserialize<'de> for Members {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let map = std::collections::BTreeMap::<WorkerId, VolumeLogReplicaState>::deserialize(
			deserializer,
		)?;
		Ok(map.into_iter().collect())
	}
}


