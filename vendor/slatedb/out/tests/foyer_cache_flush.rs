//! End-to-end tests for per-`Db` block-cache flushing
//! ([`DbCacheManagerOps::flush_cache_to_disk`]) against a real
//! `foyer` hybrid (memory + disk) cache.
//!
//! These exercise the core flow: on "task stop" a `Db` spills its resident
//! cache entries to the disk tier and evicts them from the memory tier of a
//! cache that may be shared with other `Db`s, so (a) a later reader can
//! warm-start from disk instead of object storage, and (b) the freed memory
//! tier capacity is immediately available to other tasks sharing the cache.
//!
//! `db_cache_id` is caller-supplied (not derived by SlateDB — see
//! `DbBuilder::with_db_cache`), so every test that shares a cache across
//! multiple `Db`s picks its own ids and passes the same one across a
//! simulated task restart, exactly as an embedder with its own durable
//! per-task identity would.
//!
//! [`CountingObjectStore`] lets the tests observe *without* reaching into
//! cache internals whether a read was served from the cache (no GET) or fell
//! through to object storage (a GET), which is the externally-observable
//! signal the whole feature is trying to produce.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use foyer::{
    BlockEngineConfig, DeviceBuilder, FsDeviceBuilder, HybridCacheBuilder, PsyncIoEngineConfig,
};
use futures::stream::BoxStream;
use object_store::memory::InMemory;
use object_store::path::Path;
use object_store::{
    CopyOptions, GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore,
    PutMultipartOptions, PutOptions, PutPayload, PutResult, RenameOptions,
};
use slatedb::config::{FlushOptions, FlushType, Settings, WriteOptions};
use slatedb::db_cache::foyer_hybrid::FoyerHybridCache;
use slatedb::db_cache::{CacheTarget, CachedEntry, DbCache};
use slatedb::{BlockCachePolicy, Db, DbCacheManagerOps, DbReader};
use tempfile::TempDir;

/// Wraps an [`ObjectStore`] and counts `get_opts` calls against compacted SST
/// files (the primitive every read of SST data/index/filter/stats bytes
/// funnels through). Used to assert, from outside the crate, whether a read
/// was served from cache or fell through to the backing store.
///
/// Deliberately excludes manifest/GC/compaction-job housekeeping paths:
/// `Db::builder` starts background GC and compactor tasks that poll those
/// paths on their own timers, independent of any block-cache behavior, so
/// counting them would make assertions flaky against unrelated background
/// chatter. Counting only compacted-SST GETs isolates the actual signal this
/// test suite cares about — did a `Db::get` need to touch object storage.
#[derive(Debug)]
struct CountingObjectStore {
    inner: Arc<dyn ObjectStore>,
    compacted_sst_gets: AtomicUsize,
}

impl CountingObjectStore {
    fn new(inner: Arc<dyn ObjectStore>) -> Self {
        Self {
            inner,
            compacted_sst_gets: AtomicUsize::new(0),
        }
    }

    fn compacted_sst_get_count(&self) -> usize {
        self.compacted_sst_gets.load(Ordering::SeqCst)
    }
}

impl std::fmt::Display for CountingObjectStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CountingObjectStore({})", self.inner)
    }
}

#[async_trait]
impl ObjectStore for CountingObjectStore {
    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> object_store::Result<PutResult> {
        self.inner.put_opts(location, payload, opts).await
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOptions,
    ) -> object_store::Result<Box<dyn MultipartUpload>> {
        self.inner.put_multipart_opts(location, opts).await
    }

    async fn get_opts(
        &self,
        location: &Path,
        options: GetOptions,
    ) -> object_store::Result<GetResult> {
        if location.as_ref().contains("/compacted/") {
            self.compacted_sst_gets.fetch_add(1, Ordering::SeqCst);
        }
        self.inner.get_opts(location, options).await
    }

