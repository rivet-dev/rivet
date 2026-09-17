//! Range reads come back one bounded page at a time.
//!
//! A driver that reads a whole range for one `get_range` call holds every key and value of it in
//! memory at once, whatever the consumer of the stream does with it. These tests pin the page
//! contract instead: a page is bounded by its streaming mode's byte budget, `Values::more` plus
//! `RangeOption::next_range` walk the range page by page, and the read-your-writes merge puts a
//! pending write in the page that covers its key rather than in the first page.

use std::{
	collections::BTreeMap,
	sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	},
};

use anyhow::{Context, Result, ensure};
use futures_util::TryStreamExt;
use rivet_test_deps_docker::TestDatabase;
use universaldb::{
	Database,
	options::{MutationType, StreamingMode},
	range_option::RangeOption,
	utils::{
		IsolationLevel::{Serializable, Snapshot},
		end_of_key_range,
	},
};
use uuid::Uuid;

/// Rows seeded per scenario. At `VALUE_BYTES` each they span several pages in every streaming mode.
const ROWS: u32 = 600;
const VALUE_BYTES: usize = 1024;
/// Key plus value bytes of one seeded row.
const ROW_BYTES: usize = KEY_PREFIX_BYTES + 4 + VALUE_BYTES;
const KEY_PREFIX_BYTES: usize = 16;
/// Byte budget of a `StreamingMode::WantAll` page.
const WANT_ALL_PAGE_BYTES: usize = 120_000;
/// Byte budget of the first `StreamingMode::Iterator` page.
const FIRST_ITERATOR_PAGE_BYTES: usize = 4_096;

/// A config carrying the compiled universaldb commit protocol version.
///
/// `RuntimeProtocols::default()` reports version 0 so that a process which never negotiated cannot
/// silently reach the wire, which means a test has to supply the real version itself.
fn test_config() -> rivet_config::Config {
	rivet_config::Config::from_root_with_build_meta(
		rivet_config::config::Root::default(),
		rivet_config::BuildMeta::default(),
		rivet_config::RuntimeProtocols {
			universaldb_commit: rivet_config::RuntimeProtocol::new(
				rivet_config::RuntimeProtocolKind::UniversaldbCommit,
				rivet_universaldb_commit::PROTOCOL_VERSION,
			),
			..Default::default()
		},
	)
}

async fn rocksdb_database() -> Result<Database> {
	let (db_config, _docker_config) = TestDatabase::FileSystem.config(Uuid::new_v4(), 1).await?;
	let rivet_config::config::Database::FileSystem(fs_config) = db_config else {
		unreachable!()
	};
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(fs_config.path).await?;

	Ok(Database::new(Arc::new(driver)))
}

async fn postgres_database() -> Result<Database> {
	let (db_config, docker_config) = TestDatabase::Postgres.config(Uuid::new_v4(), 1).await?;
	let mut docker_config = docker_config.context("postgres test database has no docker config")?;
	docker_config.start().await?;
	TestDatabase::Postgres
		.wait_for_ready(&docker_config)
		.await?;

	let rivet_config::config::Database::Postgres(postgres_config) = db_config else {
		unreachable!()
	};
	let connection_string = postgres_config.url.read().clone();
	let driver = universaldb::driver::PostgresDatabaseDriver::new_with_config(
		test_config(),
		universaldb::driver::postgres::PostgresConfig::new(connection_string),
	)
	.await?;

	Ok(Database::new(Arc::new(driver)))
}

/// A key prefix no other scenario shares, so scenarios can run against one database.
fn unique_prefix() -> Vec<u8> {
	Uuid::new_v4().as_bytes().to_vec()
}

fn key(prefix: &[u8], idx: u32) -> Vec<u8> {
	let mut key = prefix.to_vec();
	key.extend_from_slice(&idx.to_be_bytes());
	key
}

fn idx_of(prefix: &[u8], key: &[u8]) -> u32 {
	u32::from_be_bytes(key[prefix.len()..].try_into().unwrap())
}

fn seeded_value(idx: u32) -> Vec<u8> {
	vec![(idx % 251) as u8; VALUE_BYTES]
}

fn whole_range(prefix: &[u8]) -> (Vec<u8>, Vec<u8>) {
	(key(prefix, 0), end_of_key_range(&key(prefix, u32::MAX)))
}

