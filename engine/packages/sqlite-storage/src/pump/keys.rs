//! Key builders for sqlite-storage blobs and indexes.

use anyhow::{Context, Result, ensure};
use universaldb::utils::end_of_key_range;

pub const SQLITE_SUBSPACE_PREFIX: u8 = 0x02;
pub const PAGE_SIZE: u32 = 4096;
pub const SHARD_SIZE: u32 = 64;

const META_HEAD_PATH: &[u8] = b"/META/head";
const META_COMPACT_PATH: &[u8] = b"/META/compact";
const META_QUOTA_PATH: &[u8] = b"/META/quota";
const META_COMPACTOR_LEASE_PATH: &[u8] = b"/META/compactor_lease";
const SHARD_PATH: &[u8] = b"/SHARD/";
const DELTA_PATH: &[u8] = b"/DELTA/";
const PIDX_DELTA_PATH: &[u8] = b"/PIDX/delta/";

/// Build the common actor-scoped prefix: `[0x02, actor_id_bytes]`.
pub fn actor_prefix(actor_id: &str) -> Vec<u8> {
	let actor_bytes = actor_id.as_bytes();
	let mut key = Vec::with_capacity(1 + actor_bytes.len());
	key.push(SQLITE_SUBSPACE_PREFIX);
	key.extend_from_slice(actor_bytes);
	key
}

pub fn actor_range(actor_id: &str) -> (Vec<u8>, Vec<u8>) {
	let start = actor_prefix(actor_id);
	let end = end_of_key_range(&start);
	(start, end)
}

pub fn meta_head_key(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + META_HEAD_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_HEAD_PATH);
	key
}

pub fn meta_compact_key(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + META_COMPACT_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_COMPACT_PATH);
	key
}

pub fn meta_quota_key(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + META_QUOTA_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_QUOTA_PATH);
	key
}

pub fn meta_compactor_lease_key(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + META_COMPACTOR_LEASE_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_COMPACTOR_LEASE_PATH);
	key
}

pub fn shard_key(actor_id: &str, shard_id: u32) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + SHARD_PATH.len() + std::mem::size_of::<u32>());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(SHARD_PATH);
	key.extend_from_slice(&shard_id.to_be_bytes());
	key
}

pub fn shard_prefix(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + SHARD_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(SHARD_PATH);
	key
}

pub fn delta_prefix(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + DELTA_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(DELTA_PATH);
	key
}

pub fn delta_chunk_prefix(actor_id: &str, txid: u64) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key =
		Vec::with_capacity(prefix.len() + DELTA_PATH.len() + std::mem::size_of::<u64>() + 1);
	key.extend_from_slice(&prefix);
	key.extend_from_slice(DELTA_PATH);
	key.extend_from_slice(&txid.to_be_bytes());
	key.push(b'/');
	key
}

pub fn delta_chunk_key(actor_id: &str, txid: u64, chunk_idx: u32) -> Vec<u8> {
	let prefix = delta_chunk_prefix(actor_id, txid);
	let mut key = Vec::with_capacity(prefix.len() + std::mem::size_of::<u32>());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(&chunk_idx.to_be_bytes());
	key
}

pub fn pidx_delta_key(actor_id: &str, pgno: u32) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key =
		Vec::with_capacity(prefix.len() + PIDX_DELTA_PATH.len() + std::mem::size_of::<u32>());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(PIDX_DELTA_PATH);
	key.extend_from_slice(&pgno.to_be_bytes());
	key
}

pub fn pidx_delta_prefix(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + PIDX_DELTA_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(PIDX_DELTA_PATH);
	key
}

pub fn decode_delta_chunk_txid(actor_id: &str, key: &[u8]) -> Result<u64> {
	let prefix = delta_prefix(actor_id);
	ensure!(
		key.starts_with(&prefix),
		"delta key did not start with expected prefix"
	);
	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() >= std::mem::size_of::<u64>() + 1,
		"delta key suffix had {} bytes, expected at least {}",
		suffix.len(),
		std::mem::size_of::<u64>() + 1
	);
	ensure!(
		suffix[std::mem::size_of::<u64>()] == b'/',
		"delta key missing txid/chunk separator"
	);

	Ok(u64::from_be_bytes(
		suffix[..std::mem::size_of::<u64>()]
			.try_into()
			.context("delta txid suffix should decode as u64")?,
	))
}

pub fn decode_delta_chunk_idx(actor_id: &str, txid: u64, key: &[u8]) -> Result<u32> {
	let prefix = delta_chunk_prefix(actor_id, txid);
	ensure!(
		key.starts_with(&prefix),
		"delta chunk key did not start with expected prefix"
	);
	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() == std::mem::size_of::<u32>(),
		"delta chunk key suffix had {} bytes, expected {}",
		suffix.len(),
		std::mem::size_of::<u32>()
	);

	Ok(u32::from_be_bytes(
		suffix
			.try_into()
			.context("delta chunk suffix should decode as u32")?,
	))
}
