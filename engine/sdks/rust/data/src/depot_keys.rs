//! Domain helpers for generated Depot identities; their wire encoding stays generated.

use crate::generated::shared_depot_types_v1::{PartitionKeyHash, PartitionKeyHashRange, VolumeKey};
use std::cmp::Ordering;

impl VolumeKey {
	pub fn unnamed(partition_key: impl Into<Vec<u8>>) -> Self {
		let partition_key: Vec<u8> = partition_key.into();
		Self {
			partition_key: partition_key.into(),
			name: None,
		}
	}

	pub fn named(partition_key: impl Into<Vec<u8>>, name: impl Into<Vec<u8>>) -> Self {
		let partition_key: Vec<u8> = partition_key.into();
		let name: Vec<u8> = name.into();
		Self {
			partition_key: partition_key.into(),
			name: Some(name.into()),
		}
	}

	/// Application bytes counted against the key size limit, including the optional name.
	pub fn byte_len(&self) -> usize {
		self.partition_key
			.len()
			.saturating_add(self.name.as_ref().map_or(0, |name| name.len()))
	}
}

impl Ord for VolumeKey {
	fn cmp(&self, other: &Self) -> Ordering {
		self.partition_key
			.cmp(&other.partition_key)
			.then_with(|| self.name.cmp(&other.name))
	}
}

impl PartialOrd for VolumeKey {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Copy for PartitionKeyHashRange {}

impl PartitionKeyHashRange {
	pub const fn full() -> Self {
		Self {
			start: [0; 16],
			end: None,
		}
	}

	pub fn is_valid(&self) -> bool {
		self.end.is_none_or(|end| self.start < end)
	}

	pub fn contains(&self, hash: PartitionKeyHash) -> bool {
		self.start <= hash && self.end.is_none_or(|end| hash < end)
	}

	pub fn overlaps(&self, other: &Self) -> bool {
		self.is_valid()
			&& other.is_valid()
			&& other.end.is_none_or(|end| self.start < end)
			&& self.end.is_none_or(|end| other.start < end)
	}

	/// An interior boundary near the middle, including for the full hash space. It is a split
	/// boundary, so it is aligned to 64 bits; a range too narrow to hold an aligned interior
	/// boundary cannot split.
	pub fn midpoint(&self) -> Option<PartitionKeyHash> {
		if !self.is_valid() {
			return None;
		}
		let start = u128::from_be_bytes(self.start);
		let offset = match self.end {
			Some(end) => (u128::from_be_bytes(end) - start) / 2,
			None => (u128::MAX - start) / 2 + 1,
		};
		let midpoint = start.checked_add(offset)? & !u128::from(u64::MAX);
		let midpoint = midpoint.to_be_bytes();
		(self.start < midpoint && self.contains(midpoint)).then_some(midpoint)
	}

	/// Whether `boundary` may split a tablet: its low 64 bits are zero. Placed keys put a
	/// related group of volumes in one 2^64-wide placement range, so an aligned boundary never
	/// separates them.
	pub fn is_split_aligned(boundary: PartitionKeyHash) -> bool {
		u128::from_be_bytes(boundary) & u128::from(u64::MAX) == 0
	}

	/// Both children preserve exactly the parent interval and share no hashes.
	pub fn split(&self, boundary: PartitionKeyHash) -> Option<(Self, Self)> {
		(self.start < boundary && self.contains(boundary)).then_some((
			Self {
				start: self.start,
				end: Some(boundary),
			},
			Self {
				start: boundary,
				end: self.end,
			},
		))
	}
}

impl crate::generated::shared_depot_fs_v1::FsVolumeRecord {
	pub fn volume_key(&self) -> VolumeKey {
		VolumeKey {
			partition_key: self.partition_key.clone(),
			name: self.name.clone(),
		}
	}
}

impl crate::generated::shared_depot_fs_read_v1::FsReadRequest {
	pub fn volume_key(&self) -> &VolumeKey {
		use crate::generated::shared_depot_fs_read_v1::FsReadTarget;
		match &self.target {
			FsReadTarget::VolumeKey(key) => key,
			FsReadTarget::VolumeLogKey(key) => &key.volume_key,
		}
	}
	pub fn volume_log_key(&self) -> Option<&crate::generated::shared_depot_types_v1::VolumeLogKey> {
		match &self.target {
			crate::generated::shared_depot_fs_read_v1::FsReadTarget::VolumeLogKey(key) => Some(key),
			_ => None,
		}
	}
}

impl crate::generated::depot_client_protocol_v1::FsForkReferenceRequest {
	pub fn spec(&self) -> &crate::generated::shared_depot_fs_v1::FsForkSpec {
		use crate::generated::depot_client_protocol_v1::FsForkReferenceRequest::*;
		match self {
			FsForkAcquire(v) => &v.spec,
			FsForkRelease(v) => &v.spec,
			FsForkCheck(v) => &v.spec,
		}
	}
	pub fn volume_key(&self) -> &crate::generated::shared_depot_types_v1::VolumeKey {
		match self {
			Self::FsForkCheck(v) => &v.spec.destination_volume_key,
			_ => &self.spec().parent_volume_key,
		}
	}
	pub fn region(&self) -> u16 {
		match self {
			Self::FsForkCheck(v) => v.spec.destination_region,
			_ => self.spec().parent_region,
		}
	}
}
