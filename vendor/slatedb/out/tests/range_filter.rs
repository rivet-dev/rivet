//! Integration test for filters on plain range scans.
//!
//! A range scan evaluates SST filters the way point and prefix scans do. This
//! measures what that buys: a filter that rejects half the SSTs a scan would
//! otherwise open, and the object-store reads it saves by doing so.

use std::sync::Arc;

use slatedb::config::{FlushOptions, FlushType, Settings};
use slatedb::db_stats::{
    FILTER_KIND_LABEL, FILTER_KIND_RANGE, SST_FILTER_NEGATIVE_COUNT, SST_FILTER_POSITIVE_COUNT,
};
use slatedb::instrumented_object_store_stats::REQUEST_COUNT as OBJECT_STORE_REQUEST_COUNT;
use slatedb::object_store::memory::InMemory;
use slatedb::object_store::ObjectStore;
use slatedb::{Db, Filter, FilterBuilder, FilterPolicy, FilterQuery, FilterTarget, RowEntry};
use slatedb_common::metrics::{DefaultMetricsRecorder, MetricValue};

/// SSTs written per test, each holding [`KEYS_PER_SST`] keys.
const SSTS: usize = 8;
const KEYS_PER_SST: usize = 3;

/// Accepts a range query only from SSTs whose keys all end in an even digit.
///
/// This deliberately answers `false` for SSTs that do hold keys in range,
/// which a real policy must never do. It exists to prune an exactly known half
/// of the SSTs so the saving is measurable.
struct ParityPolicy;

impl FilterPolicy for ParityPolicy {
    fn name(&self) -> &str {
        "test.last_byte_parity"
    }

    fn builder(&self) -> Box<dyn FilterBuilder> {
        Box::new(ParityBuilder { all_even: true })
    }

    fn decode(&self, data: &[u8]) -> Arc<dyn Filter> {
        Arc::new(ParityFilter {
            all_even: data[0] == 1,
        })
    }

    fn estimate_size(&self, _num_keys: usize) -> usize {
        1
    }

    fn supports_range_queries(&self) -> bool {
        true
    }
}

struct ParityBuilder {
    all_even: bool,
}

impl FilterBuilder for ParityBuilder {
    fn add_entry(&mut self, entry: &RowEntry) {
        self.all_even &= ends_even(&entry.key);
    }

    fn build(&mut self) -> Arc<dyn Filter> {
        Arc::new(ParityFilter {
            all_even: self.all_even,
        })
    }
}

struct ParityFilter {
    all_even: bool,
}

impl Filter for ParityFilter {
    fn might_match(&self, query: &FilterQuery) -> bool {
        match query.target {
            FilterTarget::Range { .. } => self.all_even,
            _ => true,
        }
    }

    fn encode(&self, writer: &mut dyn bytes::BufMut) {
        writer.put_u8(u8::from(self.all_even));
    }

    fn size(&self) -> usize {
        1
    }

    fn clamp_allocated_size(&self) -> Arc<dyn Filter> {
        Arc::new(Self {
            all_even: self.all_even,
        })
    }
}

fn ends_even(key: &[u8]) -> bool {
    key.last().expect("keys are never empty").is_multiple_of(2)
}

/// Keys for one SST, as `k<sst><digit>`.
///
/// Every digit shares the parity of `sst`, so SST `n` is kept exactly when `n`
/// is even, and the surviving keys are known without computing anything. Every
/// key shares the `k` prefix, so all SSTs fall inside a scan of the full range
/// and SST-level range pruning cannot exclude any of them. A skip on the
/// negative counter is therefore the filter.
fn sst_keys(sst: usize) -> Vec<Vec<u8>> {
    (0..KEYS_PER_SST)
        .map(|i| format!("k{sst}{}", 2 * i + sst % 2).into_bytes())
        .collect()
}

fn counter(recorder: &DefaultMetricsRecorder, name: &str, labels: &[(&str, &str)]) -> u64 {
    recorder
        .snapshot()
        .by_name_and_labels(name, labels)
        .map(|m| match m.value {
            MetricValue::Counter(v) => v,
            ref other => panic!("expected counter, got {other:?}"),
        })
        .unwrap_or(0)
}

