use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::compactor::{
	SqliteCompactSubject, decode_compact_payload,
};
use sqlite_storage::{
	keys::{
		PAGE_SIZE, delta_chunk_key, meta_compact_key, meta_head_key, pidx_delta_key, shard_key,
	},
	ltx::{LtxHeader, encode_ltx_v3},
	pump::ActorDb,
	quota::{self, SQLITE_MAX_STORAGE_BYTES},
	types::{
		DBHead, DirtyPage, FetchedPage, MetaCompact, decode_db_head, encode_db_head,
		encode_meta_compact,
	},
};
use tempfile::Builder;
use universalpubsub::{NextOutput, PubSub, driver::memory::MemoryDriver};
use universaldb::utils::IsolationLevel::Snapshot;

const TEST_ACTOR: &str = "test-actor";

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-commit-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"sqlite-storage-pump-commit-test".to_string(),
	)))
}

fn head(head_txid: u64, db_size_pages: u32) -> DBHead {
	DBHead {
		head_txid,
		db_size_pages,
		#[cfg(debug_assertions)]
		generation: 0,
	}
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; PAGE_SIZE as usize],
	}
}

fn fetched_page(pgno: u32, fill: u8) -> FetchedPage {
	FetchedPage {
		pgno,
		bytes: Some(vec![fill; PAGE_SIZE as usize]),
	}
}

fn encoded_blob(txid: u64, pages: &[(u32, u8)]) -> Result<Vec<u8>> {
	let pages = pages
		.iter()
		.map(|(pgno, fill)| page(*pgno, *fill))
		.collect::<Vec<_>>();

	encode_ltx_v3(LtxHeader::delta(txid, 1, 999), &pages)
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

async fn read_value(db: &universaldb::Database, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
	db.run(move |tx| {
		let key = key.clone();
		async move {
			Ok(tx
				.informal()
				.get(&key, Snapshot)
				.await?
				.map(Vec::<u8>::from))
		}
	})
	.await
}

async fn read_head(db: &universaldb::Database) -> Result<DBHead> {
	let bytes = read_value(db, meta_head_key(TEST_ACTOR))
		.await?
		.expect("head should exist");
	decode_db_head(&bytes)
}

async fn read_quota(db: &universaldb::Database) -> Result<i64> {
	db.run(|tx| async move { quota::read(&tx, TEST_ACTOR).await })
		.await
}

#[tokio::test]
async fn commit_lazily_initializes_meta_on_first_write() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = ActorDb::new(db.clone(), test_ups(), TEST_ACTOR.to_string(), NodeId::new());

	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;

	assert_eq!(read_head(&db).await?, head(1, 2));
	assert_eq!(
		read_value(&db, pidx_delta_key(TEST_ACTOR, 1)).await?,
		Some(1_u64.to_be_bytes().to_vec())
	);
	assert!(
		read_value(&db, delta_chunk_key(TEST_ACTOR, 1, 0))
			.await?
			.is_some()
	);
	assert!(read_quota(&db).await? > 0);
	assert_eq!(
		actor_db.get_pages(vec![1]).await?,
		vec![fetched_page(1, 0x11)]
	);

	Ok(())
}

#[tokio::test]
async fn commit_advances_head_and_updates_warm_cache() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = ActorDb::new(db.clone(), test_ups(), TEST_ACTOR.to_string(), NodeId::new());

	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	assert_eq!(
		actor_db.get_pages(vec![1]).await?,
		vec![fetched_page(1, 0x11)]
	);

	actor_db.commit(vec![page(2, 0x22)], 3, 2_000).await?;

	assert_eq!(read_head(&db).await?, head(2, 3));
	assert_eq!(
		read_value(&db, pidx_delta_key(TEST_ACTOR, 1)).await?,
		Some(1_u64.to_be_bytes().to_vec())
	);
	assert_eq!(
		read_value(&db, pidx_delta_key(TEST_ACTOR, 2)).await?,
		Some(2_u64.to_be_bytes().to_vec())
	);

	db.run(|tx| async move {
		tx.informal().clear(&pidx_delta_key(TEST_ACTOR, 2));
		Ok(())
	})
	.await?;
	assert_eq!(
		actor_db.get_pages(vec![1, 2]).await?,
		vec![fetched_page(1, 0x11), fetched_page(2, 0x22)]
	);

	Ok(())
}

