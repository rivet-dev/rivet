//! Core storage types for the SQLite VFS v2 engine implementation.
//!
//! `DBHead` and `SqliteOrigin` are owned by `rivet-sqlite-storage-protocol`
//! (BARE-schema generated, vbare-versioned). Everything else here is process-
//! local — `DirtyPage`, `FetchedPage`, and `SqliteMeta` never hit disk so they
//! stay in-crate with whatever derive set is convenient.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

pub use rivet_sqlite_storage_protocol::{DBHead, PreloadHintRange, PreloadHints, SqliteOrigin};
use rivet_sqlite_storage_protocol::versioned;

pub const SQLITE_VFS_V2_SCHEMA_VERSION: u32 = 2;
pub const SQLITE_PAGE_SIZE: u32 = 4096;
pub const SQLITE_SHARD_SIZE: u32 = 64;
pub const SQLITE_MAX_DELTA_BYTES: u64 = 8 * 1024 * 1024;
pub const SQLITE_DEFAULT_MAX_STORAGE_BYTES: u64 = 10 * 1024 * 1024 * 1024;

/// Build a fresh `DBHead` for a brand-new actor allocation.
///
/// Invariants documented on the schema:
/// - `head_txid < next_txid` always. `next_txid` reserves the txid of the *next*
///   commit, so `next_txid - head_txid` is the number of txids that have been
///   allocated but not yet promoted to head.
/// - `materialized_txid <= head_txid`.
/// - `generation` fences stale owners. It starts at 1 and advances when a new
///   open takes over an actor that this process still considers open.
pub fn new_db_head(creation_ts_ms: i64) -> DBHead {
	DBHead {
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
		origin: SqliteOrigin::CreatedOnV2,
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
pub struct GetPagesResult {
	pub pages: Vec<FetchedPage>,
	pub meta: SqliteMeta,
}

impl Deref for GetPagesResult {
	type Target = [FetchedPage];

	fn deref(&self) -> &Self::Target {
		&self.pages
	}
}

impl IntoIterator for GetPagesResult {
	type Item = FetchedPage;
	type IntoIter = std::vec::IntoIter<FetchedPage>;

	fn into_iter(self) -> Self::IntoIter {
		self.pages.into_iter()
	}
}

impl PartialEq<Vec<FetchedPage>> for GetPagesResult {
	fn eq(&self, other: &Vec<FetchedPage>) -> bool {
		&self.pages == other
	}
}

impl PartialEq<GetPagesResult> for Vec<FetchedPage> {
	fn eq(&self, other: &GetPagesResult) -> bool {
		self == &other.pages
	}
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
	pub migrated_from_v1: bool,
	pub origin: SqliteOrigin,
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
			migrated_from_v1: matches!(head.origin, SqliteOrigin::MigratedFromV1),
			origin: head.origin,
		}
	}
}

pub fn decode_db_head(bytes: &[u8]) -> Result<DBHead> {
	versioned::decode_db_head(bytes)
}

pub fn encode_db_head(head: &DBHead) -> Result<Vec<u8>> {
	versioned::encode_db_head(head.clone())
}

pub fn decode_preload_hints(bytes: &[u8]) -> Result<PreloadHints> {
	versioned::decode_preload_hints(bytes)
}

pub fn encode_preload_hints(hints: &PreloadHints) -> Result<Vec<u8>> {
	versioned::encode_preload_hints(hints.clone())
}

#[cfg(test)]
mod tests {
	use super::{
		DBHead, DirtyPage, FetchedPage, PreloadHintRange, PreloadHints,
		SQLITE_DEFAULT_MAX_STORAGE_BYTES, SQLITE_MAX_DELTA_BYTES, SQLITE_PAGE_SIZE,
		SQLITE_SHARD_SIZE, SQLITE_VFS_V2_SCHEMA_VERSION, SqliteMeta, SqliteOrigin,
		decode_db_head, decode_preload_hints, encode_db_head, encode_preload_hints, new_db_head,
	};

	#[test]
	fn db_head_new_uses_spec_defaults() {
		let head = new_db_head(1_713_456_789_000);

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
		assert_eq!(head.origin, SqliteOrigin::CreatedOnV2);
	}

	#[test]
	fn db_head_round_trips_through_versioned_encoding() {
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
			origin: SqliteOrigin::MigratedFromV1,
		};

		let encoded = encode_db_head(&head).expect("db head should serialize");
		let decoded = decode_db_head(&encoded).expect("db head should deserialize");

		assert_eq!(decoded, head);
	}

	#[test]
	fn preload_hints_round_trip_through_versioned_encoding() {
		let hints = PreloadHints {
			pgnos: vec![1, 7, 11],
			ranges: vec![PreloadHintRange {
				start_pgno: 64,
				page_count: 32,
			}],
		};

		let encoded = encode_preload_hints(&hints).expect("preload hints should serialize");
		let decoded = decode_preload_hints(&encoded).expect("preload hints should deserialize");

		assert_eq!(decoded, hints);
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
				origin: SqliteOrigin::MigratedFromV1,
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
				migrated_from_v1: true,
				origin: SqliteOrigin::MigratedFromV1,
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
