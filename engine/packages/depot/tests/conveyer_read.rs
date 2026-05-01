use std::sync::Arc;

use anyhow::Result;
use gas::prelude::Id;
use rivet_pools::NodeId;
use depot::{
	cold_tier::{ColdTier, FilesystemColdTier},
	keys::{
		branch_compaction_cold_shard_key, branch_compaction_root_key, branch_delta_chunk_key,
		branch_pidx_key, branch_shard_key, delta_chunk_key, meta_head_key, pidx_delta_key,
		shard_key, shard_version_key, PAGE_SIZE,
	},
	ltx::{LtxHeader, encode_ltx_v3},
	conveyer::{Db, branch},
	types::{
		ColdManifestChunk, ColdManifestChunkRef, ColdManifestIndex, ColdShardRef, CompactionRoot,
		DBHead, DatabaseBranchId, DirtyPage, FetchedPage, LayerEntry, LayerKind,
		SQLITE_STORAGE_COLD_SCHEMA_VERSION, encode_cold_manifest_chunk,
		encode_cold_manifest_index, encode_cold_shard_ref, encode_compaction_root, encode_db_head,
	},
};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Serializable;
use universalpubsub::{PubSub, driver::memory::MemoryDriver};

const TEST_DATABASE: &str = "test-database";

fn test_namespace() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x5678), 1)
}

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("depot-read-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"depot-conveyer-read-test".to_string(),
	)))
}

fn head(db_size_pages: u32) -> DBHead {
	head_at(4, db_size_pages)
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
	size_bytes: u64,
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
		size_bytes,
		content_hash: [3; 32],
		publish_generation,
	}
}

