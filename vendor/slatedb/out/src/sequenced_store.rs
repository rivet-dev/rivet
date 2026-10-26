//! The store every transactional manifest and `.compactions` of one database reads and
//! writes through in this process. Its latest-version reads skip the GC boundary re-read
//! when a recent check already proves the answer.
//!
//! Manifests and `.compactions` are sequences of versions. The garbage collector advances a
//! boundary file before deleting old versions, and a checked latest read re-reads that file
//! after every probe so that a cached version whose successors were deleted is not returned
//! as the latest. On an idle database every poll pays for that re-read, which object stores
//! bill as a full GET even when they answer 304 or 404.
//!
//! The re-read is skipped only inside a window after a full check. A full check at version
//! `M` that started at `T` shows that `M + 1` did not exist at `T` and `M` was above the
//! boundary. The collector advances the boundary only to a version that already had a
//! successor and was older than its `min_age`, so within `min_age` of `T` it cannot reach
//! any version at or above `M` that a later read can return without also contradicting the
//! check at `T`. A read that returns a version at or above `M` inside the window is
//! therefore a read whose boundary re-read would have passed had it been served when the
//! probe was. Writes always run the full check, since a stale writer can recreate a
//! deleted version.
//!
//! The window is a fifth of the in-process collector's `min_age`, which leaves the rest of
//! `min_age` for clock skew between the collector and the store, for the local clock
//! stepping backward, and for the gap between a version's `Last-Modified` and the moment it
//! became visible (a PUT the store held for most of `min_age`). Without boundary files or an
//! in-process collector, nothing is skipped.
//!
//! The in-process collector advances the boundary through this store, and a proof at or below
//! the new boundary is dropped as soon as the advance is durable, before anything below it is
//! deleted. For that collector the timing argument is only a second line of defense; it is
//! the whole argument only for a collector in another process.
//!
//! The store also announces each version it writes, so the writer, the compactor and the
//! compaction worker that share it learn of each other's writes without waiting for their
//! next poll. A notice only makes a component run the poll it would otherwise run later;
//! polls stay as the fallback for writers in other processes.

