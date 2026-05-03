//! Key builders for depot blobs and indexes.

use anyhow::{Context, Result, bail, ensure};
use gas::prelude::Id;
use universaldb::utils::end_of_key_range;

use super::types::{BucketBranchId, BucketId, DatabaseBranchId};

pub const SQLITE_SUBSPACE_PREFIX: u8 = 0x02;
pub const DBPTR_PARTITION: u8 = 0x10;
pub const BUCKET_PTR_PARTITION: u8 = 0x11;
pub const BUCKET_CATALOG_PARTITION: u8 = 0x12;
pub const BRANCHES_PARTITION: u8 = 0x20;
pub const BUCKET_BRANCH_PARTITION: u8 = 0x21;
pub const BR_PARTITION: u8 = 0x30;
pub const CTR_PARTITION: u8 = 0x40;
pub const RESTORE_POINT_PARTITION: u8 = 0x50;
pub const CMPC_PARTITION: u8 = 0x60;
pub const DB_PIN_PARTITION: u8 = 0x70;
pub const BUCKET_FORK_PIN_PARTITION: u8 = 0x71;
pub const BUCKET_CHILD_PARTITION: u8 = 0x72;
pub const BUCKET_CATALOG_BY_DB_PARTITION: u8 = 0x73;
pub const BUCKET_PROOF_EPOCH_PARTITION: u8 = 0x74;
pub const SQLITE_CMP_DIRTY_PARTITION: u8 = 0x75;
pub const PAGE_SIZE: u32 = 4096;
pub const SHARD_SIZE: u32 = 64;

const META_HEAD_PATH: &[u8] = b"/META/head";
const META_HEAD_AT_FORK_PATH: &[u8] = b"/META/head_at_fork";
const META_COMPACT_PATH: &[u8] = b"/META/compact";
const META_COLD_COMPACT_PATH: &[u8] = b"/META/cold_compact";
const META_QUOTA_PATH: &[u8] = b"/META/quota";
const META_COMPACTOR_LEASE_PATH: &[u8] = b"/META/compactor_lease";
const META_COLD_LEASE_PATH: &[u8] = b"/META/cold_lease";
const CMP_ROOT_PATH: &[u8] = b"/CMP/root";
const CMP_COLD_SHARD_PATH: &[u8] = b"/CMP/cold_shard/";
const CMP_RETIRED_COLD_OBJECT_PATH: &[u8] = b"/CMP/retired_cold_object/";
const CMP_STAGE_PATH: &[u8] = b"/CMP/stage/";
const CMP_STAGE_HOT_SHARD_PATH: &[u8] = b"/hot_shard/";
const SHARD_PATH: &[u8] = b"/SHARD/";
const DELTA_PATH: &[u8] = b"/DELTA/";
const PIDX_DELTA_PATH: &[u8] = b"/PIDX/delta/";
const BR_PIDX_PATH: &[u8] = b"/PIDX/";
const COMMITS_PATH: &[u8] = b"/COMMITS/";
const VTX_PATH: &[u8] = b"/VTX/";
const PITR_INTERVAL_PATH: &[u8] = b"/PITR_INTERVAL/";
const CUR_PATH: &[u8] = b"/cur";
const HISTORY_PATH: &[u8] = b"/history/";
const POLICY_PITR_PATH: &[u8] = b"/POLICY/PITR";
const POLICY_SHARD_CACHE_PATH: &[u8] = b"/POLICY/SHARD_CACHE";
const DB_POLICY_PATH: &[u8] = b"/DB_POLICY/";
const PITR_PATH: &[u8] = b"/PITR";
const SHARD_CACHE_PATH: &[u8] = b"/SHARD_CACHE";
const LIST_PATH: &[u8] = b"/list/";
const REFCOUNT_PATH: &[u8] = b"/refcount";
const DESC_PIN_PATH: &[u8] = b"/desc_pin";
const RESTORE_POINT_PIN_PATH: &[u8] = b"/restore_point_pin";
const PIN_COUNT_PATH: &[u8] = b"/pin_count";
const DATABASE_TOMBSTONES_PATH: &[u8] = b"/database_tombstones/";
const MANIFEST_COLD_DRAINED_TXID_PATH: &[u8] = b"/META/manifest/cold_drained_txid";
const MANIFEST_LAST_HOT_PASS_TXID_PATH: &[u8] = b"/META/manifest/last_hot_pass_txid";
const MANIFEST_LAST_ACCESS_TS_MS_PATH: &[u8] = b"/META/manifest/last_access_ts_ms";
const MANIFEST_LAST_ACCESS_BUCKET_PATH: &[u8] = b"/META/manifest/last_access_bucket";
const CTR_QUOTA_GLOBAL_PATH: &[u8] = b"/quota_global";
const CTR_EVICTION_INDEX_PATH: &[u8] = b"/eviction_index/";
const RESTORE_POINT_PATH: &[u8] = b"/";
const CMPC_ENQUEUE_PATH: &[u8] = b"/enqueue/";
const CMPC_LEASE_GLOBAL_PATH: &[u8] = b"/lease_global/";
const DB_PIN_PATH: &[u8] = b"/";
const BUCKET_FORK_PIN_PATH: &[u8] = b"/";
const BUCKET_CHILD_PATH: &[u8] = b"/";
const BUCKET_CATALOG_BY_DB_PATH: &[u8] = b"/";
const BUCKET_PROOF_EPOCH_PATH: &[u8] = b"/";
const SQLITE_CMP_DIRTY_PATH: &[u8] = b"/";

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

