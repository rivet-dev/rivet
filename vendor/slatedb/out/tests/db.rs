#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

use backon::{ExponentialBuilder, Retryable};
use log::warn;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use slatedb::admin;
use slatedb::config::{
    CompactorOptions, DurabilityLevel, FlushOptions, FlushType, PutOptions, ReadOptions, Settings,
    SizeTieredCompactionSchedulerOptions, WriteOptions,
};
use slatedb::fail_parallel::{self, FailPointRegistry};
use slatedb::object_store::memory::InMemory;
use slatedb::object_store::ObjectStore;
use slatedb::size_tiered_compaction::SizeTieredCompactionSchedulerSupplier;
use slatedb::{CompactorBuilder, Db};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::fmt::format::FmtSpan;

#[tokio::test]
async fn test_key_larger_than_u16_round_trips_through_sst() {
    let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    let key = vec![b'k'; u16::MAX as usize + 1];
    let value = b"value";
    let path = "/large-key-round-trip";

    let db = Db::open(path, object_store.clone()).await.unwrap();
    db.put(&key, value).await.unwrap();
    db.flush().await.unwrap();
    db.close().await.unwrap();

    let db = Db::open(path, object_store).await.unwrap();
    let actual = db.get(&key).await.unwrap().unwrap();
    assert_eq!(actual.as_ref(), value);
    db.close().await.unwrap();
}

/// Verify that writes succeed after WAL replay when replayed immutable
/// memtables have not yet been flushed to L0. Previously, the WAL buffer's
/// recent_flushed_wal_id lagged behind the imm_memtable WAL IDs, causing
/// a u64 underflow in maybe_freeze_memtable.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_replay_wal_then_write() {
    let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    let path = "/tmp/test_replay_wal_then_write";
    let fp_registry = Arc::new(FailPointRegistry::new());

    // Small l0_sst_size_bytes so replay produces multiple ReplayedMemtables,
    // creating immutable memtables during replay.
    let settings = Settings {
        flush_interval: None,
        l0_sst_size_bytes: 64,
        ..Default::default()
    };

    // Skip L0 flushes so only the WAL is persisted.
    fail_parallel::cfg(fp_registry.clone(), "flush-memtable-to-l0", "return").unwrap();

    let db = Db::builder(path, object_store.clone())
        .with_settings(settings.clone())
        .with_fp_registry(fp_registry.clone())
        .build()
        .await
        .expect("failed to open db");

    for i in 0..20 {
        let key = format!("key{:04}", i);
        let value = format!("value{:04}", i);
        db.put_with_options(
            key.as_bytes(),
            value.as_bytes(),
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .expect("failed to put");
    }

    db.flush_with_options(FlushOptions {
        flush_type: FlushType::Wal,
    })
    .await
    .expect("failed to flush WAL");

    // expect to fail as l0 upload is blocked
    assert!(db.close().await.is_err());

    // Reopen with L0 flush paused so replayed immutable memtables stay queued.
    fail_parallel::cfg(fp_registry.clone(), "flush-memtable-to-l0", "pause").unwrap();

    let db = Db::builder(path, object_store.clone())
        .with_settings(settings)
        .with_fp_registry(fp_registry.clone())
        .build()
        .await
        .expect("failed to reopen db");

    // Write after replay — previously underflowed in maybe_freeze_memtable.
    db.put_with_options(
        b"new_key",
        b"new_value",
        &PutOptions::default(),
        &WriteOptions {
            ..Default::default()
        },
    )
    .await
    .expect("put after replay should succeed");

    fail_parallel::cfg(fp_registry.clone(), "flush-memtable-to-l0", "off").unwrap();
    db.close().await.expect("failed to close db");
}
use tracing_subscriber::EnvFilter;

