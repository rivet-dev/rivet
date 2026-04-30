use std::sync::Arc;

use anyhow::Result;
use gas::prelude::Id;
use namespace::keys::metric::{Metric, MetricKey};
use rivet_pools::NodeId;
use sqlite_storage::{
	compactor::{
		SqliteCompactPayload,
		compact::{test_hooks, validate_quota},
		compact_default_batch, fold_shard, worker,
	},
	constants::{HOT_RETENTION_FLOOR_MS, MAX_SHARD_VERSIONS_PER_SHARD},
	error::SqliteStorageError,
	keys::{
		PAGE_SIZE, database_pointer_cur_key, branch_commit_key,
		branch_manifest_last_hot_pass_txid_key, branch_vtx_key, branches_bk_pin_key,
		delta_chunk_key, meta_compact_key, meta_head_key, namespace_pointer_cur_key,
		pidx_delta_key, shard_key, shard_version_key, vtx_key,
	},
	ltx::{LtxHeader, decode_ltx_v3, encode_ltx_v3},
	pump::Db,
	quota,
	types::{
		DatabaseBranchId, BookmarkStr, CommitRow, DBHead, DirtyPage, MetaCompact, NamespaceId,
		decode_database_pointer, decode_commit_row, decode_meta_compact, decode_namespace_pointer,
		encode_db_head, encode_meta_compact,
	},
};
use tempfile::Builder;
use tokio_util::sync::CancellationToken;
use universalpubsub::{PubSub, driver::memory::MemoryDriver};
use universaldb::{
	error::DatabaseError, options::DatabaseOption, utils::IsolationLevel::Snapshot,
};

const TEST_DATABASE: &str = "test-database";
static COMPACTION_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-compact-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"sqlite-storage-compact-test".to_string(),
	)))
}

fn nil_namespace() -> Id {
	Id::v1(uuid::Uuid::nil(), 1)
}

fn other_namespace() -> Id {
	Id::v1(uuid::Uuid::from_u128(1), 1)
}

