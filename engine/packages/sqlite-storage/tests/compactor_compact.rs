use std::sync::Arc;

use anyhow::Result;
use gas::prelude::Id;
use namespace::keys::metric::{Metric, MetricKey};
use sqlite_storage::{
	compactor::{
		SqliteCompactPayload,
		compact::{test_hooks, validate_quota},
		compact_default_batch, fold_shard, worker,
	},
	keys::{
		PAGE_SIZE, delta_chunk_key, meta_compact_key, meta_head_key, pidx_delta_key, shard_key,
	},
	ltx::{LtxHeader, decode_ltx_v3, encode_ltx_v3},
	quota,
	types::{
		DBHead, DirtyPage, MetaCompact, decode_meta_compact, encode_db_head,
		encode_meta_compact,
	},
};
use tempfile::Builder;
use tokio_util::sync::CancellationToken;
use universaldb::{
	error::DatabaseError, options::DatabaseOption, utils::IsolationLevel::Snapshot,
};

const TEST_ACTOR: &str = "test-actor";
static COMPACTION_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-compact-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; PAGE_SIZE as usize],
	}
}

fn update(pgno: u32, fill: u8) -> (u32, Vec<u8>) {
	(pgno, vec![fill; PAGE_SIZE as usize])
}

fn encoded_blob(txid: u64, pages: &[(u32, u8)]) -> Result<Vec<u8>> {
	let pages = pages
		.iter()
		.map(|(pgno, fill)| page(*pgno, *fill))
		.collect::<Vec<_>>();

	encode_ltx_v3(LtxHeader::delta(txid, 128, 999), &pages)
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

async fn read_pidx_txid(db: &universaldb::Database, pgno: u32) -> Result<Option<u64>> {
	Ok(read_value(db, pidx_delta_key(TEST_ACTOR, pgno))
		.await?
		.map(|value| u64::from_be_bytes(value.try_into().expect("pidx txid should be u64"))))
}

async fn read_compact_txid(db: &universaldb::Database) -> Result<u64> {
	let bytes = read_value(db, meta_compact_key(TEST_ACTOR))
		.await?
		.expect("compact meta should exist");

	Ok(decode_meta_compact(&bytes)?.materialized_txid)
}

async fn read_quota(db: &universaldb::Database, actor_id: &str) -> Result<i64> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move { quota::read(&tx, &actor_id).await }
	})
	.await
}

#[derive(Clone, Copy)]
enum TestMetric {
	StorageUsed,
	CommitBytes,
	ReadBytes,
}

async fn read_sqlite_metric(
	db: &universaldb::Database,
	namespace_id: Id,
	actor_name: &str,
	metric: TestMetric,
) -> Result<i64> {
	let actor_name = actor_name.to_string();
	db.run(move |tx| {
		let actor_name = actor_name.clone();
		async move {
			let metric = match metric {
				TestMetric::StorageUsed => Metric::SqliteStorageUsed(actor_name),
				TestMetric::CommitBytes => Metric::SqliteCommitBytes(actor_name),
				TestMetric::ReadBytes => Metric::SqliteReadBytes(actor_name),
			};
			let tx = tx.with_subspace(namespace::keys::subspace());
			Ok(tx
				.read_opt(&MetricKey::new(namespace_id, metric), Snapshot)
				.await?
				.unwrap_or(0))
		}
	})
	.await
}

async fn read_shard(db: &universaldb::Database, shard_id: u32) -> Result<Vec<DirtyPage>> {
	let bytes = db
		.run(move |tx| async move {
			Ok(tx
				.informal()
				.get(&shard_key(TEST_ACTOR, shard_id), Snapshot)
				.await?
				.map(Vec::<u8>::from))
		})
		.await?
		.expect("shard blob should exist");

	Ok(decode_ltx_v3(&bytes)?.pages)
}

async fn fold(
	db: &universaldb::Database,
	shard_id: u32,
	updates: Vec<(u32, Vec<u8>)>,
) -> Result<()> {
	db.run(move |tx| {
		let updates = updates.clone();
		async move { fold_shard(&tx, TEST_ACTOR, shard_id, updates).await }
	})
	.await
}

