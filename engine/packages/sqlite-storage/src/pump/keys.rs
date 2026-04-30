//! Key builders for sqlite-storage blobs and indexes.

use anyhow::{Context, Result, ensure};
use universaldb::utils::end_of_key_range;

use super::types::{ActorBranchId, NamespaceBranchId, NamespaceId};

pub const SQLITE_SUBSPACE_PREFIX: u8 = 0x02;
pub const APTR_PARTITION: u8 = 0x10;
pub const NSPTR_PARTITION: u8 = 0x11;
pub const BRANCHES_PARTITION: u8 = 0x20;
pub const NSBRANCH_PARTITION: u8 = 0x21;
pub const BR_PARTITION: u8 = 0x30;
pub const CTR_PARTITION: u8 = 0x40;
pub const BOOKMARK_PARTITION: u8 = 0x50;
pub const CMPC_PARTITION: u8 = 0x60;
pub const PAGE_SIZE: u32 = 4096;
pub const SHARD_SIZE: u32 = 64;

const META_HEAD_PATH: &[u8] = b"/META/head";
const META_HEAD_AT_FORK_PATH: &[u8] = b"/META/head_at_fork";
const META_COMPACT_PATH: &[u8] = b"/META/compact";
const META_COLD_COMPACT_PATH: &[u8] = b"/META/cold_compact";
const META_QUOTA_PATH: &[u8] = b"/META/quota";
const META_COMPACTOR_LEASE_PATH: &[u8] = b"/META/compactor_lease";
const META_COLD_LEASE_PATH: &[u8] = b"/META/cold_lease";
const SHARD_PATH: &[u8] = b"/SHARD/";
const DELTA_PATH: &[u8] = b"/DELTA/";
const PIDX_DELTA_PATH: &[u8] = b"/PIDX/delta/";
const BR_PIDX_PATH: &[u8] = b"/PIDX/";
const COMMITS_PATH: &[u8] = b"/COMMITS/";
const VTX_PATH: &[u8] = b"/VTX/";
const CUR_PATH: &[u8] = b"/cur";
const HISTORY_PATH: &[u8] = b"/history/";
const LIST_PATH: &[u8] = b"/list/";
const REFCOUNT_PATH: &[u8] = b"/refcount";
const DESC_PIN_PATH: &[u8] = b"/desc_pin";
const BK_PIN_PATH: &[u8] = b"/bk_pin";
const ACTOR_TOMBSTONES_PATH: &[u8] = b"/actor_tombstones/";
const MANIFEST_COLD_DRAINED_TXID_PATH: &[u8] = b"/META/manifest/cold_drained_txid";
const MANIFEST_LAST_HOT_PASS_TXID_PATH: &[u8] = b"/META/manifest/last_hot_pass_txid";
const MANIFEST_LAST_ACCESS_TS_MS_PATH: &[u8] = b"/META/manifest/last_access_ts_ms";
const MANIFEST_LAST_ACCESS_BUCKET_PATH: &[u8] = b"/META/manifest/last_access_bucket";
const CTR_QUOTA_GLOBAL_PATH: &[u8] = b"/quota_global";
const CTR_EVICTION_INDEX_PATH: &[u8] = b"/eviction_index/";
const BOOKMARK_PATH: &[u8] = b"/";
const CMPC_ENQUEUE_PATH: &[u8] = b"/enqueue/";
const CMPC_LEASE_GLOBAL_PATH: &[u8] = b"/lease_global/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactorQueueKind {
	Cold,
	Eviction,
}

impl CompactorQueueKind {
	fn as_byte(self) -> u8 {
		match self {
			Self::Cold => 0x00,
			Self::Eviction => 0x01,
		}
	}
}

fn partition_prefix(partition: u8) -> Vec<u8> {
	vec![SQLITE_SUBSPACE_PREFIX, partition]
}

fn uuid_bytes(uuid: uuid::Uuid) -> [u8; 16] {
	*uuid.as_bytes()
}

fn append_uuid(key: &mut Vec<u8>, uuid: uuid::Uuid) {
	key.extend_from_slice(&uuid_bytes(uuid));
}

fn append_ts_nonce(key: &mut Vec<u8>, ts_ms: i64, nonce: u32) {
	key.extend_from_slice(&ts_ms.to_be_bytes());
	key.extend_from_slice(&nonce.to_be_bytes());
}