    fn delete_stream(
        &self,
        locations: BoxStream<'static, object_store::Result<Path>>,
    ) -> BoxStream<'static, object_store::Result<Path>> {
        self.inner.delete_stream(locations)
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        self.inner.list(prefix)
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        self.inner.list_with_offset(prefix, offset)
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> object_store::Result<ListResult> {
        self.inner.list_with_delimiter(prefix).await
    }

    async fn copy_opts(
        &self,
        from: &Path,
        to: &Path,
        options: CopyOptions,
    ) -> object_store::Result<()> {
        self.inner.copy_opts(from, to, options).await
    }

    async fn rename_opts(
        &self,
        from: &Path,
        to: &Path,
        options: RenameOptions,
    ) -> object_store::Result<()> {
        self.inner.rename_opts(from, to, options).await
    }
}

/// Builds a real foyer hybrid (memory + disk) cache backed by a temp dir, plus
/// a handle to the raw `foyer::HybridCache` for test-side introspection
/// (`memory().usage()`) that isn't reachable through the `DbCache` trait.
async fn open_hybrid_cache(
    dir: &std::path::Path,
) -> (
    Arc<dyn DbCache>,
    foyer::HybridCache<slatedb::db_cache::CachedKey, CachedEntry>,
) {
    let raw = HybridCacheBuilder::new()
        .with_name("evacuation_test")
        .memory(16 * 1024 * 1024)
        .with_weighter(|_, v: &CachedEntry| v.size())
        .storage()
        .with_io_engine_config(PsyncIoEngineConfig::new())
        .with_engine_config(
            BlockEngineConfig::new(
                FsDeviceBuilder::new(dir)
                    .with_capacity(64 * 1024 * 1024)
                    .build()
                    .expect("failed to build fs device"),
            )
            .with_block_size(64 * 1024),
        )
        .build()
        .await
        .expect("failed to build hybrid cache");
    let cache: Arc<dyn DbCache> = Arc::new(FoyerHybridCache::new_with_cache(raw.clone()));
    (cache, raw)
}

async fn write_keys(db: &Db, count: usize) {
    // Padded values so the SST spans multiple blocks; a single tiny block
    // would make "some blocks warm, others cold" indistinguishable.
    let padding = vec![b'x'; 512];
    for i in 0..count {
        let key = format!("key{:06}", i);
        let mut value = format!("value{:06}", i).into_bytes();
        value.extend_from_slice(&padding);
        db.put_with_options(
            key.as_bytes(),
            &value,
            &Default::default(),
            &WriteOptions::default(),
        )
        .await
        .expect("put failed");
    }
}

async fn read_keys(db: &Db, count: usize) {
    for i in 0..count {
        let key = format!("key{:06}", i);
        let value = db.get(key.as_bytes()).await.expect("get failed");
        assert!(value.is_some(), "expected key{:06} to be present", i);
    }
}

fn no_periodic_flush() -> Settings {
    Settings {
        flush_interval: None,
        ..Default::default()
    }
}

