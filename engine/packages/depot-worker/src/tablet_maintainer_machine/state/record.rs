//! One volume record shared by the stored row and the filesystem registration.

use rivet_data::generated::shared_depot_fs_v1::FsVolumeRecord;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A volume record behind a shared pointer. A loaded volume's stored row and its filesystem
/// record are usually the same record, the one its last flush published, so both hold one
/// allocation; a record carries up to 64 write receipts, about 5 KiB. A change copies the
/// record first, so a holder never observes another's change. Equality, hashing and encoding
/// see only the record's contents.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SharedRecord(Arc<FsVolumeRecord>);

impl SharedRecord {
	pub fn new(record: FsVolumeRecord) -> Self {
		Self(Arc::new(record))
	}

	/// The record for changing, copied first when another holder shares it.
	pub fn make_mut(&mut self) -> &mut FsVolumeRecord {
		Arc::make_mut(&mut self.0)
	}

	/// An owned copy of the record, for effects and replies that take one.
	pub fn to_record(&self) -> FsVolumeRecord {
		FsVolumeRecord::clone(&self.0)
	}

	/// Whether both hold the same allocation.
	pub fn shares(&self, other: &SharedRecord) -> bool {
		Arc::ptr_eq(&self.0, &other.0)
	}
}

impl std::ops::Deref for SharedRecord {
	type Target = FsVolumeRecord;

	fn deref(&self) -> &FsVolumeRecord {
		&self.0
	}
}

impl From<FsVolumeRecord> for SharedRecord {
	fn from(record: FsVolumeRecord) -> Self {
		Self::new(record)
	}
}

impl Serialize for SharedRecord {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		self.0.serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for SharedRecord {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		FsVolumeRecord::deserialize(deserializer).map(Self::new)
	}
}
