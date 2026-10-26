// our microbenchmarks use pprof, but it doesn't work on windows
#![cfg(not(windows))]

use criterion::{criterion_group, criterion_main, Criterion};
use object_store::memory::InMemory;
use pprof::criterion::{Output, PProfProfiler};
use slatedb::config::{PutOptions, WriteOptions};
use slatedb::Db;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn criterion_benchmark(c: &mut Criterion) {
    let object_store = Arc::new(InMemory::new());

    let runtime = Runtime::new().expect("Failed to create Tokio runtime");
    let db = Db::open("/tmp/test_kv_store", object_store);
    #[allow(clippy::disallowed_methods)]
    let db = runtime.block_on(db).unwrap();
    c.bench_function("put", |b| {
        b.to_async(&runtime).iter(|| async {
            let key = b"key";
            let value = b"value";
            db.put_with_options(
                key,
                value,
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .expect("put failed");
        })
    });
    c.bench_function("open_close", |b| {
        b.to_async(&runtime).iter(|| async {
            let db = Db::open("/tmp/test_kv_store", Arc::new(InMemory::new()))
                .await
                .expect("open failed");
            db.close().await.expect("close failed");
        })
    });
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