/// Core flush flow, all within one long-lived `Db` instance (the common case
/// of a process that keeps running while individual tasks/keyspaces come and
/// go isn't relevant here — this is the simplest possible case, one `Db`,
/// spilled and re-read without ever closing).
#[tokio::test]
async fn flush_frees_memory_and_serves_from_disk_without_new_gets() {
    let disk_dir = TempDir::new().expect("failed to create temp dir");
    let (cache, raw_cache) = open_hybrid_cache(disk_dir.path()).await;

    let object_store = Arc::new(CountingObjectStore::new(Arc::new(InMemory::new())));
    let db = Db::builder("/evac-test", object_store.clone() as Arc<dyn ObjectStore>)
        .with_settings(no_periodic_flush())
        .with_db_cache(cache, 1)
        .build()
        .await
        .expect("failed to open db");

    const N: usize = 200;
    write_keys(&db, N).await;
    db.flush_with_options(FlushOptions {
        flush_type: FlushType::MemTable,
    })
    .await
    .expect("flush failed");

    // Warm every block, plus index/filters/stats, for every SST reachable
    // from the manifest so evacuation has a known-full set to work with.
    let manifest = db.manifest();
    for view in manifest.l0().iter() {
        db.warm_sst(
            view.sst.id,
            &[
                CacheTarget::data::<&[u8], _>(..),
                CacheTarget::Index,
                CacheTarget::Filters,
                CacheTarget::Stats,
            ],
        )
        .await
        .expect("warm_sst failed");
    }

    let mem_usage_before = raw_cache.memory().usage();
    assert!(
        mem_usage_before > 0,
        "expected warmed blocks to be resident in the memory tier"
    );

    // Reading again right now must be pure cache hits: establish a GET-count
    // baseline before evacuation touches anything.
    read_keys(&db, N).await;
    let gets_after_warm_reads = object_store.compacted_sst_get_count();

    // when: flush (this Db's simulated "task stop")
    db.flush_cache_to_disk()
        .await
        .expect("flush_cache_to_disk failed");

    // then: the memory tier footprint for this Db is gone...
    let mem_usage_after = raw_cache.memory().usage();
    assert_eq!(
        mem_usage_after, 0,
        "expected the flush to free the entire memory tier for this Db's scope"
    );

    // ...and reads for the same keys are still served without touching
    // object storage, because the data is now warm on the disk tier.
    read_keys(&db, N).await;
    assert_eq!(
        object_store.compacted_sst_get_count(),
        gets_after_warm_reads,
        "expected reads after the flush to hit the foyer disk tier, not object storage"
    );

    db.close().await.expect("close failed");
}

/// Flushing with no resident entries (nothing was ever read/warmed, and
/// flush/compaction insertion is disabled) must be a harmless no-op: no
/// panics, no spurious disk entries, cache stays empty.
#[tokio::test]
async fn flush_is_a_noop_with_nothing_resident() {
    let disk_dir = TempDir::new().expect("failed to create temp dir");
    let (cache, raw_cache) = open_hybrid_cache(disk_dir.path()).await;

    let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    let db = Db::builder("/evac-empty-test", object_store)
        .with_settings(no_periodic_flush())
        // The default policy caches flush output; disable it so this test's
        // starting point is actually an empty cache.
        .with_block_cache_policy(BlockCachePolicy::default().with_flush_targets(&[]))
        .with_db_cache(cache, 1)
        .build()
        .await
        .expect("failed to open db");

    write_keys(&db, 8).await;
    db.flush_with_options(FlushOptions {
        flush_type: FlushType::MemTable,
    })
    .await
    .expect("flush failed");

    // Nothing was read/warmed, so nothing should be resident.
    assert_eq!(raw_cache.memory().usage(), 0);

    db.flush_cache_to_disk()
        .await
        .expect("flush_cache_to_disk failed on an empty cache");

    assert_eq!(raw_cache.memory().usage(), 0);

    db.close().await.expect("close failed");
}

