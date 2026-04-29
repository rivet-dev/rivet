use std::sync::Arc;

use anyhow::Result;
use sqlite_storage::{
	keys::{delta_chunk_key, meta_head_key, pidx_delta_key, shard_key},
	takeover,
	types::{DBHead, encode_db_head},
};
use tempfile::Builder;

const TEST_ACTOR: &str = "test-actor";

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-takeover-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn head(head_txid: u64, db_size_pages: u32) -> DBHead {
	DBHead {
		head_txid,
		db_size_pages,
		#[cfg(debug_assertions)]
		generation: 0,
	}
}

async fn seed(db: &universaldb::Database, writes: Vec<(Vec<u8>, Vec<u8>)>) -> Result<()> {
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

#[tokio::test]
async fn clean_state_passes() -> Result<()> {
	let db = test_db().await?;
	seed(
		&db,
		vec![
			(meta_head_key(TEST_ACTOR), encode_db_head(head(4, 128))?),
			(delta_chunk_key(TEST_ACTOR, 4, 0), b"delta".to_vec()),
			(pidx_delta_key(TEST_ACTOR, 2), 4_u64.to_be_bytes().to_vec()),
			(shard_key(TEST_ACTOR, 1), b"shard".to_vec()),
		],
	)
	.await?;

	takeover::reconcile(&db, TEST_ACTOR).await?;

	Ok(())
}

#[tokio::test]
#[should_panic(expected = "above_eof")]
async fn orphan_above_eof_panics() {
	let db = test_db().await.expect("db should build");
	seed(
		&db,
		vec![
			(
				meta_head_key(TEST_ACTOR),
				encode_db_head(head(4, 3)).expect("head should encode"),
			),
			(delta_chunk_key(TEST_ACTOR, 4, 0), b"delta".to_vec()),
			(pidx_delta_key(TEST_ACTOR, 4), 4_u64.to_be_bytes().to_vec()),
		],
	)
	.await
	.expect("seed should succeed");

	takeover::reconcile(&db, TEST_ACTOR)
		.await
		.expect("reconcile should panic before returning");
}

#[tokio::test]
#[should_panic(expected = "above_head_txid")]
async fn orphan_above_head_txid_panics() {
	let db = test_db().await.expect("db should build");
	seed(
		&db,
		vec![
			(
				meta_head_key(TEST_ACTOR),
				encode_db_head(head(4, 128)).expect("head should encode"),
			),
			(delta_chunk_key(TEST_ACTOR, 5, 0), b"delta".to_vec()),
		],
	)
	.await
	.expect("seed should succeed");

	takeover::reconcile(&db, TEST_ACTOR)
		.await
		.expect("reconcile should panic before returning");
}

#[tokio::test]
#[should_panic(expected = "dangling_pidx_ref")]
async fn dangling_pidx_ref_panics() {
	let db = test_db().await.expect("db should build");
	seed(
		&db,
		vec![
			(
				meta_head_key(TEST_ACTOR),
				encode_db_head(head(4, 128)).expect("head should encode"),
			),
			(pidx_delta_key(TEST_ACTOR, 2), 4_u64.to_be_bytes().to_vec()),
		],
	)
	.await
	.expect("seed should succeed");

	takeover::reconcile(&db, TEST_ACTOR)
		.await
		.expect("reconcile should panic before returning");
}
