//! Integration tests for the prefix bloom filter.
//!
//! Four test modules:
//!   * [`composite_filters`]: integration tests that configure two filter
//!     policies on a single DB (one full-key, one conditional prefix) and
//!     verify reads work after closing/reopening with policies in a different
//!     order.
//!   * [`subrange`]: integration tests for `scan_prefix` restricted to a
//!     subrange, asserting both correct results and actual SST pruning via
//!     the filter-negative counter.
//!   * [`empty_prefix_filter`]: regression tests for #1966 — a persisted
//!     zero-bit prefix filter must report a miss instead of panicking, on
//!     fresh and reopened DBs.
//!   * [`prop_test`]: a property test asserting that `scan_prefix` (with and
//!     without a subrange) returns the same results with and without a prefix
//!     bloom filter configured. The filter must never introduce false
//!     negatives.

mod composite_filters {
    use std::sync::Arc;

    use bytes::Bytes;
    use slatedb::config::{
        FlushOptions, FlushType, PutOptions, ScanOptions, Settings, WriteOptions,
    };
    use slatedb::object_store::memory::InMemory;
    use slatedb::object_store::ObjectStore;
    use slatedb::{BloomFilterPolicy, Db, FilterPolicy, PrefixExtractor, PrefixTarget};

    const SAMPLE_USERS: &[&[u8]] = &[
        b"u:alice", b"u:bob", b"u:carol", b"u:dave", b"u:eve", b"u:frank",
    ];
    const SAMPLE_NON_USERS: &[&[u8]] = &[
        b"session:s1",
        b"session:s2",
        b"metric:m1",
        b"metric:m2",
        b"metric:m3",
    ];

    /// Conditional prefix extractor that extracts a fixed-length prefix only
    /// for inputs that begin with `marker`. Inputs not starting with `marker`
    /// return `None`, meaning they are not hashed into the prefix bloom (build
    /// time) and the prefix bloom is bypassed for scans whose prefix does not
    /// start with `marker` (read time).
    ///
    /// The extracted length is `marker.len() + extra_len`, so for
    /// `marker = "u:"` and `extra_len = 4` the input `"u:alice"` extracts
    /// `"u:ali"` (6 bytes) and `"session:1"` extracts nothing.
    struct ConditionalPrefixExtractor {
        marker: Bytes,
        extra_len: usize,
        name: String,
    }

    impl ConditionalPrefixExtractor {
        fn new(marker: &[u8], extra_len: usize) -> Self {
            Self {
                marker: Bytes::copy_from_slice(marker),
                extra_len,
                name: format!(
                    "cond:{}:{}",
                    std::str::from_utf8(marker).expect("marker must be utf8"),
                    extra_len
                ),
            }
        }
    }

    impl PrefixExtractor for ConditionalPrefixExtractor {
        fn name(&self) -> &str {
            &self.name
        }

        fn prefix_len(&self, target: &PrefixTarget) -> Option<usize> {
            let input = match target {
                PrefixTarget::Point(k) => k.as_ref(),
                PrefixTarget::Prefix(p) => p.as_ref(),
            };
            let total = self.marker.len() + self.extra_len;
            if input.len() >= total && input.starts_with(self.marker.as_ref()) {
                Some(total)
            } else {
                None
            }
        }
    }

    /// Composite policies under test: a full-key bloom paired with a prefix
    /// bloom that only fires for keys starting with `"u:"`.
    fn composite_policies() -> Vec<Arc<dyn FilterPolicy>> {
        vec![
            // Full-key bloom for every key.
            Arc::new(BloomFilterPolicy::new(10)),
            // Prefix bloom that only fires for keys starting with "u:".
            Arc::new(
                BloomFilterPolicy::new(10)
                    .with_whole_key_filtering(false)
                    .with_prefix_extractor(Arc::new(ConditionalPrefixExtractor::new(b"u:", 4))),
            ),
        ]
    }

    fn base_settings() -> Settings {
        Settings {
            min_filter_keys: 0,
            compactor_options: None,
            ..Settings::default()
        }
    }