async fn seed_compaction_case(
	db: &universaldb::Database,
	head_txid: u64,
	db_size_pages: u32,
	compact_txid: u64,
	deltas: &[(u64, Vec<(u32, u8)>)],
	pidx_rows: &[(u32, u64)],
) -> Result<()> {
	let mut writes = vec![
		(
			meta_head_key(TEST_ACTOR),
			encode_db_head(DBHead {
				head_txid,
				db_size_pages,
				#[cfg(debug_assertions)]
				generation: 0,
			})?,
		),
		(
			meta_compact_key(TEST_ACTOR),
			encode_meta_compact(MetaCompact {
				materialized_txid: compact_txid,
			})?,
		),
	];

	for (txid, pages) in deltas {
		writes.push((delta_chunk_key(TEST_ACTOR, *txid, 0), encoded_blob(*txid, pages)?));
	}
	for (pgno, txid) in pidx_rows {
		writes.push((pidx_delta_key(TEST_ACTOR, *pgno), txid.to_be_bytes().to_vec()));
	}

	seed(db, writes).await
}

async fn seed_quota(db: &universaldb::Database, actor_id: &str, storage_used: i64) -> Result<()> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			quota::atomic_add(&tx, &actor_id, storage_used);
			Ok(())
		}
	})
	.await
}

fn tracked_entry_size(key: &[u8], value: &[u8]) -> i64 {
	i64::try_from(key.len() + value.len()).expect("tracked entry should fit in i64")
}

async fn write_newer_page(db: &universaldb::Database, pgno: u32, txid: u64, fill: u8) -> Result<()> {
	db.run(move |tx| async move {
		tx.informal().set(
			&delta_chunk_key(TEST_ACTOR, txid, 0),
			&encoded_blob(txid, &[(pgno, fill)])?,
		);
		tx.informal()
			.set(&pidx_delta_key(TEST_ACTOR, pgno), &txid.to_be_bytes());
		tx.informal().set(
			&meta_head_key(TEST_ACTOR),
			&encode_db_head(DBHead {
				head_txid: txid,
				db_size_pages: 128,
				#[cfg(debug_assertions)]
				generation: 0,
			})?,
		);
		Ok(())
	})
	.await
}

async fn shrink_head(db: &universaldb::Database, head_txid: u64, db_size_pages: u32) -> Result<()> {
	db.run(move |tx| async move {
		tx.informal()
			.clear(&pidx_delta_key(TEST_ACTOR, db_size_pages + 60));
		tx.informal().set(
			&meta_head_key(TEST_ACTOR),
			&encode_db_head(DBHead {
				head_txid,
				db_size_pages,
				#[cfg(debug_assertions)]
				generation: 0,
			})?,
		);
		Ok(())
	})
	.await
}

fn assert_pages(actual: &[DirtyPage], expected: &[(u32, u8)]) {
	let expected = expected
		.iter()
		.map(|(pgno, fill)| page(*pgno, *fill))
		.collect::<Vec<_>>();
	assert_eq!(actual, expected.as_slice());
}

#[tokio::test]
async fn fold_into_empty_shard() -> Result<()> {
	let db = test_db().await?;

	fold(&db, 0, vec![update(3, 0x33), update(5, 0x55)]).await?;

	assert_pages(&read_shard(&db, 0).await?, &[(3, 0x33), (5, 0x55)]);
	Ok(())
}

#[tokio::test]
async fn fold_into_existing_shard_newer_wins() -> Result<()> {
	let db = test_db().await?;
	seed(
		&db,
		vec![(
			shard_key(TEST_ACTOR, 0),
			encoded_blob(1, &[(3, 0x13), (5, 0x15)])?,
		)],
	)
	.await?;

	fold(&db, 0, vec![update(3, 0x23), update(7, 0x17)]).await?;

	assert_pages(
		&read_shard(&db, 0).await?,
		&[(3, 0x23), (5, 0x15), (7, 0x17)],
	);
	Ok(())
}

#[tokio::test]
async fn fold_overwrite_all_pages() -> Result<()> {
	let db = test_db().await?;
	let existing = (64..128)
		.map(|pgno| (pgno, 0x10))
		.collect::<Vec<_>>();
	let updates = (64..128)
		.map(|pgno| update(pgno, 0x20))
		.collect::<Vec<_>>();
	let expected = (64..128)
		.map(|pgno| (pgno, 0x20))
		.collect::<Vec<_>>();
	seed(
		&db,
		vec![(shard_key(TEST_ACTOR, 1), encoded_blob(1, &existing)?)],
	)
	.await?;

	fold(&db, 1, updates).await?;

	assert_pages(&read_shard(&db, 1).await?, &expected);
	Ok(())
}

