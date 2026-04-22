//! Core storage types for the SQLite VFS v2 engine implementation.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const SQLITE_VFS_V2_SCHEMA_VERSION: u32 = 2;
pub const SQLITE_PAGE_SIZE: u32 = 4096;
pub const SQLITE_SHARD_SIZE: u32 = 64;
pub const SQLITE_MAX_DELTA_BYTES: u64 = 8 * 1024 * 1024;
pub const SQLITE_DEFAULT_MAX_STORAGE_BYTES: u64 = 10 * 1024 * 1024 * 1024;

/// Persistent head-of-log record for a SQLite v2 actor.
///
/// Invariants:
/// - `head_txid < next_txid` always. `next_txid` reserves the txid of the *next* commit, so
///   `next_txid - head_txid` is the number of txids that have been allocated but not yet
///   promoted to head. In practice this gap is at most 1 on the fast path (commit both
///   allocates and promotes) and exactly 1 in the middle of a slow-path staged commit
///   (`commit_stage_begin` bumps `next_txid`; `commit_finalize` advances `head_txid` to match).
/// - `materialized_txid <= head_txid`. Compaction folds DELTA records into SHARD blobs and
///   advances `materialized_txid` to the highest txid whose pages have all been merged.
/// - `generation` is bumped by takeover. Every commit and compaction writes a fence check on
///   `generation` so a takeover cleanly invalidates an in-flight commit from the previous owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SqliteOrigin {
	Native,
	MigratedFromV1,
	MigratingFromV1,
}

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
	pub origin: SqliteOrigin,
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
			origin: SqliteOrigin::Native,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LegacyDBHead {
	schema_version: u32,
	generation: u64,
	head_txid: u64,
	next_txid: u64,
	materialized_txid: u64,
	db_size_pages: u32,
	page_size: u32,
	shard_size: u32,
	creation_ts_ms: i64,
	sqlite_storage_used: u64,
	sqlite_max_storage: u64,
}

pub fn decode_db_head(bytes: &[u8]) -> Result<DBHead> {
	match serde_bare::from_slice(bytes) {
		Ok(head) => Ok(head),
		Err(err) => {
			let legacy: LegacyDBHead =
				serde_bare::from_slice(bytes).context("decode sqlite db head")?;
			tracing::debug!(?err, "decoded legacy sqlite db head without origin field");
			Ok(DBHead {
				schema_version: legacy.schema_version,
				generation: legacy.generation,
				head_txid: legacy.head_txid,
				next_txid: legacy.next_txid,
				materialized_txid: legacy.materialized_txid,
				db_size_pages: legacy.db_size_pages,
				page_size: legacy.page_size,
				shard_size: legacy.shard_size,
				creation_ts_ms: legacy.creation_ts_ms,
				sqlite_storage_used: legacy.sqlite_storage_used,
				sqlite_max_storage: legacy.sqlite_max_storage,
				origin: SqliteOrigin::Native,
			})
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{
		DBHead, DirtyPage, FetchedPage, SQLITE_DEFAULT_MAX_STORAGE_BYTES, SQLITE_MAX_DELTA_BYTES,
		SQLITE_PAGE_SIZE, SQLITE_SHARD_SIZE, SQLITE_VFS_V2_SCHEMA_VERSION, SqliteMeta,
		SqliteOrigin, decode_db_head,
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
		assert_eq!(head.origin, SqliteOrigin::Native);
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
			origin: SqliteOrigin::MigratedFromV1,
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
	fn decode_db_head_defaults_legacy_rows_to_native_origin() {
		let legacy = (
			SQLITE_VFS_V2_SCHEMA_VERSION,
			7_u64,
			9_u64,
			10_u64,
			5_u64,
			321_u32,
			SQLITE_PAGE_SIZE,
			SQLITE_SHARD_SIZE,
			1_713_456_789_000_i64,
			8_192_u64,
			SQLITE_DEFAULT_MAX_STORAGE_BYTES,
		);
		let encoded = serde_bare::to_vec(&legacy).expect("legacy head should serialize");
		let decoded = decode_db_head(&encoded).expect("legacy head should decode");

		assert_eq!(decoded.origin, SqliteOrigin::Native);
		assert_eq!(decoded.generation, 7);
		assert_eq!(decoded.db_size_pages, 321);
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