#[tokio::test]
async fn commit_rejects_quota_cap_before_writes() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed(
		&db,
		vec![(
			meta_head_key(TEST_ACTOR),
			encode_db_head(head(4, 1)).expect("head should encode"),
		)],
	)
	.await?;
	db.run(|tx| async move {
		quota::atomic_add(&tx, TEST_ACTOR, SQLITE_MAX_STORAGE_BYTES - 10);
		Ok(())
	})
	.await?;

	let actor_db = ActorDb::new(db.clone(), test_ups(), TEST_ACTOR.to_string(), NodeId::new());
	let err = actor_db
		.commit(vec![page(1, 0x44)], 1, 3_000)
		.await
		.expect_err("commit should exceed quota");

	assert!(
		err.downcast_ref::<sqlite_storage::error::SqliteStorageError>()
			.is_some_and(|err| matches!(
				err,
				sqlite_storage::error::SqliteStorageError::SqliteStorageQuotaExceeded { .. }
			))
	);
	assert_eq!(read_head(&db).await?, head(4, 1));
	assert!(
		read_value(&db, delta_chunk_key(TEST_ACTOR, 5, 0))
			.await?
			.is_none()
	);

	Ok(())
}

#[tokio::test]
async fn shrink_commit_deletes_above_eof_pidx_and_shards() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed(
		&db,
		vec![
			(meta_head_key(TEST_ACTOR), encode_db_head(head(7, 130))?),
			(
				delta_chunk_key(TEST_ACTOR, 7, 0),
				encoded_blob(7, &[(64, 0x64), (129, 0x81)])?,
			),
			(pidx_delta_key(TEST_ACTOR, 64), 7_u64.to_be_bytes().to_vec()),
			(pidx_delta_key(TEST_ACTOR, 129), 7_u64.to_be_bytes().to_vec()),
			(shard_key(TEST_ACTOR, 1), encoded_blob(7, &[(64, 0x64)])?),
			(shard_key(TEST_ACTOR, 2), encoded_blob(7, &[(129, 0x81)])?),
		],
	)
	.await?;
	db.run(|tx| async move {
		quota::atomic_add(&tx, TEST_ACTOR, 50_000);
		Ok(())
	})
	.await?;

	let actor_db = ActorDb::new(db.clone(), test_ups(), TEST_ACTOR.to_string(), NodeId::new());
	actor_db.commit(vec![page(1, 0x11)], 63, 4_000).await?;

	assert_eq!(read_head(&db).await?, head(8, 63));
	assert!(read_value(&db, pidx_delta_key(TEST_ACTOR, 64)).await?.is_none());
	assert!(
		read_value(&db, pidx_delta_key(TEST_ACTOR, 129))
			.await?
			.is_none()
	);
	assert!(read_value(&db, shard_key(TEST_ACTOR, 1)).await?.is_none());
	assert!(read_value(&db, shard_key(TEST_ACTOR, 2)).await?.is_none());
	assert_eq!(
		read_value(&db, pidx_delta_key(TEST_ACTOR, 1)).await?,
		Some(8_u64.to_be_bytes().to_vec())
	);

	Ok(())
}

#[tokio::test(start_paused = true)]
async fn commit_publishes_compaction_trigger_with_throttle() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let ups = test_ups();
	let mut sub = ups.queue_subscribe(SqliteCompactSubject, "compactor").await?;
	seed(
		&db,
		vec![
			(meta_head_key(TEST_ACTOR), encode_db_head(head(31, 1))?),
			(
				meta_compact_key(TEST_ACTOR),
				encode_meta_compact(MetaCompact {
					materialized_txid: 0,
				})?,
			),
		],
	)
	.await?;
	db.run(|tx| async move {
		quota::atomic_add(&tx, TEST_ACTOR, 1_000);
		Ok(())
	})
	.await?;

	let actor_db = ActorDb::new(db, ups, TEST_ACTOR.to_string(), NodeId::new());
	actor_db.commit(vec![page(1, 0x11)], 1, 5_000).await?;
	let first = next_trigger(&mut sub).await?;
	assert_eq!(first.actor_id, TEST_ACTOR);
	assert!(first.commit_bytes_since_rollup > 0);

	actor_db.commit(vec![page(1, 0x22)], 1, 5_100).await?;
	assert_no_trigger(&mut sub).await?;

	tokio::time::advance(Duration::from_millis(quota::TRIGGER_MAX_SILENCE_MS + 1)).await;
	actor_db.commit(vec![page(1, 0x33)], 1, 5_200).await?;
	let after_silence = next_trigger(&mut sub).await?;
	assert_eq!(after_silence.actor_id, TEST_ACTOR);

	Ok(())
}

async fn next_trigger(
	sub: &mut universalpubsub::Subscriber,
) -> Result<sqlite_storage::compactor::SqliteCompactPayload> {
	let msg = tokio::time::timeout(Duration::from_secs(1), sub.next())
		.await?
		.expect("subscriber should receive");
	let NextOutput::Message(msg) = msg else {
		panic!("subscriber unexpectedly unsubscribed");
	};

	decode_compact_payload(&msg.payload)
}

async fn assert_no_trigger(sub: &mut universalpubsub::Subscriber) -> Result<()> {
	let trigger = tokio::time::timeout(Duration::from_millis(1), sub.next()).await;
	assert!(trigger.is_err(), "trigger should be throttled");

	Ok(())
}