/// Seeds `indexes` in batches small enough to stay far below the transaction size limit.
async fn seed(db: &Database, prefix: &[u8], indexes: Vec<u32>) -> Result<()> {
	for batch in indexes.chunks(200) {
		let prefix = prefix.to_vec();
		let batch = batch.to_vec();
		db.txn("test_range_paging_seed", move |tx| {
			let prefix = prefix.clone();
			let batch = batch.clone();
			async move {
				for idx in batch {
					tx.informal().set(&key(&prefix, idx), &seeded_value(idx));
				}
				Ok(())
			}
		})
		.await?;
	}

	Ok(())
}

/// Reads a range the way a FoundationDB client does: one `get_range` per page, continued with
/// `next_range`. Returns every page.
async fn read_pages(
	db: &Database,
	prefix: &[u8],
	mode: StreamingMode,
	limit: Option<usize>,
	reverse: bool,
) -> Result<Vec<Vec<(u32, usize)>>> {
	let prefix = prefix.to_vec();
	db.txn("test_range_paging_pages", move |tx| {
		let prefix = prefix.clone();
		async move {
			let (begin, end) = whole_range(&prefix);
			let mut opt = Some(RangeOption {
				mode,
				limit,
				reverse,
				..(begin, end).into()
			});

			let mut pages = Vec::new();
			let mut iteration = 1;
			while let Some(current) = opt {
				let page = tx
					.informal()
					.get_range(&current, iteration, Snapshot)
					.await?;
				pages.push(
					page.iter()
						.map(|kv| (idx_of(&prefix, kv.key()), kv.key().len() + kv.value().len()))
						.collect::<Vec<_>>(),
				);

				opt = current.clone().next_range(&page);
				iteration += 1;

				ensure!(iteration < 10_000, "range read did not terminate");
			}

			Ok(pages)
		}
	})
	.await
}

/// Reads a range through the stream every engine caller uses.
async fn read_stream(
	db: &Database,
	prefix: &[u8],
	mode: StreamingMode,
	limit: Option<usize>,
	reverse: bool,
) -> Result<Vec<u32>> {
	let prefix = prefix.to_vec();
	db.txn("test_range_paging_stream", move |tx| {
		let prefix = prefix.clone();
		async move {
			let (begin, end) = whole_range(&prefix);
			let informal = tx.informal();
			let mut stream = informal.get_ranges_keyvalues(
				RangeOption {
					mode,
					limit,
					reverse,
					..(begin, end).into()
				},
				Snapshot,
			);

			let mut out = Vec::new();
			while let Some(entry) = stream.try_next().await? {
				out.push(idx_of(&prefix, entry.key()));
			}

			Ok(out)
		}
	})
	.await
}

fn expected(reverse: bool, limit: Option<usize>) -> Vec<u32> {
	let mut all = (0..ROWS).collect::<Vec<_>>();
	if reverse {
		all.reverse();
	}
	all.truncate(limit.unwrap_or(usize::MAX));
	all
}

async fn pages_are_bounded_and_cover_the_range(db: &Database) -> Result<()> {
	let prefix = unique_prefix();
	seed(db, &prefix, (0..ROWS).collect()).await?;

	for reverse in [false, true] {
		let pages = read_pages(db, &prefix, StreamingMode::WantAll, None, reverse).await?;

		// The range is about five times the page budget, so one page can no longer hold it.
		let full_pages = pages.iter().filter(|page| !page.is_empty()).count();
		ensure!(
			full_pages >= 5,
			"expected at least 5 pages, got {full_pages}"
		);

		for page in &pages {
			let bytes = page.iter().map(|(_, bytes)| bytes).sum::<usize>();
			ensure!(
				bytes < WANT_ALL_PAGE_BYTES + ROW_BYTES,
				"page of {bytes} bytes overran its budget by more than one row"
			);
		}

		let keys = pages
			.iter()
			.flatten()
			.map(|(idx, _)| *idx)
			.collect::<Vec<_>>();
		ensure!(
			keys == expected(reverse, None),
			"pages skipped or repeated rows"
		);

		// An iterator read starts with a small page so a caller that stops early reads little.
		let pages = read_pages(db, &prefix, StreamingMode::Iterator, None, reverse).await?;
		let first_page_bytes = pages[0].iter().map(|(_, bytes)| bytes).sum::<usize>();
		ensure!(
			first_page_bytes < FIRST_ITERATOR_PAGE_BYTES + ROW_BYTES,
			"first iterator page held {first_page_bytes} bytes"
		);
		let keys = pages
			.iter()
			.flatten()
			.map(|(idx, _)| *idx)
			.collect::<Vec<_>>();
		ensure!(
			keys == expected(reverse, None),
			"pages skipped or repeated rows"
		);
	}

	Ok(())
}

