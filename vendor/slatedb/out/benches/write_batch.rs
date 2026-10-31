// our microbenchmarks use pprof, but it doesn't work on windows
#![cfg(not(windows))]

// Run with: cargo bench -p slatedb --features bench-internal --bench write_batch

use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use slatedb::config::{MergeOptions, PutOptions, Ttl};
use slatedb::{write_batch_benches, MergeOperator, MergeOperatorError, WriteBatch};
use std::sync::{Arc, OnceLock};
use tokio::runtime::Runtime;

const VALUE_SIZE: usize = 128;
const EXTRACT_ENTRY_COUNT: usize = 1_024;
const MERGE_BATCH_SCENARIOS: &[(usize, usize)] = &[
    // Each scenario is (key_count, merges_per_key).
    (256, 3),
    (256, 100),
    (16, 1_000),
    (1, 1_000),
    (1, 10_000),
];

struct SizeSumMergeOperator;

impl MergeOperator for SizeSumMergeOperator {
    fn merge(
        &self,
        _key: &Bytes,
        existing_value: Option<Bytes>,
        operand: Bytes,
    ) -> Result<Bytes, MergeOperatorError> {
        let size = existing_value.as_ref().map_or(0, Bytes::len) + operand.len();
        Ok(Bytes::copy_from_slice(&(size as u64).to_le_bytes()))
    }

    fn merge_batch(
        &self,
        _key: &Bytes,
        existing_value: Option<Bytes>,
        operands: &[Bytes],
    ) -> Result<Bytes, MergeOperatorError> {
        let size = existing_value.as_ref().map_or(0, Bytes::len)
            + operands.iter().map(Bytes::len).sum::<usize>();
        Ok(Bytes::copy_from_slice(&(size as u64).to_le_bytes()))
    }
}