fn append_actor_id(key: &mut Vec<u8>, actor_id: &str) {
	key.extend_from_slice(actor_id.as_bytes());
}

fn branch_record_base(branch_id: ActorBranchId) -> Vec<u8> {
	let mut key = partition_prefix(BRANCHES_PARTITION);
	key.extend_from_slice(LIST_PATH);
	append_uuid(&mut key, branch_id.as_uuid());
	key
}

fn namespace_branch_record_base(branch_id: NamespaceBranchId) -> Vec<u8> {
	let mut key = partition_prefix(NSBRANCH_PARTITION);
	key.extend_from_slice(LIST_PATH);
	append_uuid(&mut key, branch_id.as_uuid());
	key
}

fn actor_branch_base(branch_id: ActorBranchId) -> Vec<u8> {
	let mut key = partition_prefix(BR_PARTITION);
	key.push(b'/');
	append_uuid(&mut key, branch_id.as_uuid());
	key
}

fn actor_pointer_base(namespace_branch_id: NamespaceBranchId, actor_id: &str) -> Vec<u8> {
	let mut key = partition_prefix(APTR_PARTITION);
	key.push(b'/');
	append_uuid(&mut key, namespace_branch_id.as_uuid());
	key.push(b'/');
	append_actor_id(&mut key, actor_id);
	key
}

fn namespace_pointer_base(namespace_id: NamespaceId) -> Vec<u8> {
	let mut key = partition_prefix(NSPTR_PARTITION);
	key.push(b'/');
	append_uuid(&mut key, namespace_id.as_uuid());
	key
}

fn with_suffix(mut prefix: Vec<u8>, suffix: &[u8]) -> Vec<u8> {
	prefix.extend_from_slice(suffix);
	prefix
}

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

pub fn actor_pointer_cur_key(namespace_branch_id: NamespaceBranchId, actor_id: &str) -> Vec<u8> {
	with_suffix(actor_pointer_base(namespace_branch_id, actor_id), CUR_PATH)
}

pub fn actor_pointer_history_key(
	namespace_branch_id: NamespaceBranchId,
	actor_id: &str,
	ts_ms: i64,
	nonce: u32,
) -> Vec<u8> {
	let mut key = actor_pointer_history_prefix(namespace_branch_id, actor_id);
	append_ts_nonce(&mut key, ts_ms, nonce);
	key
}

pub fn actor_pointer_history_prefix(
	namespace_branch_id: NamespaceBranchId,
	actor_id: &str,
) -> Vec<u8> {
	with_suffix(actor_pointer_base(namespace_branch_id, actor_id), HISTORY_PATH)
}

pub fn namespace_pointer_cur_key(namespace_id: NamespaceId) -> Vec<u8> {
	with_suffix(namespace_pointer_base(namespace_id), CUR_PATH)
}

pub fn namespace_pointer_history_key(
	namespace_id: NamespaceId,
	ts_ms: i64,
	nonce: u32,
) -> Vec<u8> {
	let mut key = with_suffix(namespace_pointer_base(namespace_id), HISTORY_PATH);
	append_ts_nonce(&mut key, ts_ms, nonce);
	key
}

pub fn branches_list_key(branch_id: ActorBranchId) -> Vec<u8> {
	branch_record_base(branch_id)
}

pub fn branches_refcount_key(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(branch_record_base(branch_id), REFCOUNT_PATH)
}

pub fn branches_desc_pin_key(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(branch_record_base(branch_id), DESC_PIN_PATH)
}

pub fn branches_bk_pin_key(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(branch_record_base(branch_id), BK_PIN_PATH)
}

pub fn namespace_branches_list_key(branch_id: NamespaceBranchId) -> Vec<u8> {
	namespace_branch_record_base(branch_id)
}

pub fn namespace_branches_refcount_key(branch_id: NamespaceBranchId) -> Vec<u8> {
	with_suffix(namespace_branch_record_base(branch_id), REFCOUNT_PATH)
}

pub fn namespace_branches_desc_pin_key(branch_id: NamespaceBranchId) -> Vec<u8> {
	with_suffix(namespace_branch_record_base(branch_id), DESC_PIN_PATH)
}

pub fn namespace_branches_bk_pin_key(branch_id: NamespaceBranchId) -> Vec<u8> {
	with_suffix(namespace_branch_record_base(branch_id), BK_PIN_PATH)
}