async fn exact_mode_returns_its_limit_in_one_page(db: &Database) -> Result<()> {
	let prefix = unique_prefix();
	seed(db, &prefix, (0..ROWS).collect()).await?;

	// 300 rows are well over the largest byte budget, and exact mode is bounded by its limit alone.
	let pages = read_pages(db, &prefix, StreamingMode::Exact, Some(300), false).await?;
	ensure!(
		pages[0].len() == 300,
		"exact page held {} rows",
		pages[0].len()
	);
	ensure!(
		pages.iter().flatten().count() == 300,
		"exact read returned more rows than its limit"
	);

	Ok(())
}

async fn stream_crosses_pages(db: &Database) -> Result<()> {
	let prefix = unique_prefix();
	seed(db, &prefix, (0..ROWS).collect()).await?;

	for mode in [StreamingMode::WantAll, StreamingMode::Iterator] {
		for reverse in [false, true] {
			// A limit that lands in the middle of a later page has to be carried across pages.
			for limit in [None, Some(1), Some(450)] {
				let keys = read_stream(db, &prefix, mode, limit, reverse).await?;
				ensure!(
					keys == expected(reverse, limit),
					"stream returned the wrong rows for mode {mode:?}, reverse {reverse}, limit {limit:?}"
				);
			}
		}
	}

	Ok(())
}

async fn stream_can_stop_early(db: &Database) -> Result<()> {
	let prefix = unique_prefix();
	seed(db, &prefix, (0..ROWS).collect()).await?;

	let prefix_clone = prefix.clone();
	let (head, point_read) = db
		.txn("test_range_paging_early_stop", move |tx| {
			let prefix = prefix_clone.clone();
			async move {
				let (begin, end) = whole_range(&prefix);
				let informal = tx.informal();

				let mut head = Vec::new();
				{
					let mut stream = informal.get_ranges_keyvalues(
						RangeOption {
							mode: StreamingMode::WantAll,
							..(begin, end).into()
						},
						Snapshot,
					);
					while let Some(entry) = stream.try_next().await? {
						head.push(idx_of(&prefix, entry.key()));
						if head.len() == 3 {
							break;
						}
					}
				}

				// The transaction is still usable after a stream is dropped part way through.
				let point_read = informal.get(&key(&prefix, ROWS - 1), Snapshot).await?;

				Ok((head, point_read.map(Vec::from)))
			}
		})
		.await?;

	ensure!(head == vec![0, 1, 2], "early stop returned {head:?}");
	ensure!(
		point_read == Some(seeded_value(ROWS - 1)),
		"read after an early stop failed"
	);

	Ok(())
}

/// Pending writes of one transaction, applied both to the transaction and to a model of what the
/// range has to read back as.
#[derive(Clone)]
enum PendingOp {
	Set(u32, Vec<u8>),
	Clear(u32),
	ClearRange(u32, u32),
	Add(u32, u64),
}

fn pending_ops() -> Vec<PendingOp> {
	let mut ops = vec![
		// New keys between database rows, one in the first page and the others in later pages.
		PendingOp::Set(1, b"pending_1".to_vec()),
		PendingOp::Set(301, b"pending_301".to_vec()),
		PendingOp::Set(601, b"pending_601".to_vec()),
		// New keys past the last database row.
		PendingOp::Set(1199, b"pending_1199".to_vec()),
		PendingOp::Set(5000, b"pending_5000".to_vec()),
		PendingOp::Clear(100),
		PendingOp::Clear(598),
		// Wider than a page, so at least one database page comes back with every row cleared.
		PendingOp::ClearRange(200, 500),
		// Written after the clear, so it survives inside the cleared span.
		PendingOp::Set(250, b"pending_250".to_vec()),
		PendingOp::Add(1001, 7),
	];

	// Overwrites of a run of database rows. One of them is the last row of some page, and a paged
	// read must not return it again at the start of the next page.
	for idx in (800..=1000).step_by(2) {
		ops.push(PendingOp::Set(
			idx,
			format!("overwritten_{idx}").into_bytes(),
		));
	}

	ops
}