fn size_sum_merge_operator() -> Arc<dyn MergeOperator + Send + Sync> {
    static MERGE_OPERATOR: OnceLock<Arc<dyn MergeOperator + Send + Sync>> = OnceLock::new();
    MERGE_OPERATOR
        .get_or_init(|| Arc::new(SizeSumMergeOperator))
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

fn key(index: usize) -> Bytes {
    Bytes::from(format!("key-{index:08}"))
}

fn value(index: usize) -> Bytes {
    let mut value = vec![0; VALUE_SIZE];
    value[..size_of::<usize>()].copy_from_slice(&index.to_le_bytes());
    Bytes::from(value)
}

fn make_batch_without_merges() -> WriteBatch {
    let mut batch = WriteBatch::new();
    let options = put_options();
    for index in 0..EXTRACT_ENTRY_COUNT {
        let key = key(index);
        if index % 4 == 0 {
            batch.delete(key);
        } else {
            batch.put_bytes_with_options(key, value(index), &options);
        }
    }
    batch
}

fn merge_batch_scenario_id(key_count: usize, merges_per_key: usize) -> String {
    format!("{key_count}_keys/{merges_per_key}_merges_per_key")
}

fn make_batch_with_merges(key_count: usize, merges_per_key: usize) -> WriteBatch {
    let mut batch = WriteBatch::new();
    let put_options = put_options();
    let merge_options = merge_options();
    for index in 0..key_count {
        let key = key(index);
        batch.put_bytes_with_options(key.clone(), value(index), &put_options);
        for merge_index in 0..merges_per_key {
            batch.merge_with_options(
                &key,
                value(index * merges_per_key + merge_index),
                &merge_options,
            );
        }
    }
    batch
}

fn make_batch_with_repeated_overwrites() -> WriteBatch {
    let mut batch = WriteBatch::new();
    let options = put_options();
    let key = Bytes::from_static(b"repeated-overwrite-key");
    for index in 0..EXTRACT_ENTRY_COUNT {
        batch.put_bytes_with_options(key.clone(), value(index), &options);
    }
    batch
}

fn bench_write_batch(c: &mut Criterion) {
    let runtime = Runtime::new().expect("failed to create runtime");
    let put_options = put_options();
    let merge_options = merge_options();
    let batch_without_merges = make_batch_without_merges();
    let batches_with_merges: Vec<_> = MERGE_BATCH_SCENARIOS
        .iter()
        .copied()
        .map(|(key_count, merges_per_key)| {
            (
                key_count,
                merges_per_key,
                make_batch_with_merges(key_count, merges_per_key),
            )
        })
        .collect();
    let batch_with_repeated_overwrites = make_batch_with_repeated_overwrites();

    let mut group = c.benchmark_group("write_batch");
    group.sample_size(1_000);

    group.bench_function("put_bytes_with_options/empty_batch", |b| {
        b.iter_batched(
            || {
                (
                    WriteBatch::new(),
                    Bytes::from_static(b"write_batch_key"),
                    Bytes::from_static(b"write_batch_value"),
                )
            },
            |(mut batch, key, value)| {
                batch.put_bytes_with_options(key, value, &put_options);
                black_box(batch)
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("put_bytes_with_options/new_key_populated", |b| {
        b.iter_batched(
            || {
                (
                    batch_without_merges.clone(),
                    key(EXTRACT_ENTRY_COUNT + 1),
                    value(EXTRACT_ENTRY_COUNT + 1),
                )
            },
            |(mut batch, key, value)| {
                batch.put_bytes_with_options(key, value, &put_options);
                black_box(batch)
            },
            BatchSize::LargeInput,
        );
    });

    for (key_count, merges_per_key, batch_with_merges) in &batches_with_merges {
        let key_count = *key_count;
        let merges_per_key = *merges_per_key;
        let scenario_id = merge_batch_scenario_id(key_count, merges_per_key);

        group.bench_with_input(
            BenchmarkId::new(
                "put_bytes_with_options/overwrite_existing_key",
                scenario_id.clone(),
            ),
            batch_with_merges,
            |b, batch_with_merges| {
                b.iter_batched(
                    || {
                        (
                            batch_with_merges.clone(),
                            key(0),
                            value(key_count * merges_per_key + 1),
                        )
                    },
                    |(mut batch, key, value)| {
                        batch.put_bytes_with_options(key, value, &put_options);
                        black_box(batch)
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.bench_function("merge_with_options/empty_batch", |b| {
        b.iter_batched(
            || {
                (
                    WriteBatch::new(),
                    Bytes::from_static(b"write_batch_key"),
                    Bytes::from_static(b"write_batch_value"),
                )
            },
            |(mut batch, key, value)| {
                batch.merge_with_options(key, value, &merge_options);
                black_box(batch)
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("merge_with_options/new_key_populated", |b| {
        b.iter_batched(
            || {
                (
                    batch_without_merges.clone(),
                    key(EXTRACT_ENTRY_COUNT + 1),
                    value(EXTRACT_ENTRY_COUNT + 1),
                )
            },
            |(mut batch, key, value)| {
                batch.merge_with_options(key, value, &merge_options);
                black_box(batch)
            },
            BatchSize::LargeInput,
        );
    });

    for (key_count, merges_per_key, batch_with_merges) in &batches_with_merges {
        let key_count = *key_count;
        let merges_per_key = *merges_per_key;
        let scenario_id = merge_batch_scenario_id(key_count, merges_per_key);

        group.bench_with_input(
            BenchmarkId::new(
                "merge_with_options/existing_key_with_merges",
                scenario_id.clone(),
            ),
            batch_with_merges,
            |b, batch_with_merges| {
                b.iter_batched(
                    || {
                        (
                            batch_with_merges.clone(),
                            key(0),
                            value(key_count * merges_per_key + 1),
                        )
                    },
                    |(mut batch, key, value)| {
                        batch.merge_with_options(key, value, &merge_options);
                        black_box(batch)
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.bench_function("delete/empty_batch", |b| {
        b.iter_batched(
            || (WriteBatch::new(), Bytes::from_static(b"write_batch_key")),
            |(mut batch, key)| {
                batch.delete(key);
                black_box(batch)
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("delete/new_key_populated", |b| {
        b.iter_batched(
            || (batch_without_merges.clone(), key(EXTRACT_ENTRY_COUNT + 1)),
            |(mut batch, key)| {
                batch.delete(key);
                black_box(batch)
            },
            BatchSize::LargeInput,
        );
    });

    for (key_count, merges_per_key, batch_with_merges) in &batches_with_merges {
        let key_count = *key_count;
        let merges_per_key = *merges_per_key;
        let scenario_id = merge_batch_scenario_id(key_count, merges_per_key);

        group.bench_with_input(
            BenchmarkId::new("delete/existing_key", scenario_id),
            batch_with_merges,
            |b, batch_with_merges| {
                b.iter_batched(
                    || (batch_with_merges.clone(), key(0)),
                    |(mut batch, key)| {
                        batch.delete(key);
                        black_box(batch)
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.bench_function("keys", |b| {
        b.iter_batched(
            || batch_without_merges.clone(),
            |batch| black_box(write_batch_benches::keys(&batch)),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("extract_entries/no_merges", |b| {
        b.to_async(&runtime).iter_batched(
            || batch_without_merges.clone(),
            |batch| async move {
                black_box(
                    write_batch_benches::extract_entries(&batch, 100, 1_000, None, None, None)
                        .await
                        .expect("extract_entries failed"),
                )
            },
            BatchSize::SmallInput,
        );
    });

    for (key_count, merges_per_key, batch_with_merges) in &batches_with_merges {
        let key_count = *key_count;
        let merges_per_key = *merges_per_key;
        let scenario_id = merge_batch_scenario_id(key_count, merges_per_key);

        group.bench_with_input(
            BenchmarkId::new("extract_entries/with_merges", scenario_id.clone()),
            batch_with_merges,
            |b, batch_with_merges| {
                b.to_async(&runtime).iter_batched(
                    || batch_with_merges.clone(),
                    |batch| async move {
                        black_box(
                            write_batch_benches::extract_entries(
                                &batch,
                                100,
                                1_000,
                                None,
                                Some(size_sum_merge_operator()),
                                None,
                            )
                            .await
                            .expect("extract_entries failed"),
                        )
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.bench_function("drop/no_merges", |b| {
        b.iter_batched(
            || batch_without_merges.clone(),
            |batch| drop(black_box(batch)),
            BatchSize::LargeInput,
        );
    });

    for (key_count, merges_per_key, batch_with_merges) in &batches_with_merges {
        let key_count = *key_count;
        let merges_per_key = *merges_per_key;
        let scenario_id = merge_batch_scenario_id(key_count, merges_per_key);

        group.bench_with_input(
            BenchmarkId::new("drop/with_merges", scenario_id.clone()),
            batch_with_merges,
            |b, batch_with_merges| {
                b.iter_batched(
                    || batch_with_merges.clone(),
                    |batch| drop(black_box(batch)),
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.bench_function("drop/repeated_overwrites", |b| {
        b.iter_batched(
            || batch_with_repeated_overwrites.clone(),
            |batch| drop(black_box(batch)),
            BatchSize::LargeInput,
        );
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .with_profiler(PProfProfiler::new(100, Output::Protobuf));
    targets = bench_write_batch
}

criterion_main!(benches);