    async fn flush_memtable(db: &Db) {
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .expect("memtable flush failed");
    }

    async fn write_sample_data(db: &Db) {
        let put = PutOptions::default();
        let write = WriteOptions::default();
        // Write each batch in its own SST so multiple SSTs participate in the
        // read path and the filter has something to actually skip.
        for batch in [SAMPLE_USERS, SAMPLE_NON_USERS] {
            for (i, key) in batch.iter().enumerate() {
                let value = format!(
                    "v{}-{}",
                    std::str::from_utf8(key).expect("sample key must be utf8"),
                    i
                )
                .into_bytes();
                db.put_with_options(key, &value, &put, &write)
                    .await
                    .expect("put failed");
            }
            flush_memtable(db).await;
        }
    }

    async fn assert_reads_consistent(db: &Db) {
        // Every written key is readable.
        for key in SAMPLE_USERS.iter().chain(SAMPLE_NON_USERS.iter()) {
            let got = db.get(key).await.expect("get failed");
            assert!(got.is_some(), "expected key {:?} to be present", key);
        }
        // Absent keys return None. The full-key bloom and (for "u:" keys) the
        // prefix bloom must agree on absence.
        for absent in [
            b"u:nope__".as_slice(),
            b"u:zzzzzz".as_slice(),
            b"session:nope".as_slice(),
            b"metric:nope".as_slice(),
        ] {
            let got = db.get(absent).await.expect("get failed");
            assert!(got.is_none(), "expected absent key {:?}", absent);
        }
        // scan_prefix on the prefix-extracted domain returns all matching keys.
        let mut iter = db
            .scan_prefix_with_options(b"u:", .., &ScanOptions::default())
            .await
            .expect("scan_prefix failed");
        let mut keys = Vec::new();
        while let Some(kv) = iter.next().await.expect("iterator next failed") {
            keys.push(kv.key.to_vec());
        }
        keys.sort();
        let mut expected: Vec<Vec<u8>> = SAMPLE_USERS.iter().map(|k| k.to_vec()).collect();
        expected.sort();
        assert_eq!(keys, expected, "scan_prefix(b\"u:\") mismatch");

        // scan_prefix outside the extractor's domain still works: the prefix
        // bloom is bypassed (extractor returns None) and the full-key bloom
        // is irrelevant for prefix scans, so all SSTs are visited.
        let mut iter = db
            .scan_prefix_with_options(b"metric:", .., &ScanOptions::default())
            .await
            .expect("scan_prefix failed");
        let mut keys = Vec::new();
        while let Some(kv) = iter.next().await.expect("iterator next failed") {
            keys.push(kv.key.to_vec());
        }
        keys.sort();
        let mut expected: Vec<Vec<u8>> = SAMPLE_NON_USERS
            .iter()
            .filter(|k| k.starts_with(b"metric:"))
            .map(|k| k.to_vec())
            .collect();
        expected.sort();
        assert_eq!(keys, expected, "scan_prefix(b\"metric:\") mismatch");
    }

    async fn open_db(
        path: &str,
        store: Arc<dyn ObjectStore>,
        policies: Vec<Arc<dyn FilterPolicy>>,
    ) -> Db {
        Db::builder(path, store)
            .with_settings(base_settings())
            .with_filter_policies(policies)
            .build()
            .await
            .expect("failed to build db")
    }

    #[tokio::test]
    async fn composite_policies_lifecycle() {
        let store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/test/composite_reopen";

        let db = open_db(path, store.clone(), composite_policies()).await;
        write_sample_data(&db).await;
        assert_reads_consistent(&db).await;
        db.close().await.expect("close failed");

        // Reopen with the policies in reversed order. Filter sub-blocks are
        // matched by policy name, so order should not matter.
        let mut reversed = composite_policies();
        reversed.reverse();
        let db = open_db(path, store.clone(), reversed).await;
        assert_reads_consistent(&db).await;
        db.close().await.expect("close failed");
    }
}

mod subrange {
    use std::sync::Arc;

