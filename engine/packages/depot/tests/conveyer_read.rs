mod common;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use depot::{
	ACCESS_TOUCH_THROTTLE_MS,
	cold_tier::{ColdTier, ColdTierObjectMetadata, FilesystemColdTier},
	conveyer::{Db, branch, metrics},
	error::SqliteStorageError,
	keys::{
		PAGE_SIZE, branch_commit_key, branch_compaction_cold_shard_key, branch_compaction_root_key,
		branch_delta_chunk_key, branch_delta_chunk_prefix, branch_manifest_last_access_bucket_key,
		branch_manifest_last_access_ts_ms_key, branch_meta_head_key, branch_pidx_key,
		branch_shard_key,
	},
	ltx::{LtxHeader, encode_ltx_v3},
	types::{
		ColdManifestChunk, ColdManifestChunkRef, ColdManifestIndex, ColdShardRef, CompactionRoot,
		DBHead, DatabaseBranchId, DirtyPage, FetchedPage, LayerEntry, LayerKind,
		ResolvedVersionstamp, SQLITE_STORAGE_COLD_SCHEMA_VERSION, decode_commit_row,
		decode_compaction_root, encode_cold_manifest_chunk, encode_cold_manifest_index,
		encode_cold_shard_ref, encode_compaction_root, encode_db_head,
	},
};
#[cfg(feature = "test-faults")]
use depot::{
	cold_tier::FaultyColdTier,
	fault::{DepotFaultController, DepotFaultPoint, ReadFaultPoint},
};
use futures_util::TryStreamExt;
use gas::prelude::Id;
use rivet_pools::NodeId;
use sha2::{Digest, Sha256};
use tokio::{
	sync::{Barrier, Notify},
	time::timeout,
};
use universaldb::utils::IsolationLevel::Serializable;

const TEST_DATABASE: &str = "test-database";

fn test_bucket() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x5678), 1)
}

fn head_at(head_txid: u64, db_size_pages: u32) -> DBHead {
	DBHead {
		head_txid,
		db_size_pages,
		post_apply_checksum: 0,
		branch_id: DatabaseBranchId::nil(),
		#[cfg(debug_assertions)]
		generation: 1,
	}
}

fn page(fill: u8) -> Vec<u8> {
	vec![fill; PAGE_SIZE as usize]
}

fn dirty_page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: page(fill),
	}
}

fn encoded_blob(txid: u64, pages: &[(u32, u8)]) -> Result<Vec<u8>> {
	let pages = pages
		.iter()
		.map(|(pgno, fill)| DirtyPage {
			pgno: *pgno,
			bytes: page(*fill),
		})
		.collect::<Vec<_>>();

	encode_ltx_v3(LtxHeader::delta(txid, 1, 999), &pages)
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
	let digest = Sha256::digest(bytes);
	let mut hash = [0_u8; 32];
	hash.copy_from_slice(&digest);
	hash
}

fn compaction_root(manifest_generation: u64) -> CompactionRoot {
	CompactionRoot {
		schema_version: 1,
		manifest_generation,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	}
}

fn cold_shard_ref(
	object_key: String,
	shard_id: u32,
	as_of_txid: u64,
	publish_generation: u64,
	object_bytes: &[u8],
) -> ColdShardRef {
	ColdShardRef {
		object_key,
		object_generation_id: Id::v1(uuid::Uuid::from_u128(0x1234), 7),
		shard_id,
		as_of_txid,
		min_txid: 1,
		max_txid: as_of_txid,
		min_versionstamp: [1; 16],
		max_versionstamp: [2; 16],
		size_bytes: object_bytes.len() as u64,
		content_hash: sha256(object_bytes),
		publish_generation,
	}
}

async fn read_database_branch_id(db: &universaldb::Database) -> Result<DatabaseBranchId> {
	db.run(|tx| async move {
		branch::resolve_database_branch(
			&tx,
			depot::types::BucketId::from_gas_id(test_bucket()),
			TEST_DATABASE,
			Serializable,
		)
		.await?
		.ok_or_else(|| anyhow::anyhow!("database branch should exist"))
	})
	.await
}

async fn seed(
	db: &universaldb::Database,
	writes: Vec<(Vec<u8>, Vec<u8>)>,
	deletes: Vec<Vec<u8>>,
) -> Result<()> {
	db.run(move |tx| {
		let writes = writes.clone();
		let deletes = deletes.clone();
		async move {
			for (key, value) in writes {
				tx.informal().set(&key, &value);
			}
			for key in deletes {
				tx.informal().clear(&key);
			}
			Ok(())
		}
	})
	.await
}

async fn read_i64_le(db: &universaldb::Database, key: Vec<u8>) -> Result<Option<i64>> {
	db.run(move |tx| {
		let key = key.clone();
		async move {
			tx.informal()
				.get(&key, Serializable)
				.await?
				.map(|bytes| {
					let bytes: [u8; 8] = bytes
						.as_slice()
						.try_into()
						.map_err(|_| anyhow::anyhow!("expected i64 bytes"))?;
					Ok(i64::from_le_bytes(bytes))
				})
				.transpose()
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
				.get(&key, Serializable)
				.await?
				.map(Vec::<u8>::from))
		}
	})
	.await
}

async fn read_prefix_keys(db: &universaldb::Database, prefix: Vec<u8>) -> Result<Vec<Vec<u8>>> {
	db.run(move |tx| {
		let prefix = prefix.clone();
		async move {
			let prefix_subspace =
				universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix));
			let informal = tx.informal();
			let mut stream = informal.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: universaldb::options::StreamingMode::WantAll,
					..universaldb::RangeOption::from(&prefix_subspace)
				},
				Serializable,
			);
			let mut keys = Vec::new();
			while let Some(entry) = stream.try_next().await? {
				keys.push(entry.key().to_vec());
			}
			Ok(keys)
		}
	})
	.await
}

struct PausingColdTier {
	inner: Arc<dyn ColdTier>,
	pause_key: String,
	get_started: Arc<Notify>,
	release_get: Arc<Notify>,
}

impl PausingColdTier {
	fn new(inner: Arc<dyn ColdTier>, pause_key: String) -> Self {
		Self {
			inner,
			pause_key,
			get_started: Arc::new(Notify::new()),
			release_get: Arc::new(Notify::new()),
		}
	}

	async fn wait_for_paused_get(&self) {
		self.get_started.notified().await;
	}

	fn release_paused_get(&self) {
		self.release_get.notify_one();
	}
}

#[async_trait]
impl ColdTier for PausingColdTier {
	async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
		self.inner.put_object(key, bytes).await
	}

	async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>> {
		if key == self.pause_key {
			self.get_started.notify_one();
			self.release_get.notified().await;
		}

		self.inner.get_object(key).await
	}

	async fn delete_objects(&self, keys: &[String]) -> Result<()> {
		self.inner.delete_objects(keys).await
	}

	async fn list_prefix(&self, prefix: &str) -> Result<Vec<ColdTierObjectMetadata>> {
		self.inner.list_prefix(prefix).await
	}
}

async fn assert_shard_coverage_missing(database_db: &Db, pgno: u32) -> Result<()> {
	let err = database_db
		.get_pages(vec![pgno])
		.await
		.expect_err("cold-disabled reads should fail on cold-only coverage");
	assert!(matches!(
		err.downcast_ref::<SqliteStorageError>(),
		Some(SqliteStorageError::ShardCoverageMissing { pgno: missing_pgno })
			if *missing_pgno == pgno
	));

	Ok(())
}