/// Two `Db`s sharing one cache with different caller-supplied `db_cache_id`s:
/// closing (and flushing) one must not disturb the other's resident entries,
/// and only the closed one's entries should have actually been flushed to
/// disk (acceptance criterion: scope isolation holds).
///
/// The two claims are checked the same way the other tests establish "served
/// from disk, not object storage" — by GET count, not by reaching into cache
/// internals — because `CachedKey`'s `db_cache_id`/`sst_id`/`block_id` fields
/// are private, so an external test cannot construct db_a's or db_b's scoped
/// key to probe the disk tier directly:
/// - db_b (left running, never flushed) is re-read *while still open* and
///   must cause zero new GETs — it was never touched, so it's still served
///   entirely from the memory tier.
/// - db_a is flushed, closed, and reopened as a new instance at the same path
///   *with the same db_cache_id* (simulating a caller reusing its own durable
///   per-task identity across a restart); its data is re-read and
///   must also cause zero new GETs — this time because it recovers its scope
///   and hits the disk tier, not because it was still resident in memory.
#[tokio::test]
async fn flush_and_close_one_db_does_not_disturb_a_sibling_on_the_same_cache() {
    let disk_dir = TempDir::new().expect("failed to create temp dir");
    let (cache, raw_cache) = open_hybrid_cache(disk_dir.path()).await;

    let object_store = Arc::new(CountingObjectStore::new(Arc::new(InMemory::new())));
    const PATH_A: &str = "/evac-scope-a";
    const PATH_B: &str = "/evac-scope-b";
    const SCOPE_A: u64 = 1;
    const SCOPE_B: u64 = 2;

    let db_a = Db::builder(PATH_A, object_store.clone() as Arc<dyn ObjectStore>)
        .with_settings(no_periodic_flush())
        .with_db_cache(cache.clone(), SCOPE_A)
        .build()
        .await
        .expect("failed to open db_a");
    let db_b = Db::builder(PATH_B, object_store.clone() as Arc<dyn ObjectStore>)
        .with_settings(no_periodic_flush())
        .with_db_cache(cache.clone(), SCOPE_B)
        .build()
        .await
        .expect("failed to open db_b");

    const N: usize = 32;
    for db in [&db_a, &db_b] {
        write_keys(db, N).await;
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .expect("flush failed");
        read_keys(db, N).await;
    }

    let mem_usage_both_warm = raw_cache.memory().usage();
    assert!(mem_usage_both_warm > 0);

    // when: db_a is flushed and closed ("task stop"); db_b keeps running.
    db_a.flush_cache_to_disk()
        .await
        .expect("flush_cache_to_disk failed");

    let mem_usage_after_flush = raw_cache.memory().usage();
    assert!(
        mem_usage_after_flush > 0,
        "expected db_b's entries to remain resident"
    );
    assert!(
        mem_usage_after_flush < mem_usage_both_warm,
        "expected db_a's entries to have been evicted"
    );

    db_a.close().await.expect("close db_a failed");

    // then (negative control): db_b, never flushed, is served entirely from
    // memory — reading it again must cost zero new GETs.
    let gets_before_db_b_reread = object_store.compacted_sst_get_count();
    read_keys(&db_b, N).await;
    assert_eq!(
        object_store.compacted_sst_get_count(),
        gets_before_db_b_reread,
        "db_a's flush/close must not disturb db_b's untouched entries"
    );

    // then (positive proof): db_a, reopened as a new instance at the same path
    // with the same db_cache_id, recovers its scope and is served entirely from
    // the disk tier it was flushed to — zero new GETs, this time via disk
    // rather than memory.
    let db_a2 = Db::builder(PATH_A, object_store.clone() as Arc<dyn ObjectStore>)
        .with_settings(no_periodic_flush())
        .with_db_cache(cache, SCOPE_A)
        .build()
        .await
        .expect("failed to reopen db_a");
    let gets_after_db_a_reopen = object_store.compacted_sst_get_count();
    read_keys(&db_a2, N).await;
    assert_eq!(
        object_store.compacted_sst_get_count(),
        gets_after_db_a_reopen,
        "expected the reopened db_a to serve reads from the disk tier it was flushed to"
    );

    db_a2.close().await.expect("close db_a2 failed");
    db_b.close().await.expect("close db_b failed");
}

