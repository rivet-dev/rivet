mod common;

use std::sync::Arc;

use anyhow::Result;
#[cfg(feature = "test-faults")]
use depot::fault::{CommitFaultPoint, DepotFaultController, DepotFaultPoint, FaultBoundary};
use depot::{
	ACCESS_TOUCH_THROTTLE_MS,
	conveyer::Db,
	conveyer::{
		commit::{clear_sqlite_cmp_dirty_if_observed_idle, test_hooks},
		db::CompactionSignaler,
	},
	keys::{
		PAGE_SIZE, branch_commit_key, branch_compaction_root_key, branch_delta_chunk_key,
		branch_manifest_last_access_bucket_key, branch_manifest_last_access_ts_ms_key,
		branch_meta_compact_key, branch_meta_head_key, branch_pidx_key, branch_shard_key,
		branch_vtx_key, branches_list_key, bucket_branches_list_key, bucket_branches_refcount_key,
		bucket_pointer_cur_key, ctr_eviction_index_key, database_pointer_cur_key,
		sqlite_cmp_dirty_key,
	},
	ltx::{LtxHeader, encode_ltx_v3},
	quota::{self, SQLITE_MAX_STORAGE_BYTES},
	types::{
		BucketId, CompactionRoot, DBHead, DatabaseBranchId, DirtyPage, FetchedPage, MetaCompact,
		SqliteCmpDirty, decode_bucket_branch_record, decode_bucket_pointer, decode_commit_row,
		decode_database_branch_record, decode_database_pointer, decode_db_head,
		decode_sqlite_cmp_dirty, encode_compaction_root, encode_db_head, encode_meta_compact,
		encode_sqlite_cmp_dirty,
	},
	workflows::compaction::DeltasAvailable,
};
use futures_util::FutureExt;
use gas::prelude::Id;
use parking_lot::Mutex;
use rivet_pools::NodeId;
use universaldb::utils::IsolationLevel::Snapshot;

const TEST_DATABASE: &str = "test-database";

fn test_bucket() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x1234), 1)
}

fn recording_compaction_signaler(signals: Arc<Mutex<Vec<DeltasAvailable>>>) -> CompactionSignaler {
	Arc::new(move |signal| {
		let signals = Arc::clone(&signals);
		async move {
			signals.lock().push(signal);
			Ok(())
		}
		.boxed()
	})
}

macro_rules! commit_matrix {
	($prefix:expr, |$ctx:ident, $db:ident, $database_db:ident| $body:block) => {
		common::test_matrix($prefix, |_tier, $ctx| {
			Box::pin(async move {
				let $db = $ctx.udb.clone();
				let $database_db = $ctx.make_db(test_bucket(), TEST_DATABASE);
				$body
			})
		})
		.await
	};
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

fn short_page(pgno: u32) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![0; 128],
	}
}

fn fetched_page(pgno: u32, fill: u8) -> FetchedPage {
	FetchedPage {
		pgno,
		bytes: Some(vec![fill; PAGE_SIZE as usize]),
	}
}

