//! Parallel L0 flush SST uploader.
//!
//! The uploader is intentionally narrow in scope:
//! - build an SST from an immutable memtable
//! - upload the SST to object storage
//! - report successful completion
//!
//! It does not own:
//! - manifest mutation
//! - checkpoint creation
//! - flush waiter bookkeeping
//! - timeout enforcement

use super::tracker::TrackerMessage;
use crate::db::DbInner;
#[cfg(test)]
use crate::db_state::SsTableHandle;
use crate::db_status::ClosedResultWriter;
use crate::dispatcher::{MessageHandler, MessageHandlerExecutor};
use crate::error::SlateDBError;
use crate::flush::SegmentedSstHandle;
use crate::mem_table::ImmutableMemtable;
use crate::retrying_object_store::RetryingObjectStore;
use crate::utils::SafeSender;
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
use futures::StreamExt;
use log::{info, warn};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;
use ulid::Ulid;

const UPLOADER_TASK_NAME: &str = "l0_sst_uploader";

// `BufWriter` wraps an `object_store::Error` inside `std::io::Error` through
// `AsyncWrite`. Find the original error before applying the shared retry rule.
// Without this step, `NotSupported` can retry forever.
fn should_retry_upload_error(error: &SlateDBError) -> bool {
    match error {
        SlateDBError::ObjectStoreError(error) => RetryingObjectStore::should_retry(error),
        SlateDBError::IoError(error) => {
            let Some(source) = error.get_ref() else {
                return true;
            };
            let mut source: Option<&(dyn std::error::Error + 'static)> = Some(source);
            while let Some(error) = source {
                if let Some(error) = error.downcast_ref::<object_store::Error>() {
                    return RetryingObjectStore::should_retry(error);
                }
                if let Some(error) = error.downcast_ref::<Arc<object_store::Error>>() {
                    return RetryingObjectStore::should_retry(error);
                }
                source = error.source();
            }
            true
        }
        _ => false,
    }
}

/// One immutable-memtable upload request submitted to the uploader. Physical
/// SST ids are allocated at dispatch (in sequence order) and carried here, so
/// the parallel upload workers never mint ids out of publish order (RFC-0029).
pub(crate) struct UploadJob {
    /// Immutable memtable to build into one or more SSTs.
    pub(crate) imm_memtable: Arc<ImmutableMemtable>,
    /// Pre-allocated physical SST id per segment prefix. A segment that
    /// retention prunes to empty simply leaves its id unused.
    pub(crate) segment_sst_ids: BTreeMap<Bytes, Ulid>,
}

impl std::fmt::Debug for UploadJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadJob").finish()
    }
}

impl UploadJob {
    /// Creates a new upload job with pre-allocated segment SST ids.
    pub(crate) fn new(
        imm_memtable: Arc<ImmutableMemtable>,
        segment_sst_ids: BTreeMap<Bytes, Ulid>,
    ) -> Self {
        Self {
            imm_memtable,
            segment_sst_ids,
        }
    }
}

#[derive(Clone)]
/// Result of a successfully uploaded immutable memtable. A flush produces
/// one [`SegmentedSstHandle`] per segment that received at least one
/// post-retention entry. Without an extractor configured every flush yields
/// at most a single handle with empty prefix; with an extractor it yields
/// one handle per touched segment. The Vec is empty when retention prunes
/// every entry — per-memtable progress in the manifest still advances.
pub(crate) struct UploadedMemtable {
    /// Same immutable memtable that was uploaded.
    pub(crate) imm_memtable: Arc<ImmutableMemtable>,
    /// Per-segment uploaded SSTs, sorted ascending by `prefix`. May be
    /// empty when retention pruned every entry in the memtable.
    pub(crate) segments: Vec<SegmentedSstHandle>,
    /// Lowest sequence number present in the immutable memtable.
    pub(crate) first_seq: u64,
    /// Highest sequence number present in the immutable memtable.
    pub(crate) last_seq: u64,
}

impl UploadedMemtable {
    #[cfg(test)]
    pub(crate) fn new(
        imm_memtable: Arc<ImmutableMemtable>,
        sst_handle: SsTableHandle,
        first_seq: u64,
        last_seq: u64,
    ) -> Self {
        assert!(first_seq <= last_seq);
        Self {
            imm_memtable,
            segments: vec![SegmentedSstHandle {
                prefix: Bytes::new(),
                sst_handle,
                encoded_bytes: 0,
            }],
            first_seq,
            last_seq,
        }
    }
}

