use std::sync::Arc;

use anyhow::Result;
use gas::prelude::Id;
use rivet_pools::NodeId;
use depot::{
	cold_tier::{ColdTier, FilesystemColdTier},
	debug,
	keys::{PAGE_SIZE, branch_commit_key},
	conveyer::Db,
	types::{
		DatabaseBranchId, BookmarkStr, ColdManifestChunk, ColdManifestChunkRef, ColdManifestIndex,
		DirtyPage, LayerEntry, LayerKind, SQLITE_STORAGE_COLD_SCHEMA_VERSION, decode_commit_row,
		encode_cold_manifest_chunk, encode_cold_manifest_index,
	},
};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;
use universalpubsub::{PubSub, driver::memory::MemoryDriver};

const TEST_DATABASE: &str = "test-database";

fn test_namespace() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x5678), 1)
}

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("depot-debug-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"depot-debug-test".to_string(),
	)))
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; PAGE_SIZE as usize],
	}
}

async fn commit_row(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
	txid: u64,
) -> Result<depot::types::CommitRow> {
	db.run(move |tx| async move {
		let bytes = tx
			.informal()
			.get(&branch_commit_key(branch_id, txid), Snapshot)
			.await?
			.expect("commit row should exist");

		decode_commit_row(&bytes)
	})
	.await
}

#[tokio::test]
async fn debug_dumps_ancestry_pins_bookmarks_and_gc_pin() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
	);

	database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let branch_id = debug::dump_database_ancestry(&database_db).await?[0].0;
	let first_commit = commit_row(&db, branch_id, 1).await?;
	let pinned = database_db.create_pinned_bookmark(1_010).await?;
	database_db.commit(vec![page(2, 0x22)], 3, 2_000).await?;

	let ancestry = debug::dump_database_ancestry(&database_db).await?;
	assert_eq!(ancestry, vec![(branch_id, None)]);

	let pins = debug::dump_branch_pins(&database_db).await?;
	assert_eq!(pins.branch_id, branch_id);
	assert_eq!(pins.refcount, 1);
	assert_eq!(pins.bk_pin, first_commit.versionstamp);

	let bookmarks = debug::list_bookmarks(&database_db).await?;
	assert!(bookmarks.iter().any(|entry| {
		!entry.pinned && entry.bookmark_str == BookmarkStr::format(2_000, 2).unwrap()
	}));
	assert!(bookmarks.iter().any(|entry| {
		entry.pinned && entry.bookmark_str == pinned && entry.pin_status == depot::types::PinStatus::Pending
	}));

	assert_eq!(debug::estimate_gc_pin(&database_db).await?, first_commit.versionstamp);

	Ok(())
}

#[tokio::test]
async fn debug_read_at_returns_page_state_for_versionstamp() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let database_db = Db::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
	);

	database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let branch_id = debug::dump_database_ancestry(&database_db).await?[0].0;
	let first_commit = commit_row(&db, branch_id, 1).await?;
	database_db
		.commit(vec![page(1, 0x22), page(2, 0x33)], 3, 2_000)
		.await?;
	let second_commit = commit_row(&db, branch_id, 2).await?;

	let first_state = debug::read_at(&database_db, first_commit.versionstamp).await?;
	assert_eq!(first_state.txid, 1);
	assert_eq!(first_state.db_size_pages, 2);
	assert_eq!(first_state.pages[0].bytes.as_deref(), Some(&vec![0x11; PAGE_SIZE as usize][..]));
	assert_eq!(first_state.pages[1].bytes.as_deref(), Some(&vec![0; PAGE_SIZE as usize][..]));

	let second_state = debug::read_at(&database_db, second_commit.versionstamp).await?;
	assert_eq!(second_state.txid, 2);
	assert_eq!(second_state.db_size_pages, 3);
	assert_eq!(second_state.pages[0].bytes.as_deref(), Some(&vec![0x22; PAGE_SIZE as usize][..]));
	assert_eq!(second_state.pages[1].bytes.as_deref(), Some(&vec![0x33; PAGE_SIZE as usize][..]));

	Ok(())
}

#[tokio::test]
async fn debug_dump_cold_manifest_reads_index_and_chunks() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let cold_path = Builder::new()
		.prefix("depot-debug-cold-")
		.tempdir()?
		.keep();
	let cold_tier = Arc::new(FilesystemColdTier::new(cold_path));
	let database_db = Db::new_with_cold_tier(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
		cold_tier.clone(),
	);
	database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let branch_id = debug::dump_database_ancestry(&database_db).await?[0].0;
	let chunk_key = format!(
		"db/{}/cold_manifest/chunks/debug.bare",
		branch_id.as_uuid().simple()
	);
	let index_key = format!("db/{}/cold_manifest/index.bare", branch_id.as_uuid().simple());

	cold_tier
		.put_object(
			&chunk_key,
			&encode_cold_manifest_chunk(ColdManifestChunk {
				schema_version: SQLITE_STORAGE_COLD_SCHEMA_VERSION,
				branch_id,
				pass_versionstamp: [2; 16],
				layers: vec![LayerEntry {
					kind: LayerKind::Delta,
					shard_id: None,
					min_txid: 1,
					max_txid: 1,
					min_versionstamp: [1; 16],
					max_versionstamp: [1; 16],
					byte_size: 10,
					checksum: 99,
					object_key: "db/layer.ltx".to_string(),
				}],
				bookmarks: Vec::new(),
			})?,
		)
		.await?;
	cold_tier
		.put_object(
			&index_key,
			&encode_cold_manifest_index(ColdManifestIndex {
				schema_version: SQLITE_STORAGE_COLD_SCHEMA_VERSION,
				branch_id,
				chunks: vec![ColdManifestChunkRef {
					object_key: chunk_key,
					pass_versionstamp: [2; 16],
					min_versionstamp: [1; 16],
					max_versionstamp: [1; 16],
					byte_size: 10,
				}],
				last_pass_at_ms: 2_000,
				last_pass_versionstamp: [2; 16],
			})?,
		)
		.await?;

	let manifest = debug::dump_cold_manifest(&database_db).await?;
	assert_eq!(manifest.branch_id, branch_id);
	assert!(manifest.index.is_some());
	assert_eq!(manifest.chunks.len(), 1);
	assert_eq!(manifest.chunks[0].layers[0].checksum, 99);

	Ok(())
}