fn append_id(key: &mut Vec<u8>, id: Id) {
	key.extend_from_slice(&id.as_bytes());
}

fn append_database_id(key: &mut Vec<u8>, database_id: &str) {
	key.extend_from_slice(database_id.as_bytes());
}

fn branch_record_base(branch_id: DatabaseBranchId) -> Vec<u8> {
	let mut key = partition_prefix(BRANCHES_PARTITION);
	key.extend_from_slice(LIST_PATH);
	append_uuid(&mut key, branch_id.as_uuid());
	key
}

fn bucket_branch_record_base(branch_id: BucketBranchId) -> Vec<u8> {
	let mut key = partition_prefix(BUCKET_BRANCH_PARTITION);
	key.extend_from_slice(LIST_PATH);
	append_uuid(&mut key, branch_id.as_uuid());
	key
}

fn database_branch_base(branch_id: DatabaseBranchId) -> Vec<u8> {
	let mut key = partition_prefix(BR_PARTITION);
	key.push(b'/');
	append_uuid(&mut key, branch_id.as_uuid());
	key
}

fn database_pointer_base(bucket_branch_id: BucketBranchId, database_id: &str) -> Vec<u8> {
	let mut key = partition_prefix(DBPTR_PARTITION);
	key.push(b'/');
	append_uuid(&mut key, bucket_branch_id.as_uuid());
	key.push(b'/');
	append_database_id(&mut key, database_id);
	key
}

fn bucket_pointer_base(bucket_id: BucketId) -> Vec<u8> {
	let mut key = partition_prefix(BUCKET_PTR_PARTITION);
	key.push(b'/');
	append_uuid(&mut key, bucket_id.as_uuid());
	key
}

fn bucket_catalog_base(bucket_branch_id: BucketBranchId) -> Vec<u8> {
	let mut key = partition_prefix(BUCKET_CATALOG_PARTITION);
	key.push(b'/');
	append_uuid(&mut key, bucket_branch_id.as_uuid());
	key.push(b'/');
	key
}

fn with_suffix(mut prefix: Vec<u8>, suffix: &[u8]) -> Vec<u8> {
	prefix.extend_from_slice(suffix);
	prefix
}

/// Build the common database-scoped prefix: `[0x02, database_id_bytes]`.
pub fn database_prefix(database_id: &str) -> Vec<u8> {
	let database_bytes = database_id.as_bytes();
	let mut key = Vec::with_capacity(1 + database_bytes.len());
	key.push(SQLITE_SUBSPACE_PREFIX);
	key.extend_from_slice(database_bytes);
	key
}

pub fn database_range(database_id: &str) -> (Vec<u8>, Vec<u8>) {
	let start = database_prefix(database_id);
	let end = end_of_key_range(&start);
	(start, end)
}

pub fn database_pointer_cur_key(bucket_branch_id: BucketBranchId, database_id: &str) -> Vec<u8> {
	with_suffix(
		database_pointer_base(bucket_branch_id, database_id),
		CUR_PATH,
	)
}

pub fn database_pointer_cur_prefix() -> Vec<u8> {
	let mut key = partition_prefix(DBPTR_PARTITION);
	key.push(b'/');
	key
}

pub fn decode_database_pointer_cur_key(key: &[u8]) -> Result<(BucketBranchId, String)> {
	let prefix = database_pointer_cur_prefix();
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("database pointer key did not start with expected prefix")?;
	ensure!(
		suffix.len() > std::mem::size_of::<uuid::Uuid>() + 1 + CUR_PATH.len(),
		"database pointer key suffix is too short"
	);
	let (bucket_branch_bytes, rest) = suffix.split_at(std::mem::size_of::<uuid::Uuid>());
	ensure!(
		rest.first() == Some(&b'/'),
		"database pointer key missing bucket/database separator"
	);
	let database_id_bytes = rest[1..]
		.strip_suffix(CUR_PATH)
		.context("database pointer key did not end with current pointer suffix")?;
	let uuid = uuid::Uuid::from_slice(bucket_branch_bytes)
		.context("decode database pointer bucket branch uuid")?;
	let database_id = String::from_utf8(database_id_bytes.to_vec())
		.context("database pointer database id was not utf-8")?;

	Ok((BucketBranchId::from_uuid(uuid), database_id))
}