macro_rules! read_matrix {
	($prefix:expr, |$ctx:ident, $db:ident, $database_db:ident| $body:block) => {
		common::test_matrix($prefix, |_tier, $ctx| {
			Box::pin(async move {
				#[allow(unused_variables)]
				let $db = $ctx.udb.clone();
				let $database_db = $ctx.make_db(test_bucket(), TEST_DATABASE);
				$body
			})
		})
		.await
	};
}

#[tokio::test]
async fn get_pages_rejects_page_zero() -> Result<()> {
	read_matrix!("depot-read-page-zero", |ctx, db, database_db| {
		let err = database_db
			.get_pages(vec![0])
			.await
			.expect_err("read should reject page 0");
		assert!(err.to_string().contains("get_pages does not accept page 0"));

		Ok(())
	})
}

#[tokio::test]
async fn missing_delta_without_fallback_errors_instead_of_zero_fill() -> Result<()> {
	read_matrix!(
		"depot-read-missing-delta-no-fallback",
		|ctx, db, database_db| {
			database_db
				.commit(vec![dirty_page(1, 0x11)], 1, 1_000)
				.await?;
			let branch_id = read_database_branch_id(&db).await?;
			seed(
				&db,
				Vec::new(),
				vec![branch_delta_chunk_key(branch_id, 1, 0)],
			)
			.await?;

			let err = database_db
				.get_pages(vec![1])
				.await
				.expect_err("missing delta without fallback should fail loudly");
			assert!(matches!(
				err.downcast_ref::<SqliteStorageError>(),
				Some(SqliteStorageError::ShardCoverageMissing { pgno: 1 })
			));

			Ok(())
		}
	)
}

#[tokio::test]
async fn missing_delta_chunks_fail_loudly() -> Result<()> {
	for label in ["first", "middle", "last"] {
		let db = common::test_db_arc(&format!("depot-read-missing-{label}-delta")).await?;
		let database_db = common::make_db(db.clone(), test_bucket(), TEST_DATABASE.to_string());
		let dirty_pages = (1..=20)
			.map(|pgno| dirty_page(pgno, pgno as u8))
			.collect::<Vec<_>>();
		database_db.commit(dirty_pages, 20, 1_000).await?;
		let branch_id = read_database_branch_id(&db).await?;
		let existing_chunk_keys =
			read_prefix_keys(&db, branch_delta_chunk_prefix(branch_id, 1)).await?;
		seed(&db, Vec::new(), existing_chunk_keys).await?;
		let blob = encoded_blob(
			1,
			&(1..=20).map(|pgno| (pgno, pgno as u8)).collect::<Vec<_>>(),
		)?;
		let chunk_writes = blob
			.chunks(10)
			.enumerate()
			.map(|(idx, chunk)| {
				(
					branch_delta_chunk_key(branch_id, 1, idx as u32),
					chunk.to_vec(),
				)
			})
			.collect::<Vec<_>>();
		seed(&db, chunk_writes, Vec::new()).await?;
		let chunk_keys = read_prefix_keys(&db, branch_delta_chunk_prefix(branch_id, 1)).await?;
		assert!(
			chunk_keys.len() >= 3,
			"test setup should create at least three delta chunks"
		);
		let deleted_chunk = match label {
			"first" => 0,
			"middle" => chunk_keys.len() / 2,
			"last" => chunk_keys.len() - 1,
			_ => unreachable!("test labels are fixed"),
		};
		seed(&db, Vec::new(), vec![chunk_keys[deleted_chunk].clone()]).await?;

		let err = database_db
			.get_pages(vec![20])
			.await
			.expect_err("missing delta chunk should fail loudly");
		assert!(
			err.chain().any(|cause| {
				let message = cause.to_string();
				message.contains("sqlite delta chunks must be contiguous")
					|| message.contains("decode source blob for page")
			}),
			"unexpected error for missing {label} chunk: {err:?}"
		);
	}

	Ok(())
}

