//! Key builders for sqlite-storage blobs and indexes.

use anyhow::{Context, Result, ensure};
use universaldb::utils::end_of_key_range;

pub const SQLITE_SUBSPACE_PREFIX: u8 = 0x02;
pub const PAGE_SIZE: u32 = 4096;
pub const SHARD_SIZE: u32 = 64;

const META_HEAD_PATH: &[u8] = b"/META/head";
const META_COMPACT_PATH: &[u8] = b"/META/compact";
const META_COMPACTOR_LEASE_PATH: &[u8] = b"/META/compactor_lease";
const META_RETENTION_PATH: &[u8] = b"/META/retention";
const META_CHECKPOINTS_PATH: &[u8] = b"/META/checkpoints";
const META_STORAGE_USED_LIVE_PATH: &[u8] = b"/META/storage_used_live";
const META_STORAGE_USED_PITR_PATH: &[u8] = b"/META/storage_used_pitr";
const META_ADMIN_OP_PATH: &[u8] = b"/META/admin_op/";
const META_RESTORE_IN_PROGRESS_PATH: &[u8] = b"/META/restore_in_progress";
const META_FORK_IN_PROGRESS_PATH: &[u8] = b"/META/fork_in_progress";
const CHECKPOINT_PATH: &[u8] = b"/CHECKPOINT/";
const SHARD_PATH: &[u8] = b"/SHARD/";
const DELTA_PATH: &[u8] = b"/DELTA/";
const DELTA_META_PATH: &[u8] = b"/META";
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

pub fn meta_compactor_lease_key(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + META_COMPACTOR_LEASE_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_COMPACTOR_LEASE_PATH);
	key
}

pub fn meta_retention_key(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + META_RETENTION_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_RETENTION_PATH);
	key
}

pub fn meta_checkpoints_key(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + META_CHECKPOINTS_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_CHECKPOINTS_PATH);
	key
}

pub fn meta_storage_used_live_key(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + META_STORAGE_USED_LIVE_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_STORAGE_USED_LIVE_PATH);
	key
}

pub fn meta_storage_used_pitr_key(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + META_STORAGE_USED_PITR_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_STORAGE_USED_PITR_PATH);
	key
}

pub fn meta_admin_op_key(actor_id: &str, op_id: uuid::Uuid) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + META_ADMIN_OP_PATH.len() + 16);
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_ADMIN_OP_PATH);
	key.extend_from_slice(op_id.as_bytes());
	key
}

pub fn meta_restore_in_progress_key(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + META_RESTORE_IN_PROGRESS_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_RESTORE_IN_PROGRESS_PATH);
	key
}

pub fn meta_fork_in_progress_key(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + META_FORK_IN_PROGRESS_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_FORK_IN_PROGRESS_PATH);
	key
}

pub fn checkpoint_prefix(actor_id: &str, ckp_txid: u64) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key =
		Vec::with_capacity(prefix.len() + CHECKPOINT_PATH.len() + std::mem::size_of::<u64>() + 1);
	key.extend_from_slice(&prefix);
	key.extend_from_slice(CHECKPOINT_PATH);
	key.extend_from_slice(&ckp_txid.to_be_bytes());
	key.push(b'/');
	key
}

pub fn checkpoint_meta_key(actor_id: &str, ckp_txid: u64) -> Vec<u8> {
	let mut key = checkpoint_prefix(actor_id, ckp_txid);
	key.extend_from_slice(b"META");
	key
}

pub fn checkpoint_shard_key(actor_id: &str, ckp_txid: u64, shard_id: u32) -> Vec<u8> {
	let mut key = checkpoint_prefix(actor_id, ckp_txid);
	key.extend_from_slice(b"SHARD/");
	key.extend_from_slice(&shard_id.to_be_bytes());
	key
}

pub fn checkpoint_pidx_delta_key(actor_id: &str, ckp_txid: u64, pgno: u32) -> Vec<u8> {
	let mut key = checkpoint_prefix(actor_id, ckp_txid);
	key.extend_from_slice(b"PIDX/delta/");
	key.extend_from_slice(&pgno.to_be_bytes());
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

pub fn delta_meta_key(actor_id: &str, txid: u64) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(
		prefix.len() + DELTA_PATH.len() + std::mem::size_of::<u64>() + DELTA_META_PATH.len(),
	);
	key.extend_from_slice(&prefix);
	key.extend_from_slice(DELTA_PATH);
	key.extend_from_slice(&txid.to_be_bytes());
	key.extend_from_slice(DELTA_META_PATH);
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
