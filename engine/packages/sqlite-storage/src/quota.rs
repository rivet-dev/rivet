//! Helpers for tracking SQLite-specific storage usage and quota limits.

use anyhow::{Context, Result};

use crate::keys::SQLITE_SUBSPACE_PREFIX;
use crate::types::DBHead;

const META_PATH: &[u8] = b"/META";
const SHARD_PATH: &[u8] = b"/SHARD/";
const DELTA_PATH: &[u8] = b"/DELTA/";
const PIDX_DELTA_PATH: &[u8] = b"/PIDX/delta/";

fn sqlite_path(key: &[u8]) -> Option<&[u8]> {
	if key.first().copied() != Some(SQLITE_SUBSPACE_PREFIX) {
		return None;
	}

	let slash_idx = key[1..].iter().position(|byte| *byte == b'/')?;
	Some(&key[1 + slash_idx..])
}

pub fn tracked_storage_entry_size(key: &[u8], value: &[u8]) -> Option<u64> {
	if sqlite_path(key).is_some_and(|path| {
		path == META_PATH
			|| path.starts_with(DELTA_PATH)
			|| path.starts_with(SHARD_PATH)
			|| path.starts_with(PIDX_DELTA_PATH)
	}) {
		Some((key.len() + value.len()) as u64)
	} else {
		None
	}
}

pub fn encode_db_head_with_usage(
	actor_id: &str,
	head: &DBHead,
	usage_without_meta: u64,
) -> Result<(DBHead, Vec<u8>)> {
	let meta_key_len = crate::keys::meta_key(actor_id).len() as u64;
	let mut total_usage = usage_without_meta;

	loop {
		let mut encoded_head = head.clone();
		encoded_head.sqlite_storage_used = total_usage;

		let bytes = serde_bare::to_vec(&encoded_head)
			.context("serialize sqlite db head with quota usage")?;
		let next_total_usage = usage_without_meta + meta_key_len + bytes.len() as u64;
		if next_total_usage == total_usage {
			return Ok((encoded_head, bytes));
		}

		total_usage = next_total_usage;
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;

	use super::{encode_db_head_with_usage, tracked_storage_entry_size};
	use crate::keys::{delta_chunk_key, meta_key, pidx_delta_key, shard_key};
	use crate::types::{
		DBHead, SQLITE_DEFAULT_MAX_STORAGE_BYTES, SQLITE_PAGE_SIZE, SQLITE_SHARD_SIZE, SqliteOrigin,
	};

	const TEST_ACTOR: &str = "test-actor";

	fn delta_blob_key(actor_id: &str, txid: u64) -> Vec<u8> {
		delta_chunk_key(actor_id, txid, 0)
	}

	#[test]
	fn tracked_storage_only_counts_sqlite_persistent_keys() {
		assert!(tracked_storage_entry_size(&meta_key(TEST_ACTOR), b"meta").is_some());
		assert!(tracked_storage_entry_size(&delta_blob_key(TEST_ACTOR, 3), b"delta").is_some());
		assert!(tracked_storage_entry_size(&shard_key(TEST_ACTOR, 7), b"shard").is_some());
		assert!(
			tracked_storage_entry_size(&pidx_delta_key(TEST_ACTOR, 11), &7_u64.to_be_bytes())
				.is_some()
		);
		assert!(tracked_storage_entry_size(b"/other", b"value").is_none());
	}

	#[test]
	fn encode_db_head_with_usage_converges_on_meta_size() -> Result<()> {
		let head = DBHead {
			schema_version: 2,
			generation: 4,
			head_txid: 9,
			next_txid: 10,
			materialized_txid: 8,
			db_size_pages: 64,
			page_size: SQLITE_PAGE_SIZE,
			shard_size: SQLITE_SHARD_SIZE,
			creation_ts_ms: 123,
			sqlite_storage_used: 0,
			sqlite_max_storage: SQLITE_DEFAULT_MAX_STORAGE_BYTES,
			origin: SqliteOrigin::CreatedOnV2,
		};

		let (encoded_head, encoded_bytes) = encode_db_head_with_usage(TEST_ACTOR, &head, 1_024)?;
		let expected_total = 1_024 + meta_key(TEST_ACTOR).len() as u64 + encoded_bytes.len() as u64;

		assert_eq!(encoded_head.sqlite_storage_used, expected_total);

		Ok(())
	}
}