/// Simulates a realistic "in-pod task restart": the `Db` for a given path is
/// closed (right after flushing, as a caller would on task stop) and
/// reopened as a *new* `Db` instance over the *same* shared cache, rather
/// than kept alive.
///
/// The caller must pass the *same* `db_cache_id` on both opens — SlateDB doesn't
/// derive or persist it — or the flushed data becomes unreachable under the
/// new instance's different scope. GET counts are measured only after `db2`'s
/// own open-time bootstrap, isolating the signal to whether reads hit the
/// disk tier or fell through to object storage.
#[tokio::test]
async fn reopening_a_new_db_instance_with_the_same_scope_id_recovers_the_flushed_scope() {
    let disk_dir = TempDir::new().expect("failed to create temp dir");
    let (cache, _raw_cache) = open_hybrid_cache(disk_dir.path()).await;

    let object_store = Arc::new(CountingObjectStore::new(Arc::new(InMemory::new())));
    const PATH: &str = "/evac-restart-test";
    const SCOPE: u64 = 1;

    let db1 = Db::builder(PATH, object_store.clone() as Arc<dyn ObjectStore>)
        .with_settings(no_periodic_flush())
        .with_db_cache(cache.clone(), SCOPE)
        .build()
        .await
        .expect("failed to open db1");

    const N: usize = 64;
    write_keys(&db1, N).await;
    db1.flush_with_options(FlushOptions {
        flush_type: FlushType::MemTable,
    })
    .await
    .expect("flush failed");

    // Warm everything (data, index, filters, stats), not just data blocks, so
    // a post-restart read genuinely needs nothing from object storage.
    let manifest = db1.manifest();
    for view in manifest.l0().iter() {
        db1.warm_sst(
            view.sst.id,
            &[
                CacheTarget::data::<&[u8], _>(..),
                CacheTarget::Index,
                CacheTarget::Filters,
                CacheTarget::Stats,
            ],
        )
        .await
        .expect("warm_sst failed");
    }

    db1.flush_cache_to_disk()
        .await
        .expect("flush_cache_to_disk failed");

    // "task stop": close db1. Cache is Unowned, so closing db1 does not touch it.
    db1.close().await.expect("close db1 failed");

    // "task restart": a brand-new Db instance for the same path, over the
    // same shared cache instance (simulating a caller that keeps one
    // process-wide cache), passing the same db_cache_id db1 used — as a caller
    // would, from its own durable per-task identity.
    let db2 = Db::builder(PATH, object_store.clone() as Arc<dyn ObjectStore>)
        .with_settings(no_periodic_flush())
        .with_db_cache(cache, SCOPE)
        .build()
        .await
        .expect("failed to reopen db2");

    // Baseline taken after db2's own open-time bootstrap. Counting only
    // compacted-SST GETs (see `CountingObjectStore`) additionally isolates
    // this from the background GC/compactor tasks' own boundary/manifest
    // polling, which runs independently of these reads and would otherwise
    // make a plain GET-count assertion flaky.
    let gets_after_open = object_store.compacted_sst_get_count();

    read_keys(&db2, N).await;
    let gets_after_read = object_store.compacted_sst_get_count();

    assert_eq!(
        gets_after_read, gets_after_open,
        "expected db2 to recover db1's evacuated scope and serve every read from \
         the disk tier with zero new object-store GETs"
    );

    db2.close().await.expect("close db2 failed");
}

