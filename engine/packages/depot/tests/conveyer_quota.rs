mod common;

use anyhow::Result;
use depot::quota::{
	SQLITE_MAX_STORAGE_BYTES, TRIGGER_MAX_SILENCE_MS, TRIGGER_THROTTLE_MS, atomic_add, cap_check,
	read,
};

#[tokio::test]
async fn quota_defaults_to_zero() -> Result<()> {
	common::test_matrix("depot-quota-defaults", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_id = ctx.database_id.clone();

			let storage_used = db
				.run(move |tx| {
					let database_id = database_id.clone();
					async move { read(&tx, &database_id).await }
				})
				.await?;

			assert_eq!(storage_used, 0);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn atomic_add_uses_signed_little_endian_counter() -> Result<()> {
	common::test_matrix("depot-quota-atomic-add", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_id = ctx.database_id.clone();

			db.run({
				let database_id = database_id.clone();
				move |tx| {
					let database_id = database_id.clone();
					async move {
						atomic_add(&tx, &database_id, 128);
						atomic_add(&tx, &database_id, -8);
						Ok(())
					}
				}
			})
			.await?;

			let storage_used = db
				.run(move |tx| {
					let database_id = database_id.clone();
					async move { read(&tx, &database_id).await }
				})
				.await?;

			assert_eq!(storage_used, 120);

			Ok(())
		})
	})
	.await
}

#[test]
fn cap_check_rejects_values_over_limit() {
	cap_check(SQLITE_MAX_STORAGE_BYTES).expect("limit should be accepted");

	let err = cap_check(SQLITE_MAX_STORAGE_BYTES + 64).expect_err("over limit should fail");
	let storage_err = err
		.downcast_ref::<depot::error::SqliteStorageError>()
		.expect("error should remain typed");

	assert_eq!(
		storage_err,
		&depot::error::SqliteStorageError::SqliteStorageQuotaExceeded {
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