async fn read_database_branch_id(db: &universaldb::Database) -> Result<DatabaseBranchId> {
	db.run(|tx| async move {
		branch::resolve_database_branch(
			&tx,
			depot::types::NamespaceId::from_gas_id(test_namespace()),
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

#[tokio::test]
async fn get_pages_reads_with_cold_pidx_scan() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed(
		&db,
		vec![
			(meta_head_key(TEST_DATABASE), encode_db_head(head(3))?),
			(delta_chunk_key(TEST_DATABASE, 4, 0), encoded_blob(4, &[(2, 0x22)])?),
			(pidx_delta_key(TEST_DATABASE, 2), 4_u64.to_be_bytes().to_vec()),
		],
		Vec::new(),
	)
	.await?;

	let database_db = Db::new(db, test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());

	assert_eq!(
		database_db.get_pages(vec![2]).await?,
		vec![FetchedPage {
			pgno: 2,
			bytes: Some(page(0x22)),
		}]
	);

	Ok(())
}

#[tokio::test]
async fn get_pages_uses_warm_cache_without_pidx_row() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed(
		&db,
		vec![
			(meta_head_key(TEST_DATABASE), encode_db_head(head(3))?),
			(delta_chunk_key(TEST_DATABASE, 4, 0), encoded_blob(4, &[(2, 0x22)])?),
			(pidx_delta_key(TEST_DATABASE, 2), 4_u64.to_be_bytes().to_vec()),
		],
		Vec::new(),
	)
	.await?;

	let database_db = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	assert_eq!(
		database_db.get_pages(vec![2]).await?,
		vec![FetchedPage {
			pgno: 2,
			bytes: Some(page(0x22)),
		}]
	);

	seed(&db, Vec::new(), vec![pidx_delta_key(TEST_DATABASE, 2)]).await?;

	assert_eq!(
		database_db.get_pages(vec![2]).await?,
		vec![FetchedPage {
			pgno: 2,
			bytes: Some(page(0x22)),
		}]
	);

	Ok(())
}

#[tokio::test]
async fn get_pages_falls_back_to_shard_when_cached_pidx_is_stale() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed(
		&db,
		vec![
			(meta_head_key(TEST_DATABASE), encode_db_head(head(3))?),
			(delta_chunk_key(TEST_DATABASE, 4, 0), encoded_blob(4, &[(2, 0x22)])?),
			(pidx_delta_key(TEST_DATABASE, 2), 4_u64.to_be_bytes().to_vec()),
		],
		Vec::new(),
	)
	.await?;

	let database_db = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	assert_eq!(
		database_db.get_pages(vec![2]).await?,
		vec![FetchedPage {
			pgno: 2,
			bytes: Some(page(0x22)),
		}]
	);

	seed(
		&db,
		vec![(shard_key(TEST_DATABASE, 0), encoded_blob(4, &[(2, 0x44)])?)],
		vec![
			delta_chunk_key(TEST_DATABASE, 4, 0),
			pidx_delta_key(TEST_DATABASE, 2),
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
}

#[tokio::test]
async fn get_pages_reads_latest_shard_version_not_past_head() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed(
		&db,
		vec![
			(meta_head_key(TEST_DATABASE), encode_db_head(head_at(4, 3))?),
			(shard_version_key(TEST_DATABASE, 0, 2), encoded_blob(2, &[(2, 0x22)])?),
			(shard_version_key(TEST_DATABASE, 0, 4), encoded_blob(4, &[(2, 0x44)])?),
			(shard_version_key(TEST_DATABASE, 0, 5), encoded_blob(5, &[(2, 0x55)])?),
		],
		Vec::new(),
	)
	.await?;

	let database_db = Db::new(db, test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());

	assert_eq!(
		database_db.get_pages(vec![2]).await?,
		vec![FetchedPage {
			pgno: 2,
			bytes: Some(page(0x44)),
		}]
	);

	Ok(())
}

#[tokio::test]
async fn get_pages_reads_delta_before_published_branch_shard() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	database_db.commit(vec![dirty_page(1, 0x11)], 1, 1_000).await?;
	let branch_id = read_database_branch_id(&db).await?;

	seed(
		&db,
		vec![
			(branch_compaction_root_key(branch_id), encode_compaction_root(compaction_root(1))?),
			(branch_shard_key(branch_id, 0, 1), encoded_blob(1, &[(1, 0x44)])?),
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
}

#[tokio::test]
async fn get_pages_falls_back_to_published_branch_shard_when_delta_is_missing() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	database_db.commit(vec![dirty_page(1, 0x11)], 1, 1_000).await?;
	let branch_id = read_database_branch_id(&db).await?;

	seed(
		&db,
		vec![
			(branch_compaction_root_key(branch_id), encode_compaction_root(compaction_root(1))?),
			(branch_shard_key(branch_id, 0, 1), encoded_blob(1, &[(1, 0x44)])?),
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

	Ok(())
}

#[tokio::test]
async fn get_pages_keeps_branch_shard_fallback_without_compaction_root() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(db.clone(), test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());
	database_db.commit(vec![dirty_page(1, 0x11)], 1, 1_000).await?;
	let branch_id = read_database_branch_id(&db).await?;

	seed(
		&db,
		vec![(branch_shard_key(branch_id, 0, 1), encoded_blob(1, &[(1, 0x44)])?)],
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
}

#[tokio::test]
async fn get_pages_falls_back_to_compaction_cold_shard_ref() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let cold_root = Builder::new().prefix("depot-read-workflow-cold-").tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));
	let database_db = Db::new_with_cold_tier(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
		tier.clone(),
	);
	database_db.commit(vec![dirty_page(1, 0x11)], 1, 1_000).await?;
	let branch_id = read_database_branch_id(&db).await?;
	let object_key = format!(
		"db/{}/shard/00000000/0000000000000001-{}-workflow.ltx",
		branch_id.as_uuid().simple(),
		Id::v1(uuid::Uuid::from_u128(0x1234), 7)
	);
	let object_bytes = encoded_blob(1, &[(1, 0x66)])?;

	tier.put_object(&object_key, &object_bytes).await?;
	seed(
		&db,
		vec![
			(branch_compaction_root_key(branch_id), encode_compaction_root(compaction_root(2))?),
			(
				branch_compaction_cold_shard_key(branch_id, 0, 1),
				encode_cold_shard_ref(cold_shard_ref(
					object_key,
					0,
					1,
					2,
					object_bytes.len() as u64,
				))?,
			),
		],
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
			bytes: Some(page(0x66)),
		}]
	);

	Ok(())
}

#[tokio::test]
async fn get_pages_falls_through_to_cold_tier_when_hot_branch_data_is_evicted() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let cold_root = Builder::new().prefix("depot-read-cold-").tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));
	let database_db = Db::new_with_cold_tier(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
		tier.clone(),
	);
	database_db.commit(vec![dirty_page(1, 0x66)], 1, 1_000).await?;
	let branch_id = read_database_branch_id(&db).await?;
	let layer_key = format!(
		"db/{}/image/00000000/00000000-0000000000000001.ltx",
		branch_id.as_uuid().simple()
	);
	let chunk_key = format!(
		"db/{}/cold_manifest/chunks/read-cold.bare",
		branch_id.as_uuid().simple()
	);
	let index_key = format!("db/{}/cold_manifest/index.bare", branch_id.as_uuid().simple());
	let layer_bytes = encoded_blob(1, &[(1, 0x66)])?;

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
			bookmarks: Vec::new(),
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
			bytes: Some(page(0x66)),
		}]
	);

	Ok(())
}

#[tokio::test]
async fn get_pages_returns_none_above_eof() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed(
		&db,
		vec![(meta_head_key(TEST_DATABASE), encode_db_head(head(3))?)],
		Vec::new(),
	)
	.await?;

	let database_db = Db::new(db, test_ups(), test_namespace(), TEST_DATABASE.to_string(), NodeId::new());

	assert_eq!(
		database_db.get_pages(vec![4]).await?,
		vec![FetchedPage {
			pgno: 4,
			bytes: None,
		}]
	);

	Ok(())
}