#[tokio::test]
async fn fold_partial_shard_keeps_unmodified_pages() -> Result<()> {
	let db = test_db().await?;
	let existing = (64..128)
		.map(|pgno| (pgno, pgno as u8))
		.collect::<Vec<_>>();
	let mut expected = existing.clone();
	expected[32] = (96, 0xee);
	seed(
		&db,
		vec![(shard_key(TEST_ACTOR, 1), encoded_blob(1, &existing)?)],
	)
	.await?;

	fold(&db, 1, vec![update(96, 0xee)]).await?;

	assert_pages(&read_shard(&db, 1).await?, &expected);
	Ok(())
}

#[tokio::test]
async fn fold_byte_count_metric() -> Result<()> {
	let db = test_db().await?;

	fold(
		&db,
		0,
		vec![update(3, 0x33), update(5, 0x55), update(5, 0x66)],
	)
	.await?;

	let pages = read_shard(&db, 0).await?;
	assert_eq!(pages.len(), 2);
	assert_eq!(
		pages.iter().map(|page| page.bytes.len()).sum::<usize>(),
		2 * PAGE_SIZE as usize
	);
	assert_pages(&pages, &[(3, 0x33), (5, 0x66)]);
	Ok(())
}

#[tokio::test]
async fn compact_default_batch_basic_fold() -> Result<()> {
	let _compaction_test_lock = COMPACTION_TEST_LOCK.lock().await;
	let db = test_db().await?;
	seed_compaction_case(
		&db,
		2,
		128,
		0,
		&[(1, vec![(3, 0x13), (5, 0x15)]), (2, vec![(70, 0x70)])],
		&[(3, 1), (5, 1), (70, 2)],
	)
	.await?;

	let outcome = compact_default_batch(
		Arc::new(db.clone()),
		TEST_ACTOR.to_string(),
		10,
		CancellationToken::new(),
	)
	.await?;

	assert_eq!(outcome.pages_folded, 3);
	assert_eq!(outcome.deltas_freed, 2);
	assert_eq!(outcome.compare_and_clear_noops, 0);
	assert_eq!(outcome.materialized_txid, 2);
	assert_pages(&read_shard(&db, 0).await?, &[(3, 0x13), (5, 0x15)]);
	assert_pages(&read_shard(&db, 1).await?, &[(70, 0x70)]);
	assert_eq!(read_pidx_txid(&db, 3).await?, None);
	assert_eq!(read_pidx_txid(&db, 5).await?, None);
	assert_eq!(read_pidx_txid(&db, 70).await?, None);
	assert!(
		read_value(&db, delta_chunk_key(TEST_ACTOR, 1, 0))
			.await?
			.is_none()
	);
	assert!(
		read_value(&db, delta_chunk_key(TEST_ACTOR, 2, 0))
			.await?
			.is_none()
	);
	assert_eq!(read_compact_txid(&db).await?, 2);

	Ok(())
}

#[tokio::test]
async fn validate_quota_accepts_clean_compacted_state() -> Result<()> {
	let db = test_db().await?;
	let shard_key = shard_key(TEST_ACTOR, 0);
	let shard_blob = encoded_blob(1, &[(3, 0x13), (5, 0x15)])?;
	let storage_used = tracked_entry_size(&shard_key, &shard_blob);
	seed(&db, vec![(shard_key, shard_blob)]).await?;
	seed_quota(&db, TEST_ACTOR, storage_used).await?;

	validate_quota(Arc::new(db), TEST_ACTOR.to_string()).await?;

	Ok(())
}

