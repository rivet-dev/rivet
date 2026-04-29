use anyhow::{Context, Result};
use universaldb::{options::MutationType, utils::IsolationLevel::Snapshot};

use crate::pump::{error::SqliteStorageError, keys};

pub const SQLITE_MAX_STORAGE_LIVE_BYTES: i64 = 10 * 1024 * 1024 * 1024;
pub const SQLITE_MAX_STORAGE_BYTES: i64 = SQLITE_MAX_STORAGE_LIVE_BYTES;
pub const COMPACTION_DELTA_THRESHOLD: u64 = 32;
pub const TRIGGER_THROTTLE_MS: u64 = 500;
pub const TRIGGER_MAX_SILENCE_MS: u64 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageCounter {
	Live,
	Pitr,
}

pub fn atomic_add(
	tx: &universaldb::Transaction,
	actor_id: &str,
	counter: StorageCounter,
	delta_bytes: i64,
) {
	tx.informal().atomic_op(
		&counter_key(actor_id, counter),
		&delta_bytes.to_le_bytes(),
		MutationType::Add,
	);
}

pub fn atomic_add_live(tx: &universaldb::Transaction, actor_id: &str, delta_bytes: i64) {
	atomic_add(tx, actor_id, StorageCounter::Live, delta_bytes);
}

pub fn atomic_add_pitr(tx: &universaldb::Transaction, actor_id: &str, delta_bytes: i64) {
	atomic_add(tx, actor_id, StorageCounter::Pitr, delta_bytes);
}

pub async fn read(
	tx: &universaldb::Transaction,
	actor_id: &str,
	counter: StorageCounter,
) -> Result<i64> {
	let Some(value) = tx
		.informal()
		.get(&counter_key(actor_id, counter), Snapshot)
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

pub async fn read_live(tx: &universaldb::Transaction, actor_id: &str) -> Result<i64> {
	read(tx, actor_id, StorageCounter::Live).await
}

pub async fn read_pitr(tx: &universaldb::Transaction, actor_id: &str) -> Result<i64> {
	read(tx, actor_id, StorageCounter::Pitr).await
}

pub async fn migrate_quota_split(tx: &universaldb::Transaction, actor_id: &str) -> Result<()> {
	let live_key = keys::meta_storage_used_live_key(actor_id);
	let pitr_key = keys::meta_storage_used_pitr_key(actor_id);
	let legacy_key = legacy_meta_quota_key(actor_id);
	let informal = tx.informal();

	let (legacy_value, live_value, pitr_value) = tokio::try_join!(
		informal.get(&legacy_key, Snapshot),
		informal.get(&live_key, Snapshot),
		informal.get(&pitr_key, Snapshot),
	)?;

	if let Some(legacy_value) = legacy_value {
		if live_value.is_none() && pitr_value.is_none() {
			let legacy_storage_used = decode_counter_value(Vec::from(legacy_value))?;
			tx.informal()
				.set(&live_key, &legacy_storage_used.to_le_bytes());
			tx.informal().set(&pitr_key, &0i64.to_le_bytes());
		}
		tx.informal().clear(&legacy_key);
	}

	Ok(())
}

pub fn cap_check_live(would_be: i64) -> Result<()> {
	if would_be > SQLITE_MAX_STORAGE_LIVE_BYTES {
		return Err(SqliteStorageError::SqliteStorageQuotaExceeded {
			remaining_bytes: 0,
			payload_size: would_be
				.checked_sub(SQLITE_MAX_STORAGE_LIVE_BYTES)
				.context("sqlite quota excess overflowed i64")?,
		}
		.into());
	}

	Ok(())
}

pub fn cap_check(would_be: i64) -> Result<()> {
	cap_check_live(would_be)
}

fn counter_key(actor_id: &str, counter: StorageCounter) -> Vec<u8> {
	match counter {
		StorageCounter::Live => keys::meta_storage_used_live_key(actor_id),
		StorageCounter::Pitr => keys::meta_storage_used_pitr_key(actor_id),
	}
}

fn legacy_meta_quota_key(actor_id: &str) -> Vec<u8> {
	let prefix = keys::actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + b"/META/quota".len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(b"/META/quota");
	key
}

fn decode_counter_value(value: Vec<u8>) -> Result<i64> {
	let bytes: [u8; std::mem::size_of::<i64>()] =
		value.try_into().map_err(|value: Vec<u8>| {
			anyhow::anyhow!(
				"sqlite quota counter had {} bytes, expected {}",
				value.len(),
				std::mem::size_of::<i64>()
			)
		})?;

	Ok(i64::from_le_bytes(bytes))
}