/// Wrapper around the upload job channel. Owns the tx channel and registers
/// workers with the executor on [`start`](Self::start).
#[derive(Clone)]
pub(crate) struct Uploader {
    jobs_tx: SafeSender<UploadJob>,
}

impl Uploader {
    pub(crate) fn start(
        db: Arc<DbInner>,
        closed_result: &dyn ClosedResultWriter,
        tracker_tx: SafeSender<TrackerMessage>,
        executor: &MessageHandlerExecutor,
        tokio_handle: &Handle,
    ) -> Result<Self, SlateDBError> {
        let (jobs_tx, jobs_rx) = SafeSender::unbounded_channel(closed_result.result_reader());
        let uploader = Uploader { jobs_tx };
        let handlers = Self::build_handlers(db, tracker_tx);
        executor.add_handlers(
            UPLOADER_TASK_NAME.to_string(),
            handlers,
            jobs_rx,
            tokio_handle,
        )?;
        Ok(uploader)
    }

    pub(crate) fn build_handlers(
        db: Arc<DbInner>,
        tracker_tx: SafeSender<TrackerMessage>,
    ) -> Vec<Box<dyn MessageHandler<UploadJob>>> {
        let parallelism = db.settings.l0_flush_parallelism;
        let retry_backoff = db.settings.manifest_poll_interval;
        (0..parallelism)
            .map(|_| {
                Box::new(UploadHandler::new(
                    Arc::clone(&db),
                    tracker_tx.clone(),
                    retry_backoff,
                )) as Box<dyn MessageHandler<UploadJob>>
            })
            .collect()
    }

    /// Submits a new upload job.
    pub(crate) fn submit(&self, job: UploadJob) -> Result<(), SlateDBError> {
        self.jobs_tx.send(job)
    }

    pub(crate) async fn shutdown(executor: &MessageHandlerExecutor) {
        if let Err(e) = executor.shutdown_task(UPLOADER_TASK_NAME).await {
            warn!("failed to shutdown l0 sst uploader [error={:?}]", e);
        }
    }
}

/// MessageHandler that builds and uploads one SST per job.
pub(crate) struct UploadHandler {
    db: Arc<DbInner>,
    tracker_tx: SafeSender<TrackerMessage>,
    retry_backoff: Duration,
}

impl UploadHandler {
    pub(crate) fn new(
        db: Arc<DbInner>,
        tracker_tx: SafeSender<TrackerMessage>,
        retry_backoff: Duration,
    ) -> Self {
        Self {
            db,
            tracker_tx,
            retry_backoff,
        }
    }

    async fn upload_with_retry(&self, job: &UploadJob) -> Result<UploadedMemtable, SlateDBError> {
        // Read the retention boundary once, before the first attempt
        // below. Every retry reuses this same value, so every attempt
        // keeps the same entries.
        let min_retention_seq = self.db.compute_min_retention_seq();
        let first_seq = job
            .imm_memtable
            .table()
            .first_seq()
            .expect("flush of l0 with no entries");
        let last_seq = job
            .imm_memtable
            .table()
            .last_seq()
            .expect("flush of l0 with no entries");

        let segments = loop {
            match self
                .db
                .stream_imm_ssts(
                    job.imm_memtable.table(),
                    min_retention_seq,
                    &job.segment_sst_ids,
                )
                .await
            {
                Ok(segments) => break segments,
                Err(e) if should_retry_upload_error(&e) => {
                    // When the WAL is enabled and the database is shutting
                    // down, give up immediately. The data is already durable
                    // in the WAL and will be recovered on the next startup.
                    if self.db.wal_enabled && self.db.check_closed().is_err() {
                        info!("skipping l0 flush retry during shutdown [error={:?}]", e);
                        return Err(e);
                    }
                    self.db.system_clock.sleep(self.retry_backoff).await;
                }
                Err(e) => return Err(e),
            }
        };

        let written_bytes: u64 = segments.iter().map(|s| s.encoded_bytes).sum();
        self.db.db_stats.l0_flush_bytes.increment(written_bytes);

        Ok(UploadedMemtable {
            imm_memtable: Arc::clone(&job.imm_memtable),
            segments,
            first_seq,
            last_seq,
        })
    }
}

#[async_trait]
impl MessageHandler<UploadJob> for UploadHandler {
    async fn handle(&mut self, job: UploadJob) -> Result<(), SlateDBError> {
        let success = self.upload_with_retry(&job).await?;
        self.tracker_tx
            .send(TrackerMessage::UploadComplete(success))?;
        Ok(())
    }