pub fn database_pointer_history_key(
	bucket_branch_id: BucketBranchId,
	database_id: &str,
	ts_ms: i64,
	nonce: u32,
) -> Vec<u8> {
	let mut key = database_pointer_history_prefix(bucket_branch_id, database_id);
	append_ts_nonce(&mut key, ts_ms, nonce);
	key
}

pub fn database_pointer_history_prefix(
	bucket_branch_id: BucketBranchId,
	database_id: &str,
) -> Vec<u8> {
	with_suffix(
		database_pointer_base(bucket_branch_id, database_id),
		HISTORY_PATH,
	)
}

pub fn bucket_pointer_cur_key(bucket_id: BucketId) -> Vec<u8> {
	with_suffix(bucket_pointer_base(bucket_id), CUR_PATH)
}

pub fn bucket_pointer_cur_prefix() -> Vec<u8> {
	let mut key = partition_prefix(BUCKET_PTR_PARTITION);
	key.push(b'/');
	key
}

pub fn decode_bucket_pointer_cur_bucket_id(key: &[u8]) -> Result<BucketId> {
	let prefix = bucket_pointer_cur_prefix();
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("bucket pointer key did not start with expected prefix")?;
	let Some(bucket_id_bytes) = suffix.strip_suffix(CUR_PATH) else {
		bail!("bucket pointer key did not end with current pointer suffix");
	};
	ensure!(
		bucket_id_bytes.len() == std::mem::size_of::<uuid::Uuid>(),
		"bucket pointer key bucket id had {} bytes, expected {}",
		bucket_id_bytes.len(),
		std::mem::size_of::<uuid::Uuid>()
	);
	let uuid = uuid::Uuid::from_slice(bucket_id_bytes).context("decode bucket pointer uuid")?;

	Ok(BucketId::from_uuid(uuid))
}

pub fn bucket_pointer_history_key(bucket_id: BucketId, ts_ms: i64, nonce: u32) -> Vec<u8> {
	let mut key = bucket_pointer_history_prefix(bucket_id);
	append_ts_nonce(&mut key, ts_ms, nonce);
	key
}

pub fn bucket_pointer_history_prefix(bucket_id: BucketId) -> Vec<u8> {
	with_suffix(bucket_pointer_base(bucket_id), HISTORY_PATH)
}

pub fn bucket_policy_pitr_key(bucket_id: BucketId) -> Vec<u8> {
	with_suffix(bucket_pointer_base(bucket_id), POLICY_PITR_PATH)
}

pub fn bucket_policy_shard_cache_key(bucket_id: BucketId) -> Vec<u8> {
	with_suffix(bucket_pointer_base(bucket_id), POLICY_SHARD_CACHE_PATH)
}

pub fn database_pitr_policy_key(bucket_id: BucketId, database_id: &str) -> Vec<u8> {
	let mut key = with_suffix(bucket_pointer_base(bucket_id), DB_POLICY_PATH);
	append_database_id(&mut key, database_id);
	key.extend_from_slice(PITR_PATH);
	key
}

pub fn database_shard_cache_policy_key(bucket_id: BucketId, database_id: &str) -> Vec<u8> {
	let mut key = with_suffix(bucket_pointer_base(bucket_id), DB_POLICY_PATH);
	append_database_id(&mut key, database_id);
	key.extend_from_slice(SHARD_CACHE_PATH);
	key
}

pub fn branches_list_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	branch_record_base(branch_id)
}

pub fn branches_refcount_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(branch_record_base(branch_id), REFCOUNT_PATH)
}

pub fn branches_desc_pin_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(branch_record_base(branch_id), DESC_PIN_PATH)
}

pub fn branches_restore_point_pin_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(branch_record_base(branch_id), RESTORE_POINT_PIN_PATH)
}

pub fn bucket_branches_list_key(branch_id: BucketBranchId) -> Vec<u8> {
	bucket_branch_record_base(branch_id)
}

pub fn bucket_branches_refcount_key(branch_id: BucketBranchId) -> Vec<u8> {
	with_suffix(bucket_branch_record_base(branch_id), REFCOUNT_PATH)
}

pub fn bucket_branches_desc_pin_key(branch_id: BucketBranchId) -> Vec<u8> {
	with_suffix(bucket_branch_record_base(branch_id), DESC_PIN_PATH)
}