#[cfg(feature = "test-faults")]
#[tokio::test]
async fn read_fault_before_return_pages_fails_with_page_scope() -> Result<()> {
	let db = common::test_db_arc("depot-read-fault-before-return").await?;
	let writer = common::make_db(db.clone(), test_bucket(), TEST_DATABASE.to_string());
	writer.commit(vec![dirty_page(1, 0x11)], 1, 1_000).await?;
	let controller = DepotFaultController::new();
	controller
		.at(DepotFaultPoint::Read(ReadFaultPoint::BeforeReturnPages))
		.page_number(1)
		.once()
		.fail("before return failed")?;
	let reader = Db::new_with_fault_controller_for_test(
		db,
		test_bucket(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
		controller.clone(),
	);

	let err = reader
		.get_pages(vec![1])
		.await
		.expect_err("read fault should fail get_pages");
	assert!(err.to_string().contains("before return failed"));
	controller.assert_expected_fired()?;

	Ok(())
}

#[tokio::test]
async fn cold_ref_retired_during_cold_object_fetch_errors_instead_of_zero_fill() -> Result<()> {
	let (db, _db_dir) = common::test_db_with_dir("depot-read-cold-ref-retire-race").await?;
	let cold_dir = tempfile::tempdir()?;
	let filesystem_tier: Arc<dyn ColdTier> = Arc::new(FilesystemColdTier::new(cold_dir.path()));
	let writer_db = common::make_db(db.clone(), test_bucket(), TEST_DATABASE.to_string());
	writer_db
		.commit(vec![dirty_page(1, 0x11)], 1, 1_000)
		.await?;
	let branch_id = read_database_branch_id(&db).await?;
	let object_key = format!(
		"db/{}/shard/00000000/0000000000000001-retire-race.ltx",
		branch_id.as_uuid().simple(),
	);
	let object_bytes = encoded_blob(1, &[(1, 0x99)])?;
	let cold_ref = cold_shard_ref(object_key.clone(), 0, 1, 2, &object_bytes);
	filesystem_tier
		.put_object(&object_key, &object_bytes)
		.await?;
	seed(
		&db,
		vec![
			(
				branch_compaction_root_key(branch_id),
				encode_compaction_root(compaction_root(2))?,
			),
			(
				branch_compaction_cold_shard_key(branch_id, 0, 1),
				encode_cold_shard_ref(cold_ref)?,
			),
		],
		vec![
			branch_delta_chunk_key(branch_id, 1, 0),
			branch_pidx_key(branch_id, 1),
		],
	)
	.await?;

	let pausing_tier = Arc::new(PausingColdTier::new(
		filesystem_tier.clone(),
		object_key.clone(),
	));
	let database_db = Arc::new(Db::new_with_cold_tier(
		db.clone(),
		test_bucket(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
		pausing_tier.clone(),
	));
	let read_task = tokio::spawn({
		let database_db = database_db.clone();
		async move { database_db.get_pages(vec![1]).await }
	});
	timeout(Duration::from_secs(5), pausing_tier.wait_for_paused_get()).await?;

	seed(
		&db,
		vec![(
			branch_compaction_root_key(branch_id),
			encode_compaction_root(compaction_root(3))?,
		)],
		vec![branch_compaction_cold_shard_key(branch_id, 0, 1)],
	)
	.await?;
	filesystem_tier
		.delete_objects(std::slice::from_ref(&object_key))
		.await?;
	pausing_tier.release_paused_get();

	let err = read_task
		.await?
		.expect_err("retired cold ref should error instead of returning a zero-filled page");
	assert!(matches!(
		err.downcast_ref::<SqliteStorageError>(),
		Some(SqliteStorageError::ShardCoverageMissing { pgno: 1 })
	));

	Ok(())
}

#[tokio::test]
async fn get_pages_reads_with_cold_pidx_scan() -> Result<()> {
	read_matrix!("depot-read-pidx-scan", |ctx, db, database_db| {
		database_db
			.commit(vec![dirty_page(2, 0x22)], 3, 1_000)
			.await?;

		assert_eq!(
			database_db.get_pages(vec![2]).await?,
			vec![FetchedPage {
				pgno: 2,
				bytes: Some(page(0x22)),
			}]
		);

		Ok(())
	})
}

#[tokio::test]
async fn branch_cache_snapshot_is_atomic_across_dbptr_move() -> Result<()> {
	read_matrix!(
		"depot-read-cache-snapshot-atomic",
		|ctx, db, database_db| {
			let database_db = Arc::new(database_db);
			database_db
				.commit(vec![dirty_page(1, 0x11)], 2, 1_000)
				.await?;
			database_db
				.commit(vec![dirty_page(1, 0x22)], 2, 2_000)
				.await?;
			let old_branch_id = read_database_branch_id(&db).await?;
			let first_commit = decode_commit_row(
				&read_value(&db, branch_commit_key(old_branch_id, 1))
					.await?
					.expect("first commit row should exist"),
			)?;

			assert_eq!(
				database_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(page(0x22)),
				}]
			);
			let (cached_branch_id, cached_root_branch_id, _, _) = database_db
				.branch_cache_snapshot_for_test()
				.await
				.expect("branch cache should be warm");
			assert_eq!(cached_branch_id, old_branch_id);
			assert_eq!(cached_root_branch_id, old_branch_id);

			let new_branch_id = branch::rollback_database(
				&db,
				depot::types::BucketId::from_gas_id(test_bucket()),
				TEST_DATABASE.to_string(),
				ResolvedVersionstamp {
					versionstamp: first_commit.versionstamp,
					restore_point: None,
				},
			)
			.await?;
			assert_ne!(new_branch_id, old_branch_id);

			let start = Arc::new(Barrier::new(4));
			let mut readers = Vec::new();
			for _ in 0..2 {
				let reader_db = Arc::clone(&database_db);
				let reader_start = Arc::clone(&start);
				readers.push(tokio::spawn(async move {
					reader_start.wait().await;
					reader_db.get_pages(vec![1]).await
				}));
			}

			let observer_db = Arc::clone(&database_db);
			let observer_start = Arc::clone(&start);
			let observer = tokio::spawn(async move {
				observer_start.wait().await;
				for _ in 0..8 {
					if let Some((branch_id, root_branch_id, _, _)) =
						observer_db.branch_cache_snapshot_for_test().await
					{
						assert!(
							branch_id != new_branch_id || root_branch_id == new_branch_id,
							"branch cache exposed new branch id with stale ancestry"
						);
					}
					tokio::task::yield_now().await;
				}
			});

			start.wait().await;
			for reader in readers {
				let pages = reader.await??;
				assert_eq!(
					pages,
					vec![FetchedPage {
						pgno: 1,
						bytes: Some(page(0x11)),
					}]
				);
			}
			observer.await?;

			let (cached_branch_id, cached_root_branch_id, _, _) = database_db
				.branch_cache_snapshot_for_test()
				.await
				.expect("branch cache should stay warm");
			assert_eq!(cached_branch_id, new_branch_id);
			assert_eq!(cached_root_branch_id, new_branch_id);

			Ok(())
		}
	)
}

#[tokio::test]
async fn get_pages_uses_warm_cache_without_pidx_row() -> Result<()> {
	read_matrix!("depot-read-warm-cache", |ctx, db, database_db| {
		database_db
			.commit(vec![dirty_page(2, 0x22)], 3, 1_000)
			.await?;
		let branch_id = read_database_branch_id(&db).await?;
		assert_eq!(
			database_db.get_pages(vec![2]).await?,
			vec![FetchedPage {
				pgno: 2,
				bytes: Some(page(0x22)),
			}]
		);

		seed(&db, Vec::new(), vec![branch_pidx_key(branch_id, 2)]).await?;

		assert_eq!(
			database_db.get_pages(vec![2]).await?,
			vec![FetchedPage {
				pgno: 2,
				bytes: Some(page(0x22)),
			}]
		);

		Ok(())
	})
}

#[tokio::test]
async fn get_pages_falls_back_to_shard_when_cached_pidx_is_stale() -> Result<()> {
	read_matrix!("depot-read-stale-pidx", |ctx, db, database_db| {
		database_db
			.commit(vec![dirty_page(2, 0x22)], 3, 1_000)
			.await?;
		let branch_id = read_database_branch_id(&db).await?;
		assert_eq!(
			database_db.get_pages(vec![2]).await?,
			vec![FetchedPage {
				pgno: 2,
				bytes: Some(page(0x22)),
			}]
		);

		seed(
			&db,
			vec![(
				branch_shard_key(branch_id, 0, 1),
				encoded_blob(1, &[(2, 0x44)])?,
			)],
			vec![
				branch_delta_chunk_key(branch_id, 1, 0),
				branch_pidx_key(branch_id, 2),
			],
		)
		.await?;

		assert_eq!(
			database_db.get_pages(vec![2]).await?,
			vec![FetchedPage {
				pgno: 2,
				bytes: Some(page(0x44)),
			}]
		);

		Ok(())
	})
}

#[tokio::test]
async fn get_pages_reads_latest_shard_version_not_past_head() -> Result<()> {
	read_matrix!("depot-read-shard-version", |ctx, db, database_db| {
		database_db
			.commit(vec![dirty_page(1, 0x11)], 3, 1_000)
			.await?;
		let branch_id = read_database_branch_id(&db).await?;
		seed(
			&db,
			vec![
				(
					branch_meta_head_key(branch_id),
					encode_db_head(head_at(4, 3))?,
				),
				(
					branch_shard_key(branch_id, 0, 2),
					encoded_blob(2, &[(2, 0x22)])?,
				),
				(
					branch_shard_key(branch_id, 0, 4),
					encoded_blob(4, &[(2, 0x44)])?,
				),
				(
					branch_shard_key(branch_id, 0, 5),
					encoded_blob(5, &[(2, 0x55)])?,
				),
			],
			Vec::new(),
		)
		.await?;

		assert_eq!(
			database_db.get_pages(vec![2]).await?,
			vec![FetchedPage {
				pgno: 2,
				bytes: Some(page(0x44)),
			}]
		);

		Ok(())
	})
}

