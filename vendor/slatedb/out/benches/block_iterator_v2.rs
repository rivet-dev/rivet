// our microbenchmarks use pprof, but it doesn't work on windows
#![cfg(not(windows))]

// Run with: cargo bench --features bench-internal --bench block_iterator_v2
// The `bench-internal` feature gates `slatedb::block_iterator_v2_bench`. That
// pub fn builds the block and yields iteration as a closure so this bench
// doesn't need crate-private items.

use criterion::{criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use slatedb::block_iterator_v2_benches::BlockIteratorV2BenchConfig;

#[allow(clippy::redundant_closure)]
fn criterion_benchmark(c: &mut Criterion) {
    // Small values: pool of 16B-key/512B-value entries filling a 4KB block.
    slatedb::block_iterator_v2_benches::block_iterator_v2_bench(
        BlockIteratorV2BenchConfig {
            block_size: 4096,
            key_size: 16,
            value_size: 512,
            num_entries: 32,
        },
        |inner| {
            c.bench_function("block_iterator_v2_iterate_small_values", |b| {
                b.iter(|| inner());
            });
        },
    );

    // Single 100KB value in a nominally 4KB block. The builder's
    // oversized-entry rule accepts the first entry unconditionally, so the
    // block data expands to ~100KB with exactly one row.
    slatedb::block_iterator_v2_benches::block_iterator_v2_bench(
        BlockIteratorV2BenchConfig {
            block_size: 4096,
            key_size: 16,
            value_size: 100 * 1024,
            num_entries: 1,
        },
        |inner| {
            c.bench_function("block_iterator_v2_iterate_large_value", |b| {
                b.iter(|| inner());
            });
        },
    );
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(1_000)
        // This only runs when `--profile-time <num_seconds>` is set
        .with_profiler(PProfProfiler::new(100, Output::Protobuf));
    targets = criterion_benchmark
}

criterion_main!(benches);