/// Total object-store GET-shaped requests against the main store.
fn main_store_gets(recorder: &DefaultMetricsRecorder) -> u64 {
    ["get", "get_range", "get_ranges", "head"]
        .iter()
        .map(|api| {
            counter(
                recorder,
                OBJECT_STORE_REQUEST_COUNT,
                &[
                    ("component", "db"),
                    ("store_type", "main"),
                    ("op", "get"),
                    ("api", api),
                ],
            )
        })
        .sum()
}

/// Builds a DB holding [`SSTS`] L0 SSTs, with either the parity policy or the
/// default bloom policy registered.
async fn open_db(path: &str, filtered: bool) -> (Db, Arc<DefaultMetricsRecorder>) {
    let recorder = Arc::new(DefaultMetricsRecorder::new());
    let store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    let mut builder = Db::builder(path, store)
        .with_settings(Settings {
            min_filter_keys: 0,
            compactor_options: None,
            ..Settings::default()
        })
        .with_metrics_recorder(recorder.clone())
        .with_db_cache_disabled();
    if filtered {
        builder = builder.with_filter_policies(vec![Arc::new(ParityPolicy)]);
    }
    let db = builder.build().await.expect("failed to build db");

    // Values big enough that each SST spans several blocks, so skipping one
    // saves more reads than consulting its filter costs.
    let value = vec![b'v'; 8 * 1024];
    for sst in 0..SSTS {
        for key in sst_keys(sst) {
            db.put(&key, &value).await.expect("put failed");
        }
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .expect("flush failed");
    }
    (db, recorder)
}

async fn scan_all_keys(db: &Db) -> Vec<Vec<u8>> {
    let mut iter = db.scan(..).await.expect("scan failed");
    let mut keys = Vec::new();
    while let Some(kv) = iter.next().await.expect("iterator next failed") {
        keys.push(kv.key.to_vec());
    }
    keys
}

#[tokio::test]
async fn range_scan_filter_skips_ssts_and_saves_object_store_reads() {
    // Half the SSTs hold odd-digit keys, and the filter rejects exactly those.
    const SKIPPED: usize = SSTS / 2;

    let (plain, plain_recorder) = open_db("/test/range_filter_plain", false).await;
    let (filtered, filtered_recorder) = open_db("/test/range_filter_filtered", true).await;

    let plain_before = main_store_gets(&plain_recorder);
    let plain_keys = scan_all_keys(&plain).await;
    let plain_gets = main_store_gets(&plain_recorder) - plain_before;

    let filtered_before = main_store_gets(&filtered_recorder);
    let filtered_keys = scan_all_keys(&filtered).await;
    let filtered_gets = main_store_gets(&filtered_recorder) - filtered_before;

    // The baseline DB keeps the default bloom policy, which cannot answer a
    // range and says so through `supports_range_queries`. Its range scan
    // therefore builds no evaluator, reads no filters, records no verdict, and
    // sees every key.
    let all_keys: Vec<Vec<u8>> = (0..SSTS).flat_map(sst_keys).collect();
    assert_eq!(plain_keys, all_keys);
    assert_eq!(filter_verdicts(&plain_recorder), (0, 0));

    // With the parity policy the odd-digit SSTs are rejected, so exactly the
    // even-numbered SSTs' keys survive, already in scan order.
    let expected: Vec<Vec<u8>> = (0..SSTS).step_by(2).flat_map(sst_keys).collect();
    assert_eq!(expected.len(), (SSTS - SKIPPED) * KEYS_PER_SST);
    assert_eq!(filtered_keys, expected);
    assert_eq!(
        filter_verdicts(&filtered_recorder),
        ((SSTS - SKIPPED) as u64, SKIPPED as u64)
    );

    // A skipped SST costs no index or block reads, which outweighs the one
    // filter read every SST pays. Exact counts, so a change to the read-ahead
    // default or to block batching breaks this on purpose.
    assert_eq!(plain_gets, 32);
    assert_eq!(filtered_gets, 24);

    plain.close().await.expect("close failed");
    filtered.close().await.expect("close failed");
}

/// `(positives, negatives)` recorded for range queries.
fn filter_verdicts(recorder: &DefaultMetricsRecorder) -> (u64, u64) {
    let labels = [(FILTER_KIND_LABEL, FILTER_KIND_RANGE)];
    (
        counter(recorder, SST_FILTER_POSITIVE_COUNT, &labels),
        counter(recorder, SST_FILTER_NEGATIVE_COUNT, &labels),
    )
}
