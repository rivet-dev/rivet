use std::sync::Arc;

use anyhow::{Result, anyhow};
use gas::prelude::Id;
use depot::keys::{
	delta_chunk_key, meta_compact_key, meta_compactor_lease_key, meta_head_key, meta_quota_key,
	pidx_delta_key, shard_key,
};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new()
		.prefix("pegboard-sqlite-destroy-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn sqlite_keys(actor_id: Id) -> Vec<Vec<u8>> {
	let actor_id = actor_id.to_string();
	vec![
		meta_head_key(&actor_id),
		meta_compact_key(&actor_id),
		meta_quota_key(&actor_id),
		meta_compactor_lease_key(&actor_id),
		pidx_delta_key(&actor_id, 1),
		delta_chunk_key(&actor_id, 1, 0),
		shard_key(&actor_id, 0),
	]
}

async fn seed(db: &universaldb::Database, keys: &[Vec<u8>]) -> Result<()> {
	let writes = keys
		.iter()
		.cloned()
		.map(|key| (key, b"present".to_vec()))
		.collect::<Vec<_>>();
	db.run(move |tx| {
		let writes = writes.clone();
		async move {
			for (key, value) in writes {
				tx.informal().set(&key, &value);
			}
			Ok(())
		}
	})
	.await
}

async fn value_exists(db: &universaldb::Database, key: Vec<u8>) -> Result<bool> {
	db.run(move |tx| {
		let key = key.clone();
		async move { Ok(tx.informal().get(&key, Snapshot).await?.is_some()) }
	})
	.await
}

#[tokio::test]
async fn actor_destroy_clears_compactor_lease() -> Result<()> {
	let db = test_db().await?;
	let actor_id = Id::new_v1(1);
	let keys = sqlite_keys(actor_id);
	seed(&db, &keys).await?;

	db.run(move |tx| async move {
		pegboard::actor_sqlite::clear_v2_storage_for_destroy(&tx, actor_id);
		Ok(())
	})
	.await?;

	for key in keys {
		assert!(!value_exists(&db, key).await?);
	}

	Ok(())
}

#[tokio::test]
async fn actor_destroy_in_one_tx() -> Result<()> {
	let db = test_db().await?;
	let actor_id = Id::new_v1(1);
	let keys = sqlite_keys(actor_id);
	seed(&db, &keys).await?;

	db
		.run(move |tx| async move {
			pegboard::actor_sqlite::clear_v2_storage_for_destroy(&tx, actor_id);
			Err::<(), anyhow::Error>(anyhow!("rollback sqlite destroy"))
		})
		.await
		.expect_err("failed transaction should roll back sqlite clears");

	for key in keys {
		assert!(value_exists(&db, key).await?);
	}

	Ok(())
}
