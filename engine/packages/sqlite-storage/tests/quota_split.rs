use std::sync::Arc;

use anyhow::Result;
use sqlite_storage::{
	keys::{actor_prefix, meta_storage_used_live_key, meta_storage_used_pitr_key},
	quota,
};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-quota-split-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

#[tokio::test]
async fn migrate_quota_split_first_run() -> Result<()> {
	let db = test_db().await?;
	let actor_id = "actor-a";

	db.run(move |tx| async move {
		tx.informal()
			.set(&legacy_meta_quota_key(actor_id), &1024i64.to_le_bytes());
		Ok(())
	})
	.await?;

	db.run(move |tx| async move { quota::migrate_quota_split(&tx, actor_id).await })
		.await?;

	let (live, pitr, legacy_present) = read_split_state(&db, actor_id).await?;
	assert_eq!(live, Some(1024));
	assert_eq!(pitr, Some(0));
	assert!(!legacy_present);

	Ok(())
}

#[tokio::test]
async fn migrate_quota_split_idempotent() -> Result<()> {
	let db = test_db().await?;
	let actor_id = "actor-a";

	db.run(move |tx| async move {
		tx.informal()
			.set(&legacy_meta_quota_key(actor_id), &1024i64.to_le_bytes());
		Ok(())
	})
	.await?;

	db.run(move |tx| async move { quota::migrate_quota_split(&tx, actor_id).await })
		.await?;
	db.run(move |tx| async move { quota::migrate_quota_split(&tx, actor_id).await })
		.await?;

	let (live, pitr, legacy_present) = read_split_state(&db, actor_id).await?;
	assert_eq!(live, Some(1024));
	assert_eq!(pitr, Some(0));
	assert!(!legacy_present);

	Ok(())
}

#[tokio::test]
async fn migrate_quota_split_fresh_actor() -> Result<()> {
	let db = test_db().await?;
	let actor_id = "fresh-actor";

	db.run(move |tx| async move { quota::migrate_quota_split(&tx, actor_id).await })
		.await?;

	let (live, pitr, legacy_present) = read_split_state(&db, actor_id).await?;
	assert_eq!(live, None);
	assert_eq!(pitr, None);
	assert!(!legacy_present);

	Ok(())
}

async fn read_split_state(
	db: &universaldb::Database,
	actor_id: &str,
) -> Result<(Option<i64>, Option<i64>, bool)> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();

		async move {
			let live = tx
				.informal()
				.get(&meta_storage_used_live_key(&actor_id), Snapshot)
				.await?
				.map(Vec::<u8>::from)
				.map(decode_i64)
				.transpose()?;
			let pitr = tx
				.informal()
				.get(&meta_storage_used_pitr_key(&actor_id), Snapshot)
				.await?
				.map(Vec::<u8>::from)
				.map(decode_i64)
				.transpose()?;
			let legacy_present = tx
				.informal()
				.get(&legacy_meta_quota_key(&actor_id), Snapshot)
				.await?
				.is_some();

			Ok((live, pitr, legacy_present))
		}
	})
	.await
}

fn legacy_meta_quota_key(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + b"/META/quota".len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(b"/META/quota");
	key
}

fn decode_i64(value: Vec<u8>) -> Result<i64> {
	Ok(i64::from_le_bytes(
		value
			.try_into()
			.map_err(|value: Vec<u8>| anyhow::anyhow!("invalid counter length {}", value.len()))?,
	))
}
