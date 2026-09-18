//! The continuation loop that turns per-fetch chunks into a range stream.
//!
//! These drive [`stream_range`] against a fake fetch so the loop's own behavior is observable:
//! how many fetches it issues, what range each one asks for, and when it stops. The drivers are
//! covered end to end in `tests/range_chunking.rs`.

use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use futures_util::TryStreamExt;

use super::{chunk_budget, stream_range};
use crate::{
	key_selector::KeySelector,
	options::StreamingMode,
	range_option::RangeOption,
	value::{KeyValue, Values},
};

/// One row per key in `0..rows`, so a fetch can be described by the key range it covers.
fn key(idx: u32) -> Vec<u8> {
	idx.to_be_bytes().to_vec()
}

fn idx(key: &[u8]) -> u32 {
	u32::from_be_bytes(key.try_into().expect("keys are 4 bytes"))
}

/// A fake database holding `rows` keys, which answers a fetch the way a driver does: honor the
/// selectors and the row limit, and flag whether the range still has more.
#[derive(Clone)]
struct FakeDb {
	rows: u32,
	/// The `(limit, first key, last key)` of every fetch issued, in order.
	fetches: Arc<Mutex<Vec<(Option<usize>, Option<u32>, Option<u32>)>>>,
}

impl FakeDb {
	fn new(rows: u32) -> Self {
		FakeDb {
			rows,
			fetches: Arc::new(Mutex::new(Vec::new())),
		}
	}

	fn fetch(&self, opt: &RangeOption<'_>) -> Values {
		// The drivers turn a selector with offset 1 and `or_equal` into a strict comparison.
		let lo_exclusive = opt.begin.offset() == 1 && opt.begin.or_equal();
		let lo = idx(opt.begin.key()) + u32::from(lo_exclusive);
		let hi = idx(opt.end.key()).min(self.rows);

		let limit = opt.limit.unwrap_or(usize::MAX);
		let taken: Vec<u32> = if opt.reverse {
			(lo..hi).rev().take(limit).collect()
		} else {
			(lo..hi).take(limit).collect()
		};

		let more = taken.len() < (hi.saturating_sub(lo)) as usize;
		let values: Vec<KeyValue> = taken
			.iter()
			.map(|idx| KeyValue::new(key(*idx), b"v".to_vec()))
			.collect();
		let last_db_key = values.last().map(|kv| kv.key().to_vec());

		self.fetches.lock().unwrap().push((
			opt.limit,
			taken.first().copied(),
			taken.last().copied(),
		));

		Values::chunk(values, more, last_db_key)
	}

	fn fetch_count(&self) -> usize {
		self.fetches.lock().unwrap().len()
	}
}

fn full_range(rows: u32, mode: StreamingMode, limit: Option<usize>) -> RangeOption<'static> {
	RangeOption {
		begin: KeySelector::first_greater_or_equal(key(0)),
		end: KeySelector::first_greater_or_equal(key(rows)),
		limit,
		mode,
		..Default::default()
	}
}

async fn drain(db: &FakeDb, opt: RangeOption<'static>) -> Result<Vec<u32>> {
	let db = db.clone();
	let stream = stream_range(opt, move |chunk_opt, _iteration| {
		let values = db.fetch(&chunk_opt);
		async move { Ok(values) }
	});

	let values: Vec<_> = stream.try_collect().await?;

	Ok(values.iter().map(|value| idx(value.key())).collect())
}

#[tokio::test]
async fn drains_the_whole_range_across_chunks() -> Result<()> {
	// `Small` caps a fetch at 256 rows, so 600 rows cannot come back in one.
	let db = FakeDb::new(600);
	let keys = drain(&db, full_range(600, StreamingMode::Small, None)).await?;

	assert_eq!(keys, (0..600).collect::<Vec<_>>());
	assert_eq!(db.fetch_count(), 3, "600 rows at 256 per fetch takes 3");

	Ok(())
}

#[tokio::test]
async fn chunks_do_not_repeat_or_skip_at_the_boundary() -> Result<()> {
	let db = FakeDb::new(600);
	drain(&db, full_range(600, StreamingMode::Small, None)).await?;

	let fetches = db.fetches.lock().unwrap().clone();
	assert_eq!(
		fetches
			.iter()
			.map(|(_, first, last)| (*first, *last))
			.collect::<Vec<_>>(),
		vec![
			(Some(0), Some(255)),
			(Some(256), Some(511)),
			(Some(512), Some(599)),
		],
		"each fetch must start immediately after the previous one ended"
	);

	Ok(())
}

#[tokio::test]
async fn a_single_fetch_ends_the_scan_when_the_range_fits() -> Result<()> {
	let db = FakeDb::new(10);
	let keys = drain(&db, full_range(10, StreamingMode::Small, None)).await?;

	assert_eq!(keys, (0..10).collect::<Vec<_>>());
	assert_eq!(
		db.fetch_count(),
		1,
		"a range that fits in one fetch must not cost a second round trip to discover that"
	);

	Ok(())
}

