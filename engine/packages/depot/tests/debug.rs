mod common;

use anyhow::Result;
use depot::{
	debug,
	keys::{PAGE_SIZE, branch_commit_key},
	types::{
		ColdManifestChunk, ColdManifestChunkRef, ColdManifestIndex, DatabaseBranchId, DirtyPage,
		LayerEntry, LayerKind, SQLITE_STORAGE_COLD_SCHEMA_VERSION, decode_commit_row,
		encode_cold_manifest_chunk, encode_cold_manifest_index,
	},
};
use universaldb::utils::IsolationLevel::Snapshot;

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
async fn debug_dumps_ancestry_pins_restore_points_and_gc_pin() -> Result<()> {
	common::test_matrix("depot-debug-ancestry", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_db = ctx.db;

			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let branch_id = debug::dump_database_ancestry(&database_db).await?[0].0;
			let first_commit = commit_row(&db, branch_id, 1).await?;
			let pinned = database_db
				.create_restore_point(depot::types::SnapshotSelector::Latest)
				.await?;
			database_db.commit(vec![page(2, 0x22)], 3, 2_000).await?;

			let ancestry = debug::dump_database_ancestry(&database_db).await?;
			assert_eq!(ancestry, vec![(branch_id, None)]);

			let pins = debug::dump_branch_pins(&database_db).await?;
			assert_eq!(pins.branch_id, branch_id);
			assert_eq!(pins.refcount, 1);
			assert_eq!(pins.restore_point_pin, first_commit.versionstamp);

			let restore_points = debug::list_restore_points(&database_db).await?;
			assert!(restore_points.iter().any(|entry| {
				entry.restore_point_id == pinned
					&& entry.pin_status == depot::types::PinStatus::Ready
			}));

			assert_eq!(
				debug::estimate_gc_pin(&database_db).await?,
				first_commit.versionstamp
			);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn debug_read_at_returns_page_state_for_versionstamp() -> Result<()> {
	common::test_matrix("depot-debug-read-at", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_db = ctx.db;

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
			assert_eq!(
				first_state.pages[0].bytes.as_deref(),
				Some(&vec![0x11; PAGE_SIZE as usize][..])
			);
			assert_eq!(
				first_state.pages[1].bytes.as_deref(),
				Some(&vec![0; PAGE_SIZE as usize][..])
			);

			let second_state = debug::read_at(&database_db, second_commit.versionstamp).await?;
			assert_eq!(second_state.txid, 2);
			assert_eq!(second_state.db_size_pages, 3);
			assert_eq!(
				second_state.pages[0].bytes.as_deref(),
				Some(&vec![0x22; PAGE_SIZE as usize][..])
			);
			assert_eq!(
				second_state.pages[1].bytes.as_deref(),
				Some(&vec![0x33; PAGE_SIZE as usize][..])
			);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn debug_dump_cold_manifest_reads_index_and_chunks() -> Result<()> {
	common::test_matrix("depot-debug-cold-manifest", |tier, ctx| {
		Box::pin(async move {
			let database_db = ctx.db;
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let branch_id = debug::dump_database_ancestry(&database_db).await?[0].0;

			if tier == common::TierMode::Disabled {
				let manifest = debug::dump_cold_manifest(&database_db).await?;
				assert_eq!(manifest.branch_id, branch_id);
				assert!(manifest.index.is_none());
				assert!(manifest.chunks.is_empty());
				return Ok(());
			}

			let cold_tier = ctx.cold_tier.expect("filesystem tier should be configured");
			let chunk_key = format!(
				"db/{}/cold_manifest/chunks/debug.bare",
				branch_id.as_uuid().simple()
			);
			let index_key = format!(
				"db/{}/cold_manifest/index.bare",
				branch_id.as_uuid().simple()
			);

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
						restore_points: Vec::new(),
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
		})
	})
	.await
}