/// `flush_cache_to_disk` must bound its enumeration to SSTs this `Db` has
/// actually cached something for, not walk every SST reachable from the
/// manifest. Five separate flushes produce five separate L0 SSTs; only one is
/// ever read. Flushing must cost zero new object-store GETs — in particular,
/// it must not read the index of the four SSTs it never touched just to
/// discover they have nothing resident.
#[tokio::test]
async fn flush_does_not_read_the_index_of_untouched_ssts() {
    let disk_dir = TempDir::new().expect("failed to create temp dir");
    let (cache, _raw_cache) = open_hybrid_cache(disk_dir.path()).await;
    let object_store = Arc::new(CountingObjectStore::new(Arc::new(InMemory::new())));

    let db = Db::builder(
        "/evac-untouched-ssts",
        object_store.clone() as Arc<dyn ObjectStore>,
    )
    .with_settings(no_periodic_flush())
    // Disable flush-time caching so the only way an SST becomes "touched" is
    // by actually being read, isolating the signal this test checks.
    .with_block_cache_policy(BlockCachePolicy::default().with_flush_targets(&[]))
    .with_db_cache(cache, 1)
    .build()
    .await
    .expect("failed to open db");

    const SSTS: usize = 5;
    const KEYS_PER_SST: usize = 16;
    let padding = vec![b'x'; 512];
    for batch in 0..SSTS {
        for i in 0..KEYS_PER_SST {
            let key = format!("batch{:02}-key{:06}", batch, i);
            let mut value = format!("value{:06}", i).into_bytes();
            value.extend_from_slice(&padding);
            db.put_with_options(
                key.as_bytes(),
                &value,
                &Default::default(),
                &WriteOptions::default(),
            )
            .await
            .expect("put failed");
        }
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .expect("flush failed");
    }

    let manifest = db.manifest();
    assert_eq!(
        manifest.l0().len(),
        SSTS,
        "expected one L0 SST per flush, none merged by compaction yet"
    );

    // when: only the first batch's keys are ever read.
    for i in 0..KEYS_PER_SST {
        let key = format!("batch00-key{:06}", i);
        let value = db.get(key.as_bytes()).await.expect("get failed");
        assert!(value.is_some());
    }

    let gets_before_flush = object_store.compacted_sst_get_count();

    db.flush_cache_to_disk()
        .await
        .expect("flush_cache_to_disk failed");

    // then: the flush touched object storage zero times — it only considered
    // the one SST it had actually cached something for, not the other four.
    assert_eq!(
        object_store.compacted_sst_get_count(),
        gets_before_flush,
        "expected the flush to skip the index reads of SSTs this Db never touched"
    );

    db.close().await.expect("close failed");
}

/// A `Db` (writer) and `DbReader` sharing a path share one cache scope, so
/// flushing from the reader must also evacuate the writer's entries — and the
/// writer, left open throughout, must keep working afterward.
#[tokio::test]
async fn flush_from_a_reader_flushes_the_shared_scope_including_the_live_writers_entries() {
    let disk_dir = TempDir::new().expect("failed to create temp dir");
    let (cache, raw_cache) = open_hybrid_cache(disk_dir.path()).await;
    let object_store = Arc::new(CountingObjectStore::new(Arc::new(InMemory::new())));
    const PATH: &str = "/evac-reader-flush-test";
    const SCOPE: u64 = 1;

    let db = Db::builder(PATH, object_store.clone() as Arc<dyn ObjectStore>)
        .with_settings(no_periodic_flush())
        .with_db_cache(cache.clone(), SCOPE)
        .build()
        .await
        .expect("failed to open db");

    const N: usize = 32;
    write_keys(&db, N).await;
    db.flush_with_options(FlushOptions {
        flush_type: FlushType::MemTable,
    })
    .await
    .expect("flush failed");
    read_keys(&db, N).await;

    let reader = DbReader::builder(PATH, object_store.clone() as Arc<dyn ObjectStore>)
        .with_db_cache(cache, SCOPE)
        .build()
        .await
        .expect("failed to open reader");

    reader.get(b"key000000").await.expect("reader get failed");

    assert!(
        raw_cache.memory().usage() > 0,
        "expected the writer's own reads to have populated the shared scope"
    );

    // when: the reader — not the writer — flushes the shared scope.
    reader
        .flush_cache_to_disk()
        .await
        .expect("flush_cache_to_disk failed");

    // then: the whole shared scope's memory footprint is gone, including
    // entries the reader never itself read — they were the writer's.
    assert_eq!(
        raw_cache.memory().usage(),
        0,
        "expected the reader's flush to evacuate the writer's entries too, \
         since they share one scope"
    );

    // and: the writer still works, served from the disk tier the reader's
    // flush spilled to.
    let gets_before_reread = object_store.compacted_sst_get_count();
    read_keys(&db, N).await;
    assert_eq!(
        object_store.compacted_sst_get_count(),
        gets_before_reread,
        "expected the writer's post-flush reads to hit the disk tier, not object storage"
    );

    reader.close().await.expect("close reader failed");
    db.close().await.expect("close db failed");
}
