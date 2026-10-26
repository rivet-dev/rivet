// our microbenchmarks use pprof, but it doesn't work on windows
#![cfg(not(windows))]

// Run with: cargo bench -p slatedb --features bench-internal --bench db_transaction

use std::sync::{Arc, OnceLock};
use std::time::Duration;

#[allow(clippy::disallowed_types)]
use std::time::Instant;

use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use object_store::memory::InMemory;
use pprof::criterion::{Output, PProfProfiler};
use slatedb::config::{
    MergeOptions, PutOptions, ReadOptions, ScanOptions, Settings, Ttl, WriteOptions,
};
use slatedb::{Db, DbTransaction, IsolationLevel, MergeOperator, MergeOperatorError};
use tokio::runtime::Runtime;

const VALUE_SIZE: usize = 128;
const DB_ENTRY_COUNT: usize = 1_024;
const TXN_ENTRY_COUNT: usize = 1_024;
const TXN_MERGE_KEY_COUNT: usize = 256;
const MERGES_PER_KEY: usize = 3;

struct ConcatMergeOperator;

impl MergeOperator for ConcatMergeOperator {
    fn merge(
        &self,
        _key: &Bytes,
        existing_value: Option<Bytes>,
        operand: Bytes,
    ) -> Result<Bytes, MergeOperatorError> {
        let Some(base) = existing_value else {
            return Ok(operand);
        };
        let mut merged = Vec::with_capacity(base.len() + operand.len());
        merged.extend_from_slice(&base);
        merged.extend_from_slice(&operand);
        Ok(Bytes::from(merged))
    }
}

fn concat_merge_operator() -> Arc<dyn MergeOperator + Send + Sync> {
    static MERGE_OPERATOR: OnceLock<Arc<dyn MergeOperator + Send + Sync>> = OnceLock::new();
    MERGE_OPERATOR
        .get_or_init(|| Arc::new(ConcatMergeOperator))
        .clone()
}

fn put_options() -> PutOptions {
    PutOptions {
        ttl: Ttl::ExpireAfterMillis(3_600_000),
    }
}

fn merge_options() -> MergeOptions {
    MergeOptions {
        ttl: Ttl::ExpireAfterMillis(3_600_000),
    }
}

fn write_options() -> WriteOptions {
    WriteOptions {
        ..WriteOptions::default()
    }
}

fn key(index: usize) -> Bytes {
    Bytes::from(format!("txn-key-{index:08}"))
}

fn value(index: usize) -> Bytes {
    let mut value = vec![0; VALUE_SIZE];
    value[..size_of::<usize>()].copy_from_slice(&index.to_le_bytes());
    Bytes::from(value)
}

fn bench_settings() -> Settings {
    Settings {
        flush_interval: None,
        compactor_options: None,
        ..Settings::default()
    }
}

async fn build_empty_db() -> Db {
    Db::builder("/bench/db_transaction", Arc::new(InMemory::new()))
        .with_settings(bench_settings())
        .with_merge_operator(concat_merge_operator())
        .build()
        .await
        .expect("failed to build db")
}

async fn build_db() -> Db {
    let db = build_empty_db().await;

    let put_options = put_options();
    let write_options = write_options();
    for index in 0..DB_ENTRY_COUNT {
        db.put_with_options(key(index), value(index), &put_options, &write_options)
            .await
            .expect("populate put failed");
    }

    db
}

fn begin_txn(runtime: &Runtime, db: &Db) -> DbTransaction {
    runtime
        .block_on(db.begin(IsolationLevel::Snapshot))
        .expect("begin transaction failed")
}

fn txn_without_merges(runtime: &Runtime, db: &Db) -> DbTransaction {
    let txn = begin_txn(runtime, db);
    let options = put_options();
    for index in 0..TXN_ENTRY_COUNT {
        let key = key(index);
        if index % 4 == 0 {
            txn.delete(key).expect("delete failed");
        } else {
            txn.put_with_options(key, value(index), &options)
                .expect("put_with_options failed");
        }
    }
    txn
}

fn txn_with_merges(runtime: &Runtime, db: &Db) -> DbTransaction {
    let txn = begin_txn(runtime, db);
    let put_options = put_options();
    let merge_options = merge_options();
    for index in 0..TXN_MERGE_KEY_COUNT {
        let key = key(index);
        txn.put_with_options(key.clone(), value(index), &put_options)
            .expect("put_with_options failed");
        for merge_index in 0..MERGES_PER_KEY {
            txn.merge_with_options(
                &key,
                value(index * MERGES_PER_KEY + merge_index),
                &merge_options,
            )
            .expect("merge_with_options failed");
        }
    }
    txn
}

