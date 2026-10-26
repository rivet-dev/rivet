//! Focused integration regressions for the external leadership epoch patch.
use std::num::NonZeroU64;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use slatedb::config::{CloseOptions, DurabilityLevel, ReadOptions};
use slatedb::fail_parallel::{self, FailPointRegistry};
use slatedb::object_store::prefix::PrefixStore;
use slatedb::object_store::{memory::InMemory, ObjectStore};
use slatedb::{CloseReason, Db, ErrorKind, Settings};
use tokio::sync::Notify;
use tokio::time::timeout;

const LIMIT: Duration = Duration::from_secs(15);

fn settings() -> Settings {
    Settings {
        flush_interval: None,
        manifest_poll_interval: Duration::from_secs(60),
        garbage_collector_options: None,
        object_store_max_retries: Some(0),
        ..Settings::default()
    }
}

async fn open(store: Arc<dyn ObjectStore>, epoch: u64) -> Db {
    Db::builder("leadership", store)
        .with_settings(settings())
        .with_leadership_epoch(NonZeroU64::new(epoch).unwrap())
        .build()
        .await
        .unwrap()
}

async fn durable_put(db: &Db, key: &[u8], value: &[u8]) {
    let write = db.put(key, value).await.unwrap();
    db.flush().await.unwrap();
    write.await_durable().await.unwrap();
}

async fn durable_get(db: &Db, key: &[u8]) -> Option<bytes::Bytes> {
    db.get_with_options(
        key,
        &ReadOptions::new().with_durability_filter(DurabilityLevel::Remote),
    )
    .await
    .unwrap()
}

/// A callback barrier at an actual SlateDB initialization/flush boundary.
/// Each test has a separate registry and enough runtime threads for the paused
/// native callback. The owning guard always releases it, including on failure.
struct Barrier {
    registry: Arc<FailPointRegistry>,
    name: &'static str,
    entered: Arc<Notify>,
    gate: Arc<(Mutex<bool>, Condvar)>,
}

impl Barrier {
    fn new(registry: Arc<FailPointRegistry>, name: &'static str) -> Self {
        let entered = Arc::new(Notify::new());
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let callback_entered = entered.clone();
        let callback_gate = gate.clone();
        fail_parallel::cfg_callback(registry.clone(), name, move || {
            callback_entered.notify_one();
            let (lock, cv) = &*callback_gate;
            let (released, _) = cv
                .wait_timeout_while(lock.lock().unwrap(), LIMIT, |released| !*released)
                .unwrap();
            assert!(*released, "test did not release native barrier {name}");
        })
        .unwrap();
        Self {
            registry,
            name,
            entered,
            gate,
        }
    }

    async fn wait(&self) {
        timeout(LIMIT, self.entered.notified()).await.unwrap();
    }

    fn release(&self) {
        *self.gate.0.lock().unwrap() = true;
        self.gate.1.notify_all();
        fail_parallel::remove(self.registry.clone(), self.name);
    }
}