    use slatedb::config::{FlushOptions, FlushType, PutOptions, Settings, WriteOptions};
    use slatedb::db_stats::{FILTER_KIND_LABEL, FILTER_KIND_PREFIX, SST_FILTER_NEGATIVE_COUNT};
    use slatedb::object_store::memory::InMemory;
    use slatedb::{BloomFilterPolicy, Db, PrefixExtractor, PrefixTarget};
    use slatedb_common::metrics::{DefaultMetricsRecorder, MetricValue};

    const PREFIX_LEN: usize = 3;

    /// Fixed-length prefix extractor, as in [`crate::prop_test`].
    struct FixedPrefixExtractor;

    impl PrefixExtractor for FixedPrefixExtractor {
        fn name(&self) -> &str {
            "fixed3"
        }

        fn prefix_len(&self, target: &PrefixTarget) -> Option<usize> {
            let input = match target {
                PrefixTarget::Point(k) => k.as_ref(),
                PrefixTarget::Prefix(p) => p.as_ref(),
            };
            (input.len() >= PREFIX_LEN).then_some(PREFIX_LEN)
        }
    }

    fn prefix_filter_negatives(recorder: &DefaultMetricsRecorder) -> u64 {
        recorder
            .snapshot()
            .by_name_and_labels(
                SST_FILTER_NEGATIVE_COUNT,
                &[(FILTER_KIND_LABEL, FILTER_KIND_PREFIX)],
            )
            .map(|m| match m.value {
                MetricValue::Counter(v) => v,
                ref other => panic!("expected counter, got {:?}", other),
            })
            .unwrap_or(0)
    }

    async fn collect_keys(mut iter: slatedb::DbIterator) -> Vec<Vec<u8>> {
        let mut keys = Vec::new();
        while let Some(kv) = iter.next().await.expect("iterator next failed") {
            keys.push(kv.key.to_vec());
        }
        keys
    }

    /// Three SSTs: one holding the `bbb` keys and two "sandwich" SSTs whose
    /// key ranges (`aaa..ccc`) overlap any `bbb` scan range but contain no
    /// `bbb` keys. Range-based pruning cannot exclude the sandwich SSTs, so
    /// any skip observed on the filter-negative counter is the prefix bloom
    /// filter consulted through the subrange scan path.
    #[tokio::test]
    async fn subrange_scan_prunes_ssts_and_returns_exact_keys() {
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder("/test/subrange", Arc::new(InMemory::new()))
            .with_settings(Settings {
                min_filter_keys: 0,
                compactor_options: None,
                ..Settings::default()
            })
            .with_filter_policies(vec![Arc::new(
                BloomFilterPolicy::new(10).with_prefix_extractor(Arc::new(FixedPrefixExtractor)),
            )])
            .with_metrics_recorder(recorder.clone())
            .build()
            .await
            .expect("failed to build db");

        let put = PutOptions::default();
        let write = WriteOptions::default();
        let ssts: &[&[&[u8]]] = &[
            &[b"aaa1", b"ccc1"], // sandwich
            &[b"bbb1", b"bbb2", b"bbb3", b"bbb4"],
            &[b"aaa2", b"ccc2"], // sandwich
        ];
        for sst in ssts {
            for key in *sst {
                db.put_with_options(key, b"v", &put, &write)
                    .await
                    .expect("put failed");
            }
            db.flush_with_options(FlushOptions {
                flush_type: FlushType::MemTable,
            })
            .await
            .expect("memtable flush failed");
        }

        // Subrange scan returns exactly the keys whose suffix falls in the
        // subrange, and the two sandwich SSTs are skipped on the filter.
        let negatives_before = prefix_filter_negatives(&recorder);
        let iter = db
            .scan_prefix(b"bbb", b"2".as_slice()..b"4".as_slice())
            .await
            .expect("scan_prefix failed");
        assert_eq!(
            collect_keys(iter).await,
            vec![b"bbb2".to_vec(), b"bbb3".to_vec()]
        );
        assert_eq!(
            prefix_filter_negatives(&recorder) - negatives_before,
            2,
            "expected both sandwich SSTs to be skipped on the prefix filter"
        );

        // Inclusive end bound.
        let iter = db
            .scan_prefix(b"bbb", ..=b"2".as_slice())
            .await
            .expect("scan_prefix failed");
        assert_eq!(
            collect_keys(iter).await,
            vec![b"bbb1".to_vec(), b"bbb2".to_vec()]
        );

        // Full subrange is equivalent to the classic prefix scan.
        let iter = db
            .scan_prefix(b"bbb", ..)
            .await
            .expect("scan_prefix failed");
        assert_eq!(
            collect_keys(iter).await,
            vec![
                b"bbb1".to_vec(),
                b"bbb2".to_vec(),
                b"bbb3".to_vec(),
                b"bbb4".to_vec()
            ]
        );

        // A plain range scan over the same composed bounds returns the same
        // keys but does not consult prefix filters.
        let negatives_before = prefix_filter_negatives(&recorder);
        let iter = db
            .scan(b"bbb2".as_slice()..b"bbb4".as_slice())
            .await
            .expect("scan failed");
        assert_eq!(
            collect_keys(iter).await,
            vec![b"bbb2".to_vec(), b"bbb3".to_vec()]
        );
        assert_eq!(
            prefix_filter_negatives(&recorder),
            negatives_before,
            "plain range scans must not consult prefix filters"
        );

        db.close().await.expect("close failed");
    }
}

