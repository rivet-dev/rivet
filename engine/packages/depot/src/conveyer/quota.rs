use anyhow::{Context, Error, Result};
use universaldb::{options::MutationType, utils::IsolationLevel::Snapshot};

use crate::conveyer::{
	branch,
	error::SqliteStorageError,
	keys,
	types::{BucketId, DatabaseBranchId},
};

pub const SQLITE_MAX_STORAGE_BYTES: i64 = 10 * 1024 * 1024 * 1024;
pub const COMPACTION_DELTA_THRESHOLD: u64 = 32;
pub const TRIGGER_THROTTLE_MS: u64 = 500;
pub const TRIGGER_MAX_SILENCE_MS: u64 = 30_000;

pub fn atomic_add(tx: &universaldb::Transaction, database_id: &str, delta_bytes: i64) {
	tx.informal().atomic_op(
		&keys::meta_quota_key(database_id),
		&delta_bytes.to_le_bytes(),
		MutationType::Add,
	);
}

pub fn atomic_add_branch(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	delta_bytes: i64,
) {
	tx.informal().atomic_op(
		&keys::branch_meta_quota_key(branch_id),
		&delta_bytes.to_le_bytes(),
		MutationType::Add,
	);
}

pub async fn read(tx: &universaldb::Transaction, database_id: &str) -> Result<i64> {
	read_in_bucket(tx, BucketId::nil(), database_id).await
}

pub async fn read_in_bucket(
	tx: &universaldb::Transaction,
	bucket_id: BucketId,
	database_id: &str,
) -> Result<i64> {
	if let Some(branch_id) = branch::resolve_database_branch(tx, bucket_id, database_id, Snapshot)
		.await
		.context("resolve sqlite database branch for quota read")?
	{
		return read_branch(tx, branch_id).await;
	}

	let Some(value) = tx
		.informal()
		.get(&keys::meta_quota_key(database_id), Snapshot)
		.await?
	else {
		return Ok(0);
	};

	let bytes: [u8; std::mem::size_of::<i64>()] =
		Vec::from(value).try_into().map_err(|value: Vec<u8>| {
			Error::msg(format!(
				"sqlite quota counter had {} bytes, expected {}",
				value.len(),
				std::mem::size_of::<i64>()
			))
		})?;

	Ok(i64::from_le_bytes(bytes))
}

pub async fn read_branch(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
) -> Result<i64> {
	let Some(value) = tx
		.informal()
		.get(&keys::branch_meta_quota_key(branch_id), Snapshot)
		.await?
	else {
		return Ok(0);
	};

	let bytes: [u8; std::mem::size_of::<i64>()] =
		Vec::from(value).try_into().map_err(|value: Vec<u8>| {
			Error::msg(format!(
				"sqlite branch quota counter had {} bytes, expected {}",
				value.len(),
				std::mem::size_of::<i64>()
			))
		})?;

	Ok(i64::from_le_bytes(bytes))
}

pub fn cap_check(would_be: i64) -> Result<()> {
	cap_check_with_cap(would_be, SQLITE_MAX_STORAGE_BYTES)
}

pub fn cap_check_with_cap(would_be: i64, cap_bytes: i64) -> Result<()> {
	if would_be > cap_bytes {
		return Err(SqliteStorageError::SqliteStorageQuotaExceeded {
			remaining_bytes: 0,
			payload_size: would_be
				.checked_sub(cap_bytes)
				.context("sqlite quota excess overflowed i64")?,
		}
		.into());
	}

	Ok(())
}