pub fn bucket_branches_restore_point_pin_key(branch_id: BucketBranchId) -> Vec<u8> {
	with_suffix(bucket_branch_record_base(branch_id), RESTORE_POINT_PIN_PATH)
}

pub fn bucket_branches_pin_count_key(branch_id: BucketBranchId) -> Vec<u8> {
	with_suffix(bucket_branch_record_base(branch_id), PIN_COUNT_PATH)
}

pub fn bucket_branches_database_name_tombstone_key(
	branch_id: BucketBranchId,
	database_id: &str,
) -> Vec<u8> {
	let mut key = with_suffix(
		bucket_branch_record_base(branch_id),
		DATABASE_TOMBSTONES_PATH,
	);
	append_database_id(&mut key, database_id);
	key
}

pub fn bucket_branches_database_tombstone_key(
	branch_id: BucketBranchId,
	database_id: DatabaseBranchId,
) -> Vec<u8> {
	let mut key = with_suffix(
		bucket_branch_record_base(branch_id),
		DATABASE_TOMBSTONES_PATH,
	);
	append_uuid(&mut key, database_id.as_uuid());
	key
}

pub fn bucket_branches_database_tombstone_prefix(branch_id: BucketBranchId) -> Vec<u8> {
	with_suffix(
		bucket_branch_record_base(branch_id),
		DATABASE_TOMBSTONES_PATH,
	)
}

pub fn decode_bucket_branches_database_tombstone_id(
	branch_id: BucketBranchId,
	key: &[u8],
) -> Result<DatabaseBranchId> {
	let prefix = bucket_branches_database_tombstone_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("bucket database tombstone key did not start with expected prefix")?;
	ensure!(
		suffix.len() == std::mem::size_of::<uuid::Uuid>(),
		"bucket database tombstone key suffix had {} bytes, expected {}",
		suffix.len(),
		std::mem::size_of::<uuid::Uuid>()
	);
	let uuid = uuid::Uuid::from_slice(suffix).context("decode bucket database tombstone uuid")?;

	Ok(DatabaseBranchId::from_uuid(uuid))
}

pub fn bucket_catalog_key(
	bucket_branch_id: BucketBranchId,
	database_id: DatabaseBranchId,
) -> Vec<u8> {
	let mut key = bucket_catalog_prefix(bucket_branch_id);
	append_uuid(&mut key, database_id.as_uuid());
	key
}

pub fn bucket_catalog_prefix(bucket_branch_id: BucketBranchId) -> Vec<u8> {
	bucket_catalog_base(bucket_branch_id)
}

pub fn decode_bucket_catalog_database_id(
	bucket_branch_id: BucketBranchId,
	key: &[u8],
) -> Result<DatabaseBranchId> {
	let prefix = bucket_catalog_prefix(bucket_branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("bucket catalog key did not start with expected prefix")?;
	ensure!(
		suffix.len() == std::mem::size_of::<uuid::Uuid>(),
		"bucket catalog key suffix had {} bytes, expected {}",
		suffix.len(),
		std::mem::size_of::<uuid::Uuid>()
	);
	let uuid = uuid::Uuid::from_slice(suffix).context("decode bucket catalog database uuid")?;

	Ok(DatabaseBranchId::from_uuid(uuid))
}

pub fn branch_prefix(branch_id: DatabaseBranchId) -> Vec<u8> {
	database_branch_base(branch_id)
}

pub fn branch_range(branch_id: DatabaseBranchId) -> (Vec<u8>, Vec<u8>) {
	let start = branch_prefix(branch_id);
	let end = end_of_key_range(&start);
	(start, end)
}

pub fn branch_meta_head_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), META_HEAD_PATH)
}

pub fn branch_meta_head_at_fork_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), META_HEAD_AT_FORK_PATH)
}

pub fn branch_meta_compact_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), META_COMPACT_PATH)
}

pub fn branch_meta_cold_compact_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), META_COLD_COMPACT_PATH)
}

pub fn branch_meta_quota_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), META_QUOTA_PATH)
}

pub fn branch_meta_compactor_lease_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), META_COMPACTOR_LEASE_PATH)
}

pub fn branch_meta_cold_lease_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), META_COLD_LEASE_PATH)
}

pub fn branch_manifest_cold_drained_txid_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(
		database_branch_base(branch_id),
		MANIFEST_COLD_DRAINED_TXID_PATH,
	)
}

pub fn branch_manifest_last_hot_pass_txid_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(
		database_branch_base(branch_id),
		MANIFEST_LAST_HOT_PASS_TXID_PATH,
	)
}

pub fn branch_manifest_last_access_ts_ms_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(
		database_branch_base(branch_id),
		MANIFEST_LAST_ACCESS_TS_MS_PATH,
	)
}

