//! End-to-end scan tests that compare a `Db` against a `BTreeMap` model.
//!
//! The fixture deliberately drives compaction so that a key's versions end up
//! split across the SSTs of a sorted run, which is the layout a descending scan
//! has to reassemble. Both the blocks and the SSTs are tiny to have
//! compaction split keys across multiple SSTs.

#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

use std::collections::BTreeMap;
use std::ops::Bound::{self, Included, Unbounded};
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};
use slatedb::config::{
    CompactionWorkerOptions, CompactorOptions, ScanOptions, Settings,
    SizeTieredCompactionSchedulerOptions,
};
use slatedb::object_store::{memory::InMemory, ObjectStore};
use slatedb::size_tiered_compaction::SizeTieredCompactionSchedulerSupplier;
use slatedb::{CompactorBuilder, Db, IterationOrder, SstBlockSize};
use tokio::runtime::Runtime;

/// The alphabet writes draw keys from. Deliberately tiny, so a short op
/// sequence still puts many versions on each key. That is what lets compaction
/// split one key's versions across SSTs.
const KEY_ALPHABET: u8 = 8;

/// A write applied to both the db and the model.
#[derive(Debug, Clone)]
enum ScanOp {
    Put(Bytes, Bytes),
    Delete(Bytes),
    Flush,
    /// Flush to L0 and compact it into a sorted run.
    Compact,
}

fn scan_ops(max_ops: usize) -> impl Strategy<Value = Vec<ScanOp>> {
    let key = (0..KEY_ALPHABET).prop_map(|byte| Bytes::from(vec![byte]));
    // Padded so a handful of versions fills a 1KiB block, which is what makes
    // the compactor roll over to a new SST partway through a key.
    let value = proptest::collection::vec(any::<u8>(), 64..=128).prop_map(Bytes::from);
    // Relative weights. Puts dominate so each key piles up versions, which is
    // what gives compaction something to split across SSTs. Flush moves the
    // memtable into L0 and Compact folds L0 into a sorted run, so a scan has to
    // merge all three.
    let op = prop_oneof![
        6 => (key.clone(), value).prop_map(|(k, v)| ScanOp::Put(k, v)),
        2 => key.prop_map(ScanOp::Delete),
        1 => Just(ScanOp::Flush),
        1 => Just(ScanOp::Compact),
    ];
    proptest::collection::vec(op, 1..=max_ops)
}

type KeyRange = (Bound<Bytes>, Bound<Bytes>);

/// Ranges to scan. Mixes arbitrary bounded ranges with single-key ranges drawn
/// from the same alphabet the writes use, so the point-range path is exercised
/// against keys that actually exist rather than always landing in a gap.
fn scan_range() -> impl Strategy<Value = KeyRange> {
    // Bounds are drawn from the key alphabet (plus one past its end) rather
    // than the whole byte space. Random 1-2 byte bounds over an 8 key space
    // land above every key almost every time, which makes for ranges that
    // scan nothing.
    let bound = prop_oneof![
        3 => (0..=KEY_ALPHABET).prop_map(|byte| Included(Bytes::from(vec![byte]))),
        1 => Just(Unbounded),
    ];
    // Mostly bounded ranges, with a quarter of them single-key so the point
    // range path is covered.
    prop_oneof![
        3 => (bound.clone(), bound).prop_filter_map("non-empty range", |(start, end)| {
            match (&start, &end) {
                (Included(s), Included(e)) if s > e => None,
                _ => Some((start, end)),
            }
        }),
        1 => (0..KEY_ALPHABET).prop_map(|byte| {
            let key = Bytes::from(vec![byte]);
            (Included(key.clone()), Included(key))
        }),
    ]
}

