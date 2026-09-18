//! Range scans that span more than one fetch.
//!
//! A driver answers a range read one chunk at a time, so a scan over a large range is stitched back
//! together by the continuation loop in `universaldb::chunk`. These exercise that stitching against a
//! real RocksDB-backed database, with the transaction holding pending writes so every read also goes
//! through the read-your-writes merge. The merge is where chunking is easiest to get wrong: it can
//! pull a pending write in from past the end of a chunk, or clear a chunk down to nothing, and either
//! one silently changes where the next chunk starts.

use std::sync::Arc;

use anyhow::Result;
use futures_util::TryStreamExt;
use universaldb::{
	Database,
	options::StreamingMode,
	range_option::RangeOption,
	utils::{IsolationLevel::Serializable, end_of_key_range},
};
use uuid::Uuid;

/// Enough rows at this value size to overrun `StreamingMode::Small`'s byte budget several times, so
/// every scan below is genuinely chunked rather than a single fetch that happens to fit.
const VALUE_BYTES: usize = 4 * 1024;
const ROWS: u32 = 24;

async fn rocksdb_database() -> Result<Database> {
	let test_id = Uuid::new_v4();
	let (db_config, _docker_config) = rivet_test_deps_docker::TestDatabase::FileSystem
		.config(test_id, 1)
		.await?;
	let rivet_config::config::Database::FileSystem(fs_config) = db_config else {
		unreachable!()
	};
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(fs_config.path).await?;

	Ok(Database::new(Arc::new(driver)))
}

fn key(prefix: &[u8], idx: u32) -> Vec<u8> {
	let mut key = prefix.to_vec();
	key.extend_from_slice(&idx.to_be_bytes());
	key
}

fn idx(prefix: &[u8], key: &[u8]) -> u32 {
	u32::from_be_bytes(key[prefix.len()..].try_into().expect("keys end in a u32"))
}

async fn seed(db: &Database, prefix: &[u8], rows: u32, value_bytes: usize) -> Result<()> {
	let prefix = prefix.to_vec();
	db.txn("test_range_chunking_seed", move |tx| {
		let prefix = prefix.clone();
		async move {
			for i in 0..rows {
				tx.informal()
					.set(&key(&prefix, i), &vec![b'x'; value_bytes]);
			}
			Ok(())
		}
	})
	.await
}

/// What a pending write in the scanning transaction does before the scan runs. Any of these routes
/// the read through the read-your-writes merge.
#[derive(Clone, Copy)]
enum Pending {
	None,
	/// Write a key past every row the database holds but still inside the scanned range.
	SetPastEnd(u32),
	/// Clear a band of keys that falls in the middle of the scan.
	ClearBand(u32, u32),
}

async fn scan(
	db: &Database,
	prefix: &[u8],
	pending: Pending,
	reverse: bool,
	limit: Option<usize>,
) -> Result<Vec<u32>> {
	let prefix = prefix.to_vec();
	db.txn("test_range_chunking_scan", move |tx| {
		let prefix = prefix.clone();
		async move {
			match pending {
				Pending::None => {}
				Pending::SetPastEnd(at) => tx.informal().set(&key(&prefix, at), b"pending"),
				Pending::ClearBand(from, to) => tx
					.informal()
					.clear_range(&key(&prefix, from), &key(&prefix, to)),
			}

			let end = end_of_key_range(&key(&prefix, u32::MAX));
			let informal = tx.informal();
			let mut stream = informal.get_ranges_keyvalues(
				RangeOption {
					// The smallest budget, so these modest row counts still span several fetches.
					mode: StreamingMode::Small,
					limit,
					reverse,
					..(prefix.as_slice(), end.as_slice()).into()
				},
				Serializable,
			);

			let mut out = Vec::new();
			while let Some(entry) = stream.try_next().await? {
				out.push(idx(&prefix, entry.key()));
			}

			Ok(out)
		}
	})
	.await
}

#[tokio::test]
async fn a_chunked_scan_returns_every_row_in_order() -> Result<()> {
	let db = rocksdb_database().await?;
	let prefix = b"chunk-all/".to_vec();
	seed(&db, &prefix, ROWS, VALUE_BYTES).await?;

	let keys = scan(&db, &prefix, Pending::None, false, None).await?;

	assert_eq!(
		keys,
		(0..ROWS).collect::<Vec<_>>(),
		"a scan spanning several fetches must return the range exactly once, in order"
	);

	Ok(())
}