use chrono::{DateTime, Utc};
use log::warn;
use parking_lot::Mutex;
use slatedb_common::clock::SystemClock;
use slatedb_txn_obj::{
    MonotonicId, SequencedStorageProtocol, TransactionalObjectError, TransactionalStorageProtocol,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

use crate::config::{GarbageCollectorDirectoryOptions, GarbageCollectorOptions};
use crate::dispatcher::Notifier;

/// How much of the collector's `min_age` one full check covers.
const WINDOW_FRACTION_OF_MIN_AGE: u32 = 5;

/// Proof from a full boundary check that later reads may reuse.
#[derive(Clone, Copy, Debug)]
struct CheckedLatest {
    id: MonotonicId,
    /// When the probe or write that produced `id` started, which is no later than the moment
    /// `id + 1` was observed missing.
    started_at: DateTime<Utc>,
}

#[derive(Clone)]
pub(crate) struct BoundaryReuse {
    clock: Arc<dyn SystemClock>,
    window: Duration,
}

impl BoundaryReuse {
    /// The reuse window for one sequenced directory, derived from the collector that runs in
    /// this process. `select` picks that directory's options.
    pub(crate) fn for_collector(
        clock: Arc<dyn SystemClock>,
        options: Option<&GarbageCollectorOptions>,
        select: fn(&GarbageCollectorOptions) -> Option<&GarbageCollectorDirectoryOptions>,
    ) -> Option<Self> {
        let options = options?;
        if !options.boundary_files_enabled {
            return None;
        }
        let window = select(options)?.min_age / WINDOW_FRACTION_OF_MIN_AGE;
        if window.is_zero() {
            return None;
        }
        Some(Self { clock, window })
    }
}

/// Wraps a sequenced store so latest-version reads reuse a recent boundary check. Writes,
/// listings and deletes keep the inner store's behavior.
pub(crate) struct SequencedStore<T> {
    inner: Arc<dyn SequencedStorageProtocol<T>>,
    reuse: Option<BoundaryReuse>,
    checked: Mutex<Option<CheckedLatest>>,
    /// The highest version written through this store.
    written: watch::Sender<u64>,
}

impl<T> SequencedStore<T> {
    pub(crate) fn new(
        inner: Arc<dyn SequencedStorageProtocol<T>>,
        reuse: Option<BoundaryReuse>,
    ) -> Self {
        let (written, _) = watch::channel(0);
        Self {
            inner,
            reuse,
            checked: Mutex::new(None),
            written,
        }
    }

    /// Follows the highest version written through this store.
    pub(crate) fn subscribe_writes(&self) -> watch::Receiver<u64> {
        self.written.subscribe()
    }

    fn announce(&self, id: MonotonicId) {
        self.written.send_if_modified(|latest| {
            if id.id() > *latest {
                *latest = id.id();
                true
            } else {
                false
            }
        });
    }

    fn now(&self) -> Option<DateTime<Utc>> {
        self.reuse.as_ref().map(|reuse| reuse.clock.now())
    }

    /// Records a passed full check, keeping whichever proof started last.
    fn record(&self, id: MonotonicId, started_at: Option<DateTime<Utc>>) {
        let Some(started_at) = started_at else {
            return;
        };
        let mut checked = self.checked.lock();
        let newer = match checked.as_ref() {
            Some(previous) => previous.started_at <= started_at,
            None => true,
        };
        if newer {
            *checked = Some(CheckedLatest { id, started_at });
        }
    }

    /// Advances the durable boundary, then drops a proof the new boundary reaches, so a
    /// later read cannot skip the re-read for versions this collector may now delete.
    pub(crate) async fn advance(&self, boundary: MonotonicId) -> Result<(), TransactionalObjectError> {
        self.inner.advance(boundary).await?;
        let mut checked = self.checked.lock();
        if matches!(checked.as_ref(), Some(proof) if proof.id <= boundary) {
            *checked = None;
        }
        Ok(())
    }

    /// Whether a recent full check proves that `id` passes the boundary check now.
    fn covered(&self, id: MonotonicId) -> bool {
        let Some(reuse) = self.reuse.as_ref() else {
            return false;
        };
        let Some(checked) = *self.checked.lock() else {
            return false;
        };
        let now = reuse.clock.now();
        // A clock that stepped behind the proof cannot measure its age.
        if now < checked.started_at {
            return false;
        }
        let Ok(age) = (now - checked.started_at).to_std() else {
            return false;
        };
        id >= checked.id && age < reuse.window
    }
}

#[async_trait::async_trait]
impl<T: Send + Sync> TransactionalStorageProtocol<T, MonotonicId> for SequencedStore<T> {
    async fn write(
        &self,
        current_id: Option<MonotonicId>,
        new_value: &T,
    ) -> Result<MonotonicId, TransactionalObjectError> {
        let started_at = self.now();
        let id = self.inner.write(current_id, new_value).await?;
        self.record(id, started_at);
        self.announce(id);
        Ok(id)
    }

    /// Same loop as the inner store's checked read, except that a covered result skips
    /// the boundary re-read.
    async fn try_read_latest(&self) -> Result<Option<(MonotonicId, T)>, TransactionalObjectError> {
        loop {
            let started_at = self.now();
            let Some((id, value)) = self.inner.try_read_latest_unchecked().await? else {
                return Ok(None);
            };
            if self.covered(id) {
                return Ok(Some((id, value)));
            }
            match self.inner.check(id).await {
                Ok(()) => {
                    self.record(id, started_at);
                    return Ok(Some((id, value)));
                }
                Err(TransactionalObjectError::ObjectVersionExists) => {
                    warn!("sequenced read behind boundary: id={id:?}");
                    continue;
                }
                Err(err) => return Err(err),
            }
        }
    }
}

/// The most often writes elsewhere in this process trigger an extra poll in one component.
/// Components whose own poll is faster use their poll interval instead.
pub(crate) const MIN_WRITE_NOTICE_INTERVAL: Duration = Duration::from_secs(1);

/// Turns versions written in this process into poll messages for a component that shares
/// the store. Notices closer together than `min_interval` are coalesced into one, so a burst
/// of writes costs at most one extra poll per interval.
pub(crate) struct WriteNotifier<M> {
    writes: watch::Receiver<u64>,
    clock: Arc<dyn SystemClock>,
    min_interval: Duration,
    last_notified_at: Option<DateTime<Utc>>,
    message: fn(u64) -> M,
}

impl<M> WriteNotifier<M> {
    pub(crate) fn new(
        writes: watch::Receiver<u64>,
        clock: Arc<dyn SystemClock>,
        min_interval: Duration,
        message: fn(u64) -> M,
    ) -> Self {
        Self {
            writes,
            clock,
            min_interval,
            last_notified_at: None,
            message,
        }
    }
}

#[async_trait::async_trait]
impl<M: Send> Notifier<M> for WriteNotifier<M> {
    async fn notify(&mut self) -> M {
        // The sender lives as long as the store; once it is gone the database is closing and
        // the dispatcher's cancellation ends the loop.
        if self.writes.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
        if let Some(last) = self.last_notified_at {
            let next = last + self.min_interval;
            let now = self.clock.now();
            if now < next {
                // A wall clock that stepped backward never stretches the wait past the interval.
                if let Ok(wait) = (next - now).to_std() {
                    self.clock.sleep(wait.min(self.min_interval)).await;
                }
            }
        }
        self.last_notified_at = Some(self.clock.now());
        let id = *self.writes.borrow_and_update();
        (self.message)(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GarbageCollectorOptions;
    use crate::manifest::store::{ManifestStore, StoredManifest};
    use crate::manifest::ManifestCore;
    use futures::stream::BoxStream;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::{
        CopyOptions, GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore,
        PutMultipartOptions, PutOptions, PutPayload, PutResult,
    };
    use slatedb_common::clock::MockSystemClock;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const ROOT: &str = "/root";

    /// Counts reads of the GC boundary files.
    #[derive(Debug)]
    struct BoundaryReads {
        inner: Arc<dyn ObjectStore>,
        reads: AtomicUsize,
    }

    impl BoundaryReads {
        fn reads(&self) -> usize {
            self.reads.load(Ordering::SeqCst)
        }
    }

    impl std::fmt::Display for BoundaryReads {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "BoundaryReads")
        }
    }

    #[async_trait::async_trait]
    impl ObjectStore for BoundaryReads {
        async fn put_opts(
            &self,
            path: &Path,
            payload: PutPayload,
            options: PutOptions,
        ) -> object_store::Result<PutResult> {
            self.inner.put_opts(path, payload, options).await
        }

        async fn put_multipart_opts(
            &self,
            path: &Path,
            options: PutMultipartOptions,
        ) -> object_store::Result<Box<dyn MultipartUpload>> {
            self.inner.put_multipart_opts(path, options).await
        }

        async fn get_opts(
            &self,
            path: &Path,
            options: GetOptions,
        ) -> object_store::Result<GetResult> {
            if path.as_ref().contains("/gc/") {
                self.reads.fetch_add(1, Ordering::SeqCst);
            }
            self.inner.get_opts(path, options).await
        }

        fn delete_stream(
            &self,
            paths: BoxStream<'static, object_store::Result<Path>>,
        ) -> BoxStream<'static, object_store::Result<Path>> {
            self.inner.delete_stream(paths)
        }

        fn list(
            &self,
            prefix: Option<&Path>,
        ) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
            self.inner.list(prefix)
        }

        async fn list_with_delimiter(
            &self,
            prefix: Option<&Path>,
        ) -> object_store::Result<ListResult> {
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
    }

    struct Fixture {
        objects: Arc<BoundaryReads>,
        clock: Arc<MockSystemClock>,
        store: Arc<ManifestStore>,
    }

    /// A manifest store with the default collector's reuse window (60 s) and one manifest.
    async fn fixture() -> Fixture {
        let objects = Arc::new(BoundaryReads {
            inner: Arc::new(InMemory::new()),
            reads: AtomicUsize::new(0),
        });
        let clock = Arc::new(MockSystemClock::with_time(1_000_000));
        let options = GarbageCollectorOptions::default();
        let reuse = BoundaryReuse::for_collector(clock.clone(), Some(&options), |options| {
            options.manifest_options.as_ref()
        });
        assert_eq!(Duration::from_secs(60), reuse.as_ref().unwrap().window);
        let store = Arc::new(ManifestStore::with_boundary_reuse(
            &Path::from(ROOT),
            objects.clone(),
            reuse,
        ));
        StoredManifest::create_new_db(store.clone(), ManifestCore::new(), clock.clone())
            .await
            .unwrap();
        Fixture {
            objects,
            clock,
            store,
        }
    }

    /// Writes `count` manifests after the latest through a separate store, as another
    /// process would.
    async fn write_elsewhere(objects: Arc<dyn ObjectStore>, count: usize) {
        let other = Arc::new(ManifestStore::new(&Path::from(ROOT), objects));
        let clock = Arc::new(MockSystemClock::new());
        let mut stored = StoredManifest::load(other, clock).await.unwrap();
        for _ in 0..count {
            let dirty = stored.prepare_dirty().unwrap();
            stored.update(dirty).await.unwrap();
        }
    }

    #[tokio::test]
    async fn polls_inside_the_window_skip_the_boundary_read() {
        let f = fixture().await;
        // The initial write ran a full check, which later reads reuse.
        let after_write = f.objects.reads();
        assert!(after_write >= 1);
        for _ in 0..5 {
            f.clock.set(f.clock.now().timestamp_millis() + 10_000);
            f.store.read_latest_manifest().await.unwrap();
        }
        assert_eq!(after_write, f.objects.reads());

        // The proof expires a window after the write started.
        f.clock.set(f.clock.now().timestamp_millis() + 11_000);
        f.store.read_latest_manifest().await.unwrap();
        assert_eq!(after_write + 1, f.objects.reads());
    }

    #[tokio::test]
    async fn a_newer_version_inside_the_window_is_covered() {
        let f = fixture().await;
        f.store.read_latest_manifest().await.unwrap();
        let reads = f.objects.reads();
        write_elsewhere(f.objects.inner.clone(), 2).await;
        let latest = f.store.read_latest_manifest().await.unwrap();
        assert_eq!(3, latest.id);
        assert_eq!(reads, f.objects.reads());
    }

    #[tokio::test]
    async fn a_clock_behind_the_proof_runs_the_full_check() {
        let f = fixture().await;
        let reads = f.objects.reads();
        f.clock.set(f.clock.now().timestamp_millis() - 1);
        f.store.read_latest_manifest().await.unwrap();
        assert_eq!(reads + 1, f.objects.reads());
    }

    #[tokio::test]
    async fn after_the_window_a_collected_cache_is_not_returned() {
        let f = fixture().await;
        f.store.read_latest_manifest().await.unwrap();

        // Another writer moves far ahead, and a collector deletes every version this
        // process saw, so the cached version's successors are gone. This needs the collector's
        // min_age to have passed, which is past the reuse window.
        write_elsewhere(f.objects.inner.clone(), 6).await;
        f.clock.set(f.clock.now().timestamp_millis() + 301_000);
        let other = ManifestStore::new(&Path::from(ROOT), f.objects.inner.clone());
        other.advance_boundary(5).await.unwrap();
        for id in 1..=5 {
            other.delete_manifest_unchecked(id).await.unwrap();
        }

        let latest = f.store.read_latest_manifest().await.unwrap();
        assert_eq!(7, latest.id);
    }

    #[tokio::test]
    async fn an_in_process_advance_drops_the_proof_it_reaches() {
        let f = fixture().await;
        f.store.read_latest_manifest().await.unwrap();
        write_elsewhere(f.objects.inner.clone(), 2).await;
        // This process's collector advances past the version it proved, before deleting.
        f.store.advance_boundary(2).await.unwrap();
        let reads = f.objects.reads();
        let latest = f.store.read_latest_manifest().await.unwrap();
        assert_eq!(3, latest.id);
        assert!(f.objects.reads() > reads);
    }

    #[tokio::test]
    async fn without_a_collector_every_read_checks() {
        let objects = Arc::new(BoundaryReads {
            inner: Arc::new(InMemory::new()),
            reads: AtomicUsize::new(0),
        });
        let clock = Arc::new(MockSystemClock::new());
        let store = Arc::new(ManifestStore::with_boundary_reuse(
            &Path::from(ROOT),
            objects.clone(),
            BoundaryReuse::for_collector(clock.clone(), None, |options| {
                options.manifest_options.as_ref()
            }),
        ));
        StoredManifest::create_new_db(store.clone(), ManifestCore::new(), clock)
            .await
            .unwrap();
        let reads = objects.reads();
        store.read_latest_manifest().await.unwrap();
        store.read_latest_manifest().await.unwrap();
        assert_eq!(reads + 2, objects.reads());
    }

    #[tokio::test]
    async fn writes_always_run_the_full_check() {
        let f = fixture().await;
        let mut stored = StoredManifest::load(f.store.clone(), f.clock.clone())
            .await
            .unwrap();
        let reads = f.objects.reads();
        let dirty = stored.prepare_dirty().unwrap();
        stored.update(dirty).await.unwrap();
        assert_eq!(reads + 1, f.objects.reads());
    }

    #[tokio::test(start_paused = true)]
    async fn notices_coalesce_a_burst_of_writes() {
        let f = fixture().await;
        let clock: Arc<dyn SystemClock> = Arc::new(slatedb_common::clock::DefaultSystemClock::new());
        let mut notifier = WriteNotifier::new(
            f.store.subscribe_writes(),
            clock.clone(),
            Duration::from_secs(1),
            |id| id,
        );
        let mut stored = StoredManifest::load(f.store.clone(), f.clock.clone())
            .await
            .unwrap();
        async fn write(stored: &mut StoredManifest) {
            let dirty = stored.prepare_dirty().unwrap();
            stored.update(dirty).await.unwrap();
        }

        // The first write after a quiet period is announced at once.
        write(&mut stored).await;
        let started = clock.now();
        assert_eq!(2, notifier.notify().await);
        assert_eq!(started, clock.now());

        // A burst inside the interval becomes one notice for its last version.
        write(&mut stored).await;
        write(&mut stored).await;
        write(&mut stored).await;
        assert_eq!(5, notifier.notify().await);
        assert!(clock.now() - started >= chrono::Duration::seconds(1));

        // Nothing new: no notice.
        let quiet = tokio::time::timeout(Duration::from_secs(60), notifier.notify()).await;
        assert!(quiet.is_err());
    }

    #[tokio::test]
    async fn notices_ignore_writes_to_other_stores() {
        let f = fixture().await;
        let mut writes = f.store.subscribe_writes();
        writes.borrow_and_update();
        write_elsewhere(f.objects.inner.clone(), 1).await;
        assert!(!writes.has_changed().unwrap());
    }

    /// With polls an hour apart, L0 must still drain: the compactor learns of each flush,
    /// the worker of each job and the writer of each result from the notices alone.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn compactor_drains_l0_without_polling() {
        use crate::config::{CompactorOptions, FlushOptions, FlushType, Settings};
        use crate::Db;

        let hour = Duration::from_secs(3600);
        let mut compactor = CompactorOptions {
            poll_interval: hour,
            commit_compacted_interval: Duration::from_millis(50),
            ..Default::default()
        };
        if let Some(worker) = compactor.worker.as_mut() {
            worker.compactions_poll_interval = hour;
        }
        let settings = Settings {
            flush_interval: None,
            manifest_poll_interval: hour,
            l0_max_ssts: 6,
            compactor_options: Some(compactor),
            ..Default::default()
        };
        let db = Db::builder("/tmp/notice_drain", Arc::new(InMemory::new()))
            .with_settings(settings)
            .build()
            .await
            .unwrap();
        let flush = FlushOptions {
            flush_type: FlushType::MemTable,
        };
        let result = tokio::time::timeout(Duration::from_secs(60), async {
            // Without notices the seventh flush would wait for a poll an hour away.
            for i in 0..24u32 {
                db.put(i.to_be_bytes(), [7u8; 64]).await.unwrap();
                db.flush_with_options(flush.clone()).await.unwrap();
                assert!(db.manifest().l0().len() <= 6);
            }
            loop {
                let manifest = db.manifest();
                if !manifest.compacted().is_empty() && manifest.l0().len() < 4 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await;
        assert!(
            result.is_ok(),
            "L0 {} after the writes",
            db.manifest().l0().len()
        );
        for i in 0..24u32 {
            assert_eq!(
                Some(bytes::Bytes::from(vec![7u8; 64])),
                db.get(i.to_be_bytes()).await.unwrap()
            );
        }
        db.close().await.unwrap();
    }
}
