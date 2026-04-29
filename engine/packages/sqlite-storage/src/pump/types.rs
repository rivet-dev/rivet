use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use vbare::OwnedVersionedData;

pub const SQLITE_STORAGE_META_VERSION: u16 = 1;
pub const SQLITE_PAGE_SIZE: u32 = crate::keys::PAGE_SIZE;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DBHead {
	pub head_txid: u64,
	pub db_size_pages: u32,
	#[cfg(debug_assertions)]
	pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaCompact {
	pub materialized_txid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtyPage {
	pub pgno: u32,
	pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchedPage {
	pub pgno: u32,
	pub bytes: Option<Vec<u8>>,
}

enum VersionedDBHead {
	V1(DBHead),
}

impl OwnedVersionedData for VersionedDBHead {
	type Latest = DBHead;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V1(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid sqlite-storage DBHead version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

enum VersionedMetaCompact {
	V1(MetaCompact),
}

impl OwnedVersionedData for VersionedMetaCompact {
	type Latest = MetaCompact;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V1(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid sqlite-storage MetaCompact version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub fn encode_db_head(head: DBHead) -> Result<Vec<u8>> {
	VersionedDBHead::wrap_latest(head)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite db head")
}

pub fn decode_db_head(payload: &[u8]) -> Result<DBHead> {
	VersionedDBHead::deserialize_with_embedded_version(payload).context("decode sqlite db head")
}

pub fn encode_meta_compact(compact: MetaCompact) -> Result<Vec<u8>> {
	VersionedMetaCompact::wrap_latest(compact)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite compact meta")
}

pub fn decode_meta_compact(payload: &[u8]) -> Result<MetaCompact> {
	VersionedMetaCompact::deserialize_with_embedded_version(payload)
		.context("decode sqlite compact meta")
}

#[cfg(test)]
mod tests {
	use super::{
		DBHead, MetaCompact, SQLITE_STORAGE_META_VERSION, decode_db_head, decode_meta_compact,
		encode_db_head, encode_meta_compact,
	};

	#[test]
	fn db_head_round_trips_with_embedded_version() {
		let head = DBHead {
			head_txid: 42,
			db_size_pages: 128,
			#[cfg(debug_assertions)]
			generation: 7,
		};

		let encoded = encode_db_head(head.clone()).expect("db head should encode");
		assert_eq!(
			u16::from_le_bytes([encoded[0], encoded[1]]),
			SQLITE_STORAGE_META_VERSION
		);

		let decoded = decode_db_head(&encoded).expect("db head should decode");
		assert_eq!(decoded, head);
	}

	#[test]
	fn meta_compact_round_trips_with_embedded_version() {
		let compact = MetaCompact {
			materialized_txid: 24,
		};

		let encoded = encode_meta_compact(compact.clone()).expect("compact meta should encode");
		assert_eq!(
			u16::from_le_bytes([encoded[0], encoded[1]]),
			SQLITE_STORAGE_META_VERSION
		);

		let decoded = decode_meta_compact(&encoded).expect("compact meta should decode");
		assert_eq!(decoded, compact);
	}
}