fn now_ms() -> i64 {
	let elapsed = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.expect("system clock should be after unix epoch");
	i64::try_from(elapsed.as_millis()).expect("timestamp should fit i64")
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

async fn read_manifest_last_hot_pass_txid(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
) -> Result<Option<u64>> {
	Ok(read_value(db, branch_manifest_last_hot_pass_txid_key(branch_id))
		.await?
		.map(|value| u64::from_be_bytes(value.try_into().expect("manifest txid should be u64"))))
}

async fn read_branch_id(db: &universaldb::Database) -> Result<DatabaseBranchId> {
	read_branch_id_for(db, nil_namespace(), TEST_DATABASE).await
}

async fn read_branch_id_for(
	db: &universaldb::Database,
	namespace_id: Id,
	database_id: &str,
) -> Result<DatabaseBranchId> {
	let namespace_id = NamespaceId::from_gas_id(namespace_id);
	let namespace_pointer_bytes = read_value(db, namespace_pointer_cur_key(namespace_id))
		.await?
		.expect("namespace pointer should exist");
	let namespace_branch = decode_namespace_pointer(&namespace_pointer_bytes)?.current_branch;
	let database_pointer_bytes = read_value(db, database_pointer_cur_key(namespace_branch, database_id))
		.await?
		.expect("database pointer should exist");

	Ok(decode_database_pointer(&database_pointer_bytes)?.current_branch)
}

async fn read_commit_row(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
	txid: u64,
) -> Result<Option<CommitRow>> {
	Ok(read_value(db, branch_commit_key(branch_id, txid))
		.await?
		.map(|value| decode_commit_row(&value))
		.transpose()?)
}

async fn read_pidx_txid(db: &universaldb::Database, pgno: u32) -> Result<Option<u64>> {
	Ok(read_value(db, pidx_delta_key(TEST_DATABASE, pgno))
		.await?
		.map(|value| u64::from_be_bytes(value.try_into().expect("pidx txid should be u64"))))
}

async fn read_compact_txid(db: &universaldb::Database) -> Result<u64> {
	let bytes = read_value(db, meta_compact_key(TEST_DATABASE))
		.await?
		.expect("compact meta should exist");

	Ok(decode_meta_compact(&bytes)?.materialized_txid)
}

async fn read_branch_compact_txid(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
) -> Result<u64> {
	let bytes = read_value(db, sqlite_storage::keys::branch_meta_compact_key(branch_id))
		.await?
		.expect("branch compact meta should exist");

	Ok(decode_meta_compact(&bytes)?.materialized_txid)
}

async fn read_quota(db: &universaldb::Database, database_id: &str) -> Result<i64> {
	let database_id = database_id.to_string();
	db.run(move |tx| {
		let database_id = database_id.clone();
		async move { quota::read(&tx, &database_id).await }
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
	database_name: &str,
	metric: TestMetric,
) -> Result<i64> {
	let database_name = database_name.to_string();
	db.run(move |tx| {
		let database_name = database_name.clone();
		async move {
			let metric = match metric {
				TestMetric::StorageUsed => Metric::SqliteStorageUsed(database_name),
				TestMetric::CommitBytes => Metric::SqliteCommitBytes(database_name),
				TestMetric::ReadBytes => Metric::SqliteReadBytes(database_name),
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

async fn read_shard(
	db: &universaldb::Database,
	shard_id: u32,
	as_of_txid: u64,
) -> Result<Vec<DirtyPage>> {
	let bytes = db
		.run(move |tx| async move {
			Ok(tx
				.informal()
				.get(&shard_version_key(TEST_DATABASE, shard_id, as_of_txid), Snapshot)
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
	as_of_txid: u64,
	updates: Vec<(u32, Vec<u8>)>,
) -> Result<()> {
	db.run(move |tx| {
		let updates = updates.clone();
		async move {
			fold_shard(&tx, TEST_DATABASE, shard_id, as_of_txid, updates).await?;
			Ok(())
		}
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
			meta_head_key(TEST_DATABASE),
			encode_db_head(DBHead {
				head_txid,
				db_size_pages,
				post_apply_checksum: 0,
				branch_id: DatabaseBranchId::nil(),
				#[cfg(debug_assertions)]
				generation: 0,
			})?,
		),
		(
			meta_compact_key(TEST_DATABASE),
			encode_meta_compact(MetaCompact {
				materialized_txid: compact_txid,
			})?,
		),
	];

	for (txid, pages) in deltas {
		writes.push((delta_chunk_key(TEST_DATABASE, *txid, 0), encoded_blob(*txid, pages)?));
	}
	for (pgno, txid) in pidx_rows {
		writes.push((pidx_delta_key(TEST_DATABASE, *pgno), txid.to_be_bytes().to_vec()));
	}

	seed(db, writes).await
}

async fn seed_quota(db: &universaldb::Database, database_id: &str, storage_used: i64) -> Result<()> {
	let database_id = database_id.to_string();
	db.run(move |tx| {
		let database_id = database_id.clone();
		async move {
			quota::atomic_add(&tx, &database_id, storage_used);
			Ok(())
		}
	})
	.await
}

async fn seed_shard_versions(
	db: &universaldb::Database,
	shard_id: u32,
	txids: std::ops::RangeInclusive<u64>,
) -> Result<()> {
	let writes = txids
		.map(|txid| {
			Ok((
				shard_version_key(TEST_DATABASE, shard_id, txid),
				encoded_blob(txid, &[(3, txid as u8)])?,
			))
		})
		.collect::<Result<Vec<_>>>()?;

	seed(db, writes).await
}

fn tracked_entry_size(key: &[u8], value: &[u8]) -> i64 {
	i64::try_from(key.len() + value.len()).expect("tracked entry should fit in i64")
}

async fn write_newer_page(db: &universaldb::Database, pgno: u32, txid: u64, fill: u8) -> Result<()> {
	db.run(move |tx| async move {
		tx.informal().set(
			&delta_chunk_key(TEST_DATABASE, txid, 0),
			&encoded_blob(txid, &[(pgno, fill)])?,
		);
		tx.informal()
			.set(&pidx_delta_key(TEST_DATABASE, pgno), &txid.to_be_bytes());
		tx.informal().set(
			&meta_head_key(TEST_DATABASE),
			&encode_db_head(DBHead {
				head_txid: txid,
				db_size_pages: 128,
				post_apply_checksum: 0,
				branch_id: DatabaseBranchId::nil(),
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
			.clear(&pidx_delta_key(TEST_DATABASE, db_size_pages + 60));
		tx.informal().set(
			&meta_head_key(TEST_DATABASE),
			&encode_db_head(DBHead {
				head_txid,
				db_size_pages,
				post_apply_checksum: 0,
				branch_id: DatabaseBranchId::nil(),
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

	fold(&db, 0, 1, vec![update(3, 0x33), update(5, 0x55)]).await?;

	assert_pages(&read_shard(&db, 0, 1).await?, &[(3, 0x33), (5, 0x55)]);
	Ok(())
}

#[tokio::test]
async fn fold_into_existing_shard_newer_wins() -> Result<()> {
	let db = test_db().await?;
	seed(
		&db,
		vec![(
			shard_key(TEST_DATABASE, 0),
			encoded_blob(1, &[(3, 0x13), (5, 0x15)])?,
		)],
	)
	.await?;

	fold(&db, 0, 2, vec![update(3, 0x23), update(7, 0x17)]).await?;

	assert_pages(
		&read_shard(&db, 0, 2).await?,
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
		vec![(shard_key(TEST_DATABASE, 1), encoded_blob(1, &existing)?)],
	)
	.await?;

	fold(&db, 1, 2, updates).await?;

	assert_pages(&read_shard(&db, 1, 2).await?, &expected);
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
		vec![(shard_key(TEST_DATABASE, 1), encoded_blob(1, &existing)?)],
	)
	.await?;

	fold(&db, 1, 2, vec![update(96, 0xee)]).await?;

	assert_pages(&read_shard(&db, 1, 2).await?, &expected);
	Ok(())
}

#[tokio::test]
async fn fold_byte_count_metric() -> Result<()> {
	let db = test_db().await?;

	fold(
		&db,
		0,
		1,
		vec![update(3, 0x33), update(5, 0x55), update(5, 0x66)],
	)
	.await?;

	let pages = read_shard(&db, 0, 1).await?;
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
		TEST_DATABASE.to_string(),
		10,
		CancellationToken::new(),
	)
	.await?;

	assert_eq!(outcome.pages_folded, 3);
	assert_eq!(outcome.deltas_freed, 2);
	assert_eq!(outcome.compare_and_clear_noops, 0);
	assert_eq!(outcome.materialized_txid, 2);
	assert_pages(&read_shard(&db, 0, 2).await?, &[(3, 0x13), (5, 0x15)]);
	assert_pages(&read_shard(&db, 1, 2).await?, &[(70, 0x70)]);
	assert_eq!(read_pidx_txid(&db, 3).await?, None);
	assert_eq!(read_pidx_txid(&db, 5).await?, None);
	assert_eq!(read_pidx_txid(&db, 70).await?, None);
	assert_eq!(
		read_value(
			&db,
			branch_manifest_last_hot_pass_txid_key(DatabaseBranchId::nil())
		)
		.await?
		.map(|value| u64::from_be_bytes(value.try_into().expect("manifest txid should be u64"))),
		Some(2)
	);
	assert!(
		read_value(&db, delta_chunk_key(TEST_DATABASE, 1, 0))
			.await?
			.is_none()
	);
	assert!(
		read_value(&db, delta_chunk_key(TEST_DATABASE, 2, 0))
			.await?
			.is_none()
	);
	assert_eq!(read_compact_txid(&db).await?, 2);

	Ok(())
}

#[tokio::test]
async fn compact_updates_branch_manifest_last_hot_pass_txid_each_pass() -> Result<()> {
	let _compaction_test_lock = COMPACTION_TEST_LOCK.lock().await;
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(
		db.clone(),
		test_ups(),
		nil_namespace(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
	);

	database_db.commit(vec![page(3, 0x13)], 8, 1_000).await?;
	database_db.commit(vec![page(5, 0x15)], 8, 2_000).await?;
	let branch_id = read_branch_id(&db).await?;

	let first = compact_default_batch(
		db.clone(),
		TEST_DATABASE.to_string(),
		1,
		CancellationToken::new(),
	)
	.await?;

	assert_eq!(first.materialized_txid, 1);
	assert_eq!(read_branch_compact_txid(&db, branch_id).await?, 1);
	assert_eq!(read_manifest_last_hot_pass_txid(&db, branch_id).await?, Some(1));

	database_db.commit(vec![page(7, 0x17)], 8, 3_000).await?;
	let second = compact_default_batch(
		db.clone(),
		TEST_DATABASE.to_string(),
		10,
		CancellationToken::new(),
	)
	.await?;

	assert_eq!(second.materialized_txid, 3);
	assert_eq!(read_branch_compact_txid(&db, branch_id).await?, 3);
	assert_eq!(read_manifest_last_hot_pass_txid(&db, branch_id).await?, Some(3));

	Ok(())
}

#[tokio::test]
async fn compact_sweeps_old_commits_and_vtx_without_tier_gate() -> Result<()> {
	let _compaction_test_lock = COMPACTION_TEST_LOCK.lock().await;
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(
		db.clone(),
		test_ups(),
		nil_namespace(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
	);
	let current_ms = now_ms();
	let old_ms = current_ms - HOT_RETENTION_FLOOR_MS - 1_000;
	let recent_ms = current_ms - 1_000;

	database_db.commit(vec![page(3, 0x13)], 8, old_ms).await?;
	let old_bookmark = BookmarkStr::format(old_ms, 1)?;
	database_db.commit(vec![page(5, 0x15)], 8, recent_ms).await?;
	let branch_id = read_branch_id(&db).await?;
	let old_row = read_commit_row(&db, branch_id, 1)
		.await?
		.expect("old commit row should exist before compaction");
	let recent_row = read_commit_row(&db, branch_id, 2)
		.await?
		.expect("recent commit row should exist before compaction");

	compact_default_batch(
		db.clone(),
		TEST_DATABASE.to_string(),
		10,
		CancellationToken::new(),
	)
	.await?;

	assert!(read_commit_row(&db, branch_id, 1).await?.is_none());
	assert!(
		read_value(&db, branch_vtx_key(branch_id, old_row.versionstamp))
			.await?
			.is_none()
	);
	assert!(read_commit_row(&db, branch_id, 2).await?.is_some());
	assert!(
		read_value(&db, branch_vtx_key(branch_id, recent_row.versionstamp))
			.await?
			.is_some()
	);

	let err = database_db
		.resolve_bookmark(old_bookmark)
		.await
		.expect_err("bookmark for swept hot row should be expired without cold layer coverage");
	assert!(err.chain().any(|cause| {
		cause
			.downcast_ref::<SqliteStorageError>()
			.is_some_and(|err| matches!(err, SqliteStorageError::BookmarkExpired))
	}));

	Ok(())
}

#[tokio::test]
async fn compact_sweeps_old_commits_even_without_selected_deltas() -> Result<()> {
	let _compaction_test_lock = COMPACTION_TEST_LOCK.lock().await;
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(
		db.clone(),
		test_ups(),
		nil_namespace(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
	);
	let old_ms = now_ms() - HOT_RETENTION_FLOOR_MS - 1_000;

	database_db.commit(vec![page(3, 0x13)], 8, old_ms).await?;
	let branch_id = read_branch_id(&db).await?;
	let old_row = read_commit_row(&db, branch_id, 1)
		.await?
		.expect("old commit row should exist before compaction");
	seed(
		&db,
		vec![(
			sqlite_storage::keys::branch_meta_compact_key(branch_id),
			encode_meta_compact(MetaCompact {
				materialized_txid: 1,
			})?,
		)],
	)
	.await?;

	let outcome = compact_default_batch(
		db.clone(),
		TEST_DATABASE.to_string(),
		10,
		CancellationToken::new(),
	)
	.await?;

	assert_eq!(outcome.deltas_freed, 0);
	assert_eq!(outcome.materialized_txid, 1);
	assert!(read_commit_row(&db, branch_id, 1).await?.is_none());
	assert!(
		read_value(&db, branch_vtx_key(branch_id, old_row.versionstamp))
			.await?
			.is_none()
	);

	Ok(())
}

#[tokio::test]
async fn compact_retention_sweep_applies_across_namespaces() -> Result<()> {
	let _compaction_test_lock = COMPACTION_TEST_LOCK.lock().await;
	let db = Arc::new(test_db().await?);
	let current_ms = now_ms();
	let old_ms = current_ms - HOT_RETENTION_FLOOR_MS - 1_000;
	let recent_ms = current_ms - 1_000;
	let other_database = "other-database";
	let other_namespace = other_namespace();
	let first = Db::new(
		db.clone(),
		test_ups(),
		nil_namespace(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
	);
	let second = Db::new(
		db.clone(),
		test_ups(),
		other_namespace,
		other_database.to_string(),
		NodeId::new(),
	);

	first.commit(vec![page(3, 0x13)], 8, old_ms).await?;
	first.commit(vec![page(5, 0x15)], 8, recent_ms).await?;
	second.commit(vec![page(7, 0x17)], 8, old_ms).await?;
	second.commit(vec![page(9, 0x19)], 8, recent_ms).await?;
	let first_branch_id = read_branch_id(&db).await?;
	let second_branch_id = read_branch_id_for(&db, other_namespace, other_database).await?;

	compact_default_batch(
		db.clone(),
		TEST_DATABASE.to_string(),
		10,
		CancellationToken::new(),
	)
	.await?;
	assert!(read_commit_row(&db, first_branch_id, 1).await?.is_none());
	assert!(
		read_commit_row(&db, second_branch_id, 1)
			.await?
			.is_some()
	);

	worker::test_hooks::handle_payload_once(
		db.clone(),
		SqliteCompactPayload {
			database_id: other_database.to_string(),
			namespace_id: Some(other_namespace),
			database_name: None,
			commit_bytes_since_rollup: 0,
			read_bytes_since_rollup: 0,
		},
		worker::CompactorConfig {
			batch_size_deltas: 10,
			..Default::default()
		},
		CancellationToken::new(),
	)
	.await?;

	assert!(read_commit_row(&db, second_branch_id, 1).await?.is_none());
	assert!(
		read_commit_row(&db, second_branch_id, 2)
			.await?
			.is_some()
	);

	Ok(())
}

#[tokio::test]
async fn compact_force_evicts_oldest_unpinned_shard_version_at_cap() -> Result<()> {
	let _compaction_test_lock = COMPACTION_TEST_LOCK.lock().await;
	let db = test_db().await?;
	let cap = MAX_SHARD_VERSIONS_PER_SHARD as u64;
	seed_compaction_case(
		&db,
		cap + 1,
		128,
		cap,
		&[(cap + 1, vec![(3, 0xee)])],
		&[(3, cap + 1)],
	)
	.await?;
	seed_shard_versions(&db, 0, 1..=cap).await?;

	let outcome = compact_default_batch(
		Arc::new(db.clone()),
		TEST_DATABASE.to_string(),
		10,
		CancellationToken::new(),
	)
	.await?;

	assert_eq!(outcome.materialized_txid, cap + 1);
	assert!(
		read_value(&db, shard_version_key(TEST_DATABASE, 0, 1))
			.await?
			.is_none()
	);
	assert!(
		read_value(&db, shard_version_key(TEST_DATABASE, 0, cap))
			.await?
			.is_some()
	);
	assert_pages(
		&read_shard(&db, 0, cap + 1).await?,
		&[(3, 0xee)],
	);

	Ok(())
}

#[tokio::test]
async fn compact_errors_when_all_shard_versions_are_pinned_at_cap() -> Result<()> {
	let _compaction_test_lock = COMPACTION_TEST_LOCK.lock().await;
	let db = test_db().await?;
	let cap = MAX_SHARD_VERSIONS_PER_SHARD as u64;
	let pin_versionstamp = [0x11; 16];
	seed_compaction_case(
		&db,
		cap + 1,
		128,
		cap,
		&[(cap + 1, vec![(3, 0xee)])],
		&[(3, cap + 1)],
	)
	.await?;
	seed_shard_versions(&db, 0, 1..=cap).await?;
	seed(
		&db,
		vec![
			(
				branches_bk_pin_key(DatabaseBranchId::nil()),
				pin_versionstamp.to_vec(),
			),
			(vtx_key(TEST_DATABASE, pin_versionstamp), 1_u64.to_be_bytes().to_vec()),
		],
	)
	.await?;

	let err = compact_default_batch(
		Arc::new(db.clone()),
		TEST_DATABASE.to_string(),
		10,
		CancellationToken::new(),
	)
	.await
	.expect_err("compaction should fail when every shard version is pinned");
	assert!(err.chain().any(|cause| {
		cause
			.downcast_ref::<SqliteStorageError>()
			.is_some_and(|err| matches!(err, SqliteStorageError::ShardVersionCapExhausted))
	}));
	assert!(
		read_value(&db, shard_version_key(TEST_DATABASE, 0, 1))
			.await?
			.is_some()
	);
	assert!(
		read_value(&db, shard_version_key(TEST_DATABASE, 0, cap + 1))
			.await?
			.is_none()
	);

	Ok(())
}

#[tokio::test]
async fn validate_quota_accepts_clean_compacted_state() -> Result<()> {
	let db = test_db().await?;
	let shard_key = shard_key(TEST_DATABASE, 0);
	let shard_blob = encoded_blob(1, &[(3, 0x13), (5, 0x15)])?;
	let storage_used = tracked_entry_size(&shard_key, &shard_blob);
	seed(&db, vec![(shard_key, shard_blob)]).await?;
	seed_quota(&db, TEST_DATABASE, storage_used).await?;

	validate_quota(Arc::new(db), TEST_DATABASE.to_string()).await?;

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
	let (_guard, reached, release) = test_hooks::pause_after_plan(TEST_DATABASE);
	let task = tokio::spawn({
		let db = Arc::new(db.clone());
		async move {
			compact_default_batch(
				db,
				TEST_DATABASE.to_string(),
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
		read_value(&db, delta_chunk_key(TEST_DATABASE, 1, 0))
			.await?
			.is_none()
	);
	assert!(
		read_value(&db, delta_chunk_key(TEST_DATABASE, 2, 0))
			.await?
			.is_some()
	);
	assert_pages(&read_shard(&db, 0, 1).await?, &[(3, 0x13)]);

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
	let (_guard, reached, release) = test_hooks::pause_after_write_head_read(TEST_DATABASE);
	let task = tokio::spawn({
		let db = Arc::new(db.clone());
		async move {
			compact_default_batch(
				db,
				TEST_DATABASE.to_string(),
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
	let database_id = TEST_DATABASE;
	let database_name = "metered-database";
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
	seed_quota(&db, database_id, 1_000_000).await?;

	worker::test_hooks::handle_payload_once(
		Arc::clone(&db),
		SqliteCompactPayload {
			database_id: database_id.to_string(),
			namespace_id: Some(namespace_id),
			database_name: Some(database_name.to_string()),
			commit_bytes_since_rollup: commit_bytes,
			read_bytes_since_rollup: read_bytes,
		},
		worker::CompactorConfig::default(),
		CancellationToken::new(),
	)
	.await?;

	let storage_used = read_quota(&db, database_id).await?;
	assert_eq!(
		read_sqlite_metric(&db, namespace_id, database_name, TestMetric::StorageUsed).await?,
		storage_used,
	);
	assert_eq!(
		read_sqlite_metric(&db, namespace_id, database_name, TestMetric::CommitBytes).await?,
		(commit_bytes / util::metric::KV_BILLABLE_CHUNK * util::metric::KV_BILLABLE_CHUNK)
			as i64,
	);
	assert_eq!(
		read_sqlite_metric(&db, namespace_id, database_name, TestMetric::ReadBytes).await?,
		(read_bytes / util::metric::KV_BILLABLE_CHUNK * util::metric::KV_BILLABLE_CHUNK) as i64,
	);

	Ok(())
}