fn model_after(ops: &[PendingOp], seeded: &[u32]) -> BTreeMap<u32, Vec<u8>> {
	let mut model = seeded
		.iter()
		.map(|idx| (*idx, seeded_value(*idx)))
		.collect::<BTreeMap<_, _>>();

	for op in ops {
		match op {
			PendingOp::Set(idx, value) => {
				model.insert(*idx, value.clone());
			}
			PendingOp::Clear(idx) => {
				model.remove(idx);
			}
			PendingOp::ClearRange(begin, end) => {
				model.retain(|idx, _| idx < begin || idx >= end);
			}
			PendingOp::Add(idx, addend) => {
				model.insert(*idx, addend.to_le_bytes().to_vec());
			}
		}
	}

	model
}

async fn pending_writes_land_in_the_page_that_covers_them(db: &Database) -> Result<()> {
	// Database rows sit on even indexes so pending writes can land between them.
	let seeded = (0..ROWS).map(|idx| idx * 2).collect::<Vec<_>>();
	let ops = pending_ops();
	let model = model_after(&ops, &seeded);

	let prefix = unique_prefix();
	seed(db, &prefix, seeded).await?;

	for mode in [StreamingMode::WantAll, StreamingMode::Iterator] {
		for reverse in [false, true] {
			for limit in [None, Some(500)] {
				let prefix_clone = prefix.clone();
				let ops = ops.clone();
				let rows = db
					.txn("test_range_paging_pending", move |tx| {
						let prefix = prefix_clone.clone();
						let ops = ops.clone();
						async move {
							let informal = tx.informal();
							for op in &ops {
								match op {
									PendingOp::Set(idx, value) => {
										informal.set(&key(&prefix, *idx), value)
									}
									PendingOp::Clear(idx) => informal.clear(&key(&prefix, *idx)),
									PendingOp::ClearRange(begin, end) => informal
										.clear_range(&key(&prefix, *begin), &key(&prefix, *end)),
									PendingOp::Add(idx, addend) => informal.atomic_op(
										&key(&prefix, *idx),
										&addend.to_le_bytes(),
										MutationType::Add,
									),
								}
							}

							let (begin, end) = whole_range(&prefix);
							let mut stream = informal.get_ranges_keyvalues(
								RangeOption {
									mode,
									limit,
									reverse,
									..(begin, end).into()
								},
								Serializable,
							);

							let mut rows = Vec::new();
							while let Some(entry) = stream.try_next().await? {
								rows.push((idx_of(&prefix, entry.key()), entry.value().to_vec()));
							}
							drop(stream);

							// Leave the database as it was seeded for the next combination.
							informal.cancel();

							Ok(rows)
						}
					})
					.await?;

				let mut want = model.clone().into_iter().collect::<Vec<_>>();
				if reverse {
					want.reverse();
				}
				want.truncate(limit.unwrap_or(usize::MAX));

				let got_keys = rows.iter().map(|(idx, _)| *idx).collect::<Vec<_>>();
				let want_keys = want.iter().map(|(idx, _)| *idx).collect::<Vec<_>>();
				ensure!(
					got_keys == want_keys,
					"wrong rows for mode {mode:?}, reverse {reverse}, limit {limit:?}"
				);
				ensure!(
					rows == want,
					"wrong values for mode {mode:?}, reverse {reverse}, limit {limit:?}"
				);
			}
		}
	}

	Ok(())
}