mod empty_prefix_filter {
    use std::sync::Arc;

    use slatedb::config::{FlushOptions, FlushType, PutOptions, Settings, WriteOptions};
    use slatedb::db_stats::{
        FILTER_KIND_LABEL, FILTER_KIND_POINT, FILTER_KIND_PREFIX, SST_FILTER_NEGATIVE_COUNT,
    };
    use slatedb::object_store::memory::InMemory;
    use slatedb::object_store::ObjectStore;
    use slatedb::{BloomFilterPolicy, Db, PrefixExtractor, PrefixTarget};
    use slatedb_common::metrics::{DefaultMetricsRecorder, MetricValue};

    const PREFIX_LEN: usize = 4;

    /// Extracts a 4-byte prefix only from inputs of at least 4 bytes, so an
    /// SST holding only shorter keys builds a zero-bit prefix filter.
    struct GatedPrefixExtractor;

    impl PrefixExtractor for GatedPrefixExtractor {
        fn name(&self) -> &str {
            "gated4"
        }

        fn prefix_len(&self, target: &PrefixTarget) -> Option<usize> {
            let input = match target {
                PrefixTarget::Point(k) => k.as_ref(),
                PrefixTarget::Prefix(p) => p.as_ref(),
            };
            (input.len() >= PREFIX_LEN).then_some(PREFIX_LEN)
        }
    }

    fn filter_negatives(recorder: &DefaultMetricsRecorder, kind: &'static str) -> u64 {
        recorder
            .snapshot()
            .by_name_and_labels(SST_FILTER_NEGATIVE_COUNT, &[(FILTER_KIND_LABEL, kind)])
            .map(|m| match m.value {
                MetricValue::Counter(v) => v,
                ref other => panic!("expected counter, got {:?}", other),
            })
            .unwrap_or(0)
    }

    async fn open_db(store: Arc<dyn ObjectStore>, recorder: Arc<DefaultMetricsRecorder>) -> Db {
        Db::builder("/test/empty_prefix_filter", store)
            .with_settings(Settings {
                min_filter_keys: 0,
                compactor_options: None,
                ..Settings::default()
            })
            .with_filter_policies(vec![Arc::new(
                BloomFilterPolicy::new(10)
                    .with_whole_key_filtering(false)
                    .with_prefix_extractor(Arc::new(GatedPrefixExtractor)),
            )])
            .with_metrics_recorder(recorder)
            .build()
            .await
            .expect("failed to build db")
    }

    async fn collect_keys(mut iter: slatedb::DbIterator) -> Vec<Vec<u8>> {
        let mut keys = Vec::new();
        while let Some(kv) = iter.next().await.expect("iterator next failed") {
            keys.push(kv.key.to_vec());
        }
        keys
    }