pub fn branch_manifest_last_access_bucket_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(
		database_branch_base(branch_id),
		MANIFEST_LAST_ACCESS_BUCKET_PATH,
	)
}

pub fn branch_compaction_root_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), CMP_ROOT_PATH)
}

pub fn branch_compaction_cold_shard_prefix(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), CMP_COLD_SHARD_PATH)
}

pub fn branch_compaction_retired_cold_object_prefix(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(
		database_branch_base(branch_id),
		CMP_RETIRED_COLD_OBJECT_PATH,
	)
}

pub fn branch_compaction_stage_prefix(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), CMP_STAGE_PATH)
}

pub fn branch_compaction_cold_shard_version_prefix(
	branch_id: DatabaseBranchId,
	shard_id: u32,
) -> Vec<u8> {
	let mut key = branch_compaction_cold_shard_prefix(branch_id);
	key.extend_from_slice(&shard_id.to_be_bytes());
	key.push(b'/');
	key
}

pub fn branch_compaction_cold_shard_key(
	branch_id: DatabaseBranchId,
	shard_id: u32,
	as_of_txid: u64,
) -> Vec<u8> {
	let mut key = branch_compaction_cold_shard_version_prefix(branch_id, shard_id);
	key.extend_from_slice(&as_of_txid.to_be_bytes());
	key
}

pub fn branch_compaction_retired_cold_object_key(
	branch_id: DatabaseBranchId,
	object_key_hash: [u8; 32],
) -> Vec<u8> {
	let mut key = with_suffix(
		database_branch_base(branch_id),
		CMP_RETIRED_COLD_OBJECT_PATH,
	);
	key.extend_from_slice(&object_key_hash);
	key
}

pub fn branch_compaction_stage_hot_shard_prefix(
	branch_id: DatabaseBranchId,
	job_id: Id,
) -> Vec<u8> {
	let mut key = with_suffix(database_branch_base(branch_id), CMP_STAGE_PATH);
	append_id(&mut key, job_id);
	key.extend_from_slice(CMP_STAGE_HOT_SHARD_PATH);
	key
}

pub fn branch_compaction_stage_hot_shard_version_prefix(
	branch_id: DatabaseBranchId,
	job_id: Id,
	shard_id: u32,
) -> Vec<u8> {
	let mut key = branch_compaction_stage_hot_shard_prefix(branch_id, job_id);
	key.extend_from_slice(&shard_id.to_be_bytes());
	key.push(b'/');
	key
}

pub fn branch_compaction_stage_hot_shard_key(
	branch_id: DatabaseBranchId,
	job_id: Id,
	shard_id: u32,
	as_of_txid: u64,
	chunk_idx: u32,
) -> Vec<u8> {
	let mut key = branch_compaction_stage_hot_shard_version_prefix(branch_id, job_id, shard_id);
	key.extend_from_slice(&as_of_txid.to_be_bytes());
	key.push(b'/');
	key.extend_from_slice(&chunk_idx.to_be_bytes());
	key
}

pub fn branch_commit_key(branch_id: DatabaseBranchId, txid: u64) -> Vec<u8> {
	let mut key = branch_commit_prefix(branch_id);
	key.extend_from_slice(&txid.to_be_bytes());
	key
}

pub fn branch_commit_prefix(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), COMMITS_PATH)
}

pub fn branch_vtx_key(branch_id: DatabaseBranchId, versionstamp: [u8; 16]) -> Vec<u8> {
	let mut key = branch_vtx_prefix(branch_id);
	key.extend_from_slice(&versionstamp);
	key
}

pub fn branch_vtx_prefix(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), VTX_PATH)
}

pub fn branch_pitr_interval_key(branch_id: DatabaseBranchId, bucket_start_ms: i64) -> Vec<u8> {
	let mut key = branch_pitr_interval_prefix(branch_id);
	key.extend_from_slice(&bucket_start_ms.to_be_bytes());
	key
}

pub fn branch_pitr_interval_prefix(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), PITR_INTERVAL_PATH)
}

pub fn decode_branch_pitr_interval_bucket(branch_id: DatabaseBranchId, key: &[u8]) -> Result<i64> {
	let prefix = branch_pitr_interval_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("branch PITR interval key did not start with expected prefix")?;
	ensure!(
		suffix.len() == std::mem::size_of::<i64>(),
		"branch PITR interval key suffix had {} bytes, expected {}",
		suffix.len(),
		std::mem::size_of::<i64>()
	);

	Ok(i64::from_be_bytes(suffix.try_into().context(
		"branch PITR interval suffix should decode as i64",
	)?))
}

pub fn branch_pidx_key(branch_id: DatabaseBranchId, pgno: u32) -> Vec<u8> {
	let mut key = with_suffix(database_branch_base(branch_id), BR_PIDX_PATH);
	key.extend_from_slice(&pgno.to_be_bytes());
	key
}

