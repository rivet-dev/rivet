use anyhow::{Context, Result};
use universaldb::{options::MutationType, utils::IsolationLevel::Snapshot};

use crate::pump::{error::SqliteStorageError, keys};

pub const SQLITE_MAX_STORAGE_BYTES: i64 = 10 * 1024 * 1024 * 1024;
pub const COMPACTION_DELTA_THRESHOLD: u64 = 32;
pub const TRIGGER_THROTTLE_MS: u64 = 500;
pub const TRIGGER_MAX_SILENCE_MS: u64 = 30_000;

pub fn atomic_add(tx: &universaldb::Transaction, actor_id: &str, delta_bytes: i64) {
	tx.informal().atomic_op(
		&keys::meta_quota_key(actor_id),
		&delta_bytes.to_le_bytes(),
		MutationType::Add,
	);
}

pub async fn read(tx: &universaldb::Transaction, actor_id: &str) -> Result<i64> {
	let Some(value) = tx
		.informal()
		.get(&keys::meta_quota_key(actor_id), Snapshot)
		.await?
	else {
		return Ok(0);
	};

	let bytes: [u8; std::mem::size_of::<i64>()] = Vec::from(value)
		.try_into()
		.map_err(|value: Vec<u8>| {
			anyhow::anyhow!(
				"sqlite quota counter had {} bytes, expected {}",
				value.len(),
				std::mem::size_of::<i64>()
			)
		})?;

	Ok(i64::from_le_bytes(bytes))
}

pub fn cap_check(would_be: i64) -> Result<()> {
	if would_be > SQLITE_MAX_STORAGE_BYTES {
		return Err(SqliteStorageError::SqliteStorageQuotaExceeded {
			remaining_bytes: 0,
			payload_size: would_be
				.checked_sub(SQLITE_MAX_STORAGE_BYTES)
				.context("sqlite quota excess overflowed i64")?,
		}
		.into());
	}

	Ok(())
}
