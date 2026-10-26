use std::{borrow::Borrow, ops::Deref};

use bytes::Bytes;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Shared immutable bytes for one SQLite page version.
///
/// Adopting an owned `Vec<u8>` is zero-copy, and cloning this value only increments the backing
/// buffer's reference count. Converting back to `Vec<u8>` is intentionally explicit because a
/// generated protocol or storage boundary may require a new owned contiguous buffer.
#[derive(Clone, Default, Eq, Hash, PartialEq)]
pub struct PageBytes(Bytes);

impl PageBytes {
	pub fn copy_from_slice(bytes: &[u8]) -> Self {
		Self(Bytes::copy_from_slice(bytes))
	}

	pub fn as_slice(&self) -> &[u8] {
		self.0.as_ref()
	}

	pub fn len(&self) -> usize {
		self.0.len()
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	pub fn to_vec(&self) -> Vec<u8> {
		self.0.to_vec()
	}

	/// Returns mutable owned storage, reusing the page allocation when this is its only owner.
	pub fn into_vec(self) -> Vec<u8> {
		self.0.into()
	}

	/// Returns the shared buffer without copying, for protocol messages that hold `Bytes`.
	pub fn into_bytes(self) -> Bytes {
		self.0
	}
}

impl std::fmt::Debug for PageBytes {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter
			.debug_tuple("PageBytes")
			.field(&self.as_slice())
			.finish()
	}
}

impl From<Vec<u8>> for PageBytes {
	fn from(bytes: Vec<u8>) -> Self {
		Self(Bytes::from(bytes))
	}
}

impl From<Bytes> for PageBytes {
	fn from(bytes: Bytes) -> Self {
		Self(bytes)
	}
}

impl From<&[u8]> for PageBytes {
	fn from(bytes: &[u8]) -> Self {
		Self::copy_from_slice(bytes)
	}
}

impl AsRef<[u8]> for PageBytes {
	fn as_ref(&self) -> &[u8] {
		self.as_slice()
	}
}

impl Borrow<[u8]> for PageBytes {
	fn borrow(&self) -> &[u8] {
		self.as_slice()
	}
}

impl Deref for PageBytes {
	type Target = [u8];

	fn deref(&self) -> &Self::Target {
		self.as_slice()
	}
}

impl Serialize for PageBytes {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_slice().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for PageBytes {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		Vec::<u8>::deserialize(deserializer).map(Self::from)
	}
}