    async fn cleanup(
        &mut self,
        mut messages: BoxStream<'async_trait, UploadJob>,
        result: Result<(), SlateDBError>,
    ) -> Result<(), SlateDBError> {
        // On clean shutdown, drain remaining jobs.
        if result.is_ok() {
            while let Some(job) = messages.next().await {
                let success = self.upload_with_retry(&job).await?;
                self.tracker_tx
                    .send(TrackerMessage::UploadComplete(success))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{should_retry_upload_error, TrackerMessage, UploadJob, Uploader};
    use crate::block_cache_policy::BlockCachePolicy;
    use crate::config::Settings;
    use crate::db::DbInner;
    use crate::db_cache::test_utils::TestCache;
    use crate::db_cache::CacheTarget;
    use crate::db_cache::{CachedKey, DbCache};
    use crate::db_state::{SsTableId, SsTableView};
    use crate::db_status::{ClosedResultWriter, DbStatusManager};
    use crate::error::SlateDBError;
    use crate::format::sst::SsTableFormat;
    use crate::iter::RowEntryIterator;
    use crate::manifest::ManifestCore;
    use crate::mem_table::ImmutableMemtable;
    use crate::paths::PathResolver;
    use crate::sst_iter::{SstIterator, SstIteratorOptions};
    use crate::tablestore::{TableStore, TableStoreKind};
    use crate::test_utils::FixedThreeBytePrefixExtractor;
    use crate::types::{RowEntry, ValueDeletable};
    use crate::utils::WatchableOnceCell;

    use crate::wal::test_utils::FakeWalWriter;
    use crate::wal::WalWriter;
    use bytes::Bytes;
    use fail_parallel::FailPointRegistry;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use slatedb_common::clock::{DefaultSystemClock, SystemClock};
    use slatedb_common::metrics::MetricsRecorderHelper;
    use slatedb_common::DbRand;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::runtime::Handle;
    use tokio::time::timeout;
    use ulid::Ulid;

    fn not_supported_error() -> object_store::Error {
        object_store::Error::NotSupported {
            source: Box::new(std::io::Error::other("not supported")),
        }
    }

    #[test]
    fn should_not_retry_not_supported_upload_error() {
        let direct = SlateDBError::from(not_supported_error());
        assert!(!should_retry_upload_error(&direct));

        let wrapped = SlateDBError::from(std::io::Error::from(not_supported_error()));
        assert!(!should_retry_upload_error(&wrapped));
    }

    #[test]
    fn should_retry_io_error_without_object_store_source() {
        let error = SlateDBError::from(std::io::Error::other("temporary failure"));
        assert!(should_retry_upload_error(&error));
    }

    /// Build a pre-allocated id map for a test job, mirroring dispatch-time
    /// allocation: one id per segment prefix, falling back to the empty prefix
    /// when no extractor recorded segments. The tracker owns the real
    /// allocation path; tests only need a valid map covering the built SSTs.
    fn preallocate_ids(imm: &ImmutableMemtable) -> BTreeMap<Bytes, Ulid> {
        let mut prefixes = imm.touched_segments();
        if prefixes.is_empty() {
            prefixes.insert(Bytes::new());
        }
        prefixes.into_iter().map(|p| (p, Ulid::new())).collect()
    }

    async fn setup_db(path: &str, fp_registry: Arc<FailPointRegistry>) -> Arc<DbInner> {
        setup_db_with_extractor(path, fp_registry, None).await
    }

    async fn setup_db_with_extractor(
        path: &str,
        fp_registry: Arc<FailPointRegistry>,
        segment_extractor: Option<Arc<dyn crate::prefix_extractor::PrefixExtractor>>,
    ) -> Arc<DbInner> {
        setup_db_with_cache_policy(
            path,
            fp_registry,
            segment_extractor,
            None,
            BlockCachePolicy::default(),
        )
        .await
    }

    async fn setup_db_with_cache_policy(
        path: &str,
        fp_registry: Arc<FailPointRegistry>,
        segment_extractor: Option<Arc<dyn crate::prefix_extractor::PrefixExtractor>>,
        cache: Option<Arc<dyn DbCache>>,
        block_cache_policy: BlockCachePolicy,
    ) -> Arc<DbInner> {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let settings = Settings::default();
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());
        let rand = Arc::new(DbRand::new(42));
        let db_metrics = MetricsRecorderHelper::noop();
        let manifest_store = Arc::new(crate::manifest::store::ManifestStore::new(
            &Path::from(path),
            Arc::clone(&object_store),
        ));
        let stored_manifest = crate::manifest::store::StoredManifest::create_new_db(
            manifest_store,
            ManifestCore::new_with_wal_object_store(None),
            Arc::clone(&system_clock),
        )
        .await
        .unwrap();
        let table_store = Arc::new(TableStore::new_with_fp_registry(
            Arc::clone(&object_store),
            SsTableFormat::default(),
            PathResolver::from_root(Path::from(path)),
            fp_registry.clone(),
            cache,
            TableStoreKind::Main,
            block_cache_policy,
        ));
        let status_manager = DbStatusManager::new(0);
        let (write_tx, _) =
            crate::utils::SafeSender::unbounded_channel(status_manager.result_reader());
        let wal_writer = Box::new(FakeWalWriter::new(0));
        Arc::new(
            DbInner::new(
                settings,
                Arc::clone(&system_clock),
                Arc::clone(&rand),
                Arc::clone(&table_store),
                stored_manifest.prepare_dirty().unwrap(),
                Arc::new(crate::memtable_flusher::MemtableFlusher::new(
                    &status_manager,
                )),
                write_tx,
                wal_writer.observer(),
                db_metrics,
                fp_registry,
                None,
                Arc::new(status_manager),
                segment_extractor,
            )
            .await
            .unwrap(),
        )
    }

    fn freeze_imm(db: &DbInner, key: &[u8], value: &[u8], seq: u64) -> Arc<ImmutableMemtable> {
        let mut guard = db.state.write();
        guard.memtable().put(RowEntry::new_value(key, value, seq));
        guard.freeze_memtable(0);
        guard.state().imm_memtable.front().cloned().unwrap()
    }

    fn next_upload_job(db: &DbInner, key: &[u8], value: &[u8], seq: u64) -> UploadJob {
        let imm_memtable = freeze_imm(db, key, value, seq);
        let segment_sst_ids = preallocate_ids(&imm_memtable);
        UploadJob::new(imm_memtable, segment_sst_ids)
    }

    struct TestUploader {
        uploader: Uploader,
        tracker_rx: async_channel::Receiver<TrackerMessage>,
        executor: Arc<crate::dispatcher::MessageHandlerExecutor>,
        closed_result: WatchableOnceCell<Result<(), SlateDBError>>,
    }

    impl TestUploader {
        /// Wait for the executor to report a closed result (error or clean shutdown).
        async fn await_closed(&self) -> Result<(), SlateDBError> {
            self.closed_result.reader().await_value().await
        }

        async fn shutdown(&self) {
            Uploader::shutdown(&self.executor).await;
        }
    }

    impl std::ops::Deref for TestUploader {
        type Target = Uploader;
        fn deref(&self) -> &Self::Target {
            &self.uploader
        }
    }

    fn start_test_uploader(db: &Arc<DbInner>) -> TestUploader {
        let closed_result: WatchableOnceCell<Result<(), SlateDBError>> = WatchableOnceCell::new();
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());
        let (tracker_tx, tracker_rx) =
            crate::utils::SafeSender::unbounded_channel(closed_result.result_reader());
        let executor = Arc::new(crate::dispatcher::MessageHandlerExecutor::new(
            Arc::new(closed_result.clone()),
            system_clock,
        ));
        let uploader = Uploader::start(
            Arc::clone(db),
            &closed_result,
            tracker_tx,
            &executor,
            &Handle::current(),
        )
        .unwrap();
        executor.monitor_on(&Handle::current()).unwrap();
        TestUploader {
            uploader,
            tracker_rx,
            executor,
            closed_result,
        }
    }

    #[tokio::test]
    async fn should_emit_uploaded_event_for_successful_job() {
        let db = setup_db(
            "/tmp/test_parallel_l0_flush_uploader_success",
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        let job = next_upload_job(&db, b"key", b"value", 1);

        let test = start_test_uploader(&db);
        test.submit(job).unwrap();

        let msg = timeout(Duration::from_secs(5), test.tracker_rx.recv())
            .await
            .unwrap()
            .unwrap();
        let TrackerMessage::UploadComplete(event) = msg else {
            panic!("expected UploadComplete");
        };
        assert_eq!(event.first_seq, 1);
        assert_eq!(event.last_seq, 1);

        // Verify the uploaded SST contains the expected key-value entry.
        assert_eq!(event.segments.len(), 1);
        let segment = &event.segments[0];
        assert!(segment.prefix.is_empty());
        let sst_view = SsTableView::identity(
            db.table_store
                .open_sst(&segment.sst_handle.id, Some(segment.prefix.clone()))
                .await
                .unwrap(),
        );
        let mut iter = SstIterator::new_owned_initialized(
            ..,
            sst_view,
            Arc::clone(&db.table_store),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("expected non-empty SST");
        let entry = iter
            .next()
            .await
            .unwrap()
            .expect("expected at least one entry");
        assert_eq!(entry.key.as_ref(), b"key");
        assert_eq!(entry.value, ValueDeletable::Value(Bytes::from("value")));
        assert_eq!(entry.seq, 1);
        assert!(
            iter.next().await.unwrap().is_none(),
            "expected exactly one entry"
        );

        test.shutdown().await;
    }

    #[tokio::test]
    async fn should_write_sst_to_preallocated_id() {
        // The worker must write each segment SST to the id allocated at
        // dispatch (carried in the job), not mint a fresh one (RFC-0029).
        let db = setup_db(
            "/tmp/test_parallel_l0_flush_uploader_preallocated_id",
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        let job = next_upload_job(&db, b"key", b"value", 1);
        let expected_id = *job
            .segment_sst_ids
            .get(&Bytes::new())
            .expect("empty-prefix id should be pre-allocated");

        let test = start_test_uploader(&db);
        test.submit(job).unwrap();

        let msg = timeout(Duration::from_secs(5), test.tracker_rx.recv())
            .await
            .unwrap()
            .unwrap();
        let TrackerMessage::UploadComplete(event) = msg else {
            panic!("expected UploadComplete");
        };
        assert_eq!(event.segments.len(), 1);
        assert_eq!(
            event.segments[0].sst_handle.id,
            SsTableId::from(expected_id)
        );

        test.shutdown().await;
    }

    #[tokio::test]
    async fn should_apply_flush_cache_policy() {
        let cache = Arc::new(TestCache::new());
        let db = setup_db_with_cache_policy(
            "/tmp/test_parallel_l0_flush_cache_policy",
            Arc::new(FailPointRegistry::new()),
            None,
            Some(cache.clone()),
            BlockCachePolicy::default().with_flush_targets(&[CacheTarget::Filters]),
        )
        .await;
        let job = next_upload_job(&db, b"key", b"value", 1);
        let test = start_test_uploader(&db);

        test.submit(job).unwrap();
        let msg = timeout(Duration::from_secs(5), test.tracker_rx.recv())
            .await
            .unwrap()
            .unwrap();
        let TrackerMessage::UploadComplete(event) = msg else {
            panic!("expected UploadComplete");
        };
        let handle = &event.segments[0].sst_handle;
        let filter_key: CachedKey = (handle.id, handle.info.filter_offset).into();

        assert!(cache.get_filter(&filter_key).await.unwrap().is_some());
        assert_eq!(cache.entry_count(), 1);
        test.shutdown().await;
    }

    #[tokio::test]
    async fn should_retry_upload_failures_until_success() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        fail_parallel::cfg(
            Arc::clone(&fp_registry),
            "write-compacted-sst-io-error",
            "1*return->off",
        )
        .unwrap();
        let db = setup_db("/tmp/test_parallel_l0_flush_uploader_retry", fp_registry).await;
        let job = next_upload_job(&db, b"key", b"value", 1);

        let test = start_test_uploader(&db);
        let started = std::time::Instant::now();
        test.submit(job).unwrap();

        let msg = timeout(Duration::from_secs(5), test.tracker_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(
            started.elapsed() >= db.settings.manifest_poll_interval,
            "expected the job to wait out at least one retry backoff before succeeding"
        );
        let TrackerMessage::UploadComplete(event) = msg else {
            panic!("expected UploadComplete");
        };
        assert_eq!(event.first_seq, 1);
        assert_eq!(event.last_seq, 1);

        test.shutdown().await;
    }

    #[tokio::test]
    async fn should_retry_the_whole_job_when_a_later_segment_fails() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        // First segment's write succeeds; second segment's write fails once;
        // every write after that succeeds, including the retried first
        // segment.
        fail_parallel::cfg(
            Arc::clone(&fp_registry),
            "write-compacted-sst-io-error",
            "1*off->1*return->off",
        )
        .unwrap();
        let db = setup_db_with_extractor(
            "/tmp/test_parallel_l0_flush_uploader_multi_segment_retry",
            fp_registry,
            Some(Arc::new(FixedThreeBytePrefixExtractor)),
        )
        .await;
        {
            let mut guard = db.state.write();
            for (key, value, seq) in [(&b"aaa-1"[..], b"v1", 1), (&b"bbb-1"[..], b"v2", 2)] {
                guard.memtable().put(RowEntry::new_value(key, value, seq));
            }
            guard
                .memtable()
                .table()
                .record_touched_segments(std::collections::BTreeSet::from([
                    Bytes::from_static(b"aaa"),
                    Bytes::from_static(b"bbb"),
                ]));
            guard.freeze_memtable(0);
        }
        let imm_memtable = db
            .state
            .read()
            .state()
            .imm_memtable
            .front()
            .cloned()
            .unwrap();
        let segment_sst_ids = preallocate_ids(&imm_memtable);
        let job = UploadJob::new(imm_memtable, segment_sst_ids);

        let test = start_test_uploader(&db);
        test.submit(job).unwrap();

        let msg = timeout(Duration::from_secs(5), test.tracker_rx.recv())
            .await
            .unwrap()
            .unwrap();
        let TrackerMessage::UploadComplete(event) = msg else {
            panic!("expected UploadComplete");
        };
        let prefixes: Vec<&[u8]> = event.segments.iter().map(|s| s.prefix.as_ref()).collect();
        assert_eq!(prefixes, vec![&b"aaa"[..], &b"bbb"[..]]);
        for segment in &event.segments {
            db.table_store
                .open_sst(&segment.sst_handle.id, Some(segment.prefix.clone()))
                .await
                .expect("uploaded SST should be readable");
        }

        test.shutdown().await;
    }

    #[tokio::test]
    async fn should_rebuild_from_memtable_when_a_mid_sst_block_write_fails() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        fail_parallel::cfg(
            Arc::clone(&fp_registry),
            "write-compacted-sst-io-error",
            "1*off->1*return->off",
        )
        .unwrap();
        let db = setup_db(
            "/tmp/test_parallel_l0_flush_uploader_mid_block_retry",
            fp_registry,
        )
        .await;

        let value = vec![b'x'; 1024];
        let entry_count = 32u64;
        {
            let mut guard = db.state.write();
            for seq in 1..=entry_count {
                let key = format!("key-{seq:04}");
                guard
                    .memtable()
                    .put(RowEntry::new_value(key.as_bytes(), &value, seq));
            }
            guard.freeze_memtable(0);
        }
        let imm_memtable = db
            .state
            .read()
            .state()
            .imm_memtable
            .front()
            .cloned()
            .unwrap();
        let segment_sst_ids = preallocate_ids(&imm_memtable);
        let job = UploadJob::new(imm_memtable, segment_sst_ids);

        let test = start_test_uploader(&db);
        let started = std::time::Instant::now();
        test.submit(job).unwrap();

        let msg = timeout(Duration::from_secs(5), test.tracker_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(
            started.elapsed() >= db.settings.manifest_poll_interval,
            "expected a mid-SST block write to fail and force one retry backoff"
        );
        let TrackerMessage::UploadComplete(event) = msg else {
            panic!("expected UploadComplete");
        };
        assert_eq!(event.first_seq, 1);
        assert_eq!(event.last_seq, entry_count);
        assert_eq!(event.segments.len(), 1);

        let sst_view = SsTableView::identity(
            db.table_store
                .open_sst(&event.segments[0].sst_handle.id, Some(Bytes::new()))
                .await
                .expect("rebuilt SST should be readable"),
        );
        let mut iter = SstIterator::new_owned_initialized(
            ..,
            sst_view,
            Arc::clone(&db.table_store),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("expected non-empty SST");
        let mut seen = 0u64;
        while let Some(entry) = iter.next().await.unwrap() {
            seen += 1;
            assert_eq!(entry.key.as_ref(), format!("key-{seen:04}").as_bytes());
            assert_eq!(entry.seq, seen);
        }
        assert_eq!(seen, entry_count);

        test.shutdown().await;
    }

    #[tokio::test]
    async fn should_emit_fatal_event_for_build_failure() {
        let db = setup_db(
            "/tmp/test_parallel_l0_flush_uploader_build_failure",
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        {
            let mut guard = db.state.write();
            guard
                .memtable()
                .put(RowEntry::new_merge(b"key", b"merge_operand", 1));
            guard.freeze_memtable(0);
        }
        let imm_memtable = db
            .state
            .read()
            .state()
            .imm_memtable
            .front()
            .cloned()
            .unwrap();
        let segment_sst_ids = preallocate_ids(&imm_memtable);
        let job = UploadJob::new(imm_memtable, segment_sst_ids);

        let test = start_test_uploader(&db);
        test.submit(job).unwrap();

        // The worker returns a fatal error. Wait for the executor to
        // propagate it to closed_result.
        let result = timeout(Duration::from_secs(5), test.await_closed())
            .await
            .expect("timed out waiting for fatal error");
        assert!(result.is_err());
        assert!(
            !matches!(result, Err(SlateDBError::Closed)),
            "expected specific error, got Closed"
        );
    }

    #[tokio::test]
    async fn should_process_multiple_jobs() {
        let db = setup_db(
            "/tmp/test_parallel_l0_flush_uploader_multiple",
            Arc::new(FailPointRegistry::new()),
        )
        .await;
        let job1 = next_upload_job(&db, b"key1", b"value1", 1);
        let job2 = next_upload_job(&db, b"key2", b"value2", 2);

        let test = start_test_uploader(&db);
        test.submit(job1).unwrap();
        test.submit(job2).unwrap();

        // Collect both upload completions.
        let mut uploaded_seqs = Vec::new();
        for _ in 0..2 {
            let msg = timeout(Duration::from_secs(5), test.tracker_rx.recv())
                .await
                .unwrap()
                .unwrap();
            if let TrackerMessage::UploadComplete(uploaded) = msg {
                uploaded_seqs.push(uploaded.last_seq);
            }
        }
        uploaded_seqs.sort();
        assert_eq!(uploaded_seqs, vec![1, 2]);

        test.shutdown().await;
    }

    #[tokio::test]
    async fn submit_should_fail_after_worker_fatal_error() {
        let db = setup_db(
            "/tmp/test_parallel_l0_flush_uploader_submit_after_fatal",
            Arc::new(FailPointRegistry::new()),
        )
        .await;

        // Create a merge entry without a merge operator — this causes a
        // fatal build error in the worker.
        {
            let mut guard = db.state.write();
            guard
                .memtable()
                .put(RowEntry::new_merge(b"key", b"operand", 1));
            guard.freeze_memtable(0);
        }
        let imm_memtable = db
            .state
            .read()
            .state()
            .imm_memtable
            .front()
            .cloned()
            .unwrap();
        let segment_sst_ids = preallocate_ids(&imm_memtable);
        let bad_job = UploadJob::new(imm_memtable, segment_sst_ids);

        let test = start_test_uploader(&db);
        test.submit(bad_job).unwrap();

        // Wait for the executor to propagate the error to closed_result.
        let result = timeout(Duration::from_secs(5), test.await_closed())
            .await
            .expect("timed out waiting for fatal error");
        assert!(result.is_err());

        // Subsequent submits should fail with the specific error.
        let err = test
            .submit(next_upload_job(&db, b"key2", b"value2", 2))
            .unwrap_err();
        assert!(
            !matches!(err, SlateDBError::Closed),
            "expected specific error, got Closed"
        );
    }

    #[tokio::test]
    async fn should_stop_retrying_on_shutdown_when_wal_enabled() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        fail_parallel::cfg(
            Arc::clone(&fp_registry),
            "write-compacted-sst-io-error",
            "return",
        )
        .unwrap();
        let db = setup_db(
            "/tmp/test_parallel_l0_flush_uploader_shutdown_retry",
            fp_registry,
        )
        .await;
        assert!(db.wal_enabled);
        let job = next_upload_job(&db, b"key", b"value", 1);

        let test = start_test_uploader(&db);
        test.submit(job).unwrap();

        // Mark the database as closed (simulates Db::close()).
        db.status_manager.write_result(Ok(()));

        // The uploader should give up retrying and report the error.
        let result = timeout(Duration::from_secs(5), test.await_closed())
            .await
            .expect("uploader should stop retrying on shutdown");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_emit_one_segment_handle_per_prefix() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let db = setup_db_with_extractor(
            "/tmp/test_parallel_l0_flush_uploader_multi_segment",
            fp_registry,
            Some(Arc::new(FixedThreeBytePrefixExtractor)),
        )
        .await;
        // Stage three segments worth of entries in a single memtable, then
        // freeze it. The uploader should emit a single UploadComplete with
        // three SegmentedSstHandles, sorted by prefix.
        {
            let mut guard = db.state.write();
            for (key, value, seq) in [
                (&b"aaa-1"[..], b"v1", 1),
                (&b"aaa-2"[..], b"v2", 2),
                (&b"bbb-1"[..], b"v3", 3),
                (&b"ccc-1"[..], b"v4", 4),
            ] {
                guard.memtable().put(RowEntry::new_value(key, value, seq));
            }
            // The production write path stamps these inline; this
            // test bypasses that, so record explicitly.
            guard
                .memtable()
                .table()
                .record_touched_segments(std::collections::BTreeSet::from([
                    Bytes::from_static(b"aaa"),
                    Bytes::from_static(b"bbb"),
                    Bytes::from_static(b"ccc"),
                ]));
            guard.freeze_memtable(0);
        }
        let imm_memtable = db
            .state
            .read()
            .state()
            .imm_memtable
            .front()
            .cloned()
            .unwrap();
        let segment_sst_ids = preallocate_ids(&imm_memtable);
        let job = UploadJob::new(imm_memtable, segment_sst_ids);

        let test = start_test_uploader(&db);
        test.submit(job).unwrap();

        let msg = timeout(Duration::from_secs(5), test.tracker_rx.recv())
            .await
            .unwrap()
            .unwrap();
        let TrackerMessage::UploadComplete(uploaded) = msg else {
            panic!("expected UploadComplete");
        };
        assert_eq!(uploaded.first_seq, 1);
        assert_eq!(uploaded.last_seq, 4);
        let prefixes: Vec<&[u8]> = uploaded
            .segments
            .iter()
            .map(|s| s.prefix.as_ref())
            .collect();
        assert_eq!(prefixes, vec![&b"aaa"[..], &b"bbb"[..], &b"ccc"[..]]);

        // Each SST exists in object storage with a unique id.
        let mut ids = std::collections::HashSet::new();
        for segment in &uploaded.segments {
            db.table_store
                .open_sst(&segment.sst_handle.id, Some(segment.prefix.clone()))
                .await
                .expect("uploaded SST should be readable");
            assert!(ids.insert(segment.sst_handle.id));
        }

        test.shutdown().await;
    }

    #[tokio::test]
    async fn should_stop_retrying_multi_segment_job_on_shutdown_when_wal_enabled() {
        // A multi-segment job streams and uploads all of its segments as one
        // unit, retried as a whole on failure. Configure the fail point to
        // fail every write and verify the single retry loop still observes
        // the shutdown signal and gives up, rather than retrying forever.
        let fp_registry = Arc::new(FailPointRegistry::new());
        fail_parallel::cfg(
            Arc::clone(&fp_registry),
            "write-compacted-sst-io-error",
            "return",
        )
        .unwrap();
        let db = setup_db_with_extractor(
            "/tmp/test_parallel_l0_flush_uploader_multi_segment_shutdown",
            fp_registry,
            Some(Arc::new(FixedThreeBytePrefixExtractor)),
        )
        .await;
        assert!(db.wal_enabled);
        // Stage entries that route to three distinct segments.
        {
            let mut guard = db.state.write();
            for (key, value, seq) in [
                (&b"aaa-1"[..], b"v1", 1),
                (&b"bbb-1"[..], b"v2", 2),
                (&b"ccc-1"[..], b"v3", 3),
            ] {
                guard.memtable().put(RowEntry::new_value(key, value, seq));
            }
            // The production write path stamps these inline; this test
            // bypasses that, so record explicitly.
            guard
                .memtable()
                .table()
                .record_touched_segments(std::collections::BTreeSet::from([
                    Bytes::from_static(b"aaa"),
                    Bytes::from_static(b"bbb"),
                    Bytes::from_static(b"ccc"),
                ]));
            guard.freeze_memtable(0);
        }
        let imm_memtable = db
            .state
            .read()
            .state()
            .imm_memtable
            .front()
            .cloned()
            .unwrap();
        let segment_sst_ids = preallocate_ids(&imm_memtable);
        let job = UploadJob::new(imm_memtable, segment_sst_ids);

        let test = start_test_uploader(&db);
        test.submit(job).unwrap();

        // Mark the database as closed (simulates Db::close()). Every
        // in-flight per-segment upload future should bail out.
        db.status_manager.write_result(Ok(()));

        let result = timeout(Duration::from_secs(5), test.await_closed())
            .await
            .expect("uploader should stop retrying on shutdown");
        assert!(result.is_err());
    }
}