fn zero_filled_page(pgno: u32) -> FetchedPage {
	FetchedPage {
		pgno,
		bytes: Some(vec![0; PAGE_SIZE as usize]),
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
	read_value(db, key)
		.await?
		.map(|value| {
			Ok(i64::from_le_bytes(
				value
					.as_slice()
					.try_into()
					.expect("test value should be i64"),
			))
		})
		.transpose()
}

async fn read_head(db: &universaldb::Database) -> Result<DBHead> {
	let branch_id = read_branch_id(db).await?;
	let bytes = read_value(db, branch_meta_head_key(branch_id))
		.await?
		.expect("head should exist");
	decode_db_head(&bytes)
}

async fn read_dirty_marker(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
) -> Result<Option<SqliteCmpDirty>> {
	read_value(db, sqlite_cmp_dirty_key(branch_id))
		.await?
		.map(|value| decode_sqlite_cmp_dirty(&value))
		.transpose()
}

async fn read_branch_id(db: &universaldb::Database) -> Result<DatabaseBranchId> {
	let bucket_id = BucketId::from_gas_id(test_bucket());
	let bucket_pointer_bytes = read_value(db, bucket_pointer_cur_key(bucket_id))
		.await?
		.expect("bucket pointer should exist");
	let bucket_branch = decode_bucket_pointer(&bucket_pointer_bytes)?.current_branch;
	let bytes = read_value(db, database_pointer_cur_key(bucket_branch, TEST_DATABASE))
		.await?
		.expect("database pointer should exist");

	Ok(decode_database_pointer(&bytes)?.current_branch)
}

async fn read_quota(db: &universaldb::Database) -> Result<i64> {
	db.run(|tx| async move {
		quota::read_in_bucket(&tx, BucketId::from_gas_id(test_bucket()), TEST_DATABASE).await
	})
	.await
}

async fn assert_commit_rejected(
	database_db: &Db,
	dirty_pages: Vec<DirtyPage>,
	expected: &str,
) -> Result<()> {
	let err = database_db
		.commit(dirty_pages, 2, 1_000)
		.await
		.expect_err("commit should reject invalid dirty pages");
	assert!(
		err.to_string().contains(expected),
		"expected error to contain {expected:?}, got {err:?}"
	);

	Ok(())
}

#[cfg(feature = "test-faults")]
fn error_chain_contains(err: &anyhow::Error, expected: &str) -> bool {
	err.chain()
		.any(|cause| cause.to_string().contains(expected))
}

#[cfg(feature = "test-faults")]
fn faulting_db(db: Arc<universaldb::Database>, controller: DepotFaultController) -> Db {
	Db::new_with_fault_controller_for_test(
		db,
		test_bucket(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
		controller,
	)
}

#[tokio::test]
async fn commit_lazily_initializes_meta_on_first_write() -> Result<()> {
	commit_matrix!("depot-commit-init", |ctx, db, database_db| {
		database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
		let branch_id = read_branch_id(&db).await?;

		assert_eq!(read_head(&db).await?, head_with_branch(branch_id, 1, 2));
		let bucket_id = BucketId::from_gas_id(test_bucket());
		let bucket_pointer_bytes = read_value(&db, bucket_pointer_cur_key(bucket_id))
			.await?
			.expect("bucket pointer should exist");
		let bucket_branch = decode_bucket_pointer(&bucket_pointer_bytes)?.current_branch;
		let bucket_record_bytes = read_value(&db, bucket_branches_list_key(bucket_branch))
			.await?
			.expect("bucket branch record should exist");
		let bucket_record = decode_bucket_branch_record(&bucket_record_bytes)?;
		assert_eq!(bucket_record.branch_id, bucket_branch);
		assert_eq!(bucket_record.parent, None);
		assert_eq!(
			read_value(&db, bucket_branches_refcount_key(bucket_branch)).await?,
			Some(1_i64.to_le_bytes().to_vec())
		);
		let branch_record_bytes = read_value(&db, branches_list_key(branch_id))
			.await?
			.expect("branch record should exist");
		let branch_record = decode_database_branch_record(&branch_record_bytes)?;
		assert_eq!(branch_record.branch_id, branch_id);
		assert_eq!(branch_record.bucket_branch, bucket_branch);
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
	})
}

#[tokio::test]
async fn commit_rejects_invalid_dirty_pages_before_storage_writes() -> Result<()> {
	commit_matrix!("depot-commit-invalid-dirty", |ctx, db, database_db| {
		assert_commit_rejected(
			&database_db,
			vec![page(0, 0x11)],
			"sqlite commit does not accept page 0",
		)
		.await?;
		assert_commit_rejected(
			&database_db,
			vec![short_page(1)],
			"sqlite commit page 1 had 128 bytes",
		)
		.await?;
		assert_commit_rejected(
			&database_db,
			vec![page(1, 0x11), page(1, 0x22)],
			"sqlite commit duplicated page 1",
		)
		.await?;
		assert_eq!(
			read_value(
				&db,
				bucket_pointer_cur_key(BucketId::from_gas_id(test_bucket()))
			)
			.await?,
			None
		);

		Ok(())
	})
}

#[cfg(feature = "test-faults")]
#[tokio::test]
async fn commit_pre_durable_fault_leaves_old_state() -> Result<()> {
	commit_matrix!("depot-commit-fault-pre-durable", |ctx, db, database_db| {
		database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
		let branch_id = read_branch_id(&db).await?;

		let controller = DepotFaultController::new();
		controller
			.at(DepotFaultPoint::Commit(CommitFaultPoint::BeforeHeadWrite))
			.database_id(TEST_DATABASE)
			.once()
			.fail("stop before head write")?;
		let faulting_db = faulting_db(db.clone(), controller.clone());
		let err = faulting_db
			.commit(vec![page(2, 0x22)], 3, 2_000)
			.await
			.expect_err("pre-durable fault should fail the commit");

		assert!(
			error_chain_contains(&err, "stop before head write"),
			"expected injected fault error, got {err:?}"
		);
		controller.assert_expected_fired()?;
		assert_eq!(
			controller.replay_log()[0].boundary,
			FaultBoundary::PreDurableCommit
		);
		assert_eq!(read_head(&db).await?, head_with_branch(branch_id, 1, 2));
		assert!(
			read_value(&db, branch_delta_chunk_key(branch_id, 2, 0))
				.await?
				.is_none()
		);
		assert!(
			read_value(&db, branch_pidx_key(branch_id, 2))
				.await?
				.is_none()
		);

		Ok(())
	})
}

#[cfg(feature = "test-faults")]
#[tokio::test]
async fn commit_after_udb_fault_reports_ambiguous_committed_state() -> Result<()> {
	commit_matrix!("depot-commit-fault-after-udb", |ctx, db, database_db| {
		database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
		let branch_id = read_branch_id(&db).await?;

		let controller = DepotFaultController::new();
		controller
			.at(DepotFaultPoint::Commit(CommitFaultPoint::AfterUdbCommit))
			.database_id(TEST_DATABASE)
			.once()
			.fail("stop after durable commit")?;
		let faulting_db = faulting_db(db.clone(), controller.clone());
		let err = faulting_db
			.commit(vec![page(2, 0x22)], 3, 2_000)
			.await
			.expect_err("post-durable fault should return an ambiguous error");

		assert!(
			error_chain_contains(&err, "stop after durable commit"),
			"expected injected fault error, got {err:?}"
		);
		controller.assert_expected_fired()?;
		assert_eq!(
			controller.replay_log()[0].boundary,
			FaultBoundary::AmbiguousAfterDurableCommit
		);
		assert_eq!(read_head(&db).await?, head_with_branch(branch_id, 2, 3));
		let reader = ctx.make_db(test_bucket(), TEST_DATABASE);
		assert_eq!(
			reader.get_pages(vec![1, 2]).await?,
			vec![fetched_page(1, 0x11), fetched_page(2, 0x22)]
		);

		Ok(())
	})
}

#[tokio::test]
async fn commit_advances_head_and_updates_warm_cache() -> Result<()> {
	commit_matrix!("depot-commit-advance", |ctx, db, database_db| {
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
	})
}

#[tokio::test]
async fn commit_writes_commit_row_and_vtx_index() -> Result<()> {
	commit_matrix!("depot-commit-row-vtx", |ctx, db, database_db| {
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
	})
}

#[tokio::test]
async fn commit_throttles_access_touch_by_bucket() -> Result<()> {
	commit_matrix!("depot-commit-access-touch", |ctx, db, database_db| {
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
		assert!(
			read_value(&db, ctr_eviction_index_key(0, branch_id))
				.await?
				.is_none()
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
		assert!(
			read_value(&db, ctr_eviction_index_key(0, branch_id))
				.await?
				.is_none()
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
		assert!(
			read_value(&db, ctr_eviction_index_key(0, branch_id))
				.await?
				.is_none()
		);
		assert!(
			read_value(&db, ctr_eviction_index_key(1, branch_id))
				.await?
				.is_none()
		);

		Ok(())
	})
}

#[tokio::test]
async fn get_pages_does_not_touch_access_bucket_on_delta_read() -> Result<()> {
	commit_matrix!("depot-commit-delta-read-touch", |ctx, db, writer| {
		writer.commit(vec![page(1, 0x11)], 1, 1).await?;
		let branch_id = read_branch_id(&db).await?;

		let reader = ctx.make_db(test_bucket(), TEST_DATABASE);
		assert_eq!(
			reader.get_pages(vec![1]).await?,
			vec![fetched_page(1, 0x11)]
		);

		let last_access_ts_ms =
			read_i64_le(&db, branch_manifest_last_access_ts_ms_key(branch_id)).await?;
		let last_access_bucket =
			read_i64_le(&db, branch_manifest_last_access_bucket_key(branch_id)).await?;
		assert_eq!(last_access_ts_ms, Some(1));
		assert_eq!(last_access_bucket, Some(0));
		assert!(
			read_value(&db, ctr_eviction_index_key(0, branch_id))
				.await?
				.is_none()
		);

		Ok(())
	})
}

#[tokio::test]
async fn commit_rejects_quota_cap_before_writes() -> Result<()> {
	commit_matrix!("depot-commit-quota-cap", |ctx, db, database_db| {
		database_db.commit(vec![page(1, 0x11)], 1, 1_000).await?;
		let branch_id = read_branch_id(&db).await?;
		db.run(|tx| async move {
			quota::atomic_add_branch(&tx, branch_id, SQLITE_MAX_STORAGE_BYTES);
			Ok(())
		})
		.await?;

		let database_db = ctx.make_db(test_bucket(), TEST_DATABASE);
		let err = database_db
			.commit(vec![page(1, 0x44)], 1, 3_000)
			.await
			.expect_err("commit should exceed quota");

		assert!(
			err.downcast_ref::<depot::error::SqliteStorageError>()
				.is_some_and(|err| matches!(
					err,
					depot::error::SqliteStorageError::SqliteStorageQuotaExceeded { .. }
				))
		);
		assert_eq!(read_head(&db).await?, head_with_branch(branch_id, 1, 1));
		assert!(
			read_value(&db, branch_delta_chunk_key(branch_id, 2, 0))
				.await?
				.is_none()
		);

		Ok(())
	})
}

#[tokio::test]
async fn commit_uses_burst_adjusted_quota_cap() -> Result<()> {
	commit_matrix!("depot-commit-burst-quota", |ctx, db, database_db| {
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
					branch_compaction_root_key(branch_id),
					encode_compaction_root(CompactionRoot {
						schema_version: 1,
						manifest_generation: 1,
						hot_watermark_txid: 0,
						cold_watermark_txid: 0,
						cold_watermark_versionstamp: [0; 16],
					})?,
				),
			],
		)
		.await?;
		db.run(move |tx| async move {
			quota::atomic_add_branch(&tx, branch_id, SQLITE_MAX_STORAGE_BYTES - storage_used);
			Ok(())
		})
		.await?;

		let database_db = ctx.make_db(test_bucket(), TEST_DATABASE);
		database_db.commit(vec![page(1, 0x44)], 1, 3_000).await?;
		assert!(read_quota(&db).await? > SQLITE_MAX_STORAGE_BYTES);

		seed(
			&db,
			vec![(
				branch_compaction_root_key(branch_id),
				encode_compaction_root(CompactionRoot {
					schema_version: 1,
					manifest_generation: 2,
					hot_watermark_txid: 1025,
					cold_watermark_txid: 1025,
					cold_watermark_versionstamp: [1; 16],
				})?,
			)],
		)
		.await?;
		let err = database_db
			.commit(vec![page(1, 0x55)], 1, 4_000)
			.await
			.expect_err("commit should exceed quota after cold lag recovers");
		assert!(
			err.downcast_ref::<depot::error::SqliteStorageError>()
				.is_some_and(|err| matches!(
					err,
					depot::error::SqliteStorageError::SqliteStorageQuotaExceeded { .. }
				))
		);
		assert_eq!(read_head(&db).await?, head_with_branch(branch_id, 1025, 1));

		Ok(())
	})
}

#[tokio::test]
async fn truncate_prunes_boundary_shard_pages_above_eof() -> Result<()> {
	commit_matrix!("depot-commit-truncate-boundary", |ctx, db, database_db| {
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
					branch_shard_key(branch_id, 1, 7),
					encoded_blob(7, &[(64, 0x64), (65, 0x65)])?,
				),
			],
		)
		.await?;
		db.run(|tx| async move {
			quota::atomic_add_branch(&tx, branch_id, 50_000);
			Ok(())
		})
		.await?;

		let database_db = ctx.make_db(test_bucket(), TEST_DATABASE);
		database_db.commit(vec![page(1, 0x11)], 64, 4_000).await?;
		assert_eq!(
			database_db.get_pages(vec![64]).await?,
			vec![fetched_page(64, 0x64)]
		);

		database_db.commit(Vec::new(), 65, 5_000).await?;

		assert_eq!(
			database_db.get_pages(vec![65]).await?,
			vec![zero_filled_page(65)]
		);

		Ok(())
	})
}

