// our microbenchmarks use pprof, but it doesn't work on windows
#![cfg(not(windows))]

//! Measures how much of a prefix scan's setup cost comes from handing a
//! `SortedRun` to a scan iterator when the run holds far more SSTs than the
//! query range covers.
//!
//! Fixture: one sorted run of 1000 SSTs, one key per SST, disjoint and ordered
//! key ranges, with no memtable or L0 data left behind. Keys are `sr/000`
//! through `sr/999`, so each shorter prefix selects ten times as many SSTs.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use chrono::TimeDelta;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use object_store::memory::InMemory;
use pprof::criterion::{Output, PProfProfiler};
use slatedb::config::{
    CompactorOptions, DurabilityLevel, FlushOptions, FlushType, ScanOptions, Settings,
};
use slatedb::Db;
use slatedb_common::clock::{DefaultSystemClock, SystemClock};
use tokio::runtime::Runtime;

/// Keys are `sr/000` through `sr/999`, one per SST.
const NUM_SSTS: usize = 1000;

struct Case {
    prefix: &'static str,
    /// Keys under the prefix, which is also the number of rows a full prefix
    /// scan returns.
    keys: usize,
    /// SST views `tables_covering_range` returns for the prefix range.
    covering: usize,
}

/// Each prefix drops one digit, so it selects ten times as many keys as the one
/// below it.
const CASES: [Case; 3] = [
    Case {
        prefix: "sr/222",
        keys: 1,
        covering: 1, // the prefix is a single key, so it covers one SST view.
    },
    Case {
        prefix: "sr/22",
        keys: 10,
        covering: 11, // the prefix covers 11 SST views.
    },
    Case {
        prefix: "sr/2",
        keys: 100,
        covering: 101, // the prefix covers 101 SST views.
    },
];

fn make_key(idx: usize) -> Bytes {
    Bytes::from(format!("sr/{idx:03}"))
}

fn prefix_start(prefix: &'static str) -> Bytes {
    Bytes::from_static(prefix.as_bytes())
}

/// Exclusive upper bound of the prefix range. Every prefix here ends in `2`, so
/// incrementing the last byte is enough.
fn prefix_end(prefix: &str) -> Bytes {
    let mut end = prefix.as_bytes().to_vec();
    *end.last_mut().expect("prefix is non-empty") += 1;
    Bytes::from(end)
}

fn settings() -> Settings {
    let mut scheduler_options = HashMap::new();
    // Fire exactly one compaction, and only once every L0 SST exists.
    scheduler_options.insert("min_compaction_sources".to_string(), NUM_SSTS.to_string());
    scheduler_options.insert("max_compaction_sources".to_string(), NUM_SSTS.to_string());

    Settings {
        // Writers stall at these limits and the fixture parks NUM_SSTS in L0.
        l0_max_ssts: NUM_SSTS + 16,
        l0_max_ssts_per_key: NUM_SSTS + 16,
        // One key per SST stays under the default min_filter_keys, so no SST
        // carries a filter and every case is decided by key range overlap.
        compactor_options: Some(CompactorOptions {
            poll_interval: Duration::from_millis(50),
            max_concurrent_compactions: 1,
            // A trivial move preserves the one flush per SST topology. A
            // rewriting compaction would choose its own output boundaries.
            enable_trivial_move: true,
            // No worker, so the coordinator either completes the compaction as
            // a trivial move or the fixture assertion fails.
            worker: None,
            scheduler_options,
            ..CompactorOptions::default()
        }),
        ..Settings::default()
    }
}

fn scan_options() -> ScanOptions {
    ScanOptions {
        cache_blocks: true,
        durability_filter: DurabilityLevel::Remote,
        ..ScanOptions::default()
    }
}

async fn populate(db: &Db) {
    for idx in 0..NUM_SSTS {
        let key = make_key(idx);
        db.put(&key, &key).await.expect("put failed");
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .expect("flush failed");
    }
}

async fn await_single_sorted_run(db: &Db) {
    let clock = DefaultSystemClock::new();
    let deadline = clock.now() + TimeDelta::seconds(300);
    loop {
        db.refresh_manifest()
            .await
            .expect("refresh_manifest failed");
        let manifest = db.manifest();
        let l0 = manifest.l0().len();
        let runs = manifest.compacted();
        let ssts = runs
            .first()
            .map_or(0, |run| run.tables_covering_range(..).len());
        if l0 == 0 && runs.len() == 1 && ssts == NUM_SSTS {
            return;
        }
        assert!(
            clock.now() < deadline,
            "compaction never produced one sorted run of {NUM_SSTS} ssts (l0={l0}, runs={}, ssts={ssts})",
            runs.len()
        );
        clock.sleep(Duration::from_millis(50)).await;
    }
}

async fn count_prefix_rows(db: &Db, prefix: &'static str) -> usize {
    let mut iter = db
        .scan_prefix_with_options(prefix_start(prefix), .., &scan_options())
        .await
        .expect("scan_prefix failed");
    let mut count = 0usize;
    while iter.next().await.expect("iterator next failed").is_some() {
        count += 1;
    }
    count
}

async fn validate_fixture(db: &Db) {
    let manifest = db.manifest();
    let run = manifest
        .compacted()
        .first()
        .expect("fixture has one sorted run");
    for case in &CASES {
        let covering = run
            .tables_covering_range(prefix_start(case.prefix)..prefix_end(case.prefix))
            .len();
        assert_eq!(
            covering, case.covering,
            "prefix {} covers {covering} sst views",
            case.prefix
        );
        let rows = count_prefix_rows(db, case.prefix).await;
        assert_eq!(
            rows, case.keys,
            "prefix {} scanned {rows} rows",
            case.prefix
        );
    }
}

async fn build_fixture() -> Db {
    let store = Arc::new(InMemory::new());
    let db = Db::builder("/bench/sorted_run", store)
        .with_settings(settings())
        .build()
        .await
        .expect("failed to build db");
    populate(&db).await;
    await_single_sorted_run(&db).await;
    validate_fixture(&db).await;
    db
}

fn bench_scan_prefix_large_sorted_run(c: &mut Criterion) {
    let runtime = Runtime::new().expect("failed to create runtime");
    let db = runtime.block_on(build_fixture());

    let scan_opts = scan_options();
    let mut group = c.benchmark_group("scan_prefix_large_sorted_run");

    for case in &CASES {
        let label = format!("K={}", case.keys);

        group.bench_function(BenchmarkId::new("first_entry", &label), |b| {
            b.to_async(&runtime).iter(|| async {
                let mut iter = db
                    .scan_prefix_with_options(prefix_start(case.prefix), .., &scan_opts)
                    .await
                    .expect("scan_prefix failed");
                let entry = iter.next().await.expect("iterator next failed");
                assert!(entry.is_some());
            });
        });

        group.bench_function(BenchmarkId::new("first_entry_by_recency", &label), |b| {
            b.to_async(&runtime).iter(|| async {
                let mut iter = db
                    .scan_prefix_by_recency_with_options(prefix_start(case.prefix), &scan_opts)
                    .await
                    .expect("scan_prefix_by_recency failed");
                let entry = iter.next_entry().await.expect("iterator next_entry failed");
                assert!(entry.is_some());
            });
        });
    }

    group.finish();
    runtime.block_on(async { db.close().await.expect("close failed") });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .with_profiler(PProfProfiler::new(100, Output::Protobuf));
    targets = bench_scan_prefix_large_sorted_run
}

criterion_main!(benches);