impl Drop for Barrier {
    fn drop(&mut self) {
        self.release();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn supplied_epoch_rejects_equal_and_stale_with_compaction_enabled() {
    let store = Arc::new(InMemory::new());
    let db = open(store.clone(), 7).await;
    durable_put(&db, b"key", b"durable").await;
    db.refresh_manifest().await.unwrap();
    assert_eq!(db.status().current_manifest.writer_epoch(), 7);
    assert_eq!(db.status().current_manifest.compactor_epoch(), 7);
    for epoch in [6, 7] {
        let error = Db::builder("leadership", store.clone())
            .with_settings(settings())
            .with_leadership_epoch(NonZeroU64::new(epoch).unwrap())
            .build()
            .await
            .err()
            .expect("stale/equal open must fail");
        assert_eq!(error.kind(), ErrorKind::Closed(CloseReason::Fenced));
    }
    durable_put(&db, b"still-live", b"yes").await;
    db.close().await.unwrap();
    let next = open(store, 9).await;
    assert_eq!(durable_get(&next, b"key").await.unwrap(), b"durable"[..]);
    next.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delayed_old_manifest_open_cannot_fence_successor() {
    let store = Arc::new(InMemory::new());
    let registry = Arc::new(FailPointRegistry::new());
    let barrier = Barrier::new(registry.clone(), "leadership-before-writer-fence");
    let a_store = store.clone();
    let old = tokio::spawn(async move {
        Db::builder("leadership", a_store)
            .with_settings(settings())
            .with_leadership_epoch(NonZeroU64::new(1).unwrap())
            .with_fp_registry(registry)
            .build()
            .await
    });
    barrier.wait().await;
    let current = open(store, 2).await;
    durable_put(&current, b"current", b"value").await;
    barrier.release();
    let err = timeout(LIMIT, old).await.unwrap().unwrap().err().unwrap();
    assert_eq!(err.kind(), ErrorKind::Closed(CloseReason::Fenced));
    durable_put(&current, b"after", b"safe").await;
    current.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delayed_old_compactor_open_cannot_fence_successor() {
    let store = Arc::new(InMemory::new());
    let registry = Arc::new(FailPointRegistry::new());
    let barrier = Barrier::new(registry.clone(), "leadership-before-compactor");
    let a_store = store.clone();
    let old = tokio::spawn(async move {
        Db::builder("leadership", a_store)
            .with_settings(settings())
            .with_leadership_epoch(NonZeroU64::new(1).unwrap())
            .with_fp_registry(registry)
            .build()
            .await
    });
    barrier.wait().await;
    let current = open(store, 2).await;
    durable_put(&current, b"current", b"value").await;
    barrier.release();
    let err = timeout(LIMIT, old).await.unwrap().unwrap().err().unwrap();
    assert_eq!(err.kind(), ErrorKind::Closed(CloseReason::Fenced));
    current.refresh_manifest().await.unwrap();
    assert_eq!(current.status().current_manifest.compactor_epoch(), 2);
    durable_put(&current, b"after", b"safe").await;
    current.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn wal_put_between_manifest_and_wal_fence_is_recovered() {
    let store = Arc::new(InMemory::new());
    let old = open(store.clone(), 1).await;
    let pending = old.put(b"racing", b"included").await.unwrap();
    let registry = Arc::new(FailPointRegistry::new());
    let barrier = Barrier::new(registry.clone(), "leadership-after-manifest");
    let successor = tokio::spawn(async move {
        Db::builder("leadership", store)
            .with_settings(settings())
            .with_leadership_epoch(NonZeroU64::new(2).unwrap())
            .with_fp_registry(registry)
            .build()
            .await
    });
    barrier.wait().await;
    // The new manifest alone does not fence the independent WAL stream.
    old.flush().await.unwrap();
    pending.await_durable().await.unwrap();
    barrier.release();
    let current = timeout(LIMIT, successor).await.unwrap().unwrap().unwrap();
    assert_eq!(
        durable_get(&current, b"racing").await.unwrap(),
        b"included"[..]
    );
    let _ = old.close().await;
    current.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pending_old_close_and_new_open_preserve_acknowledged_data() {
    let store = Arc::new(InMemory::new());
    let registry = Arc::new(FailPointRegistry::new());
    let old = Arc::new(
        Db::builder("leadership", store.clone())
            .with_settings(settings())
            .with_leadership_epoch(NonZeroU64::new(1).unwrap())
            .with_fp_registry(registry.clone())
            .build()
            .await
            .unwrap(),
    );
    durable_put(&old, b"acked", b"kept").await;
    let barrier = Barrier::new(registry, "flush-memtable-to-l0");
    let closing = {
        let db = old.clone();
        tokio::spawn(async move { db.close().await })
    };
    barrier.wait().await;
    assert!(!closing.is_finished());
    let current = timeout(LIMIT, open(store, 2)).await.unwrap();
    assert_eq!(durable_get(&current, b"acked").await.unwrap(), b"kept"[..]);
    barrier.release();
    // A fenced cleanup is allowed to return an error; it is never evidence of
    // graceful release at the leadership adapter boundary.
    let _ = timeout(LIMIT, closing).await.unwrap().unwrap();
    durable_put(&current, b"after-close", b"works").await;
    current.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn failed_open_drops_store_before_and_after_monitor_start() {
    for hook in ["leadership-before-compactor", "leadership-before-replay"] {
        for _ in 0..3 {
            let store = Arc::new(InMemory::new());
            let weak = Arc::downgrade(&store);
            let registry = Arc::new(FailPointRegistry::new());
            fail_parallel::cfg(registry.clone(), hook, "return").unwrap();
            let result = timeout(
                LIMIT,
                Db::builder("leadership", store)
                    .with_settings(settings())
                    .with_leadership_epoch(NonZeroU64::new(1).unwrap())
                    .with_fp_registry(registry)
                    .build(),
            )
            .await
            .unwrap();
            assert!(result.is_err());
            assert!(
                weak.upgrade().is_none(),
                "failed open retained store at {hook}"
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delayed_replay_is_owned_through_late_completion_and_cleanup() {
    let store = Arc::new(InMemory::new());
    let first = open(store.clone(), 1).await;
    durable_put(&first, b"acked", b"kept").await;
    first
        .close_with_options(CloseOptions::default().with_flush_type(None))
        .await
        .unwrap();
    drop(first);

    // This distinct wrapper shares the same backend but lets the test observe
    // when all resources from just the delayed opener have been released.
    let delayed_store = Arc::new(PrefixStore::new(store.clone(), ""));
    let weak = Arc::downgrade(&delayed_store);
    let registry = Arc::new(FailPointRegistry::new());
    let barrier = Barrier::new(registry.clone(), "leadership-before-replay");
    let mut opening = tokio::spawn(async move {
        Db::builder("leadership", delayed_store)
            .with_settings(settings())
            .with_leadership_epoch(NonZeroU64::new(2).unwrap())
            .with_fp_registry(registry)
            .build()
            .await
    });
    barrier.wait().await;
    // A timeout borrows the retained task rather than cancelling its build.
    assert!(timeout(Duration::from_millis(1), &mut opening)
        .await
        .is_err());
    assert!(weak.upgrade().is_some());
    let current = open(store, 3).await;
    assert_eq!(durable_get(&current, b"acked").await.unwrap(), b"kept"[..]);
    barrier.release();
    // Native open can discover fencing or late-complete successfully. The
    // adapter must retain either outcome and close a late successful handle.
    if let Ok(late_db) = timeout(LIMIT, opening).await.unwrap().unwrap() {
        let _ = late_db.close().await;
    }
    assert!(weak.upgrade().is_none());
    durable_put(&current, b"after-replay", b"works").await;
    current.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ordinary_builder_still_allocates_epochs_and_recovers() {
    let store = Arc::new(InMemory::new());
    let first = Db::builder("leadership", store.clone())
        .with_settings(settings())
        .build()
        .await
        .unwrap();
    durable_put(&first, b"ordinary", b"value").await;
    first
        .close_with_options(CloseOptions::default().with_flush_type(None))
        .await
        .unwrap();
    let second = Db::builder("leadership", store)
        .with_settings(settings())
        .build()
        .await
        .unwrap();
    second.refresh_manifest().await.unwrap();
    assert_eq!(second.status().current_manifest.writer_epoch(), 2);
    assert_eq!(second.status().current_manifest.compactor_epoch(), 2);
    assert_eq!(
        durable_get(&second, b"ordinary").await.unwrap(),
        b"value"[..]
    );
    second.close().await.unwrap();
}