async fn open_db(path: &str) -> Db {
    let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    let settings = Settings {
        manifest_poll_interval: Duration::from_millis(10),
        // Load-bearing: bigger L0 SSTs mean compaction never splits a key
        // across the sorted run's SSTs, and the spanning guard below fails.
        l0_sst_size_bytes: 1024,
        l0_max_ssts: 10_000,
        l0_max_ssts_per_key: 10_000,
        ..Settings::default()
    };
    let compactor_options = CompactorOptions {
        // Every interval here defaults to seconds, which would dominate the
        // cost of a test that compacts once per case.
        poll_interval: Duration::from_millis(1),
        commit_compacted_interval: Duration::from_millis(1),
        scheduler_options: SizeTieredCompactionSchedulerOptions {
            min_compaction_sources: 1,
            ..Default::default()
        }
        .into(),
        worker: Some(CompactionWorkerOptions {
            // The worker polls for jobs on its own interval, which also
            // defaults to 5 seconds.
            compactions_poll_interval: Duration::from_millis(1),
            max_sst_size: 64,
            ..Default::default()
        }),
        ..Default::default()
    };
    Db::builder(path, object_store.clone())
        .with_settings(settings)
        // The compactor rolls over on finished block sizes, so blocks have to
        // be small enough that a short workload finishes several of them.
        // 1KiB is the smallest the public API offers, which is why the values
        // below are padded rather than tiny.
        .with_sst_block_size(SstBlockSize::Block1Kib)
        .with_compactor_builder(
            CompactorBuilder::new(path, object_store)
                .with_scheduler_supplier(Arc::new(SizeTieredCompactionSchedulerSupplier::new()))
                .with_options(compactor_options),
        )
        .build()
        .await
        .unwrap()
}

/// Flushes L0 and waits for the compactor to drain it into a sorted run.
async fn compact_l0(db: &Db) {
    db.flush().await.unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        while !db.manifest().l0().is_empty() {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .expect("compactor did not drain L0 within 30s");
}

async fn collect(db: &Db, range: KeyRange, order: IterationOrder) -> Vec<(Bytes, Bytes)> {
    let options = ScanOptions::default().with_order(order);
    let mut iter = db.scan_with_options(range, &options).await.unwrap();
    let mut got = Vec::new();
    while let Some(kv) = iter.next().await.unwrap() {
        got.push((kv.key, kv.value));
    }
    got
}

/// Applies a generated write sequence to both a db and a `BTreeMap`, then
/// checks that a scan of a generated range in a generated direction returns
/// exactly what the model holds.
#[test]
fn test_scan_matches_model() {
    // Each case opens a db and compacts, so cap the cases well below proptest's
    // default 256 to keep the test around a second. At this count roughly a
    // third of cases still build the spanning layout.
    let config = Config {
        cases: 24,
        source_file: Some(file!()),
        ..Config::default()
    };
    let mut runner = TestRunner::new(config);
    let runtime = Runtime::new().unwrap();

    runner
        .run(
            &(scan_ops(60), proptest::collection::vec(scan_range(), 1..=4)),
            |(ops, ranges)| {
                runtime.block_on(async {
                    let db = open_db("/tmp/test_scan_matches_model").await;
                    // Taken before any write so compaction retains every
                    // version, which is what lets a key span SSTs.
                    let _snapshot = db.snapshot().await.unwrap();

                    let mut model: BTreeMap<Bytes, Bytes> = BTreeMap::new();
                    for op in &ops {
                        match op {
                            ScanOp::Put(key, value) => {
                                db.put(key, value).await.unwrap();
                                model.insert(key.clone(), value.clone());
                            }
                            ScanOp::Delete(key) => {
                                db.delete(key).await.unwrap();
                                model.remove(key);
                            }
                            ScanOp::Flush => db.flush().await.unwrap(),
                            ScanOp::Compact => compact_l0(&db).await,
                        }
                    }

                    // Building the db dominates the cost of a case, so check
                    // several ranges in both directions rather than one.
                    for range in &ranges {
                        let ascending: Vec<(Bytes, Bytes)> = model
                            .range(range.clone())
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        let mut descending = ascending.clone();
                        descending.reverse();
                        for (order, expected) in [
                            (IterationOrder::Ascending, &ascending),
                            (IterationOrder::Descending, &descending),
                        ] {
                            assert_eq!(
                                &collect(&db, range.clone(), order).await,
                                expected,
                                "scan mismatch for range {range:?} order {order:?}"
                            );
                        }
                    }
                    db.close().await.unwrap();
                });
                Ok(())
            },
        )
        .unwrap();
}