#[tokio::test]
async fn a_value_larger_than_the_byte_budget_still_makes_progress() -> Result<()> {
	let db = rocksdb_database().await?;
	let prefix = b"chunk-oversized/".to_vec();
	// Each value on its own blows the whole byte budget for a fetch.
	seed(&db, &prefix, 4, 64 * 1024).await?;

	let keys = scan(&db, &prefix, Pending::None, false, None).await?;

	assert_eq!(
		keys,
		vec![0, 1, 2, 3],
		"a fetch must always take one row, or an oversized value stalls the scan on itself"
	);

	Ok(())
}

#[tokio::test]
async fn a_pending_write_past_the_chunk_end_does_not_skip_rows() -> Result<()> {
	let db = rocksdb_database().await?;
	let prefix = b"chunk-pending-set/".to_vec();
	seed(&db, &prefix, ROWS, VALUE_BYTES).await?;

	let keys = scan(&db, &prefix, Pending::SetPastEnd(10_000), false, None).await?;

	let mut expected = (0..ROWS).collect::<Vec<_>>();
	expected.push(10_000);
	assert_eq!(
		keys, expected,
		"the pending key sits past the first chunk, so returning it early would skip every row \
		 between it and the end of that chunk"
	);

	Ok(())
}

#[tokio::test]
async fn a_pending_clear_over_a_whole_chunk_does_not_end_the_scan() -> Result<()> {
	let db = rocksdb_database().await?;
	let prefix = b"chunk-pending-clear/".to_vec();
	seed(&db, &prefix, ROWS, VALUE_BYTES).await?;

	// Wide enough to empty at least one whole fetch, which leaves that chunk with no rows to
	// continue from.
	let keys = scan(&db, &prefix, Pending::ClearBand(4, 20), false, None).await?;

	let expected = (0..4).chain(20..ROWS).collect::<Vec<_>>();
	assert_eq!(
		keys, expected,
		"a chunk emptied by a pending clear must still advance the scan past it"
	);

	Ok(())
}

#[tokio::test]
async fn a_reverse_chunked_scan_returns_every_row_in_order() -> Result<()> {
	let db = rocksdb_database().await?;
	let prefix = b"chunk-reverse/".to_vec();
	seed(&db, &prefix, ROWS, VALUE_BYTES).await?;

	let clean = scan(&db, &prefix, Pending::None, true, None).await?;
	assert_eq!(clean, (0..ROWS).rev().collect::<Vec<_>>());

	let pending = scan(&db, &prefix, Pending::SetPastEnd(10_000), true, None).await?;
	let mut expected = vec![10_000];
	expected.extend((0..ROWS).rev());
	assert_eq!(
		pending, expected,
		"a reverse scan must walk back through every chunk, pending write first"
	);

	Ok(())
}

#[tokio::test]
async fn a_limit_is_honored_across_chunks() -> Result<()> {
	let db = rocksdb_database().await?;
	let prefix = b"chunk-limit/".to_vec();
	seed(&db, &prefix, ROWS, VALUE_BYTES).await?;

	let clean = scan(&db, &prefix, Pending::None, false, Some(20)).await?;
	assert_eq!(clean, (0..20).collect::<Vec<_>>());

	let pending = scan(&db, &prefix, Pending::ClearBand(4, 8), false, Some(12)).await?;
	let expected = (0..4).chain(8..16).collect::<Vec<_>>();
	assert_eq!(
		pending, expected,
		"a limit counts the rows the caller receives, not the rows the database returned"
	);

	Ok(())
}

#[tokio::test]
async fn a_pending_write_that_overruns_the_chunk_limit_does_not_drop_a_row() -> Result<()> {
	let db = rocksdb_database().await?;
	let prefix = b"chunk-row-limit/".to_vec();

	// Only even keys, so a pending write can land between two rows the database holds, and enough of
	// them that a fetch stops on `StreamingMode::Small`'s row ceiling rather than its byte ceiling.
	let seed_prefix = prefix.clone();
	db.txn("test_range_chunking_seed", move |tx| {
		let prefix = seed_prefix.clone();
		async move {
			for i in 0..300u32 {
				tx.informal().set(&key(&prefix, i * 2), b"v");
			}
			Ok(())
		}
	})
	.await?;

	// The pending key pushes the first fetch one row over its ceiling, so the row the limit drops has
	// to come back in the next chunk rather than being stepped over.
	let keys = scan(&db, &prefix, Pending::SetPastEnd(1), false, None).await?;

	let mut expected = vec![0, 1];
	expected.extend((1..300u32).map(|i| i * 2));
	assert_eq!(
		keys, expected,
		"a row displaced by the chunk's row limit must be returned by the next chunk"
	);

	Ok(())
}