async fn serializable_read_adds_one_conflict_range(db: &Database) -> Result<()> {
	let prefix = unique_prefix();
	seed(db, &prefix, (0..ROWS).collect()).await?;

	// Both reads cover the same range, so they must leave the same conflict ranges behind however
	// many pages it took to read them.
	let mut sizes = Vec::new();
	for limit in [Some(1), None] {
		let prefix = prefix.clone();
		let size = db
			.txn("test_range_paging_conflict_size", move |tx| {
				let prefix = prefix.clone();
				async move {
					let (begin, end) = whole_range(&prefix);
					let informal = tx.informal();
					let mut stream = informal.get_ranges_keyvalues(
						RangeOption {
							mode: StreamingMode::WantAll,
							limit,
							..(begin, end).into()
						},
						Serializable,
					);
					while stream.try_next().await?.is_some() {}
					drop(stream);

					tx.approximate_size().await
				}
			})
			.await?;
		sizes.push(size);
	}

	ensure!(
		sizes[0] == sizes[1],
		"a paged read left {} bytes of conflict ranges, a one page read left {}",
		sizes[1],
		sizes[0]
	);

	Ok(())
}

async fn write_to_a_later_page_conflicts(db: &Database) -> Result<()> {
	let prefix = unique_prefix();
	seed(db, &prefix, (0..ROWS).collect()).await?;

	let attempts = Arc::new(AtomicUsize::new(0));

	let attempts_clone = attempts.clone();
	let db_clone = db.clone();
	let prefix_clone = prefix.clone();
	db.txn("test_range_paging_reader", move |tx| {
		let attempts = attempts_clone.clone();
		let db = db_clone.clone();
		let prefix = prefix_clone.clone();
		async move {
			let attempt = attempts.fetch_add(1, Ordering::SeqCst);

			let (begin, end) = whole_range(&prefix);
			let informal = tx.informal();
			let mut stream = informal.get_ranges_keyvalues(
				RangeOption {
					mode: StreamingMode::WantAll,
					..(begin, end).into()
				},
				Serializable,
			);
			while stream.try_next().await?.is_some() {}
			drop(stream);

			// On the first attempt another transaction commits a write to the last row, which only
			// the last page of the read above covered.
			if attempt == 0 {
				let prefix = prefix.clone();
				db.txn("test_range_paging_writer", move |tx| {
					let prefix = prefix.clone();
					async move {
						tx.informal().set(&key(&prefix, ROWS - 1), b"concurrent");
						Ok(())
					}
				})
				.await?;
			}

			// A transaction with no writes never conflicts, so give this one a write.
			informal.set(&key(&prefix, ROWS + 1), b"reader");

			Ok(())
		}
	})
	.await?;

	let attempts = attempts.load(Ordering::SeqCst);
	ensure!(
		attempts == 2,
		"a write under the last page should retry the reader once, saw {attempts} attempts"
	);

	Ok(())
}

#[tokio::test]
async fn rocksdb_pages_are_bounded_and_cover_the_range() -> Result<()> {
	pages_are_bounded_and_cover_the_range(&rocksdb_database().await?).await
}

#[tokio::test]
async fn rocksdb_exact_mode_returns_its_limit_in_one_page() -> Result<()> {
	exact_mode_returns_its_limit_in_one_page(&rocksdb_database().await?).await
}

#[tokio::test]
async fn rocksdb_stream_crosses_pages() -> Result<()> {
	stream_crosses_pages(&rocksdb_database().await?).await
}

#[tokio::test]
async fn rocksdb_stream_can_stop_early() -> Result<()> {
	stream_can_stop_early(&rocksdb_database().await?).await
}

#[tokio::test]
async fn rocksdb_pending_writes_land_in_the_page_that_covers_them() -> Result<()> {
	pending_writes_land_in_the_page_that_covers_them(&rocksdb_database().await?).await
}

#[tokio::test]
async fn rocksdb_serializable_read_adds_one_conflict_range() -> Result<()> {
	serializable_read_adds_one_conflict_range(&rocksdb_database().await?).await
}

#[tokio::test]
async fn rocksdb_write_to_a_later_page_conflicts() -> Result<()> {
	write_to_a_later_page_conflicts(&rocksdb_database().await?).await
}

/// Every scenario against one Postgres container, which is slow to start.
#[tokio::test]
async fn postgres_range_paging() -> Result<()> {
	let db = postgres_database().await?;

	pages_are_bounded_and_cover_the_range(&db).await?;
	exact_mode_returns_its_limit_in_one_page(&db).await?;
	stream_crosses_pages(&db).await?;
	stream_can_stop_early(&db).await?;
	pending_writes_land_in_the_page_that_covers_them(&db).await?;
	serializable_read_adds_one_conflict_range(&db).await?;
	write_to_a_later_page_conflicts(&db).await?;

	Ok(())
}
