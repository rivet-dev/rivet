use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use gas::prelude::Id;
use rivet_pools::NodeId;
use sqlite_storage::compactor::{
	SqliteCompactSubject, decode_compact_payload,
};
use sqlite_storage::{
	ACCESS_TOUCH_THROTTLE_MS,
		keys::{
			PAGE_SIZE, database_pointer_cur_key, branch_commit_key, branch_delta_chunk_key,
			branch_manifest_cold_drained_txid_key, branch_manifest_last_access_bucket_key,
			branch_manifest_last_access_ts_ms_key, branch_meta_compact_key, branch_meta_head_key,
			branch_pidx_key, branch_shard_key, branch_vtx_key, branches_list_key,
			ctr_eviction_index_key, namespace_branches_list_key, namespace_branches_refcount_key,
			namespace_pointer_cur_key,
		},
	ltx::{LtxHeader, encode_ltx_v3},
	pump::Db,
	quota::{self, SQLITE_MAX_STORAGE_BYTES},
	types::{
		DatabaseBranchId, DBHead, DirtyPage, FetchedPage, MetaCompact, NamespaceId,
		decode_database_branch_record, decode_database_pointer, decode_commit_row, decode_db_head,
		decode_namespace_branch_record, decode_namespace_pointer, encode_db_head, encode_meta_compact,
	},
};
use tempfile::Builder;
use universalpubsub::{NextOutput, PubSub, driver::memory::MemoryDriver};
use universaldb::utils::IsolationLevel::Snapshot;

const TEST_DATABASE: &str = "test-database";

fn test_namespace() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x1234), 1)
}

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

fn head_with_branch(branch_id: DatabaseBranchId, head_txid: u64, db_size_pages: u32) -> DBHead {
	DBHead {
		head_txid,
		db_size_pages,
		post_apply_checksum: 0,
		branch_id,
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

async fn read_i64_le(db: &universaldb::Database, key: Vec<u8>) -> Result<Option<i64>> {
	read_value(db, key).await?.map(|value| {
		Ok(i64::from_le_bytes(
			value
				.as_slice()
				.try_into()
				.expect("test value should be i64"),
		))
	}).transpose()
}

async fn read_head(db: &universaldb::Database) -> Result<DBHead> {
	let branch_id = read_branch_id(db).await?;
	let bytes = read_value(db, branch_meta_head_key(branch_id))
		.await?
		.expect("head should exist");
	decode_db_head(&bytes)
}

async fn read_branch_id(db: &universaldb::Database) -> Result<DatabaseBranchId> {
	let namespace_id = NamespaceId::from_gas_id(test_namespace());
	let namespace_pointer_bytes = read_value(db, namespace_pointer_cur_key(namespace_id))
		.await?
		.expect("namespace pointer should exist");
	let namespace_branch = decode_namespace_pointer(&namespace_pointer_bytes)?.current_branch;
	let bytes = read_value(
		db,
		database_pointer_cur_key(namespace_branch, TEST_DATABASE),
	)
	.await?
	.expect("database pointer should exist");

	Ok(decode_database_pointer(&bytes)?.current_branch)
}

async fn read_quota(db: &universaldb::Database) -> Result<i64> {
	db.run(|tx| async move {
		quota::read_in_namespace(&tx, NamespaceId::from_gas_id(test_namespace()), TEST_DATABASE).await
	})
		.await
}

#[tokio::test]
async fn commit_lazily_initializes_meta_on_first_write() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
	);

	database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let branch_id = read_branch_id(&db).await?;

	assert_eq!(read_head(&db).await?, head_with_branch(branch_id, 1, 2));
	let namespace_id = NamespaceId::from_gas_id(test_namespace());
	let namespace_pointer_bytes = read_value(&db, namespace_pointer_cur_key(namespace_id))
		.await?
		.expect("namespace pointer should exist");
	let namespace_branch = decode_namespace_pointer(&namespace_pointer_bytes)?.current_branch;
	let namespace_record_bytes = read_value(&db, namespace_branches_list_key(namespace_branch))
		.await?
		.expect("namespace branch record should exist");
	let namespace_record = decode_namespace_branch_record(&namespace_record_bytes)?;
	assert_eq!(namespace_record.branch_id, namespace_branch);
	assert_eq!(namespace_record.parent, None);
	assert_eq!(
		read_value(&db, namespace_branches_refcount_key(namespace_branch)).await?,
		Some(1_i64.to_le_bytes().to_vec())
	);
	let branch_record_bytes = read_value(&db, branches_list_key(branch_id))
		.await?
		.expect("branch record should exist");
	let branch_record = decode_database_branch_record(&branch_record_bytes)?;
	assert_eq!(branch_record.branch_id, branch_id);
	assert_eq!(branch_record.namespace_branch, namespace_branch);
	assert_eq!(branch_record.parent, None);
	assert_eq!(
		read_value(&db, branch_pidx_key(branch_id, 1)).await?,
		Some(1_u64.to_be_bytes().to_vec())
	);
	assert!(
		read_value(&db, branch_delta_chunk_key(branch_id, 1, 0))
			.await?
			.is_some()
	);
	assert!(read_quota(&db).await? > 0);
	assert_eq!(
		database_db.get_pages(vec![1]).await?,
		vec![fetched_page(1, 0x11)]
	);

	Ok(())
}