pub fn namespace_branches_actor_tombstone_key(
	branch_id: NamespaceBranchId,
	actor_id: &str,
) -> Vec<u8> {
	let mut key = with_suffix(namespace_branch_record_base(branch_id), ACTOR_TOMBSTONES_PATH);
	append_actor_id(&mut key, actor_id);
	key
}

pub fn branch_prefix(branch_id: ActorBranchId) -> Vec<u8> {
	actor_branch_base(branch_id)
}

pub fn branch_range(branch_id: ActorBranchId) -> (Vec<u8>, Vec<u8>) {
	let start = branch_prefix(branch_id);
	let end = end_of_key_range(&start);
	(start, end)
}

pub fn branch_meta_head_key(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(actor_branch_base(branch_id), META_HEAD_PATH)
}

pub fn branch_meta_head_at_fork_key(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(actor_branch_base(branch_id), META_HEAD_AT_FORK_PATH)
}

pub fn branch_meta_compact_key(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(actor_branch_base(branch_id), META_COMPACT_PATH)
}

pub fn branch_meta_cold_compact_key(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(actor_branch_base(branch_id), META_COLD_COMPACT_PATH)
}

pub fn branch_meta_quota_key(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(actor_branch_base(branch_id), META_QUOTA_PATH)
}

pub fn branch_meta_compactor_lease_key(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(actor_branch_base(branch_id), META_COMPACTOR_LEASE_PATH)
}

pub fn branch_meta_cold_lease_key(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(actor_branch_base(branch_id), META_COLD_LEASE_PATH)
}

pub fn branch_manifest_cold_drained_txid_key(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(actor_branch_base(branch_id), MANIFEST_COLD_DRAINED_TXID_PATH)
}

pub fn branch_manifest_last_hot_pass_txid_key(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(actor_branch_base(branch_id), MANIFEST_LAST_HOT_PASS_TXID_PATH)
}

pub fn branch_manifest_last_access_ts_ms_key(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(actor_branch_base(branch_id), MANIFEST_LAST_ACCESS_TS_MS_PATH)
}

pub fn branch_manifest_last_access_bucket_key(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(actor_branch_base(branch_id), MANIFEST_LAST_ACCESS_BUCKET_PATH)
}

pub fn branch_commit_key(branch_id: ActorBranchId, txid: u64) -> Vec<u8> {
	let mut key = with_suffix(actor_branch_base(branch_id), COMMITS_PATH);
	key.extend_from_slice(&txid.to_be_bytes());
	key
}

pub fn branch_vtx_key(branch_id: ActorBranchId, versionstamp: [u8; 16]) -> Vec<u8> {
	let mut key = with_suffix(actor_branch_base(branch_id), VTX_PATH);
	key.extend_from_slice(&versionstamp);
	key
}

pub fn branch_pidx_key(branch_id: ActorBranchId, pgno: u32) -> Vec<u8> {
	let mut key = with_suffix(actor_branch_base(branch_id), BR_PIDX_PATH);
	key.extend_from_slice(&pgno.to_be_bytes());
	key
}

pub fn branch_pidx_prefix(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(actor_branch_base(branch_id), BR_PIDX_PATH)
}

pub fn branch_delta_prefix(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(actor_branch_base(branch_id), DELTA_PATH)
}

pub fn branch_delta_chunk_prefix(branch_id: ActorBranchId, txid: u64) -> Vec<u8> {
	let mut key = branch_delta_prefix(branch_id);
	key.extend_from_slice(&txid.to_be_bytes());
	key.push(b'/');
	key
}

pub fn branch_delta_chunk_key(branch_id: ActorBranchId, txid: u64, chunk_idx: u32) -> Vec<u8> {
	let mut key = branch_delta_chunk_prefix(branch_id, txid);
	key.extend_from_slice(&chunk_idx.to_be_bytes());
	key
}

pub fn decode_branch_delta_chunk_txid(branch_id: ActorBranchId, key: &[u8]) -> Result<u64> {
	let prefix = branch_delta_prefix(branch_id);
	ensure!(
		key.starts_with(&prefix),
		"branch delta key did not start with expected prefix"
	);
	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() >= std::mem::size_of::<u64>() + 1,
		"branch delta key suffix had {} bytes, expected at least {}",
		suffix.len(),
		std::mem::size_of::<u64>() + 1
	);
	ensure!(
		suffix[std::mem::size_of::<u64>()] == b'/',
		"branch delta key missing txid/chunk separator"
	);

	Ok(u64::from_be_bytes(
		suffix[..std::mem::size_of::<u64>()]
			.try_into()
			.context("branch delta txid suffix should decode as u64")?,
	))
}