/// This test exercises SlateDB under concurrent load. By default, it uses an in-memory
/// object store for fast local testing, but it can also be used with a remote object store
/// (e.g., S3-compatible) by setting the `CLOUD_PROVIDER` environment variable.
///
/// The test behaves as follows:
///
/// - Spawns N writer tasks, each writing an incrementing `u64` value to a unique key.
/// - Spawns M reader tasks which sample random keys and assert values are non-decreasing.
/// - Writers call `flush()` at the end to accelerate durability during CI/chaos runs.
/// - If `CLOUD_PROVIDER` is set, the object store is loaded from the environment (see
///   [`slatedb::admin::load_object_store_from_env`] for details).
///
/// ## Environment variables
/// - `SLATEDB_TEST_NUM_WRITERS` (default: `10`) — number of writer tasks.
/// - `SLATEDB_TEST_NUM_READERS` (default: `2`) — number of concurrent reader tasks.
/// - `SLATEDB_TEST_WRITES_PER_TASK` (default: `100`) — number of writes per writer.
/// - `SLATEDB_TEST_KEY_LENGTH` (default: `256`) — padded key length in byte.
/// - `SLATEDB_TEST_*` (e.g. `SLATEDB_TEST_FLUSH_INTERVAL`) — override [`Settings`] fields
///   used for this test; unspecified fields fall back to the defaults below.
///
/// ## Usage
///
/// This test runs as a normal integration test as part of the crate's test suite. It is also
/// used as a chaos test in the nightly.yaml CI test suite to verify that SlateDB functions as
/// expected under intermittent network and object store failures. Take a look at
/// `scripts/run_chaos_scenarios.sh` to see how the chaos tests work.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_concurrent_writers_and_readers() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_test_writer()
        .init();

    let num_writers: usize = std::env::var("SLATEDB_TEST_NUM_WRITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let num_readers: usize = std::env::var("SLATEDB_TEST_NUM_READERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);
    let writes_per_task: usize = std::env::var("SLATEDB_TEST_WRITES_PER_TASK")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);
    let key_length: usize = std::env::var("SLATEDB_TEST_KEY_LENGTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(256);

    // Pad keys to allow us to control how many blocks we take up
    // Since block size is not configurable
    fn zero_pad_key(key: u64, length: usize) -> Vec<u8> {
        let mut padded_key = vec![0; length];
        // Set the first 8 bytes to the key
        padded_key[0..8].copy_from_slice(&key.to_le_bytes());
        padded_key
    }

    let object_store: Arc<dyn ObjectStore> = if let Ok(provider) = std::env::var("CLOUD_PROVIDER") {
        log::info!("Using object store from env (CLOUD_PROVIDER={})", provider);
        admin::load_object_store_from_env(None).expect("failed to load object store from env")
    } else {
        Arc::new(InMemory::new()) as Arc<dyn ObjectStore>
    };

    let compactor_options = CompactorOptions {
        poll_interval: Duration::from_millis(100),
        scheduler_options: SizeTieredCompactionSchedulerOptions {
            min_compaction_sources: 1,
            ..Default::default()
        }
        .into(),
        ..Default::default()
    };

    let config = Settings::from_env_with_default(
        "SLATEDB_TEST_",
        Settings {
            flush_interval: Some(Duration::from_millis(100)),
            manifest_poll_interval: Duration::from_millis(100),
            manifest_update_timeout: Duration::from_secs(300),
            // Allow 32KB of unflushed data (must exceed l0_sst_size_bytes)
            max_unflushed_bytes: 8 * 4096,
            min_filter_keys: 0,
            // Allow up to four 4096-byte blocks per-SST
            l0_sst_size_bytes: 4 * 4096,
            compactor_options: None,
            ..Default::default()
        },
    )
    .expect("failed to load db settings from environment");
    let supplier = Arc::new(SizeTieredCompactionSchedulerSupplier::new());
    // Build the DB with exponential backoff to tolerate transient object store errors.
    let retry_builder = ExponentialBuilder::default()
        .without_max_times()
        // Retry fast so we don't wait too long for transient errors
        .with_min_delay(Duration::from_millis(1))
        .with_max_delay(Duration::from_millis(1));
    // Always use a unique DB path per test run to avoid cross-run residue
    // in remote object stores (important for chaos scenarios).
    let db = Arc::new(
        (|| async {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let db_path = format!("/tmp/test_concurrent_writers_readers_{}", ts);
            Db::builder(db_path.clone(), object_store.clone())
                .with_settings(config.clone())
                .with_compactor_builder(
                    CompactorBuilder::new(db_path, object_store.clone())
                        .with_options(compactor_options.clone())
                        .with_scheduler_supplier(supplier.clone()),
                )
                .build()
                .await
        })
        .retry(retry_builder)
        .notify(|err, dur| {
            warn!("retrying error {:?} with sleeping {:?}", err, dur);
        })
        .await
        .expect("Failed to build DB after retries"),
    );
    let reader_cancellation_token = CancellationToken::new();

    // Writer tasks: each writer writes to its own key with incrementing values
    let writer_handles = (0..num_writers)
        .map(|writer_id| {
            let db = db.clone();
            tokio::spawn(async move {
                let key = zero_pad_key(writer_id.try_into().unwrap(), key_length);
                for i in 1..=writes_per_task {
                    // Write the incremented value
                    db.put_with_options(
                        &key,
                        i.to_be_bytes().as_ref(),
                        &PutOptions::default(),
                        &WriteOptions {
                            ..Default::default()
                        },
                    )
                    .await
                    .expect("Failed to write value");

                    if i % 10 == 0 {
                        log::info!("wrote values [writer_id={}, write_count={}]", writer_id, i);
                    }
                }
                // Flush to ensure all writes are durable before returning
                db.flush().await.expect("Failed to flush");
            })
        })
        .collect::<Vec<_>>();

    // Reader tasks: each reader reads all keys and verifies values are increasing
    let reader_handles = (0..num_readers)
        .map(|reader_id| {
            let db = db.clone();
            let reader_cancellation_token = reader_cancellation_token.clone();

            tokio::spawn(async move {
                let mut latest_values = HashMap::<usize, AtomicU64>::new();
                let mut iterations = 0;
                let mut rng = StdRng::from_os_rng();

                while !reader_cancellation_token.is_cancelled() {
                    // Pick a random key and validate that it's higher than the last value for that key
                    let key = rng.random_range(0..num_writers);
                    if let Some(bytes) = db
                        .get_with_options(
                            zero_pad_key(key.try_into().unwrap(), key_length),
                            // Make sure we don't see in-memory values that might later get lost in chaos tests
                            &ReadOptions::new().with_durability_filter(DurabilityLevel::Remote),
                        ).await?
                    {
                        // Convert bytes to u64 value
                        let value_bytes: [u8; 8] = bytes.as_ref().try_into().expect("invalid byte conversion");
                        let current_value = u64::from_be_bytes(value_bytes);

                        // Check if this value is greater than the last seen value for this key
                        let last_seen_atomic = latest_values
                            .entry(key)
                            .or_insert(AtomicU64::new(current_value));
                        let last_seen_value = last_seen_atomic.load(Ordering::SeqCst);

                        assert!(
                            current_value >= last_seen_value,
                            "Value {} is less than last seen value {}",
                            current_value,
                            last_seen_value
                        );

                        // Update the latest value seen
                        last_seen_atomic
                            .compare_exchange(
                                last_seen_value,
                                current_value,
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            )
                            .unwrap();

                        iterations += 1;

                        if current_value != last_seen_value {
                            log::info!(
                                "progress [reader_id={}, iterations={}, sample_key={}, last_seen_value={}, current_value={}]",
                                reader_id,
                                iterations,
                                key,
                                last_seen_value,
                                current_value
                            );
                        }
                    } else {
                        assert!(
                            !latest_values.contains_key(&key),
                            "No key {} in DB, but found in latest_values",
                            key,
                        );
                    }
                }
                Ok::<(), slatedb::Error>(())
            })
        })
        .collect::<Vec<_>>();

    // Wait for writers to complete
    futures::future::try_join_all(writer_handles)
        .await
        .expect("Writer handles failed");

    // Shut down readers
    reader_cancellation_token.cancel();
    futures::future::try_join_all(reader_handles)
        .await
        .expect("Reader handles failed");

    // Close with retries to handle transient failures on teardown.
    (|| async { db.close().await })
        .retry(retry_builder)
        .notify(|err, dur| {
            warn!("retrying error {:?} with sleeping {:?}", err, dur);
        })
        .await
        .expect("Failed to close DB after retries");
}

/// Verify that refresh_manifest enqueues a poll and returns once it completes,
/// and returns an error once the DB is closed.
#[tokio::test]
async fn test_refresh_manifest() {
    let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    let db = Db::builder("test_refresh_manifest", object_store.clone())
        .with_settings(Settings {
            manifest_poll_interval: Duration::from_secs(60 * 60),
            ..Default::default()
        })
        .build()
        .await
        .expect("failed to open db");

    // Should return once the enqueued poll completes.
    db.refresh_manifest()
        .await
        .expect("refresh_manifest returned error");

    // After close, refresh_manifest should return an error.
    db.close().await.expect("failed to close db");
    let result = db.refresh_manifest().await;
    assert!(
        result.is_err(),
        "expected error after close, got {:?}",
        result
    );
}