#[tokio::test]
async fn commit_advances_head_and_updates_warm_cache() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());

	database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let branch_id = read_branch_id(&db).await?;
	assert_eq!(
		database_db.get_pages(vec![1]).await?,
		vec![fetched_page(1, 0x11)]
	);

	database_db.commit(vec![page(2, 0x22)], 3, 2_000).await?;

	assert_eq!(read_head(&db).await?, head_with_branch(branch_id, 2, 3));
	assert_eq!(
		read_value(&db, branch_pidx_key(branch_id, 1)).await?,
		Some(1_u64.to_be_bytes().to_vec())
	);
	assert_eq!(
		read_value(&db, branch_pidx_key(branch_id, 2)).await?,
		Some(2_u64.to_be_bytes().to_vec())
	);

	db.run(|tx| async move {
		tx.informal().clear(&branch_pidx_key(branch_id, 2));
		Ok(())
	})
	.await?;
	assert_eq!(
		database_db.get_pages(vec![1, 2]).await?,
		vec![fetched_page(1, 0x11), fetched_page(2, 0x22)]
	);

	Ok(())
}

#[tokio::test]
async fn commit_writes_commit_row_and_vtx_index() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());

	database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	database_db.commit(vec![page(2, 0x22)], 3, 2_000).await?;
	let branch_id = read_branch_id(&db).await?;

	let first_row_bytes = read_value(&db, branch_commit_key(branch_id, 1))
		.await?
		.expect("first commit row should exist");
	let first_row = decode_commit_row(&first_row_bytes)?;
	assert_eq!(first_row.wall_clock_ms, 1_000);
	assert_eq!(first_row.db_size_pages, 2);
	assert_eq!(first_row.post_apply_checksum, 0);
	assert_ne!(&first_row.versionstamp[..10], &[0xff; 10]);
	assert_eq!(
		read_value(&db, branch_vtx_key(branch_id, first_row.versionstamp)).await?,
		Some(1_u64.to_be_bytes().to_vec())
	);

	let second_row_bytes = read_value(&db, branch_commit_key(branch_id, 2))
		.await?
		.expect("second commit row should exist");
	let second_row = decode_commit_row(&second_row_bytes)?;
	assert_eq!(second_row.wall_clock_ms, 2_000);
	assert_eq!(second_row.db_size_pages, 3);
	assert_ne!(second_row.versionstamp, first_row.versionstamp);
	assert_eq!(
		read_value(&db, branch_vtx_key(branch_id, second_row.versionstamp)).await?,
		Some(2_u64.to_be_bytes().to_vec())
	);

	Ok(())
}