#[tokio::test]
async fn truncate_does_not_clobber_concurrently_rewritten_boundary_shard() -> Result<()> {
	commit_matrix!("depot-commit-truncate-race", |ctx, db, database_db| {
		database_db.commit(vec![page(1, 0x01)], 130, 1_000).await?;
		let branch_id = read_branch_id(&db).await?;
		let shard_key = branch_shard_key(branch_id, 1, 7);
		seed(
			&db,
			vec![
				(
					branch_meta_head_key(branch_id),
					encode_db_head(head_with_branch(branch_id, 7, 130))?,
				),
				(
					shard_key.clone(),
					encoded_blob(7, &[(64, 0x64), (65, 0x65)])?,
				),
			],
		)
		.await?;
		db.run(|tx| async move {
			quota::atomic_add_branch(&tx, branch_id, 50_000);
			Ok(())
		})
		.await?;

		let (guard, reached, release) = test_hooks::pause_after_truncate_cleanup(TEST_DATABASE);
		let commit_task = tokio::spawn({
			let database_db = ctx.make_db(test_bucket(), TEST_DATABASE);
			async move { database_db.commit(vec![page(1, 0x11)], 64, 4_000).await }
		});

		reached.notified().await;
		seed(&db, vec![(shard_key, encoded_blob(7, &[(64, 0x99)])?)]).await?;
		release.notify_waiters();
		drop(guard);
		commit_task.await??;

		let database_db = ctx.make_db(test_bucket(), TEST_DATABASE);
		assert_eq!(
			database_db.get_pages(vec![64]).await?,
			vec![fetched_page(64, 0x99)]
		);

		Ok(())
	})
}