pub fn branch_pidx_prefix(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), BR_PIDX_PATH)
}

pub fn branch_delta_prefix(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), DELTA_PATH)
}

pub fn branch_delta_chunk_prefix(branch_id: DatabaseBranchId, txid: u64) -> Vec<u8> {
	let mut key = branch_delta_prefix(branch_id);
	key.extend_from_slice(&txid.to_be_bytes());
	key.push(b'/');
	key
}

pub fn branch_delta_chunk_key(branch_id: DatabaseBranchId, txid: u64, chunk_idx: u32) -> Vec<u8> {
	let mut key = branch_delta_chunk_prefix(branch_id, txid);
	key.extend_from_slice(&chunk_idx.to_be_bytes());
	key
}

pub fn decode_branch_delta_chunk_txid(branch_id: DatabaseBranchId, key: &[u8]) -> Result<u64> {
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
	branch_id: DatabaseBranchId,
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

	Ok(u32::from_be_bytes(suffix.try_into().context(
		"branch delta chunk suffix should decode as u32",
	)?))
}

pub fn branch_shard_prefix(branch_id: DatabaseBranchId) -> Vec<u8> {
	with_suffix(database_branch_base(branch_id), SHARD_PATH)
}

pub fn branch_shard_version_prefix(branch_id: DatabaseBranchId, shard_id: u32) -> Vec<u8> {
	let mut key = branch_shard_prefix(branch_id);
	key.extend_from_slice(&shard_id.to_be_bytes());
	key.push(b'/');
	key
}

pub fn branch_shard_key(branch_id: DatabaseBranchId, shard_id: u32, as_of_txid: u64) -> Vec<u8> {
	let mut key = branch_shard_version_prefix(branch_id, shard_id);
	key.extend_from_slice(&as_of_txid.to_be_bytes());
	key
}

pub fn ctr_quota_global_key() -> Vec<u8> {
	with_suffix(partition_prefix(CTR_PARTITION), CTR_QUOTA_GLOBAL_PATH)
}

pub fn ctr_eviction_index_key(last_access_bucket: i64, branch_id: DatabaseBranchId) -> Vec<u8> {
	let mut key = with_suffix(partition_prefix(CTR_PARTITION), CTR_EVICTION_INDEX_PATH);
	key.extend_from_slice(&last_access_bucket.to_be_bytes());
	key.push(b'/');
	append_uuid(&mut key, branch_id.as_uuid());
	key
}

pub fn ctr_eviction_index_prefix() -> Vec<u8> {
	with_suffix(partition_prefix(CTR_PARTITION), CTR_EVICTION_INDEX_PATH)
}

pub fn ctr_eviction_index_range() -> (Vec<u8>, Vec<u8>) {
	universaldb::tuple::Subspace::from_bytes(ctr_eviction_index_prefix()).range()
}

pub fn decode_ctr_eviction_index_key(key: &[u8]) -> Result<(i64, DatabaseBranchId)> {
	let prefix = ctr_eviction_index_prefix();
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("eviction index key did not start with expected prefix")?;
	let expected_len = std::mem::size_of::<i64>() + 1 + std::mem::size_of::<uuid::Uuid>();
	ensure!(
		suffix.len() == expected_len,
		"eviction index key suffix had {} bytes, expected {}",
		suffix.len(),
		expected_len
	);
	let bucket_bytes: [u8; std::mem::size_of::<i64>()] = suffix[..8]
		.try_into()
		.context("decode eviction index bucket")?;
	ensure!(
		suffix[8] == b'/',
		"eviction index key missing branch separator"
	);
	let branch_id =
		uuid::Uuid::from_slice(&suffix[9..]).context("decode eviction index branch id")?;

	Ok((
		i64::from_be_bytes(bucket_bytes),
		DatabaseBranchId::from_uuid(branch_id),
	))
}

pub fn restore_point_prefix(database_id: &str) -> Vec<u8> {
	let mut key = with_suffix(
		partition_prefix(RESTORE_POINT_PARTITION),
		RESTORE_POINT_PATH,
	);
	append_database_id(&mut key, database_id);
	key.push(b'/');
	key
}

pub fn restore_point_key(database_id: &str, restore_point: &str) -> Vec<u8> {
	let mut key = restore_point_prefix(database_id);
	key.extend_from_slice(restore_point.as_bytes());
	key
}

pub fn compactor_enqueue_key(ts_ms: i64, database_id: &str, kind: CompactorQueueKind) -> Vec<u8> {
	let mut key = with_suffix(partition_prefix(CMPC_PARTITION), CMPC_ENQUEUE_PATH);
	key.extend_from_slice(&ts_ms.to_be_bytes());
	key.push(b'/');
	append_database_id(&mut key, database_id);
	key.push(b'/');
	key.push(kind.as_byte());
	key
}