fn txn_with_repeated_overwrites(runtime: &Runtime, db: &Db) -> DbTransaction {
    let txn = begin_txn(runtime, db);
    let options = put_options();
    let key = Bytes::from_static(b"txn-repeated-overwrite-key");
    for index in 0..TXN_ENTRY_COUNT {
        txn.put_with_options(&key, value(index), &options)
            .expect("put_with_options failed");
    }
    txn
}

fn bench_db_transaction(c: &mut Criterion) {
    let runtime = Runtime::new().expect("failed to create runtime");
    let db = runtime.block_on(build_db());
    let put_options = put_options();
    let merge_options = merge_options();
    let read_options = ReadOptions::default();
    let scan_options = ScanOptions::default();
    let write_options = write_options();
    let read_txn = begin_txn(&runtime, &db);
    let get_key = key(0);
    let scan_start = key(100);
    let scan_end = key(900);
    let scan_prefix = Bytes::from_static(b"txn-key-00000");

    let mut group = c.benchmark_group("db_transaction");
    group.sample_size(100);

    group.bench_function("get_key_value_with_options", |b| {
        b.to_async(&runtime).iter(|| async {
            let result = read_txn
                .get_key_value_with_options(get_key.clone(), &read_options)
                .await
                .expect("get_key_value_with_options failed");
            assert!(result.is_some());
            black_box(result);
        });
    });

    group.bench_function("scan_with_options", |b| {
        b.to_async(&runtime).iter(|| async {
            let mut iter = read_txn
                .scan_with_options(scan_start.clone()..scan_end.clone(), &scan_options)
                .await
                .expect("scan_with_options failed");
            let mut count = 0usize;
            while let Some(kv) = iter.next().await.expect("iterator next failed") {
                count += 1;
                black_box(kv);
            }
            assert!(count > 0);
            black_box(count);
        });
    });

    group.bench_function("scan_prefix_with_options", |b| {
        b.to_async(&runtime).iter(|| async {
            let mut iter = read_txn
                .scan_prefix_with_options(scan_prefix.clone(), .., &scan_options)
                .await
                .expect("scan_prefix_with_options failed");
            let mut count = 0usize;
            while let Some(kv) = iter.next().await.expect("iterator next failed") {
                count += 1;
                black_box(kv);
            }
            assert!(count > 0);
            black_box(count);
        });
    });

    group.bench_function("put_with_options/empty_transaction", |b| {
        b.iter_batched(
            || {
                (
                    begin_txn(&runtime, &db),
                    Bytes::from_static(b"txn_bench_key"),
                    Bytes::from_static(b"txn_bench_value"),
                )
            },
            |(txn, key, value)| {
                txn.put_with_options(key, value, &put_options)
                    .expect("put_with_options failed");
                black_box(txn)
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("put_with_options/new_key_populated", |b| {
        b.iter_batched(
            || {
                (
                    txn_without_merges(&runtime, &db),
                    key(TXN_ENTRY_COUNT + 1),
                    value(TXN_ENTRY_COUNT + 1),
                )
            },
            |(txn, key, value)| {
                txn.put_with_options(key, value, &put_options)
                    .expect("put_with_options failed");
                black_box(txn)
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("put_with_options/overwrite_existing_key", |b| {
        b.iter_batched(
            || {
                (
                    txn_with_merges(&runtime, &db),
                    key(0),
                    value(TXN_MERGE_KEY_COUNT + 1),
                )
            },
            |(txn, key, value)| {
                txn.put_with_options(key, value, &put_options)
                    .expect("put_with_options failed");
                black_box(txn)
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("mark_read", |b| {
        b.iter_batched(
            || {
                (
                    begin_txn(&runtime, &db),
                    Bytes::from_static(b"txn_bench_key"),
                )
            },
            |(txn, key)| {
                txn.mark_read([key]).expect("mark_read failed");
                black_box(txn)
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("unmark_write", |b| {
        b.iter_batched(
            || {
                (
                    begin_txn(&runtime, &db),
                    Bytes::from_static(b"txn_bench_key"),
                )
            },
            |(txn, key)| {
                txn.unmark_write([key]).expect("unmark_write failed");
                black_box(txn)
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("merge_with_options/empty_transaction", |b| {
        b.iter_batched(
            || {
                (
                    begin_txn(&runtime, &db),
                    Bytes::from_static(b"txn_bench_key"),
                    Bytes::from_static(b"txn_bench_value"),
                )
            },
            |(txn, key, value)| {
                txn.merge_with_options(key, value, &merge_options)
                    .expect("merge_with_options failed");
                black_box(txn)
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("merge_with_options/new_key_populated", |b| {
        b.iter_batched(
            || {
                (
                    txn_without_merges(&runtime, &db),
                    key(TXN_ENTRY_COUNT + 1),
                    value(TXN_ENTRY_COUNT + 1),
                )
            },
            |(txn, key, value)| {
                txn.merge_with_options(key, value, &merge_options)
                    .expect("merge_with_options failed");
                black_box(txn)
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("merge_with_options/existing_key_with_merges", |b| {
        b.iter_batched(
            || {
                (
                    txn_with_merges(&runtime, &db),
                    key(0),
                    value(TXN_MERGE_KEY_COUNT + 1),
                )
            },
            |(txn, key, value)| {
                txn.merge_with_options(key, value, &merge_options)
                    .expect("merge_with_options failed");
                black_box(txn)
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("delete/empty_transaction", |b| {
        b.iter_batched(
            || {
                (
                    begin_txn(&runtime, &db),
                    Bytes::from_static(b"txn_bench_key"),
                )
            },
            |(txn, key)| {
                txn.delete(key).expect("delete failed");
                black_box(txn)
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("delete/new_key_populated", |b| {
        b.iter_batched(
            || (txn_without_merges(&runtime, &db), key(TXN_ENTRY_COUNT + 1)),
            |(txn, key)| {
                txn.delete(key).expect("delete failed");
                black_box(txn)
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("delete/existing_key", |b| {
        b.iter_batched(
            || (txn_with_merges(&runtime, &db), key(0)),
            |(txn, key)| {
                txn.delete(key).expect("delete failed");
                black_box(txn)
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("commit_with_options/empty_transaction", |b| {
        b.to_async(&runtime).iter_custom(|iters| {
            let write_options = write_options.clone();
            async move {
                let mut elapsed = Duration::ZERO;
                for _ in 0..iters {
                    let db = build_empty_db().await;
                    let txn = db
                        .begin(IsolationLevel::Snapshot)
                        .await
                        .expect("begin transaction failed");
                    #[allow(clippy::disallowed_types)]
                    let start = Instant::now();
                    let result = txn.commit_with_options(&write_options).await;
                    elapsed += start.elapsed();
                    black_box(result.expect("commit failed"));
                    db.close().await.expect("close failed");
                }
                elapsed
            }
        });
    });

    group.bench_function("commit_with_options/one_put", |b| {
        b.to_async(&runtime).iter_custom(|iters| {
            let put_options = put_options.clone();
            let write_options = write_options.clone();
            async move {
                let mut elapsed = Duration::ZERO;
                for _ in 0..iters {
                    let db = build_empty_db().await;
                    let txn = db
                        .begin(IsolationLevel::Snapshot)
                        .await
                        .expect("begin transaction failed");
                    txn.put_with_options(key(0), value(0), &put_options)
                        .expect("put_with_options failed");
                    #[allow(clippy::disallowed_types)]
                    let start = Instant::now();
                    let result = txn.commit_with_options(&write_options).await;
                    elapsed += start.elapsed();
                    black_box(result.expect("commit failed"));
                    db.close().await.expect("close failed");
                }
                elapsed
            }
        });
    });

    group.bench_function("drop/no_merges", |b| {
        b.iter_batched(
            || txn_without_merges(&runtime, &db),
            |txn| drop(black_box(txn)),
            BatchSize::LargeInput,
        );
    });

    group.bench_function("drop/with_merges", |b| {
        b.iter_batched(
            || txn_with_merges(&runtime, &db),
            |txn| drop(black_box(txn)),
            BatchSize::LargeInput,
        );
    });

    group.bench_function("drop/repeated_overwrites", |b| {
        b.iter_batched(
            || txn_with_repeated_overwrites(&runtime, &db),
            |txn| drop(black_box(txn)),
            BatchSize::LargeInput,
        );
    });

    group.finish();

    drop(read_txn);
    runtime.block_on(db.close()).expect("close failed");
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .with_profiler(PProfProfiler::new(100, Output::Protobuf));
    targets = bench_db_transaction
}

criterion_main!(benches);