#[tokio::test]
async fn shrink_commit_deletes_above_eof_pidx_and_shards() -> Result<()> {
	commit_matrix!("depot-commit-shrink", |ctx, db, database_db| {
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
				(
					branch_pidx_key(branch_id, 129),
					7_u64.to_be_bytes().to_vec(),
				),
				(
					branch_shard_key(branch_id, 1, 7),
					encoded_blob(7, &[(64, 0x64)])?,
				),
				(
					branch_shard_key(branch_id, 2, 7),
					encoded_blob(7, &[(129, 0x81)])?,
				),
			],
		)
		.await?;
		db.run(|tx| async move {
			quota::atomic_add_branch(&tx, branch_id, 50_000);
			Ok(())
		})
		.await?;

		let database_db = ctx.make_db(test_bucket(), TEST_DATABASE);
		database_db.commit(vec![page(1, 0x11)], 63, 4_000).await?;

		assert_eq!(read_head(&db).await?, head_with_branch(branch_id, 8, 63));
		assert!(
			read_value(&db, branch_pidx_key(branch_id, 64))
				.await?
				.is_none()
		);
		assert!(
			read_value(&db, branch_pidx_key(branch_id, 129))
				.await?
				.is_none()
		);
		assert!(
			read_value(&db, branch_shard_key(branch_id, 1, 7))
				.await?
				.is_none()
		);
		assert!(
			read_value(&db, branch_shard_key(branch_id, 2, 7))
				.await?
				.is_none()
		);
		assert_eq!(
			read_value(&db, branch_pidx_key(branch_id, 1)).await?,
			Some(8_u64.to_be_bytes().to_vec())
		);

		Ok(())
	})
}

