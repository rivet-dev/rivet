//! Core storage types for the SQLite VFS v2 engine implementation.

use serde::{Deserialize, Serialize};

pub const SQLITE_VFS_V2_SCHEMA_VERSION: u32 = 2;
pub const SQLITE_PAGE_SIZE: u32 = 4096;
pub const SQLITE_SHARD_SIZE: u32 = 64;
pub const SQLITE_MAX_DELTA_BYTES: u64 = 8 * 1024 * 1024;
pub const SQLITE_DEFAULT_MAX_STORAGE_BYTES: u64 = 10 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DBHead {
	pub schema_version: u32,
	pub generation: u64,
	pub head_txid: u64,
	pub next_txid: u64,
	pub materialized_txid: u64,
	pub db_size_pages: u32,
	pub page_size: u32,
	pub shard_size: u32,
	pub creation_ts_ms: i64,
	pub sqlite_storage_used: u64,
	pub sqlite_max_storage: u64,
}

impl DBHead {
	pub fn new(creation_ts_ms: i64) -> Self {
		Self {
			schema_version: SQLITE_VFS_V2_SCHEMA_VERSION,
			generation: 1,
			head_txid: 0,
			next_txid: 1,
			materialized_txid: 0,
			db_size_pages: 0,
			page_size: SQLITE_PAGE_SIZE,
			shard_size: SQLITE_SHARD_SIZE,
			creation_ts_ms,
			sqlite_storage_used: 0,
			sqlite_max_storage: SQLITE_DEFAULT_MAX_STORAGE_BYTES,
		}
	}
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqliteMeta {
	pub schema_version: u32,
	pub generation: u64,
	pub head_txid: u64,
	pub materialized_txid: u64,
	pub db_size_pages: u32,
	pub page_size: u32,
	pub creation_ts_ms: i64,
	pub max_delta_bytes: u64,
	pub sqlite_storage_used: u64,
	pub sqlite_max_storage: u64,
}

impl From<(DBHead, u64)> for SqliteMeta {
	fn from((head, max_delta_bytes): (DBHead, u64)) -> Self {
		Self {
			schema_version: head.schema_version,
			generation: head.generation,
			head_txid: head.head_txid,
			materialized_txid: head.materialized_txid,
			db_size_pages: head.db_size_pages,
			page_size: head.page_size,
			creation_ts_ms: head.creation_ts_ms,
			max_delta_bytes,
			sqlite_storage_used: head.sqlite_storage_used,
			sqlite_max_storage: head.sqlite_max_storage,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{
		DBHead, DirtyPage, FetchedPage, SQLITE_DEFAULT_MAX_STORAGE_BYTES, SQLITE_MAX_DELTA_BYTES,
		SQLITE_PAGE_SIZE, SQLITE_SHARD_SIZE, SQLITE_VFS_V2_SCHEMA_VERSION, SqliteMeta,
	};

	#[test]
	fn db_head_new_uses_spec_defaults() {
		let head = DBHead::new(1_713_456_789_000);

		assert_eq!(head.schema_version, SQLITE_VFS_V2_SCHEMA_VERSION);
		assert_eq!(head.generation, 1);
		assert_eq!(head.head_txid, 0);
		assert_eq!(head.next_txid, 1);
		assert_eq!(head.materialized_txid, 0);
		assert_eq!(head.db_size_pages, 0);
		assert_eq!(head.page_size, SQLITE_PAGE_SIZE);
		assert_eq!(head.shard_size, SQLITE_SHARD_SIZE);
		assert_eq!(head.creation_ts_ms, 1_713_456_789_000);
		assert_eq!(head.sqlite_storage_used, 0);
		assert_eq!(head.sqlite_max_storage, SQLITE_DEFAULT_MAX_STORAGE_BYTES);
	}

	#[test]
	fn db_head_round_trips_with_serde_bare() {
		let head = DBHead {
			schema_version: SQLITE_VFS_V2_SCHEMA_VERSION,
			generation: 7,
			head_txid: 9,
			next_txid: 10,
			materialized_txid: 5,
			db_size_pages: 321,
			page_size: SQLITE_PAGE_SIZE,
			shard_size: SQLITE_SHARD_SIZE,
			creation_ts_ms: 1_713_456_789_000,
			sqlite_storage_used: 8_192,
			sqlite_max_storage: SQLITE_DEFAULT_MAX_STORAGE_BYTES,
		};

		let encoded = serde_bare::to_vec(&head).expect("db head should serialize");
		let decoded: DBHead = serde_bare::from_slice(&encoded).expect("db head should deserialize");

		assert_eq!(decoded, head);
	}

	#[test]
	fn sqlite_meta_copies_runtime_fields_from_db_head() {
		let meta = SqliteMeta::from((
			DBHead {
				schema_version: SQLITE_VFS_V2_SCHEMA_VERSION,
				generation: 4,
				head_txid: 12,
				next_txid: 13,
				materialized_txid: 8,
				db_size_pages: 99,
				page_size: SQLITE_PAGE_SIZE,
				shard_size: SQLITE_SHARD_SIZE,
				creation_ts_ms: 456,
				sqlite_storage_used: 16_384,
				sqlite_max_storage: SQLITE_DEFAULT_MAX_STORAGE_BYTES / 2,
			},
			SQLITE_MAX_DELTA_BYTES,
		));

		assert_eq!(
			meta,
			SqliteMeta {
				schema_version: SQLITE_VFS_V2_SCHEMA_VERSION,
				generation: 4,
				head_txid: 12,
				materialized_txid: 8,
				db_size_pages: 99,
				page_size: SQLITE_PAGE_SIZE,
				creation_ts_ms: 456,
				max_delta_bytes: SQLITE_MAX_DELTA_BYTES,
				sqlite_storage_used: 16_384,
				sqlite_max_storage: SQLITE_DEFAULT_MAX_STORAGE_BYTES / 2,
			}
		);
	}

	#[test]
	fn page_types_preserve_payloads() {
		let dirty = DirtyPage {
			pgno: 17,
			bytes: vec![1, 2, 3, 4],
		};
		let fetched = FetchedPage {
			pgno: 18,
			bytes: Some(vec![5, 6, 7, 8]),
		};
		let missing = FetchedPage {
			pgno: 19,
			bytes: None,
		};

		assert_eq!(dirty.pgno, 17);
		assert_eq!(dirty.bytes, vec![1, 2, 3, 4]);
		assert_eq!(fetched.pgno, 18);
		assert_eq!(fetched.bytes, Some(vec![5, 6, 7, 8]));
		assert_eq!(missing.bytes, None);
	}
}
