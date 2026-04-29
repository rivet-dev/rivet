use std::sync::Arc;

use anyhow::Result;
use sqlite_storage::quota::{
	SQLITE_MAX_STORAGE_BYTES, TRIGGER_MAX_SILENCE_MS, TRIGGER_THROTTLE_MS, atomic_add_live,
	atomic_add_pitr, cap_check, read_live, read_pitr,
};
use tempfile::Builder;

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-quota-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

#[tokio::test]
async fn quota_defaults_to_zero() -> Result<()> {
	let db = test_db().await?;

	let storage_used = db
		.run(|tx| async move { read_live(&tx, "actor-a").await })
		.await?;
	let pitr_used = db
		.run(|tx| async move { read_pitr(&tx, "actor-a").await })
		.await?;

	assert_eq!(storage_used, 0);
	assert_eq!(pitr_used, 0);

	Ok(())
}

#[tokio::test]
async fn atomic_add_uses_signed_little_endian_counter() -> Result<()> {
	let db = test_db().await?;

	db.run(|tx| async move {
		atomic_add_live(&tx, "actor-a", 128);
		atomic_add_live(&tx, "actor-a", -8);
		atomic_add_pitr(&tx, "actor-a", 16);
		Ok(())
	})
	.await?;

	let storage_used = db
		.run(|tx| async move { read_live(&tx, "actor-a").await })
		.await?;
	let pitr_used = db
		.run(|tx| async move { read_pitr(&tx, "actor-a").await })
		.await?;

	assert_eq!(storage_used, 120);
	assert_eq!(pitr_used, 16);

	Ok(())
}

#[test]
fn cap_check_rejects_values_over_limit() {
	cap_check(SQLITE_MAX_STORAGE_BYTES).expect("limit should be accepted");

	let err = cap_check(SQLITE_MAX_STORAGE_BYTES + 64).expect_err("over limit should fail");
	let storage_err = err
		.downcast_ref::<sqlite_storage::error::SqliteStorageError>()
		.expect("error should remain typed");

	assert_eq!(
		storage_err,
		&sqlite_storage::error::SqliteStorageError::SqliteStorageQuotaExceeded {
			remaining_bytes: 0,
			payload_size: 64,
		}
	);
}

#[test]
fn trigger_throttle_constants_match_spec() {
	assert_eq!(TRIGGER_THROTTLE_MS, 500);
	assert_eq!(TRIGGER_MAX_SILENCE_MS, 30_000);
}