#[tokio::test]
async fn get_pages_reads_delta_before_published_branch_shard() -> Result<()> {
	read_matrix!("depot-read-delta-before-shard", |ctx, db, database_db| {
		database_db
			.commit(vec![dirty_page(1, 0x11)], 1, 1_000)
			.await?;
		let branch_id = read_database_branch_id(&db).await?;

		seed(
			&db,
			vec![
				(
					branch_compaction_root_key(branch_id),
					encode_compaction_root(compaction_root(1))?,
				),
				(
					branch_shard_key(branch_id, 0, 1),
					encoded_blob(1, &[(1, 0x44)])?,
				),
			],
			Vec::new(),
		)
		.await?;

		assert_eq!(
			database_db.get_pages(vec![1]).await?,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0x11)),
			}]
		);

		Ok(())
	})
}

#[tokio::test]
async fn get_pages_falls_back_to_published_branch_shard_when_delta_is_missing() -> Result<()> {
	read_matrix!("depot-read-shard-fallback", |ctx, db, database_db| {
		let fdb_hit = metrics::SQLITE_SHARD_CACHE_READ_TOTAL
			.with_label_values(&[metrics::SHARD_CACHE_READ_FDB_HIT]);
		let fdb_hit_before = fdb_hit.get();
		database_db
			.commit(vec![dirty_page(1, 0x11)], 1, 1_000)
			.await?;
		let branch_id = read_database_branch_id(&db).await?;

		seed(
			&db,
			vec![
				(
					branch_compaction_root_key(branch_id),
					encode_compaction_root(compaction_root(1))?,
				),
				(
					branch_shard_key(branch_id, 0, 1),
					encoded_blob(1, &[(1, 0x44)])?,
				),
			],
			vec![branch_delta_chunk_key(branch_id, 1, 0)],
		)
		.await?;

		assert_eq!(
			database_db.get_pages(vec![1]).await?,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0x44)),
			}]
		);
		assert!(fdb_hit.get() >= fdb_hit_before + 1);

		Ok(())
	})
}

#[tokio::test]
async fn get_pages_records_shard_cache_miss_when_no_shard_or_cold_ref_covers_page() -> Result<()> {
	common::test_matrix("depot-read-cache-miss", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_db = ctx.make_db(test_bucket(), TEST_DATABASE);
			let miss = metrics::SQLITE_SHARD_CACHE_READ_TOTAL
				.with_label_values(&[metrics::SHARD_CACHE_READ_MISS]);
			let miss_before = miss.get();
			database_db
				.commit(vec![dirty_page(1, 0x11)], 1, 1_000)
				.await?;
			let branch_id = read_database_branch_id(&db).await?;
			seed(
				&db,
				Vec::new(),
				vec![
					branch_delta_chunk_key(branch_id, 1, 0),
					branch_pidx_key(branch_id, 1),
				],
			)
			.await?;

			assert_eq!(
				database_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(page(0)),
				}]
			);
			assert!(miss.get() >= miss_before + 1);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn get_pages_zero_fills_sparse_page_without_any_source() -> Result<()> {
	read_matrix!("depot-read-sparse-zero", |ctx, db, database_db| {
		database_db
			.commit(vec![dirty_page(1, 0x11)], 3, 1_000)
			.await?;

		assert_eq!(
			database_db.get_pages(vec![2]).await?,
			vec![FetchedPage {
				pgno: 2,
				bytes: Some(page(0)),
			}]
		);

		Ok(())
	})
}

#[tokio::test]
async fn get_pages_errors_for_corrupted_delta_source() -> Result<()> {
	read_matrix!("depot-read-corrupt-delta", |ctx, db, database_db| {
		database_db
			.commit(vec![dirty_page(1, 0x11)], 1, 1_000)
			.await?;
		let branch_id = read_database_branch_id(&db).await?;
		seed(
			&db,
			vec![(
				branch_delta_chunk_key(branch_id, 1, 0),
				b"not an ltx blob".to_vec(),
			)],
			Vec::new(),
		)
		.await?;

		let err = database_db
			.get_pages(vec![1])
			.await
			.expect_err("corrupted delta source should error instead of zero-filling");
		assert!(
			err.chain()
				.any(|cause| cause.to_string().contains("decode source blob for page 1"))
		);

		Ok(())
	})
}

#[tokio::test]
async fn get_pages_returns_zero_for_hot_only_missing_in_range_page() -> Result<()> {
	read_matrix!("depot-read-hot-missing-zero", |ctx, db, database_db| {
		database_db
			.commit(vec![dirty_page(1, 0x11)], 3, 1_000)
			.await?;

		assert_eq!(
			database_db.get_pages(vec![2]).await?,
			vec![FetchedPage {
				pgno: 2,
				bytes: Some(page(0)),
			}]
		);

		Ok(())
	})
}

#[tokio::test]
async fn get_pages_throttles_access_touch_for_same_bucket_shard_reads() -> Result<()> {
	read_matrix!("depot-read-access-touch", |ctx, db, database_db| {
		database_db
			.commit(vec![dirty_page(1, 0x11)], 1, 1_000)
			.await?;
		let branch_id = read_database_branch_id(&db).await?;

		seed(
			&db,
			vec![(
				branch_shard_key(branch_id, 0, 1),
				encoded_blob(1, &[(1, 0x44)])?,
			)],
			vec![
				branch_delta_chunk_key(branch_id, 1, 0),
				branch_pidx_key(branch_id, 1),
			],
		)
		.await?;

		assert_eq!(
			database_db.get_pages(vec![1]).await?,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0x44)),
			}]
		);
		let first_touch = read_i64_le(&db, branch_manifest_last_access_ts_ms_key(branch_id))
			.await?
			.expect("shard read should touch access timestamp");
		let first_bucket = read_i64_le(&db, branch_manifest_last_access_bucket_key(branch_id))
			.await?
			.expect("shard read should touch access bucket");
		assert_eq!(
			first_bucket,
			first_touch.div_euclid(ACCESS_TOUCH_THROTTLE_MS)
		);

		assert_eq!(
			database_db.get_pages(vec![1]).await?,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0x44)),
			}]
		);
		assert_eq!(
			read_i64_le(&db, branch_manifest_last_access_ts_ms_key(branch_id)).await?,
			Some(first_touch)
		);
		assert_eq!(
			read_i64_le(&db, branch_manifest_last_access_bucket_key(branch_id)).await?,
			Some(first_bucket)
		);

		Ok(())
	})
}

#[tokio::test]
async fn get_pages_keeps_branch_shard_fallback_without_compaction_root() -> Result<()> {
	read_matrix!("depot-read-shard-no-root", |ctx, db, database_db| {
		database_db
			.commit(vec![dirty_page(1, 0x11)], 1, 1_000)
			.await?;
		let branch_id = read_database_branch_id(&db).await?;

		seed(
			&db,
			vec![(
				branch_shard_key(branch_id, 0, 1),
				encoded_blob(1, &[(1, 0x44)])?,
			)],
			vec![branch_delta_chunk_key(branch_id, 1, 0)],
		)
		.await?;

		assert_eq!(
			database_db.get_pages(vec![1]).await?,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0x44)),
			}]
		);

		Ok(())
	})
}