pub fn decode_branch_delta_chunk_idx(
	branch_id: ActorBranchId,
	txid: u64,
	key: &[u8],
) -> Result<u32> {
	let prefix = branch_delta_chunk_prefix(branch_id, txid);
	ensure!(
		key.starts_with(&prefix),
		"branch delta chunk key did not start with expected prefix"
	);
	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() == std::mem::size_of::<u32>(),
		"branch delta chunk key suffix had {} bytes, expected {}",
		suffix.len(),
		std::mem::size_of::<u32>()
	);

	Ok(u32::from_be_bytes(
		suffix
			.try_into()
			.context("branch delta chunk suffix should decode as u32")?,
	))
}

pub fn branch_shard_prefix(branch_id: ActorBranchId) -> Vec<u8> {
	with_suffix(actor_branch_base(branch_id), SHARD_PATH)
}

pub fn branch_shard_version_prefix(branch_id: ActorBranchId, shard_id: u32) -> Vec<u8> {
	let mut key = branch_shard_prefix(branch_id);
	key.extend_from_slice(&shard_id.to_be_bytes());
	key.push(b'/');
	key
}

pub fn branch_shard_key(branch_id: ActorBranchId, shard_id: u32, as_of_txid: u64) -> Vec<u8> {
	let mut key = branch_shard_version_prefix(branch_id, shard_id);
	key.extend_from_slice(&as_of_txid.to_be_bytes());
	key
}

pub fn ctr_quota_global_key() -> Vec<u8> {
	with_suffix(partition_prefix(CTR_PARTITION), CTR_QUOTA_GLOBAL_PATH)
}

pub fn ctr_eviction_index_key(last_access_bucket: i64, branch_id: ActorBranchId) -> Vec<u8> {
	let mut key = with_suffix(partition_prefix(CTR_PARTITION), CTR_EVICTION_INDEX_PATH);
	key.extend_from_slice(&last_access_bucket.to_be_bytes());
	key.push(b'/');
	append_uuid(&mut key, branch_id.as_uuid());
	key
}

pub fn bookmark_key(actor_id: &str, bookmark: &str) -> Vec<u8> {
	let mut key = with_suffix(partition_prefix(BOOKMARK_PARTITION), BOOKMARK_PATH);
	append_actor_id(&mut key, actor_id);
	key.push(b'/');
	key.extend_from_slice(bookmark.as_bytes());
	key
}

pub fn bookmark_pinned_key(actor_id: &str, bookmark: &str) -> Vec<u8> {
	let mut key = bookmark_key(actor_id, bookmark);
	key.extend_from_slice(b"/pinned");
	key
}

pub fn compactor_enqueue_key(ts_ms: i64, actor_id: &str, kind: CompactorQueueKind) -> Vec<u8> {
	let mut key = with_suffix(partition_prefix(CMPC_PARTITION), CMPC_ENQUEUE_PATH);
	key.extend_from_slice(&ts_ms.to_be_bytes());
	key.push(b'/');
	append_actor_id(&mut key, actor_id);
	key.push(b'/');
	key.push(kind.as_byte());
	key
}

pub fn compactor_global_lease_key(kind: CompactorQueueKind) -> Vec<u8> {
	let mut key = with_suffix(partition_prefix(CMPC_PARTITION), CMPC_LEASE_GLOBAL_PATH);
	key.push(kind.as_byte());
	key
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

pub fn commit_key(actor_id: &str, txid: u64) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + COMMITS_PATH.len() + std::mem::size_of::<u64>());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(COMMITS_PATH);
	key.extend_from_slice(&txid.to_be_bytes());
	key
}

pub fn vtx_key(actor_id: &str, versionstamp: [u8; 16]) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + VTX_PATH.len() + versionstamp.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(VTX_PATH);
	key.extend_from_slice(&versionstamp);
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

pub fn shard_version_prefix(actor_id: &str, shard_id: u32) -> Vec<u8> {
	let mut key = shard_key(actor_id, shard_id);
	key.push(b'/');
	key
}

pub fn shard_version_key(actor_id: &str, shard_id: u32, as_of_txid: u64) -> Vec<u8> {
	let mut key = shard_version_prefix(actor_id, shard_id);
	key.extend_from_slice(&as_of_txid.to_be_bytes());
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