#[tokio::test]
async fn commit_writes_dirty_marker_and_sends_first_deltas_available() -> Result<()> {
	common::test_matrix("depot-commit-dirty-first", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let signals = Arc::new(Mutex::new(Vec::new()));
			let database_db = Db::new_with_compaction_signaler(
				db.clone(),
				test_bucket(),
				TEST_DATABASE.to_string(),
				NodeId::new(),
				ctx.cold_tier.clone(),
				recording_compaction_signaler(Arc::clone(&signals)),
			);
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

			database_db.commit(vec![page(1, 0x11)], 1, 5_000).await?;

			let dirty = read_dirty_marker(&db, branch_id)
				.await?
				.expect("dirty marker should exist");
			assert_eq!(dirty.observed_head_txid, 32);
			assert_eq!(dirty.updated_at_ms, 5_000);
			let signals = signals.lock().clone();
			assert_eq!(
				signals,
				vec![DeltasAvailable {
					database_branch_id: branch_id,
					observed_head_txid: 32,
					dirty_updated_at_ms: 5_000,
				}]
			);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn commit_refreshes_dirty_marker_and_throttles_deltas_available() -> Result<()> {
	common::test_matrix("depot-commit-dirty-refresh", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let signals = Arc::new(Mutex::new(Vec::new()));
			let database_db = Db::new_with_compaction_signaler(
				db.clone(),
				test_bucket(),
				TEST_DATABASE.to_string(),
				NodeId::new(),
				ctx.cold_tier.clone(),
				recording_compaction_signaler(Arc::clone(&signals)),
			);
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

			database_db.commit(vec![page(1, 0x11)], 1, 5_000).await?;
			database_db.commit(vec![page(1, 0x22)], 1, 5_100).await?;
			let dirty = read_dirty_marker(&db, branch_id)
				.await?
				.expect("dirty marker should refresh");
			assert_eq!(dirty.observed_head_txid, 33);
			assert_eq!(dirty.updated_at_ms, 5_100);
			assert_eq!(signals.lock().len(), 1);

			database_db.commit(vec![page(1, 0x33)], 1, 5_500).await?;
			let dirty = read_dirty_marker(&db, branch_id)
				.await?
				.expect("dirty marker should refresh again");
			assert_eq!(dirty.observed_head_txid, 34);
			assert_eq!(dirty.updated_at_ms, 5_500);
			let signals = signals.lock().clone();
			assert_eq!(signals.len(), 2);
			assert_eq!(signals[1].observed_head_txid, 34);
			assert_eq!(signals[1].dirty_updated_at_ms, 5_500);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn dirty_marker_clear_rejects_stale_observed_value() -> Result<()> {
	common::test_matrix("depot-commit-dirty-stale-clear", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let branch_id = DatabaseBranchId::new_v4();
			let old_dirty = SqliteCmpDirty {
				observed_head_txid: 40,
				updated_at_ms: 1_000,
			};
			let new_dirty = SqliteCmpDirty {
				observed_head_txid: 41,
				updated_at_ms: 1_100,
			};
			seed(
				&db,
				vec![(
					sqlite_cmp_dirty_key(branch_id),
					encode_sqlite_cmp_dirty(new_dirty.clone())?,
				)],
			)
			.await?;

			assert!(
				!clear_sqlite_cmp_dirty_if_observed_idle(&db, branch_id, old_dirty).await?,
				"stale dirty marker should not clear"
			);
			assert_eq!(read_dirty_marker(&db, branch_id).await?, Some(new_dirty));

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn dirty_marker_clear_requires_workflow_compaction_root() -> Result<()> {
	common::test_matrix("depot-commit-dirty-root-required", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let branch_id = DatabaseBranchId::new_v4();
			let dirty = SqliteCmpDirty {
				observed_head_txid: 40,
				updated_at_ms: 1_000,
			};
			seed(
				&db,
				vec![
					(
						branch_meta_head_key(branch_id),
						encode_db_head(head_with_branch(branch_id, 40, 1))?,
					),
					(
						branch_meta_compact_key(branch_id),
						encode_meta_compact(MetaCompact {
							materialized_txid: 40,
						})?,
					),
					(
						sqlite_cmp_dirty_key(branch_id),
						encode_sqlite_cmp_dirty(dirty.clone())?,
					),
				],
			)
			.await?;

			let err = clear_sqlite_cmp_dirty_if_observed_idle(&db, branch_id, dirty)
				.await
				.expect_err("dirty clear without workflow compaction root should fail");
			assert!(
				err.chain().any(|cause| cause
					.to_string()
					.contains("sqlite compaction root missing for dirty clear")),
				"unexpected error: {err:?}"
			);
			assert!(read_dirty_marker(&db, branch_id).await?.is_some());

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn dirty_marker_clear_removes_exact_idle_marker() -> Result<()> {
	common::test_matrix("depot-commit-dirty-clear", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let branch_id = DatabaseBranchId::new_v4();
			let dirty = SqliteCmpDirty {
				observed_head_txid: 40,
				updated_at_ms: 1_000,
			};
			seed(
				&db,
				vec![
					(
						branch_meta_head_key(branch_id),
						encode_db_head(head_with_branch(branch_id, 40, 1))?,
					),
					(
						sqlite_cmp_dirty_key(branch_id),
						encode_sqlite_cmp_dirty(dirty.clone())?,
					),
					(
						branch_compaction_root_key(branch_id),
						encode_compaction_root(CompactionRoot {
							schema_version: 1,
							manifest_generation: 7,
							hot_watermark_txid: 40,
							cold_watermark_txid: 40,
							cold_watermark_versionstamp: [0x22; 16],
						})?,
					),
				],
			)
			.await?;

			assert!(
				clear_sqlite_cmp_dirty_if_observed_idle(&db, branch_id, dirty).await?,
				"idle exact marker should clear"
			);
			assert!(read_dirty_marker(&db, branch_id).await?.is_none());

			Ok(())
		})
	})
	.await
}