    async fn assert_empty_filter_reads(db: &Db, recorder: &DefaultMetricsRecorder) {
        // Extractor-rejected queries bypass the filter; short keys stay reachable.
        for key in [b"a".as_slice(), b"b".as_slice()] {
            let got = db.get(key).await.expect("get failed");
            assert!(got.is_some(), "expected key {:?} to be present", key);
        }
        let iter = db.scan_prefix(b"a", ..).await.expect("scan_prefix failed");
        assert_eq!(collect_keys(iter).await, vec![b"a".to_vec()]);

        // Extractor-accepted queries probe the zero-bit filter: before the
        // fix this panicked with a division by zero; now the SST is skipped.
        let negatives_before = filter_negatives(recorder, FILTER_KIND_PREFIX);
        let iter = db
            .scan_prefix(b"aaaa", ..)
            .await
            .expect("scan_prefix failed");
        assert!(collect_keys(iter).await.is_empty());
        assert_eq!(
            filter_negatives(recorder, FILTER_KIND_PREFIX) - negatives_before,
            1,
            "expected the SST to be skipped on the empty prefix filter"
        );

        let negatives_before = filter_negatives(recorder, FILTER_KIND_POINT);
        let got = db.get(b"aaaa").await.expect("get failed");
        assert!(got.is_none());
        assert_eq!(
            filter_negatives(recorder, FILTER_KIND_POINT) - negatives_before,
            1,
            "expected the SST to be skipped on the empty prefix filter"
        );
    }

    #[tokio::test]
    async fn empty_filter_reports_miss_after_write_and_reopen() {
        let store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = open_db(store.clone(), recorder.clone()).await;

        let put = PutOptions::default();
        let write = WriteOptions::default();
        for key in [b"a".as_slice(), b"b".as_slice()] {
            db.put_with_options(key, b"v", &put, &write)
                .await
                .expect("put failed");
        }
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .expect("memtable flush failed");

        assert_empty_filter_reads(&db, &recorder).await;
        db.close().await.expect("close failed");

        // Reopen so the zero-bit filter is decoded from the persisted SST.
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = open_db(store, recorder.clone()).await;
        assert_empty_filter_reads(&db, &recorder).await;
        db.close().await.expect("close failed");
    }
}

mod prop_test {
    use std::sync::Arc;

    use proptest::collection::vec;
    use proptest::prelude::*;
    use slatedb::config::{PutOptions, ScanOptions, Settings, WriteOptions};
    use slatedb::object_store::memory::InMemory;
    use slatedb::{
        BloomFilterPolicy, ByteRangeBounds, Db, FilterPolicy, PrefixExtractor, PrefixTarget,
    };
    use tokio::runtime::Runtime;

    const PREFIX_LEN: usize = 3;

    /// Fixed-length prefix extractor: returns `Some(len)` whenever the input is
    /// at least `len` bytes. Always extracts the same `len`, so it is
    /// truncation safe for `Prefix` targets.
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

    fn settings() -> Settings {
        Settings {
            min_filter_keys: 0,
            compactor_options: None,
            ..Settings::default()
        }
    }

    async fn build_db(path: &str, filter_policies: Vec<Arc<dyn FilterPolicy>>) -> Db {
        Db::builder(path, Arc::new(InMemory::new()))
            .with_settings(settings())
            .with_filter_policies(filter_policies)
            .build()
            .await
            .expect("failed to build db")
    }

    async fn write_keys(db: &Db, keys: &[Vec<u8>]) {
        let put_opts = PutOptions::default();
        let write_opts = WriteOptions::default();
        for (i, key) in keys.iter().enumerate() {
            let value = format!("v{}", i).into_bytes();
            db.put_with_options(key, &value, &put_opts, &write_opts)
                .await
                .expect("put failed");
            // Flush every few keys so writes spread across multiple SSTs and
            // the filter has something to skip.
            if i % 8 == 7 {
                db.flush().await.expect("flush failed");
            }
        }
        db.flush().await.expect("flush failed");
    }

