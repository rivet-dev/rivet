use std::sync::Arc;

use anyhow::Result;
use sqlite_storage::{
	keys::{delta_chunk_key, meta_head_key, pidx_delta_key, shard_key, PAGE_SIZE},
	ltx::{LtxHeader, encode_ltx_v3},
	pump::ActorDb,
	types::{DBHead, DirtyPage, FetchedPage, encode_db_head},
};
use tempfile::Builder;

const TEST_ACTOR: &str = "test-actor";

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-read-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn head(db_size_pages: u32) -> DBHead {
	DBHead {
		head_txid: 4,
		db_size_pages,
		#[cfg(debug_assertions)]
		generation: 1,
	}
}

fn page(fill: u8) -> Vec<u8> {
	vec![fill; PAGE_SIZE as usize]
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
			(meta_head_key(TEST_ACTOR), encode_db_head(head(3))?),
			(delta_chunk_key(TEST_ACTOR, 4, 0), encoded_blob(4, &[(2, 0x22)])?),
			(pidx_delta_key(TEST_ACTOR, 2), 4_u64.to_be_bytes().to_vec()),
		],
		Vec::new(),
	)
	.await?;

	let actor_db = ActorDb::new(db, TEST_ACTOR.to_string());

	assert_eq!(
		actor_db.get_pages(vec![2]).await?,
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
			(meta_head_key(TEST_ACTOR), encode_db_head(head(3))?),
			(delta_chunk_key(TEST_ACTOR, 4, 0), encoded_blob(4, &[(2, 0x22)])?),
			(pidx_delta_key(TEST_ACTOR, 2), 4_u64.to_be_bytes().to_vec()),
		],
		Vec::new(),
	)
	.await?;

	let actor_db = ActorDb::new(db.clone(), TEST_ACTOR.to_string());
	assert_eq!(
		actor_db.get_pages(vec![2]).await?,
		vec![FetchedPage {
			pgno: 2,
			bytes: Some(page(0x22)),
		}]
	);

	seed(&db, Vec::new(), vec![pidx_delta_key(TEST_ACTOR, 2)]).await?;

	assert_eq!(
		actor_db.get_pages(vec![2]).await?,
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
			(meta_head_key(TEST_ACTOR), encode_db_head(head(3))?),
			(delta_chunk_key(TEST_ACTOR, 4, 0), encoded_blob(4, &[(2, 0x22)])?),
			(pidx_delta_key(TEST_ACTOR, 2), 4_u64.to_be_bytes().to_vec()),
		],
		Vec::new(),
	)
	.await?;

	let actor_db = ActorDb::new(db.clone(), TEST_ACTOR.to_string());
	assert_eq!(
		actor_db.get_pages(vec![2]).await?,
		vec![FetchedPage {
			pgno: 2,
			bytes: Some(page(0x22)),
		}]
	);

	seed(
		&db,
		vec![(shard_key(TEST_ACTOR, 0), encoded_blob(4, &[(2, 0x44)])?)],
		vec![
			delta_chunk_key(TEST_ACTOR, 4, 0),
			pidx_delta_key(TEST_ACTOR, 2),
		],
	)
	.await?;

	assert_eq!(
		actor_db.get_pages(vec![2]).await?,
		vec![FetchedPage {
			pgno: 2,
			bytes: Some(page(0x44)),
		}]
	);

	Ok(())
}

#[tokio::test]
async fn get_pages_returns_none_above_eof() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed(
		&db,
		vec![(meta_head_key(TEST_ACTOR), encode_db_head(head(3))?)],
		Vec::new(),
	)
	.await?;

	let actor_db = ActorDb::new(db, TEST_ACTOR.to_string());

	assert_eq!(
		actor_db.get_pages(vec![4]).await?,
		vec![FetchedPage {
			pgno: 4,
			bytes: None,
		}]
	);

	Ok(())
}