#[tokio::test]
async fn get_pages_falls_back_to_compaction_cold_shard_ref() -> Result<()> {
	common::test_matrix("depot-read-compaction-cold-ref", |tier_mode, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_db = ctx.make_db(test_bucket(), TEST_DATABASE);
			database_db
				.commit(vec![dirty_page(1, 0x11)], 1, 1_000)
				.await?;
			let branch_id = read_database_branch_id(&db).await?;
			let object_key = format!(
				"db/{}/shard/00000000/0000000000000001-{}-workflow.ltx",
				branch_id.as_uuid().simple(),
				Id::v1(uuid::Uuid::from_u128(0x1234), 7)
			);
			let object_bytes = encoded_blob(1, &[(1, 0x66)])?;

			if let Some(tier) = &ctx.cold_tier {
				tier.put_object(&object_key, &object_bytes).await?;
			}
			seed(
				&db,
				vec![
					(
						branch_compaction_root_key(branch_id),
						encode_compaction_root(compaction_root(2))?,
					),
					(
						branch_compaction_cold_shard_key(branch_id, 0, 1),
						encode_cold_shard_ref(cold_shard_ref(object_key, 0, 1, 2, &object_bytes))?,
					),
				],
				vec![
					branch_delta_chunk_key(branch_id, 1, 0),
					branch_pidx_key(branch_id, 1),
				],
			)
			.await?;

			if tier_mode == common::TierMode::Disabled {
				let err = database_db
					.get_pages(vec![1])
					.await
					.expect_err("cold-disabled reads should fail on cold-only coverage");
				assert!(matches!(
					err.downcast_ref::<SqliteStorageError>(),
					Some(SqliteStorageError::ShardCoverageMissing { pgno: 1 })
				));
				return Ok(());
			}

			assert_eq!(
				database_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(page(0x66)),
				}]
			);
			let last_access_ts_ms =
				read_i64_le(&db, branch_manifest_last_access_ts_ms_key(branch_id))
					.await?
					.expect("cold-backed read should touch access timestamp");
			let last_access_bucket =
				read_i64_le(&db, branch_manifest_last_access_bucket_key(branch_id))
					.await?
					.expect("cold-backed read should touch access bucket");
			assert!(last_access_ts_ms > 1_000);
			assert_eq!(
				last_access_bucket,
				last_access_ts_ms.div_euclid(ACCESS_TOUCH_THROTTLE_MS)
			);

			Ok(())
		})
	})
	.await
}

#[cfg(feature = "test-faults")]
#[tokio::test]
async fn cold_tier_drop_artifact_fault_errors_instead_of_zero_fill() -> Result<()> {
	let (db, _db_dir) = common::test_db_with_dir("depot-read-cold-drop-artifact").await?;
	let cold_dir = tempfile::tempdir()?;
	let filesystem_tier = FilesystemColdTier::new(cold_dir.path());
	let writer = common::make_db(db.clone(), test_bucket(), TEST_DATABASE.to_string());
	writer.commit(vec![dirty_page(1, 0x11)], 1, 1_000).await?;
	let branch_id = read_database_branch_id(&db).await?;
	let object_key = format!(
		"db/{}/shard/00000000/0000000000000001-drop.ltx",
		branch_id.as_uuid().simple(),
	);
	let object_bytes = encoded_blob(1, &[(1, 0x66)])?;
	filesystem_tier
		.put_object(&object_key, &object_bytes)
		.await?;
	let controller = DepotFaultController::new();
	controller
		.at(DepotFaultPoint::ColdTier(
			depot::fault::ColdTierFaultPoint::GetObject,
		))
		.once()
		.drop_artifact()?;
	let faulty_tier: Arc<dyn ColdTier> =
		Arc::new(FaultyColdTier::new_with_fault_controller_for_test(
			filesystem_tier,
			"cold-drop-artifact-node",
			controller.clone(),
		));
	let reader = Db::new_with_cold_tier_and_fault_controller_for_test(
		db.clone(),
		test_bucket(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
		faulty_tier,
		controller.clone(),
	);
	seed(
		&db,
		vec![
			(
				branch_compaction_root_key(branch_id),
				encode_compaction_root(compaction_root(2))?,
			),
			(
				branch_compaction_cold_shard_key(branch_id, 0, 1),
				encode_cold_shard_ref(cold_shard_ref(object_key, 0, 1, 2, &object_bytes))?,
			),
		],
		vec![
			branch_delta_chunk_key(branch_id, 1, 0),
			branch_pidx_key(branch_id, 1),
		],
	)
	.await?;

	let err = reader
		.get_pages(vec![1])
		.await
		.expect_err("dropped cold object should fail loudly");
	assert!(matches!(
		err.downcast_ref::<SqliteStorageError>(),
		Some(SqliteStorageError::ShardCoverageMissing { pgno: 1 })
	));
	controller.assert_expected_fired()?;

	Ok(())
}

#[tokio::test]
async fn cold_shard_reads_cover_shard_boundaries() -> Result<()> {
	let (db, _db_dir) = common::test_db_with_dir("depot-read-cold-boundaries").await?;
	let cold_dir = tempfile::tempdir()?;
	let tier: Arc<dyn ColdTier> = Arc::new(FilesystemColdTier::new(cold_dir.path()));
	let database_db = Db::new_with_cold_tier(
		db.clone(),
		test_bucket(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
		tier.clone(),
	);
	let pgnos = [63, 64, 65, 127, 128, 129];
	let dirty_pages = pgnos
		.iter()
		.map(|pgno| dirty_page(*pgno, *pgno as u8))
		.collect::<Vec<_>>();
	database_db.commit(dirty_pages, 129, 1_000).await?;
	let branch_id = read_database_branch_id(&db).await?;
	let shard_0_bytes = encoded_blob(1, &[(63, 0x63)])?;
	let shard_1_bytes = encoded_blob(1, &[(64, 0x64), (65, 0x65), (127, 0x7f)])?;
	let shard_2_bytes = encoded_blob(1, &[(128, 0x80), (129, 0x81)])?;
	let shard_0_key = format!(
		"db/{}/shard/00000000/0000000000000001-boundary.ltx",
		branch_id.as_uuid().simple(),
	);
	let shard_1_key = format!(
		"db/{}/shard/00000001/0000000000000001-boundary.ltx",
		branch_id.as_uuid().simple(),
	);
	let shard_2_key = format!(
		"db/{}/shard/00000002/0000000000000001-boundary.ltx",
		branch_id.as_uuid().simple(),
	);
	tier.put_object(&shard_0_key, &shard_0_bytes).await?;
	tier.put_object(&shard_1_key, &shard_1_bytes).await?;
	tier.put_object(&shard_2_key, &shard_2_bytes).await?;
	let mut deletes = read_prefix_keys(&db, branch_delta_chunk_prefix(branch_id, 1)).await?;
	deletes.extend(pgnos.iter().map(|pgno| branch_pidx_key(branch_id, *pgno)));
	seed(
		&db,
		vec![
			(
				branch_compaction_root_key(branch_id),
				encode_compaction_root(compaction_root(2))?,
			),
			(
				branch_compaction_cold_shard_key(branch_id, 0, 1),
				encode_cold_shard_ref(cold_shard_ref(shard_0_key, 0, 1, 2, &shard_0_bytes))?,
			),
			(
				branch_compaction_cold_shard_key(branch_id, 1, 1),
				encode_cold_shard_ref(cold_shard_ref(shard_1_key, 1, 1, 2, &shard_1_bytes))?,
			),
			(
				branch_compaction_cold_shard_key(branch_id, 2, 1),
				encode_cold_shard_ref(cold_shard_ref(shard_2_key, 2, 1, 2, &shard_2_bytes))?,
			),
		],
		deletes,
	)
	.await?;

	assert_eq!(
		database_db.get_pages(pgnos.to_vec()).await?,
		vec![
			FetchedPage {
				pgno: 63,
				bytes: Some(page(0x63))
			},
			FetchedPage {
				pgno: 64,
				bytes: Some(page(0x64))
			},
			FetchedPage {
				pgno: 65,
				bytes: Some(page(0x65))
			},
			FetchedPage {
				pgno: 127,
				bytes: Some(page(0x7f))
			},
			FetchedPage {
				pgno: 128,
				bytes: Some(page(0x80))
			},
			FetchedPage {
				pgno: 129,
				bytes: Some(page(0x81))
			},
		]
	);

	Ok(())
}

#[tokio::test]
async fn cold_shard_read_returns_before_background_cache_fill() -> Result<()> {
	common::test_matrix("depot-read-fill-before", |tier_mode, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_db = match ctx.cold_tier.clone() {
				Some(tier) => Db::new_with_cold_tier_and_shard_cache_fill_limits_for_test(
					db.clone(),
					test_bucket(),
					TEST_DATABASE.to_string(),
					NodeId::new(),
					tier,
					1,
					0,
				),
				None => ctx.make_db(test_bucket(), TEST_DATABASE),
			};
			database_db
				.commit(vec![dirty_page(1, 0x11)], 1, 1_000)
				.await?;
			let branch_id = read_database_branch_id(&db).await?;
			let object_key = format!(
				"db/{}/shard/00000000/0000000000000001-read-before-fill.ltx",
				branch_id.as_uuid().simple(),
			);
			let object_bytes = encoded_blob(1, &[(1, 0x77)])?;
			let cold_ref = cold_shard_ref(object_key.clone(), 0, 1, 2, &object_bytes);

			if let Some(tier) = &ctx.cold_tier {
				tier.put_object(&object_key, &object_bytes).await?;
			}
			seed(
				&db,
				vec![
					(
						branch_compaction_root_key(branch_id),
						encode_compaction_root(compaction_root(2))?,
					),
					(
						branch_compaction_cold_shard_key(branch_id, 0, 1),
						encode_cold_shard_ref(cold_ref)?,
					),
				],
				vec![
					branch_delta_chunk_key(branch_id, 1, 0),
					branch_pidx_key(branch_id, 1),
				],
			)
			.await?;

			if tier_mode == common::TierMode::Disabled {
				assert_shard_coverage_missing(&database_db, 1).await?;
				return Ok(());
			}

			assert_eq!(
				database_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(page(0x77)),
				}]
			);
			assert_eq!(
				read_value(&db, branch_shard_key(branch_id, 0, 1)).await?,
				None
			);
			assert_eq!(database_db.shard_cache_fill_outstanding_for_test(), 1);

			Ok(())
		})
	})
	.await
}