    async fn collect_prefix_scan(
        db: &Db,
        prefix: &[u8],
        subrange: impl ByteRangeBounds + Send,
    ) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut iter = db
            .scan_prefix_with_options(prefix, subrange, &ScanOptions::default())
            .await
            .expect("scan_prefix failed");
        let mut out = Vec::new();
        while let Some(kv) = iter.next().await.expect("iterator next failed") {
            out.push((kv.key.to_vec(), kv.value.to_vec()));
        }
        out
    }

    // Keys are 3 to 8 bytes from a small alphabet so collisions on the 3-byte
    // prefix are likely; that's the case where the bloom filter actually has
    // hashes to compare against.
    fn key_strategy() -> impl Strategy<Value = Vec<u8>> {
        vec(
            prop_oneof![Just(b'a'), Just(b'b'), Just(b'c'), Just(b'd')],
            3..=8,
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            ..ProptestConfig::default()
        })]

        #[test]
        fn prefix_scan_matches_with_and_without_filter(
            keys in vec(key_strategy(), 1..50),
            query_seed in any::<u8>(),
        ) {
            let runtime = Runtime::new().expect("failed to create runtime");
            runtime.block_on(async {
                let db_no_filter = build_db("/test/no_filter", vec![]).await;
                let db_with_filter = build_db(
                    "/test/with_filter",
                    vec![Arc::new(
                        BloomFilterPolicy::new(10).with_prefix_extractor(
                            Arc::new(FixedPrefixExtractor::new(PREFIX_LEN)),
                        ),
                    )],
                )
                .await;

                write_keys(&db_no_filter, &keys).await;
                write_keys(&db_with_filter, &keys).await;

                // Build a candidate set of prefixes: every distinct full-length
                // prefix from the input, plus a couple guaranteed-absent ones.
                let mut prefixes: Vec<Vec<u8>> = keys
                    .iter()
                    .filter(|k| k.len() >= PREFIX_LEN)
                    .map(|k| k[..PREFIX_LEN].to_vec())
                    .collect();
                prefixes.sort();
                prefixes.dedup();
                prefixes.push(b"zzz".to_vec());
                prefixes.push(b"xyz".to_vec());

                let pick = prefixes[query_seed as usize % prefixes.len()].clone();

                let no_filter_results = collect_prefix_scan(&db_no_filter, &pick, ..).await;
                let with_filter_results = collect_prefix_scan(&db_with_filter, &pick, ..).await;

                prop_assert_eq!(&no_filter_results, &with_filter_results);

                // Subrange-restricted scans agree between the two DBs and
                // with a client-side filtering of the full prefix scan. Use a
                // suffix of one of the matching keys as the lower bound so
                // the subrange is usually non-trivial.
                let mut suffixes: Vec<Vec<u8>> = keys
                    .iter()
                    .filter(|k| k.starts_with(&pick) && k.len() > PREFIX_LEN)
                    .map(|k| k[PREFIX_LEN..].to_vec())
                    .collect();
                suffixes.sort();
                suffixes.dedup();
                if !suffixes.is_empty() {
                    let lo = suffixes[query_seed as usize % suffixes.len()].clone();
                    let no_filter_sub =
                        collect_prefix_scan(&db_no_filter, &pick, lo.as_slice()..).await;
                    let with_filter_sub =
                        collect_prefix_scan(&db_with_filter, &pick, lo.as_slice()..).await;
                    prop_assert_eq!(&no_filter_sub, &with_filter_sub);

                    let expected: Vec<(Vec<u8>, Vec<u8>)> = no_filter_results
                        .iter()
                        .filter(|(k, _)| k[PREFIX_LEN..] >= lo[..])
                        .cloned()
                        .collect();
                    prop_assert_eq!(&no_filter_sub, &expected);
                }

                db_no_filter.close().await.expect("close failed");
                db_with_filter.close().await.expect("close failed");
                Ok(())
            })?;
        }
    }
}