pub fn compactor_global_lease_key(kind: CompactorQueueKind) -> Vec<u8> {
	let mut key = with_suffix(partition_prefix(CMPC_PARTITION), CMPC_LEASE_GLOBAL_PATH);
	key.push(kind.as_byte());
	key
}

pub fn db_pin_key(branch_id: DatabaseBranchId, pin_id: &[u8]) -> Vec<u8> {
	let mut key = db_pin_prefix(branch_id);
	key.extend_from_slice(pin_id);
	key
}

pub fn db_pin_prefix(branch_id: DatabaseBranchId) -> Vec<u8> {
	let mut key = with_suffix(partition_prefix(DB_PIN_PARTITION), DB_PIN_PATH);
	append_uuid(&mut key, branch_id.as_uuid());
	key.push(b'/');
	key
}

pub fn bucket_fork_pin_key(
	source_bucket_branch_id: BucketBranchId,
	fork_versionstamp: [u8; 16],
	target_bucket_branch_id: BucketBranchId,
) -> Vec<u8> {
	let mut key = bucket_fork_pin_prefix(source_bucket_branch_id);
	key.extend_from_slice(&fork_versionstamp);
	key.push(b'/');
	append_uuid(&mut key, target_bucket_branch_id.as_uuid());
	key
}

pub fn bucket_fork_pin_prefix(source_bucket_branch_id: BucketBranchId) -> Vec<u8> {
	let mut key = with_suffix(
		partition_prefix(BUCKET_FORK_PIN_PARTITION),
		BUCKET_FORK_PIN_PATH,
	);
	append_uuid(&mut key, source_bucket_branch_id.as_uuid());
	key.push(b'/');
	key
}

pub fn bucket_child_key(
	source_bucket_branch_id: BucketBranchId,
	fork_versionstamp: [u8; 16],
	target_bucket_branch_id: BucketBranchId,
) -> Vec<u8> {
	let mut key = bucket_child_prefix(source_bucket_branch_id);
	key.extend_from_slice(&fork_versionstamp);
	key.push(b'/');
	append_uuid(&mut key, target_bucket_branch_id.as_uuid());
	key
}

pub fn bucket_child_prefix(source_bucket_branch_id: BucketBranchId) -> Vec<u8> {
	let mut key = with_suffix(partition_prefix(BUCKET_CHILD_PARTITION), BUCKET_CHILD_PATH);
	append_uuid(&mut key, source_bucket_branch_id.as_uuid());
	key.push(b'/');
	key
}

pub fn bucket_catalog_by_db_key(
	database_branch_id: DatabaseBranchId,
	bucket_branch_id: BucketBranchId,
) -> Vec<u8> {
	let mut key = bucket_catalog_by_db_prefix(database_branch_id);
	append_uuid(&mut key, bucket_branch_id.as_uuid());
	key
}

pub fn bucket_catalog_by_db_prefix(database_branch_id: DatabaseBranchId) -> Vec<u8> {
	let mut key = with_suffix(
		partition_prefix(BUCKET_CATALOG_BY_DB_PARTITION),
		BUCKET_CATALOG_BY_DB_PATH,
	);
	append_uuid(&mut key, database_branch_id.as_uuid());
	key.push(b'/');
	key
}

pub fn bucket_proof_epoch_key(root_bucket_branch_id: BucketBranchId) -> Vec<u8> {
	let mut key = with_suffix(
		partition_prefix(BUCKET_PROOF_EPOCH_PARTITION),
		BUCKET_PROOF_EPOCH_PATH,
	);
	append_uuid(&mut key, root_bucket_branch_id.as_uuid());
	key
}

pub fn sqlite_cmp_dirty_key(branch_id: DatabaseBranchId) -> Vec<u8> {
	let mut key = with_suffix(
		partition_prefix(SQLITE_CMP_DIRTY_PARTITION),
		SQLITE_CMP_DIRTY_PATH,
	);
	append_uuid(&mut key, branch_id.as_uuid());
	key
}

// Legacy database-scoped keys are v1-only compatibility helpers for pegboard actors.
pub fn meta_head_key(database_id: &str) -> Vec<u8> {
	let prefix = database_prefix(database_id);
	let mut key = Vec::with_capacity(prefix.len() + META_HEAD_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_HEAD_PATH);
	key
}

pub fn meta_compact_key(database_id: &str) -> Vec<u8> {
	let prefix = database_prefix(database_id);
	let mut key = Vec::with_capacity(prefix.len() + META_COMPACT_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_COMPACT_PATH);
	key
}