#[tokio::test]
async fn commit_throttles_access_touch_by_bucket() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());

	database_db.commit(vec![page(1, 0x01)], 1, 1).await?;
	let branch_id = read_branch_id(&db).await?;
	assert_eq!(
		read_i64_le(&db, branch_manifest_last_access_ts_ms_key(branch_id)).await?,
		Some(1)
	);
	assert_eq!(
		read_i64_le(&db, branch_manifest_last_access_bucket_key(branch_id)).await?,
		Some(0)
	);
	assert_eq!(
		read_value(&db, ctr_eviction_index_key(0, branch_id)).await?,
		Some(Vec::new())
	);

	for now_ms in 2..=120 {
		database_db
			.commit(vec![page(1, now_ms as u8)], 1, now_ms)
			.await?;
	}
	assert_eq!(
		read_i64_le(&db, branch_manifest_last_access_ts_ms_key(branch_id)).await?,
		Some(1)
	);
	assert_eq!(
		read_i64_le(&db, branch_manifest_last_access_bucket_key(branch_id)).await?,
		Some(0)
	);
	assert_eq!(
		read_value(&db, ctr_eviction_index_key(0, branch_id)).await?,
		Some(Vec::new())
	);

	database_db
		.commit(vec![page(1, 0xfe)], 1, ACCESS_TOUCH_THROTTLE_MS)
		.await?;
	assert_eq!(
		read_i64_le(&db, branch_manifest_last_access_ts_ms_key(branch_id)).await?,
		Some(ACCESS_TOUCH_THROTTLE_MS)
	);
	assert_eq!(
		read_i64_le(&db, branch_manifest_last_access_bucket_key(branch_id)).await?,
		Some(1)
	);
	assert!(read_value(&db, ctr_eviction_index_key(0, branch_id)).await?.is_none());
	assert_eq!(
		read_value(&db, ctr_eviction_index_key(1, branch_id)).await?,
		Some(Vec::new())
	);

	Ok(())
}

#[tokio::test]
async fn get_pages_touches_access_bucket_on_read() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let writer = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	writer.commit(vec![page(1, 0x11)], 1, 1).await?;
	let branch_id = read_branch_id(&db).await?;

	let reader = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	assert_eq!(
		reader.get_pages(vec![1]).await?,
		vec![fetched_page(1, 0x11)]
	);

	let last_access_ts_ms = read_i64_le(&db, branch_manifest_last_access_ts_ms_key(branch_id))
		.await?
		.expect("last access timestamp should exist");
	let last_access_bucket = read_i64_le(&db, branch_manifest_last_access_bucket_key(branch_id))
		.await?
		.expect("last access bucket should exist");
	assert!(last_access_ts_ms > 1);
	assert_eq!(
		last_access_bucket,
		last_access_ts_ms.div_euclid(ACCESS_TOUCH_THROTTLE_MS)
	);
	assert!(read_value(&db, ctr_eviction_index_key(0, branch_id)).await?.is_none());
	assert_eq!(
		read_value(&db, ctr_eviction_index_key(last_access_bucket, branch_id)).await?,
		Some(Vec::new())
	);

	Ok(())
}

#[tokio::test]
async fn commit_rejects_quota_cap_before_writes() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	database_db.commit(vec![page(1, 0x11)], 1, 1_000).await?;
	let branch_id = read_branch_id(&db).await?;
	db.run(|tx| async move {
		quota::atomic_add_branch(&tx, branch_id, SQLITE_MAX_STORAGE_BYTES);
		Ok(())
	})
	.await?;

	let database_db = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	let err = database_db
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
	assert_eq!(read_head(&db).await?, head_with_branch(branch_id, 1, 1));
	assert!(
		read_value(&db, branch_delta_chunk_key(branch_id, 2, 0))
			.await?
			.is_none()
	);

	Ok(())
}

#[tokio::test]
async fn commit_uses_burst_adjusted_quota_cap() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	database_db.commit(vec![page(1, 0x11)], 1, 1_000).await?;
	let branch_id = read_branch_id(&db).await?;
	let storage_used = read_quota(&db).await?;

	seed(
		&db,
		vec![
			(
				branch_meta_head_key(branch_id),
				encode_db_head(head_with_branch(branch_id, 1024, 1))?,
			),
			(
				branch_manifest_cold_drained_txid_key(branch_id),
				0_u64.to_be_bytes().to_vec(),
			),
		],
	)
	.await?;
	db.run(move |tx| async move {
		quota::atomic_add_branch(&tx, branch_id, SQLITE_MAX_STORAGE_BYTES - storage_used);
		Ok(())
	})
	.await?;

	let database_db = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	database_db.commit(vec![page(1, 0x44)], 1, 3_000).await?;
	assert!(read_quota(&db).await? > SQLITE_MAX_STORAGE_BYTES);

	seed(
		&db,
		vec![(
			branch_manifest_cold_drained_txid_key(branch_id),
			1025_u64.to_be_bytes().to_vec(),
		)],
	)
	.await?;
	let err = database_db
		.commit(vec![page(1, 0x55)], 1, 4_000)
		.await
		.expect_err("commit should exceed quota after cold lag recovers");
	assert!(
		err.downcast_ref::<sqlite_storage::error::SqliteStorageError>()
			.is_some_and(|err| matches!(
				err,
				sqlite_storage::error::SqliteStorageError::SqliteStorageQuotaExceeded { .. }
			))
	);
	assert_eq!(read_head(&db).await?, head_with_branch(branch_id, 1025, 1));

	Ok(())
}

