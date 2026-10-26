// our microbenchmarks use pprof, but it doesn't work on windows
#![cfg(not(windows))]

//! Compares `scan_prefix` with and without a prefix bloom filter.
//!
//! - 100 L0 SSTs, 100 distinct 6-byte prefixes, queried prefix lives in 10.
//! - Sentinel keys span the keyspace so only the filter can prune.
//! - Store throttled at 10ms/GET to simulate object store latency
//! - Meta cache is configured to holds filter/index, data blocks
//!   uncached so SST reads pay a real GET.
//! - Sentinel keys are inserted per SST to make sure indecies can't filter out
//!   SSTs.
//!
//! Run: cargo bench -p slatedb --bench scan_prefix_bench

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use object_store::memory::InMemory;
use object_store::throttle::{ThrottleConfig, ThrottledStore};
use pprof::criterion::{Output, PProfProfiler};
use slatedb::config::{
    DurabilityLevel, FlushOptions, FlushType, PutOptions, ScanOptions, Settings, WriteOptions,
};
use slatedb::db_cache::foyer::{FoyerCache, FoyerCacheOptions};
use slatedb::db_cache::SplitCache;
use slatedb::{BloomFilterPolicy, Db, FilterPolicy, PrefixExtractor, PrefixTarget};
use tokio::runtime::Runtime;

const NUM_PREFIXES: usize = 100;
const PREFIXES_PER_FLUSH: usize = 10;
const VERSIONS_PER_FLUSH: usize = 5;
const NUM_FLUSHES: usize = 100;
const PREFIX_LEN: usize = 6;
const QUERY_PREFIX: &[u8; 6] = b"pk0095";
const GET_LATENCY: Duration = Duration::from_millis(10);

struct FixedPrefixExtractor {
    len: usize,
    name: String,
}

impl FixedPrefixExtractor {
    fn new(len: usize) -> Self {
        Self {
            len,
            name: format!("fixed{}", len),
        }
    }
}

impl PrefixExtractor for FixedPrefixExtractor {
    fn name(&self) -> &str {
        &self.name
    }

    fn prefix_len(&self, target: &PrefixTarget) -> Option<usize> {
        let input = match target {
            PrefixTarget::Point(k) => k.as_ref(),
            PrefixTarget::Prefix(p) => p.as_ref(),
        };
        (input.len() >= self.len).then_some(self.len)
    }
}

fn make_key(prefix_idx: usize, version: u64) -> Vec<u8> {
    format!("pk{:04}:{:08}", prefix_idx, version).into_bytes()
}

fn make_throttled_store() -> Arc<ThrottledStore<InMemory>> {
    Arc::new(ThrottledStore::new(
        InMemory::new(),
        ThrottleConfig::default(),
    ))
}

fn enable_throttle(store: &ThrottledStore<InMemory>) {
    store.config_mut(|cfg| {
        cfg.wait_get_per_call = GET_LATENCY;
    });
}

fn disable_throttle(store: &ThrottledStore<InMemory>) {
    store.config_mut(|cfg| {
        cfg.wait_get_per_call = Duration::ZERO;
    });
}

// Cache filter and index blocks (so the filter check itself is free) but not
// data blocks, so each surviving SST still pays the throttled GET cost.
fn meta_only_cache() -> Arc<SplitCache> {
    let meta = Arc::new(FoyerCache::new_with_opts(FoyerCacheOptions {
        max_capacity: 256 * 1024 * 1024,
        ..Default::default()
    }));
    Arc::new(
        SplitCache::new()
            .with_block_cache(None)
            .with_meta_cache(Some(meta))
            .build(),
    )
}

fn base_settings() -> Settings {
    Settings {
        min_filter_keys: 0,
        l0_sst_size_bytes: 256 * 1024 * 1024,
        l0_max_ssts: NUM_FLUSHES + 10,
        // Sentinel keys make every SST's effective range cover every key, so
        // per-key L0 overlap equals the number of flushes. Bump the per-key
        // cap above NUM_FLUSHES; otherwise populate() blocks on backpressure.
        l0_max_ssts_per_key: NUM_FLUSHES + 10,
        compactor_options: None,
        ..Settings::default()
    }
}

fn scan_options() -> ScanOptions {
    ScanOptions {
        // Don't cache data blocks during the bench: we want every iteration
        // to pay the throttled GET cost for whichever SSTs the scan touches.
        cache_blocks: false,
        durability_filter: DurabilityLevel::Remote,
        ..ScanOptions::default()
    }
}

fn recency_scan_options() -> ScanOptions {
    ScanOptions {
        cache_blocks: false,
        durability_filter: DurabilityLevel::Remote,
        ..ScanOptions::default()
    }
}

async fn populate(db: &Db) {
    let write_opts = WriteOptions::default();
    let put_opts = PutOptions::default();
    let mut next_version = [0u64; NUM_PREFIXES];
    for round in 0..NUM_FLUSHES {
        // Sentinel keys ensure every SST's key range overlaps with every
        // prefix range, so without the filter the scan must visit them all.
        let lo = format!("aa_sentinel_{:04}", round).into_bytes();
        let hi = format!("zz_sentinel_{:04}", round).into_bytes();
        db.put_with_options(&lo, b"s", &put_opts, &write_opts)
            .await
            .expect("put failed");
        db.put_with_options(&hi, b"s", &put_opts, &write_opts)
            .await
            .expect("put failed");

        let start_px = (round * PREFIXES_PER_FLUSH) % NUM_PREFIXES;
        for i in 0..PREFIXES_PER_FLUSH {
            let px = (start_px + i) % NUM_PREFIXES;
            for _ in 0..VERSIONS_PER_FLUSH {
                let v = next_version[px];
                next_version[px] += 1;
                let key = make_key(px, v);
                db.put_with_options(&key, &key, &put_opts, &write_opts)
                    .await
                    .expect("put failed");
            }
        }
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .expect("flush failed");
    }
}

