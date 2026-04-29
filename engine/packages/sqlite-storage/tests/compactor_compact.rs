use std::sync::Arc;

use anyhow::Result;
use sqlite_storage::{
	compactor::fold_shard,
	keys::{PAGE_SIZE, shard_key},
	ltx::{LtxHeader, decode_ltx_v3, encode_ltx_v3},
	types::DirtyPage,
};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;

const TEST_ACTOR: &str = "test-actor";

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