#[tokio::test]
async fn shrink_commit_deletes_above_eof_pidx_and_shards() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	database_db.commit(vec![page(1, 0x01)], 130, 1_000).await?;
	let branch_id = read_branch_id(&db).await?;
	seed(
		&db,
		vec![
			(
				branch_meta_head_key(branch_id),
				encode_db_head(head_with_branch(branch_id, 7, 130))?,
			),
			(
				branch_delta_chunk_key(branch_id, 7, 0),
				encoded_blob(7, &[(64, 0x64), (129, 0x81)])?,
			),
			(branch_pidx_key(branch_id, 64), 7_u64.to_be_bytes().to_vec()),
			(branch_pidx_key(branch_id, 129), 7_u64.to_be_bytes().to_vec()),
			(branch_shard_key(branch_id, 1, 7), encoded_blob(7, &[(64, 0x64)])?),
			(branch_shard_key(branch_id, 2, 7), encoded_blob(7, &[(129, 0x81)])?),
		],
	)
	.await?;
	db.run(|tx| async move {
		quota::atomic_add_branch(&tx, branch_id, 50_000);
		Ok(())
	})
	.await?;

	let database_db = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	database_db.commit(vec![page(1, 0x11)], 63, 4_000).await?;

	assert_eq!(read_head(&db).await?, head_with_branch(branch_id, 8, 63));
	assert!(read_value(&db, branch_pidx_key(branch_id, 64)).await?.is_none());
	assert!(
		read_value(&db, branch_pidx_key(branch_id, 129))
			.await?
			.is_none()
	);
	assert!(read_value(&db, branch_shard_key(branch_id, 1, 7)).await?.is_none());
	assert!(read_value(&db, branch_shard_key(branch_id, 2, 7)).await?.is_none());
	assert_eq!(
		read_value(&db, branch_pidx_key(branch_id, 1)).await?,
		Some(8_u64.to_be_bytes().to_vec())
	);

	Ok(())
}

#[tokio::test(start_paused = true)]
async fn commit_publishes_compaction_trigger_with_throttle() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let ups = test_ups();
	let mut sub = ups.queue_subscribe(SqliteCompactSubject, "compactor").await?;
	let database_db = Db::new(db.clone(), ups.clone(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	database_db.commit(vec![page(1, 0x01)], 1, 1_000).await?;
	let branch_id = read_branch_id(&db).await?;
	seed(
		&db,
		vec![
			(
				branch_meta_head_key(branch_id),
				encode_db_head(head_with_branch(branch_id, 31, 1))?,
			),
			(
				branch_meta_compact_key(branch_id),
				encode_meta_compact(MetaCompact {
					materialized_txid: 0,
				})?,
			),
		],
	)
	.await?;
	db.run(|tx| async move {
		quota::atomic_add_branch(&tx, branch_id, 1_000);
		Ok(())
	})
	.await?;

	let database_db = Db::new(db, ups, test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	database_db.commit(vec![page(1, 0x11)], 1, 5_000).await?;
	let first = next_trigger(&mut sub).await?;
	assert_eq!(first.database_id, TEST_DATABASE);
	assert!(first.commit_bytes_since_rollup > 0);

	database_db.commit(vec![page(1, 0x22)], 1, 5_100).await?;
	assert_no_trigger(&mut sub).await?;

	tokio::time::advance(Duration::from_millis(quota::TRIGGER_MAX_SILENCE_MS + 1)).await;
	database_db.commit(vec![page(1, 0x33)], 1, 5_200).await?;
	let after_silence = next_trigger(&mut sub).await?;
	assert_eq!(after_silence.database_id, TEST_DATABASE);

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