pub fn meta_quota_key(database_id: &str) -> Vec<u8> {
	let prefix = database_prefix(database_id);
	let mut key = Vec::with_capacity(prefix.len() + META_QUOTA_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_QUOTA_PATH);
	key
}

pub fn meta_compactor_lease_key(database_id: &str) -> Vec<u8> {
	let prefix = database_prefix(database_id);
	let mut key = Vec::with_capacity(prefix.len() + META_COMPACTOR_LEASE_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_COMPACTOR_LEASE_PATH);
	key
}

pub fn commit_key(database_id: &str, txid: u64) -> Vec<u8> {
	let mut key = commit_prefix(database_id);
	key.extend_from_slice(&txid.to_be_bytes());
	key
}

pub fn commit_prefix(database_id: &str) -> Vec<u8> {
	let prefix = database_prefix(database_id);
	let mut key = Vec::with_capacity(prefix.len() + COMMITS_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(COMMITS_PATH);
	key
}

pub fn vtx_key(database_id: &str, versionstamp: [u8; 16]) -> Vec<u8> {
	let mut key = vtx_prefix(database_id);
	key.extend_from_slice(&versionstamp);
	key
}

pub fn vtx_prefix(database_id: &str) -> Vec<u8> {
	let prefix = database_prefix(database_id);
	let mut key = Vec::with_capacity(prefix.len() + VTX_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(VTX_PATH);
	key
}

pub fn shard_key(database_id: &str, shard_id: u32) -> Vec<u8> {
	let prefix = database_prefix(database_id);
	let mut key = Vec::with_capacity(prefix.len() + SHARD_PATH.len() + std::mem::size_of::<u32>());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(SHARD_PATH);
	key.extend_from_slice(&shard_id.to_be_bytes());
	key
}

pub fn shard_version_prefix(database_id: &str, shard_id: u32) -> Vec<u8> {
	let mut key = shard_key(database_id, shard_id);
	key.push(b'/');
	key
}

pub fn shard_version_key(database_id: &str, shard_id: u32, as_of_txid: u64) -> Vec<u8> {
	let mut key = shard_version_prefix(database_id, shard_id);
	key.extend_from_slice(&as_of_txid.to_be_bytes());
	key
}

// Legacy database-scoped prefix kept for v1 pegboard actor cleanup.
pub fn shard_prefix(database_id: &str) -> Vec<u8> {
	let prefix = database_prefix(database_id);
	let mut key = Vec::with_capacity(prefix.len() + SHARD_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(SHARD_PATH);
	key
}

// Legacy database-scoped prefix kept for v1 pegboard actor cleanup.
pub fn delta_prefix(database_id: &str) -> Vec<u8> {
	let prefix = database_prefix(database_id);
	let mut key = Vec::with_capacity(prefix.len() + DELTA_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(DELTA_PATH);
	key
}

pub fn delta_chunk_prefix(database_id: &str, txid: u64) -> Vec<u8> {
	let prefix = database_prefix(database_id);
	let mut key =
		Vec::with_capacity(prefix.len() + DELTA_PATH.len() + std::mem::size_of::<u64>() + 1);
	key.extend_from_slice(&prefix);
	key.extend_from_slice(DELTA_PATH);
	key.extend_from_slice(&txid.to_be_bytes());
	key.push(b'/');
	key
}

pub fn delta_chunk_key(database_id: &str, txid: u64, chunk_idx: u32) -> Vec<u8> {
	let prefix = delta_chunk_prefix(database_id, txid);
	let mut key = Vec::with_capacity(prefix.len() + std::mem::size_of::<u32>());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(&chunk_idx.to_be_bytes());
	key
}

pub fn pidx_delta_key(database_id: &str, pgno: u32) -> Vec<u8> {
	let prefix = database_prefix(database_id);
	let mut key =
		Vec::with_capacity(prefix.len() + PIDX_DELTA_PATH.len() + std::mem::size_of::<u32>());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(PIDX_DELTA_PATH);
	key.extend_from_slice(&pgno.to_be_bytes());
	key
}

// Legacy database-scoped prefix kept for v1 pegboard actor cleanup.
pub fn pidx_delta_prefix(database_id: &str) -> Vec<u8> {
	let prefix = database_prefix(database_id);
	let mut key = Vec::with_capacity(prefix.len() + PIDX_DELTA_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(PIDX_DELTA_PATH);
	key
}

pub fn decode_delta_chunk_txid(database_id: &str, key: &[u8]) -> Result<u64> {
	let prefix = delta_prefix(database_id);
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

pub fn decode_delta_chunk_idx(database_id: &str, txid: u64, key: &[u8]) -> Result<u32> {
	let prefix = delta_chunk_prefix(database_id, txid);
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