async fn warmup_meta_cache(db: &Db) {
    // Scans cache filter and index blocks independently of data-block caching,
    // so this warms metadata without warming data blocks.
    let warmup_opts = ScanOptions {
        cache_blocks: false,
        durability_filter: DurabilityLevel::Remote,
        ..ScanOptions::default()
    };
    let mut iter = db
        .scan_prefix_with_options(b"pk0000", .., &warmup_opts)
        .await
        .expect("warmup scan_prefix failed");
    while iter
        .next()
        .await
        .expect("warmup iterator next failed")
        .is_some()
    {}
}

struct BenchDb {
    db: Db,
    store: Arc<ThrottledStore<InMemory>>,
}

async fn build_db(path: &str, filter_policies: Vec<Arc<dyn FilterPolicy>>) -> BenchDb {
    let store = make_throttled_store();
    let db = Db::builder(path, store.clone())
        .with_settings(base_settings())
        .with_db_cache(meta_only_cache(), 0)
        .with_filter_policies(filter_policies)
        .build()
        .await
        .expect("failed to build db");
    populate(&db).await;
    warmup_meta_cache(&db).await;
    enable_throttle(&store);
    BenchDb { db, store }
}

fn bench_scan_prefix(c: &mut Criterion) {
    let runtime = Runtime::new().expect("failed to create runtime");

    let no_filter = runtime.block_on(build_db("/bench/no_filter", vec![]));
    let prefix_bloom = runtime.block_on(build_db(
        "/bench/prefix_bloom",
        vec![Arc::new(BloomFilterPolicy::new(10).with_prefix_extractor(
            Arc::new(FixedPrefixExtractor::new(PREFIX_LEN)),
        ))],
    ));

    let mut group = c.benchmark_group("scan_prefix");
    group.sample_size(20);

    // Note: with `prefix_bloom`, `first_entry` and `full_scan` come out at
    // similar latencies. The merge iterator has to read the first block of
    // every SST that passes the filter to find the globally smallest key, so
    // `first_entry` already pays one data block GET per surviving SST and
    // `full_scan` adds little on top.
    for (label, bench_db) in [("no_filter", &no_filter), ("prefix_bloom", &prefix_bloom)] {
        group.bench_function(BenchmarkId::new("first_entry", label), |b| {
            b.to_async(&runtime).iter(|| async {
                let mut iter = bench_db
                    .db
                    .scan_prefix_with_options(Bytes::from_static(QUERY_PREFIX), .., &scan_options())
                    .await
                    .expect("scan_prefix failed");
                let entry = iter.next().await.expect("iterator next failed");
                assert!(entry.is_some());
            });
        });

        group.bench_function(BenchmarkId::new("full_scan", label), |b| {
            b.to_async(&runtime).iter(|| async {
                let mut iter = bench_db
                    .db
                    .scan_prefix_with_options(Bytes::from_static(QUERY_PREFIX), .., &scan_options())
                    .await
                    .expect("scan_prefix failed");
                let mut count = 0usize;
                while iter.next().await.expect("iterator next failed").is_some() {
                    count += 1;
                }
                assert!(count > 0);
            });
        });
    }

    // The `*_by_recency` variants exercise `Db::scan_prefix_by_recency`, which
    // walks sources newest-first and stops at the first source that yields a
    // matching entry. Only run them with the prefix bloom filter: without a
    // filter, every SST passes the filter check so lazy init doesn't help,
    // and the comparison we care about is recency-vs-merge under the same
    // (filtered) source set.
    group.bench_function(
        BenchmarkId::new("first_entry_by_recency", "prefix_bloom"),
        |b| {
            b.to_async(&runtime).iter(|| async {
                let mut iter = prefix_bloom
                    .db
                    .scan_prefix_by_recency_with_options(
                        Bytes::from_static(QUERY_PREFIX),
                        &recency_scan_options(),
                    )
                    .await
                    .expect("scan_prefix_by_recency failed");
                let entry = iter.next_entry().await.expect("iterator next_entry failed");
                assert!(entry.is_some());
            });
        },
    );

    group.bench_function(
        BenchmarkId::new("full_scan_by_recency", "prefix_bloom"),
        |b| {
            b.to_async(&runtime).iter(|| async {
                let mut iter = prefix_bloom
                    .db
                    .scan_prefix_by_recency_with_options(
                        Bytes::from_static(QUERY_PREFIX),
                        &recency_scan_options(),
                    )
                    .await
                    .expect("scan_prefix_by_recency failed");
                let mut count = 0usize;
                while iter
                    .next_entry()
                    .await
                    .expect("iterator next_entry failed")
                    .is_some()
                {
                    count += 1;
                }
                assert!(count > 0);
            });
        },
    );

    group.finish();

    disable_throttle(&no_filter.store);
    disable_throttle(&prefix_bloom.store);
    runtime.block_on(async {
        no_filter.db.close().await.expect("close failed");
        prefix_bloom.db.close().await.expect("close failed");
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .with_profiler(PProfProfiler::new(100, Output::Protobuf));
    targets = bench_scan_prefix
}

criterion_main!(benches);