#[tokio::test]
async fn the_caller_limit_bounds_the_total_not_each_chunk() -> Result<()> {
	let db = FakeDb::new(600);
	let keys = drain(&db, full_range(600, StreamingMode::Small, Some(300))).await?;

	assert_eq!(keys, (0..300).collect::<Vec<_>>());
	assert_eq!(db.fetch_count(), 2);

	let fetches = db.fetches.lock().unwrap().clone();
	assert_eq!(
		fetches
			.iter()
			.map(|(limit, _, _)| *limit)
			.collect::<Vec<_>>(),
		vec![Some(256), Some(44)],
		"the second fetch must ask only for what is left of the caller's limit"
	);

	Ok(())
}

#[tokio::test]
async fn a_limit_smaller_than_the_budget_takes_one_fetch() -> Result<()> {
	let db = FakeDb::new(600);
	let keys = drain(&db, full_range(600, StreamingMode::Small, Some(5))).await?;

	assert_eq!(keys, (0..5).collect::<Vec<_>>());
	assert_eq!(db.fetch_count(), 1);

	Ok(())
}

#[tokio::test]
async fn a_partly_consumed_stream_only_fetches_what_it_needs() -> Result<()> {
	// This is the property the whole module exists for: reading the head of a large range must not
	// cost the whole range.
	let db = FakeDb::new(100_000);
	let fetch_db = db.clone();
	let mut stream = stream_range(
		full_range(100_000, StreamingMode::Small, None),
		move |chunk_opt, _iteration| {
			let values = fetch_db.fetch(&chunk_opt);
			async move { Ok(values) }
		},
	);

	let mut taken = Vec::new();
	while taken.len() < 5 {
		let value = stream
			.try_next()
			.await?
			.ok_or_else(|| anyhow!("stream ended early"))?;
		taken.push(idx(value.key()));
	}
	drop(stream);

	assert_eq!(taken, vec![0, 1, 2, 3, 4]);
	assert_eq!(
		db.fetch_count(),
		1,
		"reading 5 rows of 100k must issue exactly one fetch"
	);

	Ok(())
}

#[tokio::test]
async fn reverse_scans_walk_backwards_across_chunks() -> Result<()> {
	let db = FakeDb::new(600);
	let mut opt = full_range(600, StreamingMode::Small, None);
	opt.reverse = true;

	let keys = drain(&db, opt).await?;

	assert_eq!(keys, (0..600).rev().collect::<Vec<_>>());
	assert_eq!(db.fetch_count(), 3);

	Ok(())
}

#[tokio::test]
async fn a_fetch_error_ends_the_stream() -> Result<()> {
	let calls = Arc::new(Mutex::new(0usize));
	let fetch_calls = calls.clone();
	let stream = stream_range(
		full_range(600, StreamingMode::Small, None),
		move |_chunk_opt, _iteration| {
			*fetch_calls.lock().unwrap() += 1;
			async move { Err(anyhow!("boom")) }
		},
	);

	let result: Result<Vec<_>> = stream.try_collect().await;

	assert!(
		result.is_err(),
		"the fetch error must surface to the caller"
	);
	assert_eq!(
		*calls.lock().unwrap(),
		1,
		"a failed fetch must not be retried by the loop"
	);

	Ok(())
}

#[test]
fn the_iterator_budget_grows_and_then_holds() {
	let first = chunk_budget(StreamingMode::Iterator, 1, None);
	let second = chunk_budget(StreamingMode::Iterator, 2, None);
	let late = chunk_budget(StreamingMode::Iterator, 50, None);
	let later = chunk_budget(StreamingMode::Iterator, 5_000, None);

	assert!(first.rows < second.rows);
	assert!(first.bytes < second.bytes);
	assert_eq!(
		late, later,
		"the budget must plateau rather than grow without bound"
	);
}

#[test]
fn exact_delivers_the_whole_limit_in_one_fetch() {
	let budget = chunk_budget(StreamingMode::Exact, 1, Some(1_234));

	assert_eq!(budget.rows, 1_234);
	assert_eq!(budget.bytes, 0, "an exact row count has no byte ceiling");
}

#[test]
fn every_mode_bounds_a_fetch_without_a_limit() {
	for mode in [
		StreamingMode::WantAll,
		StreamingMode::Iterator,
		StreamingMode::Exact,
		StreamingMode::Small,
		StreamingMode::Medium,
		StreamingMode::Large,
		StreamingMode::Serial,
	] {
		let budget = chunk_budget(mode, 1, None);

		assert!(
			budget.rows < usize::MAX || budget.bytes > 0,
			"{mode:?} left a fetch with no ceiling at all"
		);
	}
}