#[cfg(feature = "test-faults")]
#[tokio::test]
async fn shard_cache_fill_enqueue_drop_artifact_skips_background_fill() -> Result<()> {
	let (db, _db_dir) = common::test_db_with_dir("depot-read-fill-drop").await?;
	let cold_dir = tempfile::tempdir()?;
	let tier: Arc<dyn ColdTier> = Arc::new(FilesystemColdTier::new(cold_dir.path()));
	let writer = Db::new_with_cold_tier(
		db.clone(),
		test_bucket(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
		tier.clone(),
	);
	writer.commit(vec![dirty_page(1, 0x11)], 1, 1_000).await?;
	let branch_id = read_database_branch_id(&db).await?;
	let object_key = format!(
		"db/{}/shard/00000000/0000000000000001-fill-drop.ltx",
		branch_id.as_uuid().simple(),
	);
	let object_bytes = encoded_blob(1, &[(1, 0x71)])?;
	let cold_ref = cold_shard_ref(object_key.clone(), 0, 1, 2, &object_bytes);
	tier.put_object(&object_key, &object_bytes).await?;
	seed(
		&db,
		vec![
			(
				branch_compaction_root_key(branch_id),
				encode_compaction_root(compaction_root(2))?,
			),
			(
				branch_compaction_cold_shard_key(branch_id, 0, 1),
				encode_cold_shard_ref(cold_ref)?,
			),
		],
		vec![
			branch_delta_chunk_key(branch_id, 1, 0),
			branch_pidx_key(branch_id, 1),
		],
	)
	.await?;
	let controller = DepotFaultController::new();
	controller
		.at(DepotFaultPoint::Read(ReadFaultPoint::ShardCacheFillEnqueue))
		.shard_id(0)
		.once()
		.drop_artifact()?;
	let reader = Db::new_with_cold_tier_and_fault_controller_for_test(
		db.clone(),
		test_bucket(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
		tier,
		controller.clone(),
	);

	assert_eq!(
		reader.get_pages(vec![1]).await?,
		vec![FetchedPage {
			pgno: 1,
			bytes: Some(page(0x71)),
		}]
	);
	reader.wait_for_shard_cache_fill_idle_for_test().await;
	assert_eq!(
		read_value(&db, branch_shard_key(branch_id, 0, 1)).await?,
		None
	);
	controller.assert_expected_fired()?;

	Ok(())
}

#[tokio::test]
async fn cold_shard_read_background_fills_shard_without_changing_watermarks() -> Result<()> {
	common::test_matrix("depot-read-fill-success", |tier_mode, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_db = ctx.make_db(test_bucket(), TEST_DATABASE);
			let cold_hit = metrics::SQLITE_SHARD_CACHE_READ_TOTAL
				.with_label_values(&[metrics::SHARD_CACHE_READ_COLD_HIT]);
			let fill_scheduled = metrics::SQLITE_SHARD_CACHE_FILL_TOTAL
				.with_label_values(&[metrics::SHARD_CACHE_FILL_SCHEDULED]);
			let fill_succeeded = metrics::SQLITE_SHARD_CACHE_FILL_TOTAL
				.with_label_values(&[metrics::SHARD_CACHE_FILL_SUCCEEDED]);
			let cold_hit_before = cold_hit.get();
			let fill_scheduled_before = fill_scheduled.get();
			let fill_succeeded_before = fill_succeeded.get();
			let fill_bytes_before = metrics::SQLITE_SHARD_CACHE_FILL_BYTES_TOTAL.get();
			database_db
				.commit(vec![dirty_page(1, 0x11)], 1, 1_000)
				.await?;
			let branch_id = read_database_branch_id(&db).await?;
			let object_key = format!(
				"db/{}/shard/00000000/0000000000000001-fill-success.ltx",
				branch_id.as_uuid().simple(),
			);
			let object_bytes = encoded_blob(1, &[(1, 0x88)])?;
			let cold_ref = cold_shard_ref(object_key.clone(), 0, 1, 2, &object_bytes);
			let root = CompactionRoot {
				hot_watermark_txid: 1,
				cold_watermark_txid: 1,
				cold_watermark_versionstamp: [8; 16],
				..compaction_root(2)
			};

			if let Some(tier) = &ctx.cold_tier {
				tier.put_object(&object_key, &object_bytes).await?;
			}
			seed(
				&db,
				vec![
					(
						branch_compaction_root_key(branch_id),
						encode_compaction_root(root.clone())?,
					),
					(
						branch_compaction_cold_shard_key(branch_id, 0, 1),
						encode_cold_shard_ref(cold_ref)?,
					),
				],
				vec![
					branch_delta_chunk_key(branch_id, 1, 0),
					branch_pidx_key(branch_id, 1),
				],
			)
			.await?;

			if tier_mode == common::TierMode::Disabled {
				assert_shard_coverage_missing(&database_db, 1).await?;
				return Ok(());
			}

			assert_eq!(
				database_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(page(0x88)),
				}]
			);
			database_db.wait_for_shard_cache_fill_idle_for_test().await;

			assert!(cold_hit.get() >= cold_hit_before + 1);
			assert!(fill_scheduled.get() >= fill_scheduled_before + 1);
			assert!(fill_succeeded.get() >= fill_succeeded_before + 1);
			assert!(
				metrics::SQLITE_SHARD_CACHE_FILL_BYTES_TOTAL.get()
					>= fill_bytes_before + u64::try_from(object_bytes.len()).unwrap_or(u64::MAX)
			);
			assert_eq!(
				read_value(&db, branch_shard_key(branch_id, 0, 1)).await?,
				Some(object_bytes)
			);
			let root_after = read_value(&db, branch_compaction_root_key(branch_id))
				.await?
				.expect("compaction root should remain present");
			let decoded = decode_compaction_root(&root_after)?;
			assert_eq!(decoded.hot_watermark_txid, root.hot_watermark_txid);
			assert_eq!(decoded.cold_watermark_txid, root.cold_watermark_txid);
			assert_eq!(
				decoded.cold_watermark_versionstamp,
				root.cold_watermark_versionstamp
			);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn shard_cache_fill_wait_idle_prearms_before_rechecking_outstanding() -> Result<()> {
	let udb = common::test_db_arc("depot-read-fill-idle-race").await?;
	let database_db = Arc::new(common::make_db(
		udb,
		test_bucket(),
		TEST_DATABASE.to_string(),
	));
	database_db.set_shard_cache_fill_outstanding_for_test(1);

	let hook_seen = Arc::new(Notify::new());
	let hook_db = database_db.clone();
	let hook_seen_for_hook = hook_seen.clone();
	database_db.set_shard_cache_fill_after_nonzero_load_hook_for_test(Arc::new(move || {
		hook_db.complete_one_shard_cache_fill_outstanding_for_test();
		hook_seen_for_hook.notify_one();
	}));

	let wait_db = database_db.clone();
	let waiter = tokio::spawn(async move {
		wait_db.wait_for_shard_cache_fill_idle_for_test().await;
	});

	timeout(Duration::from_secs(2), hook_seen.notified()).await?;
	timeout(Duration::from_secs(2), waiter).await??;
	assert_eq!(database_db.shard_cache_fill_outstanding_for_test(), 0);

	Ok(())
}

#[tokio::test]
async fn cold_shard_cache_fill_coalesces_duplicates_and_skips_when_queue_full() -> Result<()> {
	common::test_matrix("depot-read-fill-full", |tier_mode, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let node_id = NodeId::new();
			let node_label = node_id.to_string();
			let database_db = match ctx.cold_tier.clone() {
				Some(tier) => Db::new_with_cold_tier_and_shard_cache_fill_limits_for_test(
					db.clone(),
					test_bucket(),
					TEST_DATABASE.to_string(),
					node_id,
					tier,
					1,
					0,
				),
				None => ctx.make_db(test_bucket(), TEST_DATABASE),
			};
			database_db
				.commit(vec![dirty_page(1, 0x11), dirty_page(65, 0x22)], 65, 1_000)
				.await?;
			let branch_id = read_database_branch_id(&db).await?;
			let object_0_key = format!(
				"db/{}/shard/00000000/0000000000000001-full-0.ltx",
				branch_id.as_uuid().simple(),
			);
			let object_1_key = format!(
				"db/{}/shard/00000001/0000000000000001-full-1.ltx",
				branch_id.as_uuid().simple(),
			);
			let object_0_bytes = encoded_blob(1, &[(1, 0x91)])?;
			let object_1_bytes = encoded_blob(1, &[(65, 0x92)])?;
			if let Some(tier) = &ctx.cold_tier {
				tier.put_object(&object_0_key, &object_0_bytes).await?;
				tier.put_object(&object_1_key, &object_1_bytes).await?;
			}
			seed(
				&db,
				vec![
					(
						branch_compaction_root_key(branch_id),
						encode_compaction_root(compaction_root(2))?,
					),
					(
						branch_compaction_cold_shard_key(branch_id, 0, 1),
						encode_cold_shard_ref(cold_shard_ref(
							object_0_key,
							0,
							1,
							2,
							&object_0_bytes,
						))?,
					),
					(
						branch_compaction_cold_shard_key(branch_id, 1, 1),
						encode_cold_shard_ref(cold_shard_ref(
							object_1_key,
							1,
							1,
							2,
							&object_1_bytes,
						))?,
					),
				],
				vec![
					branch_delta_chunk_key(branch_id, 1, 0),
					branch_pidx_key(branch_id, 1),
					branch_pidx_key(branch_id, 65),
				],
			)
			.await?;

			if tier_mode == common::TierMode::Disabled {
				assert_shard_coverage_missing(&database_db, 1).await?;
				return Ok(());
			}

			let skipped = metrics::SQLITE_SHARD_CACHE_FILL_SKIPPED_QUEUE_FULL_TOTAL
				.with_label_values(&[node_label.as_str()]);
			let fill_scheduled = metrics::SQLITE_SHARD_CACHE_FILL_TOTAL
				.with_label_values(&[metrics::SHARD_CACHE_FILL_SCHEDULED]);
			let fill_duplicate = metrics::SQLITE_SHARD_CACHE_FILL_TOTAL
				.with_label_values(&[metrics::SHARD_CACHE_FILL_SKIPPED_DUPLICATE]);
			let fill_queue_full = metrics::SQLITE_SHARD_CACHE_FILL_TOTAL
				.with_label_values(&[metrics::SHARD_CACHE_FILL_SKIPPED_QUEUE_FULL]);
			let skipped_before = skipped.get();
			let fill_scheduled_before = fill_scheduled.get();
			let fill_duplicate_before = fill_duplicate.get();
			let fill_queue_full_before = fill_queue_full.get();
			database_db.get_pages(vec![1]).await?;
			assert_eq!(database_db.shard_cache_fill_outstanding_for_test(), 1);
			assert!(fill_scheduled.get() >= fill_scheduled_before + 1);
			database_db.get_pages(vec![1]).await?;
			assert_eq!(database_db.shard_cache_fill_outstanding_for_test(), 1);
			assert_eq!(skipped.get(), skipped_before);
			assert!(fill_duplicate.get() >= fill_duplicate_before + 1);
			database_db.get_pages(vec![65]).await?;
			assert_eq!(database_db.shard_cache_fill_outstanding_for_test(), 1);
			assert_eq!(skipped.get(), skipped_before + 1);
			assert!(fill_queue_full.get() >= fill_queue_full_before + 1);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn direct_shard_cache_fill_skips_when_cold_ref_changes() -> Result<()> {
	read_matrix!("depot-read-fill-stale", |ctx, db, database_db| {
		let skipped_no_cold_ref = metrics::SQLITE_SHARD_CACHE_FILL_TOTAL
			.with_label_values(&[metrics::SHARD_CACHE_FILL_SKIPPED_NO_COLD_REF]);
		let skipped_no_cold_ref_before = skipped_no_cold_ref.get();
		database_db
			.commit(vec![dirty_page(1, 0x11)], 1, 1_000)
			.await?;
		let branch_id = read_database_branch_id(&db).await?;
		let object_bytes = encoded_blob(1, &[(1, 0xaa)])?;
		let old_ref = cold_shard_ref("old-object.ltx".to_string(), 0, 1, 2, &object_bytes);
		let new_ref = cold_shard_ref("new-object.ltx".to_string(), 0, 1, 3, &object_bytes);
		seed(
			&db,
			vec![(
				branch_compaction_cold_shard_key(branch_id, 0, 1),
				encode_cold_shard_ref(new_ref)?,
			)],
			Vec::new(),
		)
		.await?;

		database_db
			.fill_shard_cache_once_for_test(branch_id, old_ref, object_bytes)
			.await?;
		assert_eq!(
			read_value(&db, branch_shard_key(branch_id, 0, 1)).await?,
			None
		);
		assert!(skipped_no_cold_ref.get() >= skipped_no_cold_ref_before + 1);

		Ok(())
	})
}

#[tokio::test]
async fn direct_shard_cache_fill_is_idempotent_for_matching_shard_bytes() -> Result<()> {
	read_matrix!("depot-read-fill-idem", |ctx, db, database_db| {
		database_db
			.commit(vec![dirty_page(1, 0x11)], 1, 1_000)
			.await?;
		let branch_id = read_database_branch_id(&db).await?;
		let object_bytes = encoded_blob(1, &[(1, 0xbb)])?;
		let cold_ref = cold_shard_ref("idem-object.ltx".to_string(), 0, 1, 2, &object_bytes);
		seed(
			&db,
			vec![
				(
					branch_compaction_cold_shard_key(branch_id, 0, 1),
					encode_cold_shard_ref(cold_ref.clone())?,
				),
				(branch_shard_key(branch_id, 0, 1), object_bytes.clone()),
			],
			Vec::new(),
		)
		.await?;

		database_db
			.fill_shard_cache_once_for_test(branch_id, cold_ref, object_bytes.clone())
			.await?;
		assert_eq!(
			read_value(&db, branch_shard_key(branch_id, 0, 1)).await?,
			Some(object_bytes)
		);

		Ok(())
	})
}

#[tokio::test]
async fn direct_shard_cache_fill_reports_corruption_for_conflicting_shard_bytes() -> Result<()> {
	read_matrix!("depot-read-fill-corrupt", |ctx, db, database_db| {
		let failed = metrics::SQLITE_SHARD_CACHE_FILL_TOTAL
			.with_label_values(&[metrics::SHARD_CACHE_FILL_FAILED]);
		let failed_before = failed.get();
		database_db
			.commit(vec![dirty_page(1, 0x11)], 1, 1_000)
			.await?;
		let branch_id = read_database_branch_id(&db).await?;
		let object_bytes = encoded_blob(1, &[(1, 0xcc)])?;
		let existing_bytes = encoded_blob(1, &[(1, 0xdd)])?;
		let cold_ref = cold_shard_ref("corrupt-object.ltx".to_string(), 0, 1, 2, &object_bytes);
		seed(
			&db,
			vec![
				(
					branch_compaction_cold_shard_key(branch_id, 0, 1),
					encode_cold_shard_ref(cold_ref.clone())?,
				),
				(branch_shard_key(branch_id, 0, 1), existing_bytes.clone()),
			],
			Vec::new(),
		)
		.await?;

		let err = database_db
			.fill_shard_cache_once_for_test(branch_id, cold_ref, object_bytes)
			.await
			.expect_err("conflicting shard bytes should report corruption");
		assert!(matches!(
			err.downcast_ref::<SqliteStorageError>(),
			Some(SqliteStorageError::ShardCacheCorrupt {
				shard_id: 0,
				as_of_txid: 1,
			})
		));
		assert_eq!(
			read_value(&db, branch_shard_key(branch_id, 0, 1)).await?,
			Some(existing_bytes)
		);
		assert!(failed.get() >= failed_before + 1);

		Ok(())
	})
}

#[tokio::test]
async fn get_pages_falls_through_to_cold_tier_when_hot_branch_data_is_evicted() -> Result<()> {
	common::test_matrix("depot-read-cold-manifest-fallback", |tier_mode, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_db = ctx.make_db(test_bucket(), TEST_DATABASE);
			database_db
				.commit(vec![dirty_page(1, 0x66)], 1, 1_000)
				.await?;
			let branch_id = read_database_branch_id(&db).await?;
			let layer_key = format!(
				"db/{}/image/00000000/00000000-0000000000000001.ltx",
				branch_id.as_uuid().simple()
			);
			let chunk_key = format!(
				"db/{}/cold_manifest/chunks/read-cold.bare",
				branch_id.as_uuid().simple()
			);
			let index_key = format!(
				"db/{}/cold_manifest/index.bare",
				branch_id.as_uuid().simple()
			);
			let layer_bytes = encoded_blob(1, &[(1, 0x66)])?;

			if let Some(tier) = &ctx.cold_tier {
				tier.put_object(&layer_key, &layer_bytes).await?;
				tier.put_object(
					&chunk_key,
					&encode_cold_manifest_chunk(ColdManifestChunk {
						schema_version: SQLITE_STORAGE_COLD_SCHEMA_VERSION,
						branch_id,
						pass_versionstamp: [1; 16],
						layers: vec![LayerEntry {
							kind: LayerKind::Image,
							shard_id: Some(0),
							min_txid: 1,
							max_txid: 1,
							min_versionstamp: [1; 16],
							max_versionstamp: [1; 16],
							byte_size: layer_bytes.len() as u64,
							checksum: 0,
							object_key: layer_key,
						}],
						restore_points: Vec::new(),
					})?,
				)
				.await?;
				tier.put_object(
					&index_key,
					&encode_cold_manifest_index(ColdManifestIndex {
						schema_version: SQLITE_STORAGE_COLD_SCHEMA_VERSION,
						branch_id,
						chunks: vec![ColdManifestChunkRef {
							object_key: chunk_key,
							pass_versionstamp: [1; 16],
							min_versionstamp: [1; 16],
							max_versionstamp: [1; 16],
							byte_size: 1,
						}],
						last_pass_at_ms: 1_000,
						last_pass_versionstamp: [1; 16],
					})?,
				)
				.await?;
			}

			seed(
				&db,
				Vec::new(),
				vec![
					branch_delta_chunk_key(branch_id, 1, 0),
					branch_pidx_key(branch_id, 1),
				],
			)
			.await?;

			assert_eq!(
				database_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(if tier_mode == common::TierMode::Disabled {
						page(0)
					} else {
						page(0x66)
					}),
				}]
			);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn get_pages_returns_none_above_eof() -> Result<()> {
	read_matrix!("depot-read-above-eof", |ctx, db, database_db| {
		database_db
			.commit(vec![dirty_page(1, 0x11)], 3, 1_000)
			.await?;

		assert_eq!(
			database_db.get_pages(vec![4]).await?,
			vec![FetchedPage {
				pgno: 4,
				bytes: None,
			}]
		);

		Ok(())
	})
}