#[tokio::test]
async fn compact_compare_and_clear_noop_keeps_newer_pidx() -> Result<()> {
	let _compaction_test_lock = COMPACTION_TEST_LOCK.lock().await;
	let db = test_db().await?;
	seed_compaction_case(
		&db,
		1,
		128,
		0,
		&[(1, vec![(3, 0x13)])],
		&[(3, 1)],
	)
	.await?;
	let (_guard, reached, release) = test_hooks::pause_after_plan(TEST_ACTOR);
	let task = tokio::spawn({
		let db = Arc::new(db.clone());
		async move {
			compact_default_batch(
				db,
				TEST_ACTOR.to_string(),
				10,
				CancellationToken::new(),
			)
			.await
		}
	});

	reached.notified().await;
	write_newer_page(&db, 3, 2, 0x23).await?;
	release.notify_waiters();

	let outcome = task.await??;
	assert_eq!(outcome.pages_folded, 1);
	assert_eq!(outcome.deltas_freed, 1);
	assert_eq!(outcome.compare_and_clear_noops, 1);
	assert_eq!(read_pidx_txid(&db, 3).await?, Some(2));
	assert!(
		read_value(&db, delta_chunk_key(TEST_ACTOR, 1, 0))
			.await?
			.is_none()
	);
	assert!(
		read_value(&db, delta_chunk_key(TEST_ACTOR, 2, 0))
			.await?
			.is_some()
	);
	assert_pages(&read_shard(&db, 0).await?, &[(3, 0x13)]);

	Ok(())
}

#[tokio::test]
async fn compact_conflicts_with_concurrent_shrink_after_head_read() -> Result<()> {
	let _compaction_test_lock = COMPACTION_TEST_LOCK.lock().await;
	let db = test_db().await?;
	db.set_option(DatabaseOption::TransactionRetryLimit(1))?;
	seed_compaction_case(
		&db,
		1,
		128,
		0,
		&[(1, vec![(70, 0x70)])],
		&[(70, 1)],
	)
	.await?;
	let (_guard, reached, release) = test_hooks::pause_after_write_head_read(TEST_ACTOR);
	let task = tokio::spawn({
		let db = Arc::new(db.clone());
		async move {
			compact_default_batch(
				db,
				TEST_ACTOR.to_string(),
				10,
				CancellationToken::new(),
			)
			.await
		}
	});

	reached.notified().await;
	shrink_head(&db, 2, 10).await?;
	release.notify_waiters();

	let err = task.await?.expect_err("compaction should hit an OCC retry limit");
	assert!(err.chain().any(|cause| {
		cause
			.downcast_ref::<DatabaseError>()
			.is_some_and(|err| matches!(err, DatabaseError::MaxRetriesReached))
	}));

	Ok(())
}

#[tokio::test]
async fn compact_trigger_rolls_up_sqlite_metering_metrics() -> Result<()> {
	let _compaction_test_lock = COMPACTION_TEST_LOCK.lock().await;
	let db = Arc::new(test_db().await?);
	let actor_id = TEST_ACTOR;
	let actor_name = "metered-actor";
	let namespace_id = Id::new_v1(42);
	let commit_bytes = util::metric::KV_BILLABLE_CHUNK * 3 + 123;
	let read_bytes = util::metric::KV_BILLABLE_CHUNK * 2 + 456;

	seed_compaction_case(
		&db,
		1,
		128,
		0,
		&[(1, vec![(3, 0x13), (5, 0x15)])],
		&[(3, 1), (5, 1)],
	)
	.await?;
	seed_quota(&db, actor_id, 1_000_000).await?;

	worker::test_hooks::handle_payload_once(
		Arc::clone(&db),
		SqliteCompactPayload {
			actor_id: actor_id.to_string(),
			namespace_id: Some(namespace_id),
			actor_name: Some(actor_name.to_string()),
			commit_bytes_since_rollup: commit_bytes,
			read_bytes_since_rollup: read_bytes,
		},
		worker::CompactorConfig::default(),
		CancellationToken::new(),
	)
	.await?;

	let storage_used = read_quota(&db, actor_id).await?;
	assert_eq!(
		read_sqlite_metric(&db, namespace_id, actor_name, TestMetric::StorageUsed).await?,
		storage_used,
	);
	assert_eq!(
		read_sqlite_metric(&db, namespace_id, actor_name, TestMetric::CommitBytes).await?,
		(commit_bytes / util::metric::KV_BILLABLE_CHUNK * util::metric::KV_BILLABLE_CHUNK)
			as i64,
	);
	assert_eq!(
		read_sqlite_metric(&db, namespace_id, actor_name, TestMetric::ReadBytes).await?,
		(read_bytes / util::metric::KV_BILLABLE_CHUNK * util::metric::KV_BILLABLE_CHUNK) as i64,
	);

	Ok(())
}
