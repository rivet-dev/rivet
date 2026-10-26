//! This module provides the core database functionality for SlateDB.
//! It provides methods for reading and writing to the database, as well as for flushing the database to disk.
//!
//! The `Db` struct represents a database.
//!
//! # Examples
//!
//! Basic usage of the `Db` struct:
//!
//! ```
//! use slatedb::{Db, Error};
//! use slatedb::object_store::memory::InMemory;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let object_store = Arc::new(InMemory::new());
//!     let db = Db::open("test_db", object_store).await?;
//!     Ok(())
//! }
//! ```

pub use crate::db_status::{DbStatus, SegmentPrefix};

use crate::db_cache::CacheTarget;
use std::fmt;
use std::future::Future;
use std::sync::Arc;

use bytes::Bytes;
use fail_parallel::{fail_point, FailPointRegistry};
use object_store::path::Path;
use object_store::{parse_url_opts, ObjectStore};

use crate::compactor::COMPACTOR_TASK_NAME;
use crate::db_transaction::DbTransaction;
use crate::dispatcher::MessageHandlerExecutor;
use crate::garbage_collector::GC_TASK_NAME;
use crate::transaction_manager::IsolationLevel;
use crate::{db_cache_manager, CloseReason};
use log::{debug, info, trace, warn};
use parking_lot::RwLock;
use std::time::Duration;

use crate::batch::WriteBatch;
use crate::batch_write::{BatchWriterMessage, WriteBatchRequest, WRITE_BATCH_TASK_NAME};
use crate::bytes_range::{ByteRangeBounds, BytesRange};
use crate::cached_object_store::CachedObjectStore;
use crate::clock::MonotonicClock;
use crate::config::{
    CloseOptions, FlushOptions, FlushType, MergeOptions, PutOptions, ReadOptions, ScanOptions,
    Settings, WriteOptions,
};
use crate::db_common::extract_segment_prefix;
use crate::db_iter::{DbIterator, DbRecencyIterator};
use crate::db_snapshot::DbSnapshot;
use crate::db_state::{collect_touched_segments, DbState, SsTableHandle, SsTableId};
use crate::db_stats::DbStats;
use crate::error::SlateDBError;
use crate::manifest::{Manifest, VersionedManifest};
use crate::mem_table::KVTableMetadata;
use crate::memtable_flusher::{FlushResult, FlushTarget, MemtableFlusher};
use crate::merge_operator::{instrument_merge_operator, MergeOperatorType};
use crate::oracle::{DbOracle, Oracle};
use crate::paths::PathResolver;
use crate::prefix_extractor::PrefixExtractor;
use crate::reader::{Reader, ScanContext};
use crate::snapshot_manager::SnapshotManager;
use crate::tablestore::TableStore;
use crate::transaction_manager::TransactionManager;
use crate::types::KeyValue;
use crate::utils::{format_bytes_si, SafeSender, WatchableOnceCellReader};
use crate::wal_replay::{WalReplayIterator, WalReplayOptions};
use crate::{DbCacheManagerOps, DbMetadataOps, DbReadOps, DbWriteOps};
use slatedb_common::clock::SystemClock;
use slatedb_common::metrics::MetricsRecorderHelper;
use slatedb_common::DbRand;
use slatedb_txn_obj::DirtyObject;

use crate::db_status::{ClosedResultWriter, DbStatusManager, DurabilityWaiter};
use crate::wal::{WalEvent, WalIterator, WalObserver, WalStatus};
pub use builder::DbBuilder;
pub use builder::DbReaderBuilder;

pub(crate) mod builder;

pub(crate) struct DbInner {
    pub(crate) state: Arc<RwLock<DbState>>,
    pub(crate) settings: Settings,
    pub(crate) table_store: Arc<TableStore>,
    pub(crate) memtable_flusher: Arc<MemtableFlusher>,
    pub(crate) write_notifier: SafeSender<BatchWriterMessage>,
    pub(crate) db_stats: DbStats,
    /// Kept alive so the underlying `MetricsRecorder` is not dropped while
    /// metric handles in `DbStats` (and other stats structs) are still in use.
    /// See: https://github.com/slatedb/slatedb/issues/1469
    #[allow(dead_code)]
    pub(crate) recorder: MetricsRecorderHelper,
    #[allow(dead_code)]
    pub(crate) fp_registry: Arc<FailPointRegistry>,
    /// A clock which is guaranteed to be monotonic. it's previous value is
    /// stored in the manifest and WAL, will be updated after WAL replay.
    pub(crate) mono_clock: Arc<MonotonicClock>,
    pub(crate) system_clock: Arc<dyn SystemClock>,
    pub(crate) rand: Arc<DbRand>,
    pub(crate) oracle: Arc<DbOracle>,
    pub(crate) flush_merge_operator: Option<MergeOperatorType>,
    pub(crate) reader: Reader,
    /// [`wal_observer`] inspects the status of WAL buffer. The WAL buffer itself is owned by
    /// the batch write task.
    pub(crate) wal_observer: DbWalObserver,
    pub(crate) wal_enabled: bool,
    /// [`txn_manager`] tracks all the live transactions and related metadata.
    pub(crate) txn_manager: Arc<TransactionManager>,
    pub(crate) snapshot_manager: Arc<SnapshotManager>,
    pub(crate) status_manager: Arc<DbStatusManager>,
    /// Segment extractor (RFC-0024). When `Some`, the writer routes every
    /// key through this extractor and groups flush output into per-segment
    /// L0 SSTs. When `None`, the database is the singleton `prefix=""`
    /// segment encoded in the manifest's top-level tree.
    pub(crate) segment_extractor: Option<Arc<dyn PrefixExtractor>>,
}

impl DbInner {
    pub(crate) async fn new(
        settings: Settings,
        system_clock: Arc<dyn SystemClock>,
        rand: Arc<DbRand>,
        table_store: Arc<TableStore>,
        manifest: DirtyObject<Manifest>,
        memtable_flusher: Arc<MemtableFlusher>,
        write_notifier: SafeSender<BatchWriterMessage>,
        wal_observer: Box<dyn WalObserver>,
        recorder: MetricsRecorderHelper,
        fp_registry: Arc<FailPointRegistry>,
        merge_operator: Option<MergeOperatorType>,
        status_manager: Arc<DbStatusManager>,
        segment_extractor: Option<Arc<dyn PrefixExtractor>>,
    ) -> Result<Self, SlateDBError> {
        // both last_seq and last_committed_seq will be updated after WAL replay.
        let last_l0_seq = manifest.value.core.last_l0_seq;
        let oracle = Arc::new(DbOracle::new(
            last_l0_seq,
            last_l0_seq,
            last_l0_seq,
            status_manager.clone(),
        ));

        let mono_clock = Arc::new(MonotonicClock::new(
            system_clock.clone(),
            manifest.value.core.last_l0_clock_tick,
        ));

        // state are mostly manifest, including IMM, L0, etc.
        let db_state = DbState::new(manifest);
        let state = Arc::new(RwLock::new(db_state));

        let db_stats = DbStats::new(&recorder);
        let wal_enabled = DbInner::wal_enabled_in_options(&settings);
        let flush_merge_operator = merge_operator.clone().map(|merge_operator| {
            instrument_merge_operator(
                merge_operator,
                db_stats.merge_operator_flush_operands.clone(),
            )
        });

        let reader = Reader::new(
            table_store.clone(),
            db_stats.clone(),
            mono_clock.clone(),
            oracle.clone(),
            merge_operator.clone(),
        );

        let txn_manager = Arc::new(TransactionManager::new(oracle.clone(), rand.clone()));
        let snapshot_manager = Arc::new(SnapshotManager::new(oracle.clone(), rand.clone()));
        let wal_observer = DbWalObserver::new(
            wal_observer,
            oracle.clone(),
            state.clone(),
            status_manager.clone(),
        );

        let db_inner = Self {
            state,
            settings,
            memtable_flusher,
            oracle,
            wal_enabled,
            table_store,
            wal_observer,
            write_notifier,
            db_stats,
            mono_clock,
            system_clock,
            rand,
            flush_merge_operator,
            recorder,
            fp_registry,
            reader,
            txn_manager,
            snapshot_manager,
            status_manager,
            segment_extractor,
        };
        Ok(db_inner)
    }

    /// Get the value for a given key.
    pub(crate) async fn get_with_options<K: AsRef<[u8]>>(
        &self,
        key: K,
        options: &ReadOptions,
    ) -> Result<Option<Bytes>, SlateDBError> {
        self.get_key_value_with_options(key, options)
            .await
            .map(|kv_opt| kv_opt.map(|kv| kv.value))
    }

    /// Get the full row entry for a given key.
    pub(crate) async fn get_key_value_with_options<K: AsRef<[u8]>>(
        &self,
        key: K,
        options: &ReadOptions,
    ) -> Result<Option<KeyValue>, SlateDBError> {
        self.check_closed()?;
        let db_state = self.state.read().view();
        self.reader
            .get_key_value_with_options(key, options, &db_state, None, None)
            .await
    }

    /// Shared scan path for plain range scans and prefix scans. When
    /// `prefix` is set, every key in `range` starts with it and prefix
    /// bloom filters are consulted to skip non-matching SSTs.
    pub(crate) async fn scan_with_options(
        &self,
        range: BytesRange,
        options: &ScanOptions,
        prefix: Option<Bytes>,
    ) -> Result<DbIterator, SlateDBError> {
        self.check_closed()?;
        let db_state = self.state.read().view();
        self.reader
            .scan_with_options(
                range,
                options,
                ScanContext {
                    db_state: &db_state,
                    write_batch_iter: None,
                    max_seq: None,
                    prefix,
                },
            )
            .await
    }

    pub(crate) async fn scan_prefix_by_recency_with_options(
        &self,
        prefix: Bytes,
        options: &ScanOptions,
    ) -> Result<DbRecencyIterator, SlateDBError> {
        self.check_closed()?;
        let db_state = self.state.read().view();
        self.reader
            .scan_prefix_by_recency_with_options(prefix, options, &db_state)
            .await
    }

    #[allow(unused_variables)]
    pub(crate) fn wal_enabled_in_options(settings: &Settings) -> bool {
        #[cfg(feature = "wal_disable")]
        return settings.wal_enabled;
        #[cfg(not(feature = "wal_disable"))]
        return true;
    }

    pub(crate) async fn write_with_options(
        &self,
        batch: WriteBatch,
        options: &WriteOptions,
        txn: Option<DbTransaction>,
    ) -> Result<WriteHandle, SlateDBError> {
        self.db_stats.write_batch_count.increment(1);
        self.db_stats.write_ops.increment(batch.op_count() as u64);
        self.check_closed()?;
        if batch.ops.is_empty() {
            return Err(SlateDBError::EmptyBatch);
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        let batch_msg = BatchWriterMessage::WriteBatch(WriteBatchRequest {
            batch,
            options: options.clone(),
            done: tx,
            txn,
        });

        self.maybe_apply_backpressure().await?;
        self.write_notifier.send(batch_msg)?;

        rx.await?
    }

    #[inline]
    pub(crate) async fn maybe_apply_backpressure(&self) -> Result<(), SlateDBError> {
        loop {
            self.check_closed()?;
            let wal_status = self.wal_observer.status()?;
            let (active_memtable_size_bytes, imm_memtable_size_bytes) = {
                let guard = self.state.read();
                let estimate = |metadata: KVTableMetadata| {
                    self.table_store.estimate_encoded_size_compacted(
                        metadata.entry_num,
                        metadata.entries_size_in_bytes,
                    )
                };
                let active_memtable_size_bytes = estimate(guard.memtable().table().metadata());
                let imm_memtable_size_bytes = guard
                    .state()
                    .imm_memtable
                    .iter()
                    .map(|imm| estimate(imm.table().metadata()))
                    .fold(0usize, |total, size| total.saturating_add(size));
                (active_memtable_size_bytes, imm_memtable_size_bytes)
            };
            let total_mem_size_bytes = active_memtable_size_bytes
                .saturating_add(imm_memtable_size_bytes)
                .saturating_add(wal_status.estimated_bytes);
            self.db_stats
                .total_mem_size_bytes
                .set(total_mem_size_bytes as i64);

            trace!(
                "checking backpressure [total_mem_size_bytes={}, active_memtable_size_bytes={}, imm_memtable_size_bytes={}, wal_size_bytes={}, max_unflushed_bytes={}]",
                format_bytes_si(total_mem_size_bytes as u64),
                format_bytes_si(active_memtable_size_bytes as u64),
                format_bytes_si(imm_memtable_size_bytes as u64),
                format_bytes_si(wal_status.estimated_bytes as u64),
                format_bytes_si(self.settings.max_unflushed_bytes as u64),
            );

            if total_mem_size_bytes >= self.settings.max_unflushed_bytes {
                // Every waiting write logs each pass, so a slow store would flood the log at
                // warn. The counters carry the signal.
                self.db_stats.backpressure_count.increment(1);
                debug!(
                    "unflushed WAL and memtable size exceeds max_unflushed_bytes. applying backpressure. [total_mem_size_bytes={}, active_memtable_size_bytes={}, imm_memtable_size_bytes={}, wal_size_bytes={}, max_unflushed_bytes={}]",
                    format_bytes_si(total_mem_size_bytes as u64),
                    format_bytes_si(active_memtable_size_bytes as u64),
                    format_bytes_si(imm_memtable_size_bytes as u64),
                    format_bytes_si(wal_status.estimated_bytes as u64),
                    format_bytes_si(self.settings.max_unflushed_bytes as u64),
                );

                let maybe_oldest_unflushed_memtable = {
                    let guard = self.state.read();
                    guard.state().imm_memtable.back().cloned()
                };

                // There is a window of time after total_mem_size_bytes is larger than
                // max_unflushed_bytes but before we get the memtable. During that time, if
                // the memtable and WAL are fully flushed out, we should short circuit to
                // avoid blocking indefinitely.
                if maybe_oldest_unflushed_memtable.is_none() && wal_status.estimated_bytes == 0 {
                    continue;
                }

                let await_memtable_uploaded = async {
                    if let Some(oldest_unflushed_memtable) = maybe_oldest_unflushed_memtable {
                        oldest_unflushed_memtable.await_uploaded().await
                    } else {
                        std::future::pending().await
                    }
                };

                let await_flush_wal = self
                    .wal_observer
                    .wait_until_wal_flushed(wal_status.last_flushed_wal_id);

                let timeout_fut = self.system_clock.sleep(Duration::from_secs(30));
                let await_closed = async {
                    let mut watcher = self.status_manager.result_reader();
                    match watcher.await_value().await {
                        Ok(()) => Err(SlateDBError::Closed),
                        Err(e) => Err(e),
                    }
                };

                tokio::select! {
                    biased;

                    result = await_closed => result?,
                    result = await_memtable_uploaded => result?,
                    result = await_flush_wal => result?,
                    _ = timeout_fut => {
                        self.db_stats.backpressure_timeout_count.increment(1);
                        debug!("backpressure timeout: waited 30s, no memtable/WAL flushed yet");
                    }
                };
            } else {
                break;
            }
        }
        Ok(())
    }

    async fn flush_imm_memtables(&self, target: FlushTarget) -> Result<FlushResult, SlateDBError> {
        self.memtable_flusher().flush(target).await
    }

    pub(crate) async fn flush_memtables(
        &self,
        target: FlushTarget,
    ) -> Result<FlushResult, SlateDBError> {
        // flush the batch writer to freeze the active memtable and flush all WALs to unblock
        // memtable flush
        self.request_batch_writer_flush(true).await?;
        self.flush_imm_memtables(target).await
    }

    pub(crate) fn memtable_flusher(&self) -> &MemtableFlusher {
        &self.memtable_flusher
    }

    /// Flush in-memory writes to disk. See [`Db::flush_with_options`] for details.
    ///
    /// `check_status` exists so we can call flush in [`Db::close`] after marking the
    /// database as closed.
    ///
    /// ## Arguments
    /// - `options`: the flush options to use.
    /// - `check_status`: if true, checks the database status before flushing.
    ///
    /// ## Returns
    /// - `Ok(())` if the flush was successful.
    /// - `Err(SlateDBError)` if there was an error flushing the database. If
    ///   `check_status` is true, this may return `SlateDBError::Closed` if the database
    ///   has already been closed.
    pub(crate) async fn flush(
        &self,
        options: FlushOptions,
        check_status: bool,
    ) -> Result<(), SlateDBError> {
        self.db_stats.flush_requests.increment(1);
        if check_status {
            self.check_closed()?;
        }
        match options.flush_type {
            FlushType::Wal => {
                if !self.wal_enabled {
                    return Err(SlateDBError::WalDisabled);
                }
                self.request_batch_writer_flush(false).await
            }
            FlushType::MemTable => self.flush_memtables(FlushTarget::All).await.map(|_| ()),
        }
    }

    async fn replay_wal(&self, wal_iterator: Box<dyn WalIterator>) -> Result<(), SlateDBError> {
        let mut current_memtable_wal_id = self
            .state
            .read()
            .state()
            .manifest
            .value
            .core
            .replay_after_wal_id;
        let writer_epoch = self.state.read().state().manifest.value.writer_epoch;
        fail_point!(
            Arc::clone(&self.fp_registry),
            "replay-wal-pause",
            writer_epoch == 1,
            |_| -> Result<(), SlateDBError> { Ok(()) }
        );

        let replay_options = WalReplayOptions {
            max_memtable_bytes: self.settings.l0_sst_size_bytes,
            ..Default::default()
        };

        let db_state = self.state.read().state().core().clone();
        let mut replay_iter = WalReplayIterator::for_wal_iterator(
            wal_iterator,
            &db_state,
            replay_options,
            Arc::clone(&self.table_store),
        )?;

        loop {
            let replayed_table = match replay_iter.next().await {
                Ok(Some(replayed_table)) => replayed_table,
                Ok(None) => break,
                // If the manifest or an SST referenced by the WAL is missing, it may
                // indicate that a newer writer has advanced `replay_after_wal_id` and
                // the GC has removed this WAL entry. Check the latest manifest's
                // writer_epoch to see if this client is fenced.
                Err(SlateDBError::WalTruncated(wal_id)) => {
                    self.memtable_flusher.refresh_manifest().await?;
                    if self.state.read().state().manifest.value.writer_epoch > writer_epoch {
                        return Err(SlateDBError::Fenced);
                    }
                    return Err(SlateDBError::WalTruncated(wal_id));
                }
                Err(err) => return Err(err),
            };

            // RFC-0024: re-extract each replayed entry's prefix to
            // populate the memtable's touched-segment set. Per the
            // validation model, durable WAL entries were validated when
            // the writer accepted them, so we do not re-run the
            // antichain check here. An empty/absent prefix under
            // the current extractor remains a hard error — the
            // entry can't be routed to any segment.
            if let Some(extractor) = self.segment_extractor.as_ref() {
                let mut touched_segments: std::collections::BTreeSet<Bytes> =
                    std::collections::BTreeSet::new();
                let mut iter = replayed_table.table.table().iter();
                while let Some(entry) = iter.next_sync() {
                    touched_segments
                        .insert(extract_segment_prefix(extractor.as_ref(), &entry.key)?);
                }
                replayed_table
                    .table
                    .record_touched_segments(touched_segments);
            }
            // Replayed rows come from WAL SSTs in remote storage, so they are already
            // durable. Update `last_remote_persisted_seq` before replaying to avoid a race with
            // the memtable flusher. The flusher calls flush_wals() to guarantee all data in the
            // memtable is already durable in the WAL. Since we're replaying, the WAL is empty and
            // `last_remote_persisted_seq` does not get updated; it remains at l0_last_seq. This
            // would cause the flusher's assertion that the remote persisted seq is always >= the
            // last seq in the memtable to fail. By updating `last_remote_persisted_seq` here, we
            // ensure the assertion holds true.
            assert!(self.oracle.last_remote_persisted_seq() <= replayed_table.last_seq);
            self.oracle.advance_durable_seq(replayed_table.last_seq);
            self.maybe_freeze_memtable(current_memtable_wal_id);
            self.maybe_apply_backpressure().await?;
            let replayed_table_last_wal_id = replayed_table.last_wal_id;
            self.replay_memtable(current_memtable_wal_id, replayed_table)?;
            current_memtable_wal_id = replayed_table_last_wal_id;
        }

        let guard = self.state.read();
        self.status_manager
            .report_memtable_segments(collect_touched_segments(&guard.view()));

        Ok(())
    }

    async fn preload_cache(
        &self,
        cached_obj_store: &CachedObjectStore,
        path_resolver: &PathResolver,
    ) -> Result<(), SlateDBError> {
        let state = self.state.read().state();
        let cache_opts = &self.settings.object_store_cache_options;
        crate::utils::preload_cache_from_manifest(
            &state.manifest.value.core,
            cached_obj_store,
            path_resolver,
            cache_opts.preload_disk_cache_on_startup,
            cache_opts.max_cache_size_bytes.unwrap_or(usize::MAX),
        )
        .await
    }

    /// Returns the latest database status snapshot.
    pub(crate) fn status(&self) -> DbStatus {
        self.status_manager.status()
    }

    /// Returns an error if the database has been closed.
    ///
    /// ## Returns
    /// - `Ok(())` if the DB is still open.
    /// - `Err(SlateDBError::Closed)` if the DB was closed successfully
    ///   (state.result_reader() returns Ok(())).
    /// - `Err(e)` if the DB was closed with an error, where `e` is the error
    ///   (state.result_reader() returns Err(e)).
    pub(crate) fn check_closed(&self) -> Result<(), SlateDBError> {
        if let Some(result) = self.status_manager.result_reader().read() {
            return match result {
                Ok(()) => Err(SlateDBError::Closed),
                Err(e) => Err(e),
            };
        }
        Ok(())
    }

    pub(crate) fn manifest(&self) -> VersionedManifest {
        self.state.read().state().manifest.clone().into()
    }
}

#[derive(Clone)]
pub struct Db {
    pub(crate) inner: Arc<DbInner>,
    task_executor: Arc<MessageHandlerExecutor>,
}

impl Db {
    /// Open a new database with default options.
    ///
    /// ## Arguments
    /// - `path`: the path to the database
    /// - `object_store`: the object store to use for the database
    ///
    /// ## Returns
    /// - `Db`: the database
    ///
    /// ## Errors
    /// - `Error`: if there was an error opening the database
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, Error};
    /// use slatedb::object_store::memory::InMemory;
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn open<P: Into<Path>>(
        path: P,
        object_store: Arc<dyn ObjectStore>,
    ) -> Result<Self, crate::Error> {
        // Use the builder API internally
        Self::builder(path, object_store).build().await
    }

    /// Creates a new builder for a database at the given path.
    ///
    /// ## Arguments
    /// - `path`: the path to the database
    /// - `object_store`: the object store to use for the database
    ///
    /// ## Returns
    /// - `DbBuilder`: the builder to initialize the database
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, Error};
    /// use slatedb::object_store::memory::InMemory;
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store = Arc::new(InMemory::new());
    ///     let db = Db::builder("/tmp/test_db", object_store)
    ///         .build()
    ///         .await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn builder<P: Into<Path>>(path: P, object_store: Arc<dyn ObjectStore>) -> DbBuilder<P> {
        DbBuilder::new(path, object_store)
    }

    /// Close the database.
    ///
    /// ## Returns
    /// - `Result<(), Error>`: if there was an error closing the database
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     db.close().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn close(&self) -> Result<(), crate::Error> {
        self.close_with_options(CloseOptions::default()).await
    }

    /// Close the database with custom options.
    ///
    pub async fn close_with_options(&self, options: CloseOptions) -> Result<(), crate::Error> {
        let flush_type = match self.status().close_reason {
            // If already closed, don't close again.
            Some(CloseReason::Clean) => return Err(SlateDBError::Closed.into()),
            // If in failed state, allow close, but don't flush since the database
            // might be in a bad state. Note that multiple close() calls will always
            // run when in a failed state (vs. a clean closure, which will return
            // Error::Closed(CloseReason::Clean) on subsequent calls).
            Some(_) => None,
            // Flush outstanding writes if the database is still open and the
            // caller requested a final flush.
            None => options.flush_type,
        };

        // Mark the database as closed before flushing.
        self.inner.status_manager.write_result(Ok(()));

        let result = if let Some(flush_type) = flush_type {
            self.inner
                .flush(FlushOptions { flush_type }, false)
                .await
                .map_err(Into::into)
                .inspect_err(|e| warn!("failed to flush db during close [error={:?}]", e))
        } else {
            Ok(())
        };

        MemtableFlusher::shutdown(&self.task_executor).await;

        if let Err(e) = self.task_executor.shutdown_task(COMPACTOR_TASK_NAME).await {
            warn!("failed to shutdown compactor task [error={:?}]", e);
        }

        if let Err(e) = self
            .task_executor
            .shutdown_task(crate::compaction_worker::COMPACTION_WORKER_TASK_NAME)
            .await
        {
            warn!("failed to shutdown compaction worker task [error={:?}]", e);
        }

        if let Err(e) = self.task_executor.shutdown_task(GC_TASK_NAME).await {
            warn!("failed to shutdown garbage collector task [error={:?}]", e);
        }

        if let Err(e) = self
            .task_executor
            .shutdown_task(WRITE_BATCH_TASK_NAME)
            .await
        {
            warn!("failed to shutdown writer task [error={:?}]", e);
        }

        if let Err(e) = self.inner.table_store.close_cache().await {
            warn!("failed to close block cache [error={:?}]", e);
        }

        info!("db closed");
        result
    }

    /// Create a snapshot of the database.
    ///
    /// ## Returns
    /// - `Result<Arc<DbSnapshot>, Error>`: the snapshot of the database, it represents
    ///   a consistent view of the database at the time of the snapshot.
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    /// use bytes::Bytes;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///
    ///     // Write some data and create a snapshot
    ///     db.put(b"key1", b"value1").await?;
    ///     let snapshot = db.snapshot().await?;
    ///
    ///     // Snapshot provides read-only access to database state
    ///     let value = snapshot.get(b"key1").await?;
    ///     assert_eq!(value, Some(Bytes::from(b"value1".as_ref())));
    ///
    ///     // Write more data to original database
    ///     db.put(b"key2", b"value2").await?;
    ///
    ///     // Snapshot still sees old state, original db sees new data
    ///     assert_eq!(snapshot.get(b"key2").await?, None);
    ///     assert_eq!(db.get(b"key2").await?, Some(Bytes::from(b"value2".as_ref())));
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn snapshot(&self) -> Result<Arc<DbSnapshot>, crate::Error> {
        self.inner.check_closed()?;
        let snapshot = DbSnapshot::new(self.inner.clone(), None);
        Ok(snapshot)
    }

    /// Get a value from the database with default read options.
    ///
    /// The `Bytes` object returned contains a slice of an entire
    /// 4 KiB block. The block will be held in memory as long as the
    /// caller holds a reference to the `Bytes` object. Consider
    /// copying the data if you need to hold it for a long time.
    ///
    /// ## Arguments
    /// - `key`: the key to get
    ///
    /// ## Returns
    /// - `Result<Option<Bytes>, Error>`:
    ///     - `Some(Bytes)`: the value if it exists
    ///     - `None`: if the value does not exist
    ///
    /// ## Errors
    /// - `Error`: if there was an error getting the value
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     db.put(b"key", b"value").await?;
    ///     assert_eq!(db.get(b"key").await?, Some("value".into()));
    ///     Ok(())
    /// }
    /// ```
    pub async fn get<K: AsRef<[u8]> + Send>(&self, key: K) -> Result<Option<Bytes>, crate::Error> {
        self.get_with_options(key, &ReadOptions::default()).await
    }

    /// Get a value from the database with custom read options.
    ///
    /// The `Bytes` object returned contains a slice of an entire
    /// 4 KiB block. The block will be held in memory as long as the
    /// caller holds a reference to the `Bytes` object. Consider
    /// copying the data if you need to hold it for a long time.
    ///
    /// ## Arguments
    /// - `key`: the key to get
    /// - `options`: the read options to use (Note that [`ReadOptions::read_level`] has no effect for readers, which
    ///   can only observe committed state).
    ///
    /// ## Returns
    /// - `Result<Option<Bytes>, Error>`:
    ///   - `Some(Bytes)`: the value if it exists
    ///   - `None`: if the value does not exist
    ///
    /// ## Errors
    /// - `Error`: if there was an error getting the value
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, config::ReadOptions, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     db.put(b"key", b"value").await?;
    ///     assert_eq!(db.get_with_options(b"key", &ReadOptions::default()).await?, Some("value".into()));
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_with_options<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
        options: &ReadOptions,
    ) -> Result<Option<Bytes>, crate::Error> {
        self.inner
            .get_with_options(key, options)
            .await
            .map_err(Into::into)
    }

    /// Get a key-value pair from the database with default read options.
    ///
    /// Returns the key along with its value and metadata (sequence number,
    /// creation timestamp, expiration timestamp). Unlike [`get`](Self::get),
    /// which returns only the value bytes, this method returns a [`KeyValue`]
    /// that includes row metadata.
    ///
    /// ## Arguments
    /// - `key`: the key to look up
    ///
    /// ## Returns
    /// - `Ok(Some(KeyValue))`: if the key exists and is not deleted/expired
    /// - `Ok(None)`: if the key does not exist or is deleted/expired
    ///
    /// ## Errors
    /// - `Error`: if there was an error reading from the database
    pub async fn get_key_value<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
    ) -> Result<Option<KeyValue>, crate::Error> {
        self.get_key_value_with_options(key, &ReadOptions::default())
            .await
    }

    /// Get a key-value pair from the database with custom read options.
    ///
    /// Returns the key along with its value and metadata (sequence number,
    /// creation timestamp, expiration timestamp). Unlike
    /// [`get_with_options`](Self::get_with_options), which returns only the
    /// value bytes, this method returns a [`KeyValue`] that includes row
    /// metadata.
    ///
    /// ## Arguments
    /// - `key`: the key to look up
    /// - `options`: the read options to use
    ///
    /// ## Returns
    /// - `Ok(Some(KeyValue))`: if the key exists and is not deleted/expired
    /// - `Ok(None)`: if the key does not exist or is deleted/expired
    ///
    /// ## Errors
    /// - `Error`: if there was an error reading from the database
    pub async fn get_key_value_with_options<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
        options: &ReadOptions,
    ) -> Result<Option<KeyValue>, crate::Error> {
        let kv = self
            .inner
            .get_key_value_with_options(key, options)
            .await
            .map_err(crate::Error::from)?;
        Ok(kv)
    }

    /// Scan a range of keys using the default scan options.
    ///
    /// returns a `DbIterator`
    ///
    /// ## Errors
    /// - `Error`: if there was an error scanning the range of keys
    ///
    /// ## Returns
    /// - `Result<DbIterator, Error>`: An iterator with the results of the scan
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     db.put(b"a", b"a_value").await?;
    ///     db.put(b"b", b"b_value").await?;
    ///
    ///     let mut iter = db.scan("a".."b").await?;
    ///     let kv = iter.next().await?.unwrap();
    ///     assert_eq!(kv.key.as_ref(), b"a");
    ///     assert_eq!(kv.value.as_ref(), b"a_value");
    ///     assert_eq!(None, iter.next().await?);
    ///     Ok(())
    /// }
    /// ```
    pub async fn scan<T>(&self, range: T) -> Result<DbIterator, crate::Error>
    where
        T: ByteRangeBounds + Send,
    {
        self.scan_with_options(range, &ScanOptions::default()).await
    }

    /// Scan a range of keys with the provided options.
    ///
    /// returns a `DbIterator`
    ///
    /// ## Errors
    /// - `Error`: if there was an error scanning the range of keys
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, config::ScanOptions, config::DurabilityLevel, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     db.put(b"a", b"a_value").await?;
    ///     db.put(b"b", b"b_value").await?;
    ///
    ///     let mut iter = db.scan_with_options("a".."b", &ScanOptions {
    ///         durability_filter: DurabilityLevel::Memory,
    ///         ..ScanOptions::default()
    ///     }).await?;
    ///     let kv = iter.next().await?.unwrap();
    ///     assert_eq!(kv.key.as_ref(), b"a");
    ///     assert_eq!(kv.value.as_ref(), b"a_value");
    ///     assert_eq!(None, iter.next().await?);
    ///     Ok(())
    /// }
    /// ```
    pub async fn scan_with_options<T>(
        &self,
        range: T,
        options: &ScanOptions,
    ) -> Result<DbIterator, crate::Error>
    where
        T: ByteRangeBounds + Send,
    {
        let start = range.start_bound().map(Bytes::copy_from_slice);
        let end = range.end_bound().map(Bytes::copy_from_slice);
        let range = (start, end);
        self.inner
            .scan_with_options(BytesRange::from(range), options, None)
            .await
            .map_err(Into::into)
    }

    /// Scan keys that share the provided prefix, restricted to `subrange`,
    /// using the default scan options.
    ///
    /// The subrange bounds are key *suffixes* interpreted relative to the
    /// prefix: a bound `s` selects the full key `prefix ++ s`. Pass `..` to
    /// scan the prefix's entire keyspace. When a prefix extractor is
    /// configured, prefix bloom filters are consulted to skip SSTs that
    /// contain no matching keys.
    ///
    /// ## Arguments
    /// - `prefix`: the key prefix to scan
    /// - `subrange`: the range of key suffixes (relative to `prefix`) to
    ///   scan; `..` scans all keys with the prefix
    ///
    /// ## Returns
    /// - `Result<DbIterator, Error>`: An iterator with the results of the scan
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     db.put(b"ab", b"v0").await?;
    ///     db.put(b"aba", b"v1").await?;
    ///     db.put(b"b", b"v2").await?;
    ///
    ///     let mut iter = db.scan_prefix(b"ab", ..).await?;
    ///     let kv = iter.next().await?.unwrap();
    ///     assert_eq!(kv.key.as_ref(), b"ab");
    ///     assert_eq!(kv.value.as_ref(), b"v0");
    ///     let kv = iter.next().await?.unwrap();
    ///     assert_eq!(kv.key.as_ref(), b"aba");
    ///     assert_eq!(kv.value.as_ref(), b"v1");
    ///     assert_eq!(None, iter.next().await?);
    ///
    ///     // Restrict the scan to suffixes from b"a" onward.
    ///     // Ordinary Rust range syntax works here; `as_slice()` is optional.
    ///     let mut iter = db.scan_prefix(b"ab", b"a".as_slice()..).await?;
    ///     let kv = iter.next().await?.unwrap();
    ///     assert_eq!(kv.key.as_ref(), b"aba");
    ///     assert_eq!(None, iter.next().await?);
    ///     Ok(())
    /// }
    /// ```
    pub async fn scan_prefix<P, T>(
        &self,
        prefix: P,
        subrange: T,
    ) -> Result<DbIterator, crate::Error>
    where
        P: AsRef<[u8]> + Send,
        T: ByteRangeBounds + Send,
    {
        self.scan_prefix_with_options(prefix, subrange, &ScanOptions::default())
            .await
    }

    /// Scan keys that share the provided prefix, restricted to `subrange`,
    /// with custom options. See [`Self::scan_prefix`] for the subrange
    /// semantics.
    ///
    /// ## Arguments
    /// - `prefix`: the key prefix to scan
    /// - `subrange`: the range of key suffixes (relative to `prefix`) to
    ///   scan; `..` scans all keys with the prefix
    /// - `options`: the scan options to use
    ///
    /// ## Returns
    /// - `Result<DbIterator, Error>`: An iterator with the results of the scan
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, Error};
    /// use slatedb::config::ScanOptions;
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     db.put(b"x1", b"v1").await?;
    ///     db.put(b"x2", b"v2").await?;
    ///     db.put(b"y", b"v3").await?;
    ///
    ///     let options = ScanOptions {
    ///         cache_blocks: false,
    ///         ..ScanOptions::default()
    ///     };
    ///     let mut iter = db.scan_prefix_with_options(b"x", .., &options).await?;
    ///     let kv = iter.next().await?.unwrap();
    ///     assert_eq!(kv.key.as_ref(), b"x1");
    ///     assert_eq!(kv.value.as_ref(), b"v1");
    ///     let kv = iter.next().await?.unwrap();
    ///     assert_eq!(kv.key.as_ref(), b"x2");
    ///     assert_eq!(kv.value.as_ref(), b"v2");
    ///     assert_eq!(None, iter.next().await?);
    ///     Ok(())
    /// }
    /// ```
    pub async fn scan_prefix_with_options<P, T>(
        &self,
        prefix: P,
        subrange: T,
        options: &ScanOptions,
    ) -> Result<DbIterator, crate::Error>
    where
        P: AsRef<[u8]> + Send,
        T: ByteRangeBounds + Send,
    {
        let prefix = Bytes::copy_from_slice(prefix.as_ref());
        let range = BytesRange::from_prefix_and_subrange(prefix.as_ref(), subrange);
        self.inner
            .scan_with_options(range, options, Some(prefix))
            .await
            .map_err(Into::into)
    }

    /// Scan keys that share `prefix`, walking sources newest-first.
    ///
    /// **Warning:** this is a low-level, unopinionated iterator. It does
    /// **no** merging, **no** deduping, and **no** interpretation of
    /// entries across sources, unlike [`Self::scan_prefix`]. The API
    /// makes no assumptions about what duplicates, tombstones, or merge
    /// operands should mean; every such decision is left to the caller.
    ///
    /// Within each source, entries are emitted in the order requested by
    /// `options.order`: ascending (the default) or descending. Across
    /// sources, the walk is always newest-first, independent of
    /// `options.order`. Each source restarts its own scan at its own
    /// first matching key for that order, so the global emit sequence is
    /// not a single sorted key stream and the within-source key order
    /// resets at every source boundary. The same user key can appear
    /// multiple times, both across sources (once per source that holds
    /// it, newest source first) and within a single source (one entry
    /// per stored sequence number, newest seq first within the key
    /// group): nothing collapses versions. Tombstones and merge operands
    /// are surfaced as raw [`crate::types::RowEntry`] values. The caller
    /// is responsible for any dedup, delete handling, or merge
    /// resolution. Callers that need a totally ordered, fully merged
    /// view should use [`Self::scan_prefix`] instead; use this only when
    /// you want freshest-first results with the option to early-stop and
    /// are willing to interpret raw entries.
    ///
    /// Sources are walked in this order: active memtable, immutable
    /// memtables, then within the single matching segment that segment's
    /// L0 SSTs newest-first followed by its sorted runs newest-first. Each
    /// source is fully drained before moving to the next. Sources are
    /// lazily initialized: the filter check, index load, and first data
    /// block fetch only happen when the recency walk reaches that source.
    /// A prefix read whose data lives in the active memtable therefore
    /// performs zero I/O. When the walk does have to descend to SST
    /// sources, configuring prefix bloom filters lets the scan skip
    /// non-matching SSTs without a data-block fetch, which keeps I/O
    /// proportional to how recent the data is rather than to the size of
    /// the LSM.
    ///
    /// **Multi-segment prefixes are rejected.** If the prefix overlaps
    /// more than one segment, this returns an error with
    /// [`crate::ErrorKind::Invalid`]. The recency guarantee is only
    /// well-defined within a single segment: walking one segment's oldest
    /// data before touching another segment's newest data would violate
    /// freshest-first ordering. Callers that need cross-segment scans
    /// should use [`Self::scan_prefix`] instead.
    ///
    /// ## Arguments
    /// - `prefix`: the key prefix to scan
    ///
    /// ## Returns
    /// - `Result<RecencyIterator, Error>`: an iterator that yields raw
    ///   `RowEntry` values newest-first. Use
    ///   [`RecencyIterator::next_entry`] to pull the next entry, and
    ///   inspect `entry.value` for the [`crate::types::ValueDeletable`]
    ///   variant (`Value`, `Merge`, or `Tombstone`).
    ///
    /// ## Examples
    ///
    /// Pull entries from the freshest source under the prefix and stop.
    /// Because sources are walked newest-first and lazily initialized, an
    /// early-return like this only touches the source that holds the
    /// freshest data. Note that *within* a source the order is set by
    /// `options.order` (ascending by default), so the first yielded entry
    /// is the smallest key in the freshest source, not necessarily the
    /// most recently written key. Across sources the same key may appear
    /// more than once, so callers that want only the freshest value per
    /// key should dedupe.
    ///
    /// ```
    /// use slatedb::{Db, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use slatedb::ValueDeletable;
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     db.put(b"user:42", b"alice").await?;
    ///     db.put(b"user:42", b"alice2").await?;
    ///     db.put(b"user:99", b"bob").await?;
    ///
    ///     let mut iter = db.scan_prefix_by_recency(b"user:").await?;
    ///     let entry = iter.next_entry().await?.unwrap();
    ///     // All three writes live in the active memtable (the freshest
    ///     // source). With ascending within-source order, "user:42" comes
    ///     // out first because it sorts before "user:99". Both writes to
    ///     // "user:42" are stored as separate sequence-numbered entries;
    ///     // within the "user:42" key group the newest seq is yielded
    ///     // first, so the first entry carries the latest write
    ///     // ("alice2"). A second pull would yield the older "user:42"
    ///     // entry ("alice") before advancing to "user:99".
    ///     assert_eq!(entry.key.as_ref(), b"user:42");
    ///     match entry.value {
    ///         ValueDeletable::Value(v) => assert_eq!(v.as_ref(), b"alice2"),
    ///         _ => panic!("expected a regular value"),
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn scan_prefix_by_recency<P>(
        &self,
        prefix: P,
    ) -> Result<DbRecencyIterator, crate::Error>
    where
        P: AsRef<[u8]> + Send,
    {
        self.scan_prefix_by_recency_with_options(prefix, &ScanOptions::default())
            .await
    }

    /// Recency-ordered prefix scan with custom options.
    ///
    /// Same contract as [`Self::scan_prefix_by_recency`] (raw entries
    /// emitted newest-source-first; caller handles dedupe, tombstones, and
    /// merge operands).
    ///
    /// ## Arguments
    /// - `prefix`: the key prefix to scan
    /// - `options`: the scan options to use
    ///
    /// ## Returns
    /// - `Result<RecencyIterator, Error>`: an iterator that yields raw
    ///   `RowEntry` values newest-first.
    ///
    /// ## Examples
    ///
    /// Use `cache_blocks: false` to scan recent data without polluting
    /// the block cache, and stop after pulling enough entries from the
    /// freshest source. Combined with the recency walk's early-stop, this
    /// is a cheap way to ask "is there a recent entry under this prefix?"
    /// without warming the cache for cold blocks the answer doesn't depend
    /// on.
    ///
    /// ```
    /// use slatedb::{Db, Error};
    /// use slatedb::config::ScanOptions;
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     db.put(b"event:001", b"a").await?;
    ///     db.put(b"event:002", b"b").await?;
    ///
    ///     let options = ScanOptions {
    ///         cache_blocks: false,
    ///         ..ScanOptions::default()
    ///     };
    ///     let mut iter = db
    ///         .scan_prefix_by_recency_with_options(b"event:", &options)
    ///         .await?;
    ///     let first = iter.next_entry().await?.unwrap();
    ///     // Within-source order is ascending by default, so "event:001"
    ///     // is yielded before "event:002" even though both live in the
    ///     // same (freshest) source. Stop here: we only needed to see
    ///     // that something fresh exists under the prefix.
    ///     assert_eq!(first.key.as_ref(), b"event:001");
    ///     Ok(())
    /// }
    /// ```
    pub async fn scan_prefix_by_recency_with_options<P>(
        &self,
        prefix: P,
        options: &ScanOptions,
    ) -> Result<DbRecencyIterator, crate::Error>
    where
        P: AsRef<[u8]> + Send,
    {
        let prefix = Bytes::copy_from_slice(prefix.as_ref());
        self.inner
            .scan_prefix_by_recency_with_options(prefix, options)
            .await
            .map_err(Into::into)
    }

    /// Write a value into the database with default `PutOptions` and
    /// `WriteOptions`.
    ///
    /// This method returns after updating the in-memory WAL and MemTable. It
    /// does not wait for the write to become durable in object storage. Call
    /// [`WriteHandle::await_durable`] on the returned handle to wait for this
    /// write, or [`Db::flush`] to flush all pending writes.
    ///
    /// ## Arguments
    /// - `key`: the key to write
    /// - `value`: the value to write
    ///
    /// ## Errors
    /// - `Error`: if there was an error writing the value.
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     let handle = db.put(b"key", b"value").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn put<K, V>(&self, key: K, value: V) -> Result<WriteHandle, crate::Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let mut batch = WriteBatch::new();
        batch.put(key, value);
        self.write(batch).await
    }

    /// Write a value into the database with custom `PutOptions` and `WriteOptions`.
    ///
    /// This method returns after updating the in-memory WAL and MemTable. It
    /// does not wait for the write to become durable in object storage. Call
    /// [`WriteHandle::await_durable`] on the returned handle to wait for this
    /// write, or [`Db::flush`] to flush all pending writes.
    ///
    /// ## Arguments
    /// - `key`: the key to write
    /// - `value`: the value to write
    /// - `put_opts`: the put options to use
    /// - `write_opts`: the write options to use
    ///
    /// ## Errors
    /// - `Error`: if there was an error writing the value.
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, config::{PutOptions, WriteOptions}, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     let handle = db.put_with_options(b"key", b"value", &PutOptions::default(), &WriteOptions::default()).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn put_with_options<K, V>(
        &self,
        key: K,
        value: V,
        put_opts: &PutOptions,
        write_opts: &WriteOptions,
    ) -> Result<WriteHandle, crate::Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let mut batch = WriteBatch::new();
        batch.put_with_options(key, value, put_opts);
        self.write_with_options(batch, write_opts).await
    }

    /// Write a value into the database using owned [`Bytes`], avoiding the
    /// copies that [`Db::put`] performs via `Bytes::copy_from_slice`. Prefer
    /// this form when the caller already holds the data as [`Bytes`] (e.g.
    /// from a prior read, a zero-copy buffer pool, or a client that produces
    /// [`Bytes`] directly).
    ///
    /// This method does not wait for durability. Call
    /// [`WriteHandle::await_durable`] on the returned handle to wait for this
    /// write, or [`Db::flush`] to flush all pending writes.
    pub async fn put_bytes(&self, key: Bytes, value: Bytes) -> Result<WriteHandle, crate::Error> {
        self.put_bytes_with_options(key, value, &PutOptions::default(), &WriteOptions::default())
            .await
    }

    /// Write a value into the database using owned [`Bytes`] with custom
    /// `PutOptions` and `WriteOptions`. See [`Db::put_bytes`] for why this
    /// form exists.
    ///
    /// This method does not wait for durability. Call
    /// [`WriteHandle::await_durable`] on the returned handle to wait for this
    /// write, or [`Db::flush`] to flush all pending writes.
    pub async fn put_bytes_with_options(
        &self,
        key: Bytes,
        value: Bytes,
        put_opts: &PutOptions,
        write_opts: &WriteOptions,
    ) -> Result<WriteHandle, crate::Error> {
        let mut batch = WriteBatch::new();
        batch.put_bytes_with_options(key, value, put_opts);
        self.write_with_options(batch, write_opts).await
    }

    /// Delete a key from the database with default `WriteOptions`.
    ///
    /// This method returns after updating the in-memory WAL and MemTable. It
    /// does not wait for the delete to become durable in object storage. Call
    /// [`WriteHandle::await_durable`] on the returned handle to wait for this
    /// delete, or [`Db::flush`] to flush all pending writes.
    ///
    /// ## Arguments
    /// - `key`: the key to delete
    ///
    /// ## Errors
    /// - `Error`: if there was an error deleting the key.
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     let handle = db.delete(b"key").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete<K: AsRef<[u8]>>(&self, key: K) -> Result<WriteHandle, crate::Error> {
        let mut batch = WriteBatch::new();
        batch.delete(key.as_ref());
        self.write(batch).await
    }

    /// Delete a key from the database with custom `WriteOptions`.
    ///
    /// This method returns after updating the in-memory WAL and MemTable. It
    /// does not wait for the delete to become durable in object storage. Call
    /// [`WriteHandle::await_durable`] on the returned handle to wait for this
    /// delete, or [`Db::flush`] to flush all pending writes.
    ///
    /// ## Arguments
    /// - `key`: the key to delete
    /// - `options`: the write options to use
    ///
    /// ## Errors
    /// - `Error`: if there was an error deleting the key.
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, config::WriteOptions, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     let handle = db.delete_with_options(b"key", &WriteOptions::default()).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete_with_options<K: AsRef<[u8]>>(
        &self,
        key: K,
        options: &WriteOptions,
    ) -> Result<WriteHandle, crate::Error> {
        let mut batch = WriteBatch::new();
        batch.delete(key);
        self.write_with_options(batch, options).await
    }

    /// Merge a value into the database with default `MergeOptions` and `WriteOptions`.
    ///
    /// This method returns after updating the in-memory WAL and MemTable. It
    /// does not wait for the merge to become durable in object storage. Call
    /// [`WriteHandle::await_durable`] on the returned handle to wait for this
    /// merge, or [`Db::flush`] to flush all pending writes.
    ///
    /// Merge operations allow applications to bypass the traditional read/modify/write cycle
    /// by expressing partial updates using an associative operator. The merge operator must
    /// be configured when opening the database.
    ///
    /// ## Arguments
    /// - `key`: the key to merge into
    /// - `value`: the merge operand to apply
    ///
    /// ## Errors
    /// - `Error`: if there was an error merging the value, or if no merge operator is configured.
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, Error, MergeOperator, MergeOperatorError};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    /// use bytes::Bytes;
    ///
    /// struct StringConcatMergeOperator;
    ///
    /// impl MergeOperator for StringConcatMergeOperator {
    ///     fn merge(&self, _key: &Bytes, existing_value: Option<Bytes>, value: Bytes) -> Result<Bytes, MergeOperatorError> {
    ///         let mut result = existing_value.unwrap_or_default().as_ref().to_vec();
    ///         result.extend_from_slice(&value);
    ///         Ok(Bytes::from(result))
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::builder("test_db", object_store)
    ///         .with_merge_operator(Arc::new(StringConcatMergeOperator))
    ///         .build()
    ///         .await?;
    ///     let handle = db.merge(b"key", b"value").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn merge<K, V>(&self, key: K, value: V) -> Result<WriteHandle, crate::Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.merge_with_options(
            key,
            value,
            &MergeOptions::default(),
            &WriteOptions::default(),
        )
        .await
    }

    /// Merge a value into the database with custom `MergeOptions` and `WriteOptions`.
    ///
    /// This method returns after updating the in-memory WAL and MemTable. It
    /// does not wait for the merge to become durable in object storage. Call
    /// [`WriteHandle::await_durable`] on the returned handle to wait for this
    /// merge, or [`Db::flush`] to flush all pending writes.
    ///
    /// Merge operations allow applications to bypass the traditional read/modify/write cycle
    /// by expressing partial updates using an associative operator. The merge operator must
    /// be configured when opening the database.
    ///
    /// ## Arguments
    /// - `key`: the key to merge into
    /// - `value`: the merge operand to apply
    /// - `merge_opts`: the merge options to use
    /// - `write_opts`: the write options to use
    ///
    /// ## Errors
    /// - `Error`: if there was an error merging the value, or if no merge operator is configured.
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, Error, MergeOperator, MergeOperatorError, config::{MergeOptions, WriteOptions}};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    /// use bytes::Bytes;
    ///
    /// struct StringConcatMergeOperator;
    ///
    /// impl MergeOperator for StringConcatMergeOperator {
    ///     fn merge(&self, _key: &Bytes, existing_value: Option<Bytes>, value: Bytes) -> Result<Bytes, MergeOperatorError> {
    ///         let mut result = existing_value.unwrap_or_default().as_ref().to_vec();
    ///         result.extend_from_slice(&value);
    ///         Ok(Bytes::from(result))
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::builder("test_db", object_store)
    ///         .with_merge_operator(Arc::new(StringConcatMergeOperator))
    ///         .build()
    ///         .await?;
    ///     let handle = db.merge_with_options(
    ///         b"key",
    ///         b"value",
    ///         &MergeOptions::default(),
    ///         &WriteOptions::default()
    ///     ).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn merge_with_options<K, V>(
        &self,
        key: K,
        value: V,
        merge_opts: &MergeOptions,
        write_opts: &WriteOptions,
    ) -> Result<WriteHandle, crate::Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        if self.inner.flush_merge_operator.is_none() {
            return Err(SlateDBError::MergeOperatorMissing.into());
        }

        let mut batch = WriteBatch::new();
        batch.merge_with_options(key, value, merge_opts);
        self.write_with_options(batch, write_opts).await
    }

    /// Write a batch of put/delete operations atomically to the database. Batch writes
    /// block other gets and writes until the batch is written to the WAL (or memtable if
    /// WAL is disabled).
    ///
    /// This method returns after updating the in-memory WAL and MemTable. It
    /// does not wait for the batch to become durable in object storage. Call
    /// [`WriteHandle::await_durable`] on the returned handle to wait for this
    /// batch, or [`Db::flush`] to flush all pending writes.
    ///
    /// ## Arguments
    /// - `batch`: the batch of put/delete operations to write
    ///
    /// ## Errors
    /// - `Error`: if there was an error writing the batch.
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{WriteBatch, Db, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///
    ///     let mut batch = WriteBatch::new();
    ///     batch.put(b"key1", b"value1");
    ///     batch.put(b"key2", b"value2");
    ///     batch.delete(b"key1");
    ///     let handle = db.write(batch).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn write(&self, batch: WriteBatch) -> Result<WriteHandle, crate::Error> {
        self.write_with_options(batch, &WriteOptions::default())
            .await
    }

    /// Write a batch of put/delete operations atomically to the database. Batch writes
    /// block other gets and writes until the batch is written to the WAL (or memtable if
    /// WAL is disabled).
    ///
    /// This method returns after updating the in-memory WAL and MemTable. It
    /// does not wait for the batch to become durable in object storage. Call
    /// [`WriteHandle::await_durable`] on the returned handle to wait for this
    /// batch, or [`Db::flush`] to flush all pending writes.
    ///
    /// ## Arguments
    /// - `batch`: the batch of put/delete operations to write
    /// - `options`: the write options to use
    ///
    /// ## Errors
    /// - `Error`: if there was an error writing the batch.
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{WriteBatch, Db, config::WriteOptions, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///
    ///     let mut batch = WriteBatch::new();
    ///     batch.put(b"key1", b"value1");
    ///     batch.put(b"key2", b"value2");
    ///     batch.delete(b"key1");
    ///     let handle = db.write_with_options(batch, &WriteOptions::default()).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn write_with_options(
        &self,
        batch: WriteBatch,
        options: &WriteOptions,
    ) -> Result<WriteHandle, crate::Error> {
        self.inner
            .write_with_options(batch, options, None)
            .await
            .map_err(Into::into)
    }

    /// Flush in-memory writes to object storage. This function blocks until
    /// the in-memory data has been durably written.
    ///
    /// If WAL is enabled, this method is equivalent to:
    /// `flush_with_options(FlushOptions { flush_type: FlushType::Wal })`
    ///
    /// If WAL is disabled, this method is equivalent to:
    /// `flush_with_options(FlushOptions { flush_type: FlushType::Memtable })`.
    ///
    /// ## Errors
    /// - `Error`: if there was an error flushing the database
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, Error};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     db.flush().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn flush(&self) -> Result<(), crate::Error> {
        let flush_type = if self.inner.wal_enabled {
            FlushType::Wal
        } else {
            FlushType::MemTable
        };
        self.inner
            .flush(FlushOptions { flush_type }, true)
            .await
            .map_err(Into::into)
    }

    /// Flush in-memory writes to disk with custom options.
    ///
    /// An error will be returned if `options.flush_type` is `FlushType::Wal` and the WAL
    /// is disabled.
    ///
    /// `FlushType::Memtable` is allowed even if WAL is enabled.
    ///
    /// ## Arguments
    /// - `options`: the flush options
    ///
    /// ## Returns
    /// - `Result<(), crate::Error>`: the result of the flush operation.
    ///
    /// ## Errors
    /// - `Error`: if there was an error flushing the database
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::{Db, Error};
    /// use slatedb::config::{FlushOptions, FlushType};
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     db.flush_with_options(FlushOptions {
    ///         flush_type: FlushType::Wal,
    ///     })
    ///     .await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn flush_with_options(&self, options: FlushOptions) -> Result<(), crate::Error> {
        self.inner.flush(options, true).await.map_err(Into::into)
    }

    /// Refresh the manifest immediately and wait for it to complete.
    ///
    /// The database normally refreshes its manifest on a background timer
    /// controlled by [`Settings::manifest_poll_interval`]. This method
    /// bypasses that timer, triggering an immediate refresh and waiting
    /// for it to finish.
    ///
    /// Use this when you know the manifest has changed externally and
    /// want to ensure the database has observed the update before
    /// proceeding — for example, after a compaction completes and you
    /// need to confirm that a compaction filter has been applied.
    ///
    /// ## Errors
    /// - Returns [`Error`] if the database is closed before the refresh
    ///   completes.
    pub async fn refresh_manifest(&self) -> Result<(), crate::Error> {
        self.inner.check_closed()?;
        self.inner
            .memtable_flusher
            .refresh_manifest()
            .await
            .map_err(Into::into)
    }

    /// Begin a new transaction with the specified isolation level.
    ///
    /// ## Arguments
    /// - `isolation_level`: the isolation level for the transaction
    ///
    /// ## Returns
    /// - `Result<DbTransaction, crate::Error>`: the transaction handle
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use slatedb::{Db, IsolationLevel};
    /// use slatedb::object_store::memory::InMemory;
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), slatedb::Error> {
    ///     let object_store = Arc::new(InMemory::new());
    ///     let db = Db::open("test_db", object_store).await?;
    ///     let txn = db.begin(IsolationLevel::SerializableSnapshot).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn begin(
        &self,
        isolation_level: IsolationLevel,
    ) -> Result<DbTransaction, crate::Error> {
        self.inner.check_closed()?;
        let txn = DbTransaction::new(
            self.inner.clone(),
            self.inner.txn_manager.clone(),
            isolation_level,
        );
        Ok(txn)
    }

    /// Resolve an object store from a URL.
    ///
    /// URL must not have a path component. This is an artifact of the way `object_store`
    /// handles URL parsing. Paths should be provided in the various `*Builder::new`
    /// methods that take `path` arguments, not in the URL passed to this method.
    ///
    /// ## Arguments
    /// - `url`: the URL to resolve with no trailing path, for example `s3://my-bucket`.
    ///
    /// ## Returns
    /// - `Result<Arc<dyn ObjectStore>, crate::Error>`: the resolved object store
    ///
    /// ## Errors
    /// - `Error`: if the URL is unparseable, if the URL contains a path component, or if
    ///   there was an error initializing the object store.
    pub fn resolve_object_store(url: &str) -> Result<Arc<dyn ObjectStore>, crate::Error> {
        let url = url
            .try_into()
            .map_err(|e| SlateDBError::InvalidObjectStoreURL(url.to_string(), e))?;
        // Lowercase env keys because parse_url_opts only recognizes lower case option keys.
        let env_vars = std::env::vars().map(|(key, value)| (key.to_ascii_lowercase(), value));
        let (object_store, path) = parse_url_opts(&url, env_vars).map_err(SlateDBError::from)?;
        if !path.as_ref().is_empty() {
            return Err(SlateDBError::InvalidObjectStorePath(path.to_string()))?;
        }
        Ok(Arc::from(object_store))
    }
}

#[async_trait::async_trait]
impl DbReadOps for Db {
    async fn get_with_options<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
        options: &ReadOptions,
    ) -> Result<Option<Bytes>, crate::Error> {
        Db::get_with_options(self, key, options).await
    }

    async fn get_key_value_with_options<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
        options: &ReadOptions,
    ) -> Result<Option<KeyValue>, crate::Error> {
        Db::get_key_value_with_options(self, key, options).await
    }

    async fn scan_with_options<T>(
        &self,
        range: T,
        options: &ScanOptions,
    ) -> Result<DbIterator, crate::Error>
    where
        T: ByteRangeBounds + Send,
    {
        Db::scan_with_options(self, range, options).await
    }

    async fn scan_prefix_with_options<P, T>(
        &self,
        prefix: P,
        subrange: T,
        options: &ScanOptions,
    ) -> Result<DbIterator, crate::Error>
    where
        P: AsRef<[u8]> + Send,
        T: ByteRangeBounds + Send,
    {
        Db::scan_prefix_with_options(self, prefix, subrange, options).await
    }
}

impl DbMetadataOps for Db {
    fn manifest(&self) -> VersionedManifest {
        self.inner.manifest()
    }

    fn subscribe(&self) -> tokio::sync::watch::Receiver<DbStatus> {
        self.inner.status_manager.subscribe()
    }

    fn status(&self) -> DbStatus {
        self.inner.status()
    }
}

#[async_trait::async_trait]
impl DbWriteOps for Db {
    type Transaction = DbTransaction;

    async fn put_with_options<K, V>(
        &self,
        key: K,
        value: V,
        put_opts: &PutOptions,
        write_opts: &WriteOptions,
    ) -> Result<WriteHandle, crate::Error>
    where
        K: AsRef<[u8]> + Send,
        V: AsRef<[u8]> + Send,
    {
        Db::put_with_options(self, key, value, put_opts, write_opts).await
    }

    async fn delete_with_options<K: AsRef<[u8]> + Send>(
        &self,
        key: K,
        options: &WriteOptions,
    ) -> Result<WriteHandle, crate::Error> {
        Db::delete_with_options(self, key, options).await
    }

    async fn merge_with_options<K, V>(
        &self,
        key: K,
        value: V,
        merge_opts: &MergeOptions,
        write_opts: &WriteOptions,
    ) -> Result<WriteHandle, crate::Error>
    where
        K: AsRef<[u8]> + Send,
        V: AsRef<[u8]> + Send,
    {
        Db::merge_with_options(self, key, value, merge_opts, write_opts).await
    }

    async fn write_with_options(
        &self,
        batch: WriteBatch,
        options: &WriteOptions,
    ) -> Result<WriteHandle, crate::Error> {
        Db::write_with_options(self, batch, options).await
    }

    async fn flush(&self) -> Result<(), crate::Error> {
        Db::flush(self).await
    }

    async fn flush_with_options(&self, options: FlushOptions) -> Result<(), crate::Error> {
        Db::flush_with_options(self, options).await
    }

    async fn begin(&self, isolation_level: IsolationLevel) -> Result<DbTransaction, crate::Error> {
        Db::begin(self, isolation_level).await
    }
}

impl Db {
    /// See [`DbMetadataOps::manifest`].
    pub fn manifest(&self) -> VersionedManifest {
        <Self as DbMetadataOps>::manifest(self)
    }

    /// See [`DbMetadataOps::subscribe`].
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<DbStatus> {
        <Self as DbMetadataOps>::subscribe(self)
    }

    /// See [`DbMetadataOps::status`].
    pub fn status(&self) -> DbStatus {
        <Self as DbMetadataOps>::status(self)
    }
}

#[async_trait::async_trait]
impl DbCacheManagerOps for Db {
    async fn warm_sst(
        &self,
        sst_id: SsTableId,
        targets: &[CacheTarget],
    ) -> Result<(), crate::Error> {
        self.inner.check_closed()?;
        let manifest = self.manifest();
        db_cache_manager::warm_sst_impl(&self.inner.table_store, manifest.core(), sst_id, targets)
            .await
    }

    async fn evict_cached_sst(
        &self,
        sst: &SsTableHandle,
        targets: &[CacheTarget],
    ) -> Result<(), crate::Error> {
        self.inner.check_closed()?;
        let manifest = self.manifest();
        db_cache_manager::evict_cached_sst_impl(
            &self.inner.table_store,
            manifest.core(),
            sst,
            targets,
        )
        .await
    }

    async fn flush_cache_to_disk(&self) -> Result<(), crate::Error> {
        self.inner.check_closed()?;
        db_cache_manager::flush_cache_to_disk_impl(&self.inner.table_store).await
    }
}

/// Handle returned from write operations, containing metadata about the write.
///
/// Write operations return this handle without waiting for durability. Call
/// [`WriteHandle::await_durable`] to wait until this write is durable in object
/// storage.
///
/// This structure is designed to be extensible for future enhancements.
#[derive(Clone)]
pub struct WriteHandle {
    pub(crate) seq: u64,
    pub(crate) create_ts: i64,
    durability_waiter: DurabilityWaiter,
}

impl fmt::Debug for WriteHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WriteHandle")
            .field("seq", &self.seq)
            .field("create_ts", &self.create_ts)
            .finish_non_exhaustive()
    }
}

impl WriteHandle {
    /// Creates a write handle that uses `durability_waiter` to determine when
    /// the write is durable.
    ///
    /// This constructor is intended for custom [`DbWriteOps`] implementations
    /// and test doubles whose writes are not immediately durable.
    pub fn new<F, Fut>(seq: u64, create_ts: i64, durability_waiter: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), crate::Error>> + Send + 'static,
    {
        let durability_waiter: DurabilityWaiter = Arc::new(move |_| Box::pin(durability_waiter()));

        Self {
            seq,
            create_ts,
            durability_waiter,
        }
    }

    pub(crate) fn new_with_waiter(
        seq: u64,
        create_ts: i64,
        durability_waiter: DurabilityWaiter,
    ) -> Self {
        Self {
            seq,
            create_ts,
            durability_waiter,
        }
    }

    /// Returns the sequence number assigned to this write operation.
    pub fn seqnum(&self) -> u64 {
        self.seq
    }

    /// Returns the creation timestamp assigned to this write operation.
    pub fn create_ts(&self) -> i64 {
        self.create_ts
    }

    /// Waits until this write has been durably persisted.
    ///
    /// If the database closes before the write becomes durable, this returns a
    /// closed error carrying the database's [`CloseReason`].
    ///
    /// # Errors
    ///
    /// Returns a closed error if the database closes first.
    pub async fn await_durable(&self) -> Result<(), crate::Error> {
        (self.durability_waiter)(self.seq).await
    }
}

/// Wraps [`WalObserver`] and injects a [`crate::wal::WalStatusListener`]
/// that updates the oracle and manifest, and drives cross-task notifications about wal events
/// via a [`tokio::sync::watch`] channel.
#[derive(Clone)]
pub(crate) struct DbWalObserver {
    status_rx: tokio::sync::watch::Receiver<Result<WalStatus, WalStatus>>,
    closed_reader: WatchableOnceCellReader<Result<(), SlateDBError>>,
    wrapped: Arc<dyn WalObserver>,
}

impl DbWalObserver {
    fn new(
        wrapped: Box<dyn WalObserver>,
        oracle: Arc<DbOracle>,
        db_state: Arc<RwLock<DbState>>,
        closed_writer: Arc<dyn ClosedResultWriter>,
    ) -> Self {
        let (status_tx, status_rx) = tokio::sync::watch::channel(wrapped.status());
        let closed_reader = closed_writer.result_reader();
        wrapped
            .subscribe(Arc::new(move |event| {
                let status: Result<WalStatus, WalStatus> = match event {
                    WalEvent::WalFlushed(status) => {
                        if let Some(seq) = status.last_flushed_seq {
                            oracle.advance_durable_seq(seq);
                        }
                        let mut guard = db_state.write();
                        guard.set_next_wal_id(status.last_flushed_wal_id + 1);
                        drop(guard);
                        Ok(status)
                    }
                    WalEvent::WalClosed(status) => {
                        closed_writer.write_result(Err(status.clone().into()));
                        Err(status)
                    }
                };
                let _ = status_tx.send(status);
            }))
            .expect("failed to subscribe to wal");
        Self {
            status_rx,
            closed_reader,
            wrapped: wrapped.into(),
        }
    }

    pub(crate) fn status(&self) -> Result<WalStatus, WalStatus> {
        self.wrapped.status()
    }

    async fn wait_on_condition(
        &self,
        mut predicate: impl FnMut(&WalStatus) -> bool,
    ) -> Result<(), SlateDBError> {
        let mut status_rx = self.status_rx.clone();
        let result = status_rx
            .wait_for(|s| match s {
                Err(_) => true,
                Ok(s) => predicate(s),
            })
            .await;
        let Ok(result) = result else {
            drop(result);
            debug!("wal listener tx dropped - wait on db close");
            return self.closed_reader.clone().await_value().await;
        };
        let result = result.clone();
        result?;
        Ok(())
    }

    /// Waits until the wal a given wal id is released by the wal writer
    async fn wait_until_wal_flushed(&self, last_flushed_wal_id: u64) -> Result<(), SlateDBError> {
        self.wait_on_condition(|status| status.last_flushed_wal_id > last_flushed_wal_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_cache_policy::BlockCachePolicy;
    use crate::config::DurabilityLevel::{Memory, Remote};
    use crate::config::MetricLevel;
    use crate::config::{
        CheckpointOptions, CloseOptions, CompactionWorkerOptions, CompactorOptions,
        GarbageCollectorDirectoryOptions, GarbageCollectorOptions, ObjectStoreCacheOptions,
        PutOptions, ScanOptions, Settings, SstBlockSize, Ttl, WriteOptions,
    };
    use crate::db::builder::GarbageCollectorBuilder;
    use crate::db_stats::IMMUTABLE_MEMTABLE_FLUSHES;
    use crate::format::sst::SsTableFormat;
    use crate::instrumented_object_store::stats::{
        REQUEST_COUNT as OBJECT_STORE_REQUEST_COUNT,
        REQUEST_DURATION_SECONDS as OBJECT_STORE_REQUEST_DURATION_SECONDS,
    };
    use crate::iter::{IterationOrder, RowEntryIterator};
    use crate::manifest::store::{ManifestStore, StoredManifest};
    use crate::manifest::{ManifestCore, VersionedManifest};
    use crate::merge_operator::{
        MERGE_OPERATOR_COMPACT_PATH, MERGE_OPERATOR_FLUSH_PATH, MERGE_OPERATOR_READ_PATH,
    };
    use crate::proptest_util::arbitrary;
    use crate::proptest_util::sample;
    use crate::seq_tracker::FindOption;
    use crate::sst_iter::{SstIterator, SstIteratorOptions};
    use crate::tablestore::TableStoreKind;
    use crate::test_utils::{
        assert_iterator, lookup_merge_operator_operands, GatedObjectStore,
        OnDemandCompactionSchedulerSupplier, StringConcatMergeOperator,
    };
    use crate::types::RowEntry;
    use crate::wal::slatedb::store::WalTableStore;
    use crate::wal::{SlateDbWalReaderBuilder, WalError, WalReader as _};
    use crate::{proptest_util, test_utils, CloseReason, CompactorBuilder, KeyValue};
    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use fail_parallel::FailPointRegistry;
    use futures::{future, future::join_all, FutureExt, StreamExt};
    use object_store::memory::InMemory;
    use object_store::ObjectStore;
    use proptest::test_runner::{TestRng, TestRunner};
    use slatedb_common::clock::DefaultSystemClock;
    use slatedb_common::clock::MockSystemClock;
    use slatedb_common::metrics::{
        lookup_metric, lookup_metric_with_labels, CounterFn, DefaultMetricsRecorder, GaugeFn,
        HistogramFn, MetricValue, MetricsRecorder, NoopMetricsRecorder, UpDownCounterFn,
    };
    use std::collections::BTreeMap;
    use std::collections::Bound::Included;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    use tokio::runtime::Runtime;
    use tracing::info;

    fn object_store_labels(
        component: &'static str,
        store_type: &'static str,
        op: &'static str,
        api: &'static str,
    ) -> [(&'static str, &'static str); 4] {
        [
            ("component", component),
            ("store_type", store_type),
            ("op", op),
            ("api", api),
        ]
    }

    fn lookup_object_store_histogram_count(
        recorder: &DefaultMetricsRecorder,
        labels: &[(&str, &str)],
    ) -> Option<u64> {
        recorder
            .snapshot()
            .by_name_and_labels(OBJECT_STORE_REQUEST_DURATION_SECONDS, labels)
            .map(|metric| match &metric.value {
                MetricValue::Histogram { count, .. } => *count,
                other => panic!("expected histogram metric, got {other:?}"),
            })
    }

    fn lookup_object_store_api_request_count(
        recorder: &DefaultMetricsRecorder,
        component: &'static str,
        store_type: &'static str,
        op: &'static str,
        api: &'static str,
    ) -> i64 {
        lookup_metric_with_labels(
            recorder,
            OBJECT_STORE_REQUEST_COUNT,
            &object_store_labels(component, store_type, op, api),
        )
        .unwrap_or(0)
    }

    fn lookup_object_store_api_histogram_count(
        recorder: &DefaultMetricsRecorder,
        component: &'static str,
        store_type: &'static str,
        op: &'static str,
        api: &'static str,
    ) -> u64 {
        lookup_object_store_histogram_count(
            recorder,
            &object_store_labels(component, store_type, op, api),
        )
        .unwrap_or(0)
    }

    fn lookup_object_store_op_request_count(
        recorder: &DefaultMetricsRecorder,
        component: &'static str,
        store_type: &'static str,
        op: &'static str,
    ) -> i64 {
        let apis = match op {
            "get" => &["get", "get_range", "get_ranges", "head"][..],
            "put" => &[
                "put",
                "multipart_init",
                "multipart_part",
                "multipart_complete",
            ][..],
            "delete" => &["delete"][..],
            _ => panic!("unexpected op {op}"),
        };

        apis.iter()
            .map(|api| {
                lookup_object_store_api_request_count(recorder, component, store_type, op, api)
            })
            .sum()
    }

    fn lookup_object_store_op_histogram_count(
        recorder: &DefaultMetricsRecorder,
        component: &'static str,
        store_type: &'static str,
        op: &'static str,
    ) -> u64 {
        let apis = match op {
            "get" => &["get", "get_range", "get_ranges", "head"][..],
            "put" => &[
                "put",
                "multipart_init",
                "multipart_part",
                "multipart_complete",
            ][..],
            "delete" => &["delete"][..],
            _ => panic!("unexpected op {op}"),
        };

        apis.iter()
            .map(|api| {
                lookup_object_store_api_histogram_count(recorder, component, store_type, op, api)
            })
            .sum()
    }

    #[tokio::test]
    async fn test_put_get_delete() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let kv_store = Db::builder("/tmp/test_kv_store", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        let key = b"test_key";
        let value = b"test_value";
        kv_store.put(key, value).await.unwrap();
        kv_store.flush().await.unwrap();

        assert_eq!(
            kv_store.get(key).await.unwrap(),
            Some(Bytes::from_static(value))
        );
        kv_store.delete(key).await.unwrap();
        assert_eq!(None, kv_store.get(key).await.unwrap());
        kv_store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_manifest_returns_current_versioned_manifest() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_manifest_accessor", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        db.put(b"test_key", b"test_value").await.unwrap();

        let manifest = db.manifest();
        let expected: VersionedManifest = db.inner.state.read().state().manifest.clone().into();
        assert_eq!(manifest, expected);

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_coarse_size_estimation_via_manifest() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_coarse_size_estimation";
        let should_compact = Arc::new(AtomicBool::new(false));
        let should_compact_clone = should_compact.clone();
        let compaction_scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            move |_state| should_compact_clone.swap(false, Ordering::SeqCst),
        )));
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_compactor_builder(
                CompactorBuilder::new(path, object_store.clone())
                    .with_scheduler_supplier(compaction_scheduler)
                    .with_options(fast_compactor_options()),
            )
            .build()
            .await
            .unwrap();
        let db = Arc::new(db);

        // Write keys in the range k0000..k0099 and flush to L0
        for i in 0..100u32 {
            let key = format!("k{:04}", i);
            db.put(key.as_bytes(), &[0u8; 64]).await.unwrap();
        }
        db.flush().await.unwrap();

        // estimate_size on L0 views should return non-zero
        let manifest = db.manifest();
        assert!(!manifest.manifest.core.tree.l0.is_empty());
        for view in &manifest.manifest.core.tree.l0 {
            assert!(view.estimate_size() > 0);
        }

        // Trigger compaction and wait for sorted runs
        should_compact.store(true, Ordering::SeqCst);
        let db_poll = db.clone();
        tokio::time::timeout(Duration::from_secs(10), async move {
            loop {
                {
                    let state = db_poll.inner.state.read();
                    if !state.state().core().tree.compacted.is_empty() {
                        return;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();

        let manifest = db.manifest();
        assert!(!manifest.manifest.core.tree.compacted.is_empty());

        for sr in &manifest.manifest.core.tree.compacted {
            // A range covering all keys returns results
            let covering = sr
                .tables_covering_range(Bytes::from_static(b"k0000")..Bytes::from_static(b"k0100"));
            assert!(!covering.is_empty());
            for view in &covering {
                assert!(view.estimate_size() > 0);
            }

            // A range before all keys returns nothing
            let outside = sr
                .tables_covering_range(Bytes::from_static(b"a0000")..Bytes::from_static(b"a9999"));
            assert!(outside.is_empty());
        }

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_prefix_returns_matching_keys() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let kv_store = Db::builder("/tmp/test_scan_prefix", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        kv_store.put(b"ab", b"v0").await.unwrap();
        kv_store.put(b"aba", b"v1").await.unwrap();
        kv_store.put(b"abb", b"v2").await.unwrap();
        kv_store.put(b"ac", b"v3").await.unwrap();

        let mut iter = kv_store.scan_prefix(b"ab", ..).await.unwrap();
        assert_eq!(iter.next().await.unwrap().unwrap().key.as_ref(), b"ab");
        assert_eq!(iter.next().await.unwrap().unwrap().key.as_ref(), b"aba");
        assert_eq!(iter.next().await.unwrap().unwrap().key.as_ref(), b"abb");
        assert_eq!(iter.next().await.unwrap(), None);

        kv_store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_and_prefix_range_forms_are_accepted() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let kv_store = Db::builder("/tmp/test_scan_range_forms", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        kv_store.put(b"a", b"v0").await.unwrap();
        kv_store.put(b"aa", b"v1").await.unwrap();
        kv_store.put(b"ab", b"v2").await.unwrap();
        kv_store.put(b"b", b"v3").await.unwrap();

        let mut all = kv_store.scan(..).await.unwrap();
        assert_eq!(all.next().await.unwrap().unwrap().key.as_ref(), b"a");
        assert_eq!(all.next().await.unwrap().unwrap().key.as_ref(), b"aa");
        assert_eq!(all.next().await.unwrap().unwrap().key.as_ref(), b"ab");
        assert_eq!(all.next().await.unwrap().unwrap().key.as_ref(), b"b");
        assert_eq!(all.next().await.unwrap(), None);

        let mut range = kv_store.scan(b"a".to_vec()..=b"ab".to_vec()).await.unwrap();
        assert_eq!(range.next().await.unwrap().unwrap().key.as_ref(), b"a");
        assert_eq!(range.next().await.unwrap().unwrap().key.as_ref(), b"aa");
        assert_eq!(range.next().await.unwrap().unwrap().key.as_ref(), b"ab");
        assert_eq!(range.next().await.unwrap(), None);

        let mut prefix = kv_store.scan_prefix(b"a", b"".to_vec()..).await.unwrap();
        assert_eq!(prefix.next().await.unwrap().unwrap().key.as_ref(), b"a");
        assert_eq!(prefix.next().await.unwrap().unwrap().key.as_ref(), b"aa");
        assert_eq!(prefix.next().await.unwrap().unwrap().key.as_ref(), b"ab");
        assert_eq!(prefix.next().await.unwrap(), None);

        let mut bounded_prefix = kv_store
            .scan_prefix(b"a", b"a".to_vec()..=b"b".to_vec())
            .await
            .unwrap();
        assert_eq!(
            bounded_prefix.next().await.unwrap().unwrap().key.as_ref(),
            b"aa"
        );
        assert_eq!(
            bounded_prefix.next().await.unwrap().unwrap().key.as_ref(),
            b"ab"
        );
        assert_eq!(bounded_prefix.next().await.unwrap(), None);

        kv_store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_descending_returns_records_in_reverse_order() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let kv_store = Db::builder("/tmp/test_scan_descending", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        kv_store.put(b"a", b"v0").await.unwrap();
        kv_store.put(b"b", b"v1").await.unwrap();
        kv_store.flush().await.unwrap();
        kv_store.put(b"c", b"v2").await.unwrap();
        kv_store.put(b"d", b"v3").await.unwrap();

        let scan_options = ScanOptions::default().with_order(IterationOrder::Descending);
        let mut iter = kv_store.scan_with_options(.., &scan_options).await.unwrap();
        assert_eq!(iter.next().await.unwrap().unwrap().key.as_ref(), b"d");
        assert_eq!(iter.next().await.unwrap().unwrap().key.as_ref(), b"c");
        assert_eq!(iter.next().await.unwrap().unwrap().key.as_ref(), b"b");
        assert_eq!(iter.next().await.unwrap().unwrap().key.as_ref(), b"a");
        assert_eq!(iter.next().await.unwrap(), None);

        kv_store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_seek_rejected_for_descending_scan() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let kv_store = Db::builder("/tmp/test_seek_descending_rejected", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        kv_store.put(b"a", b"v0").await.unwrap();
        kv_store.put(b"b", b"v1").await.unwrap();
        kv_store.put(b"c", b"v2").await.unwrap();

        let scan_options = ScanOptions::default().with_order(IterationOrder::Descending);
        let mut iter = kv_store.scan_with_options(.., &scan_options).await.unwrap();
        let err = iter.seek(b"b").await.unwrap_err();
        assert_eq!(err.kind(), crate::ErrorKind::Invalid);
        assert!(
            err.to_string().contains("descending"),
            "unexpected error: {err}"
        );

        // the scan itself still works
        assert_eq!(iter.next().await.unwrap().unwrap().key.as_ref(), b"c");

        // ascending scans still seek
        let mut iter = kv_store.scan(..).await.unwrap();
        iter.seek(b"b").await.unwrap();
        assert_eq!(iter.next().await.unwrap().unwrap().key.as_ref(), b"b");

        kv_store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_descending_bounded_range() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let kv_store = Db::builder("/tmp/test_scan_descending_bounded", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        kv_store.put(b"a", b"v0").await.unwrap();
        kv_store.put(b"b", b"v1").await.unwrap();
        kv_store.put(b"c", b"v2").await.unwrap();
        kv_store.put(b"d", b"v3").await.unwrap();
        kv_store.put(b"e", b"v4").await.unwrap();

        let scan_options = ScanOptions::default().with_order(IterationOrder::Descending);
        let mut iter = kv_store
            .scan_with_options(b"b".to_vec()..b"d".to_vec(), &scan_options)
            .await
            .unwrap();
        assert_eq!(iter.next().await.unwrap().unwrap().key.as_ref(), b"c");
        assert_eq!(iter.next().await.unwrap().unwrap().key.as_ref(), b"b");
        assert_eq!(iter.next().await.unwrap(), None);

        kv_store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_descending_skips_deleted_keys() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let kv_store = Db::builder("/tmp/test_scan_descending_deletes", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        kv_store.put(b"a", b"v0").await.unwrap();
        kv_store.put(b"b", b"v1").await.unwrap();
        kv_store.put(b"c", b"v2").await.unwrap();
        kv_store.put(b"d", b"v3").await.unwrap();
        kv_store.delete(b"b").await.unwrap();
        kv_store.delete(b"d").await.unwrap();

        let scan_options = ScanOptions::default().with_order(IterationOrder::Descending);
        let mut iter = kv_store.scan_with_options(.., &scan_options).await.unwrap();
        assert_eq!(iter.next().await.unwrap().unwrap().key.as_ref(), b"c");
        assert_eq!(iter.next().await.unwrap().unwrap().key.as_ref(), b"a");
        assert_eq!(iter.next().await.unwrap(), None);

        kv_store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_prefix_descending() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let kv_store = Db::builder("/tmp/test_scan_prefix_descending", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        kv_store.put(b"prefix/a", b"v0").await.unwrap();
        kv_store.put(b"prefix/b", b"v1").await.unwrap();
        kv_store.put(b"prefix/c", b"v2").await.unwrap();
        kv_store.put(b"other/a", b"v3").await.unwrap();

        let scan_options = ScanOptions::default().with_order(IterationOrder::Descending);
        let mut iter = kv_store
            .scan_prefix_with_options(b"prefix/", .., &scan_options)
            .await
            .unwrap();
        assert_eq!(
            iter.next().await.unwrap().unwrap().key.as_ref(),
            b"prefix/c"
        );
        assert_eq!(
            iter.next().await.unwrap().unwrap().key.as_ref(),
            b"prefix/b"
        );
        assert_eq!(
            iter.next().await.unwrap().unwrap().key.as_ref(),
            b"prefix/a"
        );
        assert_eq!(iter.next().await.unwrap(), None);

        kv_store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_prefix_with_options_handles_unbounded_end() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let kv_store = Db::builder("/tmp/test_scan_prefix_unbounded", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        kv_store.put(&[0xff, 0xff], b"v0").await.unwrap();
        kv_store.put(&[0xff, 0xff, 0x00], b"v1").await.unwrap();
        kv_store.put(&[0xff, 0xff, 0x10], b"v2").await.unwrap();
        kv_store.put(&[0xff, 0xff, 0xff], b"v4").await.unwrap();
        kv_store.put(&[0xff, 0xfe], b"v3").await.unwrap();

        let scan_options = ScanOptions {
            cache_blocks: false,
            ..ScanOptions::default()
        };
        let mut iter = kv_store
            .scan_prefix_with_options(&[0xff, 0xff], .., &scan_options)
            .await
            .unwrap();
        assert_eq!(
            iter.next().await.unwrap().unwrap().key.as_ref(),
            &[0xff, 0xff]
        );
        assert_eq!(
            iter.next().await.unwrap().unwrap().key.as_ref(),
            &[0xff, 0xff, 0x00]
        );
        assert_eq!(
            iter.next().await.unwrap().unwrap().key.as_ref(),
            &[0xff, 0xff, 0x10]
        );
        assert_eq!(
            iter.next().await.unwrap().unwrap().key.as_ref(),
            &[0xff, 0xff, 0xff]
        );
        assert_eq!(iter.next().await.unwrap(), None);

        kv_store.close().await.unwrap();
    }

    fn assert_value(entry: &RowEntry, expected: &[u8]) {
        match &entry.value {
            crate::types::ValueDeletable::Value(v) => assert_eq!(v.as_ref(), expected),
            other => panic!("expected Value({expected:?}), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_scan_prefix_by_recency_returns_matching_keys_from_memtable() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_recency_memtable", object_store)
            .with_settings(test_db_options(0, 64 * 1024, None))
            .build()
            .await
            .unwrap();

        db.put(b"px:a", b"v0").await.unwrap();
        db.put(b"px:b", b"v1").await.unwrap();
        db.put(b"px:c", b"v2").await.unwrap();
        db.put(b"qq:x", b"vx").await.unwrap();

        let mut iter = db.scan_prefix_by_recency(b"px:").await.unwrap();
        let e1 = iter.next_entry().await.unwrap().unwrap();
        let e2 = iter.next_entry().await.unwrap().unwrap();
        let e3 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(e1.key.as_ref(), b"px:a");
        assert_eq!(e2.key.as_ref(), b"px:b");
        assert_eq!(e3.key.as_ref(), b"px:c");
        assert!(iter.next_entry().await.unwrap().is_none());

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_prefix_by_recency_no_match_returns_none() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_recency_no_match", object_store)
            .with_settings(test_db_options(0, 64 * 1024, None))
            .build()
            .await
            .unwrap();

        db.put(b"aa", b"v0").await.unwrap();
        db.put(b"ab", b"v1").await.unwrap();

        let mut iter = db.scan_prefix_by_recency(b"zz").await.unwrap();
        assert!(iter.next_entry().await.unwrap().is_none());

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_prefix_by_recency_emits_both_versions_across_sources() {
        // No dedup: a key present in both memtable (newer) and L0 (older)
        // appears twice, newer first.
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_recency_no_dedup", object_store)
            .with_settings(test_db_options(0, 64 * 1024, None))
            .build()
            .await
            .unwrap();

        db.put(b"px:a", b"old").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        db.put(b"px:a", b"new").await.unwrap();

        let mut iter = db.scan_prefix_by_recency(b"px:").await.unwrap();
        let e1 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(e1.key.as_ref(), b"px:a");
        assert_value(&e1, b"new");
        let e2 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(e2.key.as_ref(), b"px:a");
        assert_value(&e2, b"old");
        assert!(e1.seq > e2.seq);
        assert!(iter.next_entry().await.unwrap().is_none());

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_prefix_by_recency_emits_tombstones() {
        // Tombstones are surfaced as raw entries; the caller decides what
        // they mean.
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_recency_tombstones", object_store)
            .with_settings(test_db_options(0, 64 * 1024, None))
            .build()
            .await
            .unwrap();

        db.put(b"px:a", b"va").await.unwrap();
        db.put(b"px:b", b"vb").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        db.delete(b"px:a").await.unwrap();

        let mut iter = db.scan_prefix_by_recency(b"px:").await.unwrap();
        // Memtable first: tombstone for px:a.
        let e1 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(e1.key.as_ref(), b"px:a");
        assert!(e1.value.is_tombstone());
        // L0 second: original values, ascending.
        let e2 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(e2.key.as_ref(), b"px:a");
        assert_value(&e2, b"va");
        let e3 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(e3.key.as_ref(), b"px:b");
        assert_value(&e3, b"vb");
        // Tombstone is from the freshest source so its seq dominates the
        // earlier put of px:a.
        assert!(e1.seq > e2.seq);
        assert!(iter.next_entry().await.unwrap().is_none());

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_prefix_by_recency_walks_memtable_then_l0() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_recency_multi_source", object_store)
            .with_settings(test_db_options(0, 64 * 1024, None))
            .build()
            .await
            .unwrap();

        // Older L0 SST: px:b (older value), px:c.
        db.put(b"px:b", b"old_b").await.unwrap();
        db.put(b"px:c", b"vc").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Active memtable: px:a, px:b (newer value).
        db.put(b"px:a", b"va").await.unwrap();
        db.put(b"px:b", b"new_b").await.unwrap();

        let mut iter = db.scan_prefix_by_recency(b"px:").await.unwrap();
        // Memtable drained first, ascending within source.
        let e1 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(e1.key.as_ref(), b"px:a");
        assert_value(&e1, b"va");
        let e2 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(e2.key.as_ref(), b"px:b");
        assert_value(&e2, b"new_b");
        // L0 next: px:b (older) then px:c. No dedup.
        let e3 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(e3.key.as_ref(), b"px:b");
        assert_value(&e3, b"old_b");
        let e4 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(e4.key.as_ref(), b"px:c");
        assert_value(&e4, b"vc");
        assert!(iter.next_entry().await.unwrap().is_none());

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_prefix_by_recency_active_memtable_avoids_main_object_store_gets() {
        // When the prefix is satisfied entirely by the active memtable, the
        // recency scan should not issue any GETs against the main object
        // store (it shouldn't even open an L0/SR iterator).
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_recency_no_main_gets", object_store)
            .with_settings(test_db_options(0, 64 * 1024, None))
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();

        // Older data in L0. Different prefix; we want them to be present
        // but never visited.
        db.put(b"qq:a", b"vqa").await.unwrap();
        db.put(b"qq:b", b"vqb").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Active-memtable-only prefix.
        db.put(b"px:a", b"va").await.unwrap();
        db.put(b"px:b", b"vb").await.unwrap();

        let gets_before =
            lookup_object_store_op_request_count(&metrics_recorder, "db", "main", "get");

        let mut iter = db.scan_prefix_by_recency(b"px:").await.unwrap();
        let e1 = iter.next_entry().await.unwrap().unwrap();
        let e2 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(e1.key.as_ref(), b"px:a");
        assert_eq!(e2.key.as_ref(), b"px:b");
        // Caller stops here without driving the iterator into older sources.

        let gets_after =
            lookup_object_store_op_request_count(&metrics_recorder, "db", "main", "get");
        assert_eq!(
            gets_before, gets_after,
            "scan_prefix_by_recency should not touch the main object store \
             when the prefix is fully covered by the active memtable"
        );

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_prefix_by_recency_durability_remote_filters_unflushed() {
        // With DurabilityLevel::Remote and the put issued without awaiting
        // durability, only L0/SR-resident entries should be visible. The
        // not-yet-flushed memtable write is filtered out by max_seq.
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_recency_remote_durability", object_store)
            .with_settings(test_db_options(0, 64 * 1024, None))
            .build()
            .await
            .unwrap();

        db.put(b"px:a", b"va_durable").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Skip the WAL await. Without a follow-up flush, px:b lives only
        // in the in-memory memtable.
        db.put_with_options(
            b"px:b",
            b"vb_dirty",
            &PutOptions::default(),
            &WriteOptions::default(),
        )
        .await
        .unwrap();

        let opts = ScanOptions {
            durability_filter: Remote,
            ..ScanOptions::default()
        };
        let mut iter = db
            .scan_prefix_by_recency_with_options(b"px:", &opts)
            .await
            .unwrap();
        let e = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(e.key.as_ref(), b"px:a");
        assert_value(&e, b"va_durable");
        assert!(iter.next_entry().await.unwrap().is_none());

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_prefix_by_recency_walks_memtable_then_compacted_run() {
        // Walks memtable -> L0 -> compacted sorted run, verifying that the
        // sorted-run path is constructed and drained correctly when the
        // recency walk reaches it. Same-key versions surface from each
        // source in newest-first order with no dedup.
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_recency_compacted";
        let should_compact = Arc::new(AtomicBool::new(false));
        let should_compact_clone = should_compact.clone();
        let scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            move |_state| should_compact_clone.swap(false, Ordering::SeqCst),
        )));
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 64 * 1024, None))
            .with_compactor_builder(
                CompactorBuilder::new(path, object_store.clone())
                    .with_scheduler_supplier(scheduler)
                    .with_options(fast_compactor_options()),
            )
            .build()
            .await
            .unwrap();
        let db = Arc::new(db);

        // Oldest write goes to a sorted run after compaction.
        db.put(b"px:a", b"v_oldest").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Trigger compaction and wait for it to land.
        should_compact.store(true, Ordering::SeqCst);
        let db_poll = db.clone();
        tokio::time::timeout(Duration::from_secs(10), async move {
            loop {
                {
                    let state = db_poll.inner.state.read();
                    if !state.state().core().tree.compacted.is_empty() {
                        return;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();

        // L0 layer (newer than the sorted run, older than the memtable).
        db.put(b"px:a", b"v_l0").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Active memtable (freshest).
        db.put(b"px:a", b"v_memtable").await.unwrap();

        let mut iter = db.scan_prefix_by_recency(b"px:").await.unwrap();
        let e1 = iter.next_entry().await.unwrap().unwrap();
        let e2 = iter.next_entry().await.unwrap().unwrap();
        let e3 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(e1.key.as_ref(), b"px:a");
        assert_value(&e1, b"v_memtable");
        assert_eq!(e2.key.as_ref(), b"px:a");
        assert_value(&e2, b"v_l0");
        assert_eq!(e3.key.as_ref(), b"px:a");
        assert_value(&e3, b"v_oldest");
        assert!(e1.seq > e2.seq);
        assert!(e2.seq > e3.seq);
        assert!(iter.next_entry().await.unwrap().is_none());

        Arc::into_inner(db)
            .expect("db Arc should be uniquely held")
            .close()
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_scan_prefix_by_recency_errors_on_multi_segment_prefix() {
        use crate::manifest::{LsmTreeState, Segment};
        use std::collections::VecDeque;
        use std::sync::Arc;

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_recency_multi_segment", object_store)
            .with_settings(test_db_options(0, 64 * 1024, None))
            .build()
            .await
            .unwrap();

        // Inject two non-nesting segments. Both prefixes share "hour=1"
        // but neither is a prefix of the other, so the antichain holds.
        // A scan with prefix b"hour=" overlaps both segments' intervals,
        // which is the case we want to reject.
        db.inner.state.write().modify(|m| {
            let core = &mut m.state.manifest.value.core;
            core.segment_extractor_name = Some("hour".into());
            core.segments = vec![
                Segment {
                    prefix: Bytes::from_static(b"hour=12/"),
                    tree: Arc::new(LsmTreeState {
                        last_compacted_l0_sst_view_id: None,
                        last_compacted_l0_sst_id: None,
                        l0: VecDeque::new(),
                        compacted: vec![],
                    }),
                },
                Segment {
                    prefix: Bytes::from_static(b"hour=13/"),
                    tree: Arc::new(LsmTreeState {
                        last_compacted_l0_sst_view_id: None,
                        last_compacted_l0_sst_id: None,
                        l0: VecDeque::new(),
                        compacted: vec![],
                    }),
                },
            ];
        });

        match db.scan_prefix_by_recency(b"hour=").await {
            Err(e) => assert_eq!(e.kind(), crate::ErrorKind::Invalid),
            Ok(_) => panic!("expected multi-segment error"),
        }

        // Single-segment prefixes still work.
        let mut iter = db.scan_prefix_by_recency(b"hour=12/").await.unwrap();
        assert!(iter.next_entry().await.unwrap().is_none());

        db.close().await.unwrap();
    }

    #[test]
    fn test_get_after_put() {
        let mut runner = new_proptest_runner(None);
        let runtime = Runtime::new().unwrap();

        let table = sample::table(runner.rng(), 1000, 10);
        let db_options = test_db_options(0, 1024, None);
        let db = runtime.block_on(build_database_from_table(&table, db_options, true));

        runner
            .run(
                &(arbitrary::bytes(100), arbitrary::bytes(100)),
                |(key, value)| {
                    runtime.block_on(async {
                        if !key.is_empty() {
                            db.put_with_options(
                                &key,
                                &value,
                                &PutOptions::default(),
                                &WriteOptions {
                                    ..Default::default()
                                },
                            )
                            .await
                            .unwrap();
                            assert_eq!(
                                Some(value),
                                db.get_with_options(
                                    &key,
                                    &ReadOptions {
                                        durability_filter: Memory,
                                        dirty: false,
                                        cache_blocks: true,
                                        filter_context: None,
                                        tracing_options: None,
                                    }
                                )
                                .await
                                .unwrap()
                            );
                        }
                    });
                    Ok(())
                },
            )
            .unwrap();
    }

    #[tokio::test]
    async fn test_no_flush_interval() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db_options_no_flush_interval = {
            let mut db_options = test_db_options(0, 1024, None);
            db_options.flush_interval = None;
            db_options
        };
        let kv_store = Db::builder("/tmp/test_kv_store", object_store)
            .with_settings(db_options_no_flush_interval)
            .build()
            .await
            .unwrap();
        let key = b"test_key";
        let value = b"test_value";

        kv_store
            .put_with_options(
                key,
                value,
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // a sanity check: the wal contains the most recent write
        assert_ne!(
            kv_store
                .inner
                .wal_observer
                .status()
                .unwrap()
                .estimated_bytes,
            0
        );

        // and a flush() should clear it
        kv_store.flush().await.unwrap();
        assert_eq!(
            kv_store
                .inner
                .wal_observer
                .status()
                .unwrap()
                .estimated_bytes,
            0
        );
    }

    #[tokio::test]
    async fn test_close_triggers_flush_when_open() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db_options_no_flush_interval = {
            let mut db_options = test_db_options(0, 1024, None);
            db_options.flush_interval = None;
            db_options
        };
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let kv_store = Db::builder("/tmp/test_close_triggers_flush", object_store)
            .with_settings(db_options_no_flush_interval)
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();

        kv_store
            .put_with_options(
                b"test_key",
                b"test_value",
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Sanity check: WAL has buffered entries before close.
        assert_eq!(
            kv_store
                .inner
                .wal_observer
                .status()
                .unwrap()
                .buffered_wal_entries_count,
            1
        );
        assert_eq!(
            lookup_metric(
                &metrics_recorder,
                crate::wal_buffer_stats::WAL_BUFFER_FLUSHES
            )
            .unwrap(),
            0
        );

        kv_store.close().await.unwrap();

        // close() should trigger a flush when the db is open.
        assert_eq!(
            kv_store
                .inner
                .wal_observer
                .status()
                .unwrap_err()
                .buffered_wal_entries_count,
            0
        );
        assert_eq!(
            lookup_metric(
                &metrics_recorder,
                crate::wal_buffer_stats::WAL_BUFFER_FLUSHES
            )
            .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_close_twice_returns_closed_clean() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_close_twice_returns_closed_clean", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        db.close().await.unwrap();
        let err = db.close().await.unwrap_err();
        assert_eq!(err.kind(), crate::ErrorKind::Closed(CloseReason::Clean));
    }

    #[tokio::test]
    async fn test_close_failed_state_does_not_flush() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db_options_no_flush_interval = {
            let mut db_options = test_db_options(0, 1024, None);
            db_options.flush_interval = None;
            db_options
        };
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder("/tmp/test_close_failed_state_no_flush", object_store)
            .with_settings(db_options_no_flush_interval)
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();

        let put_seq = db
            .put_with_options(
                b"test_key",
                b"test_value",
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .seq;

        // Sanity check: WAL has buffered entries before close.
        let wal_status = db.inner.wal_observer.status().unwrap();
        assert_eq!(wal_status.buffered_wal_entries_count, 1);
        assert!(wal_status.last_flushed_seq.unwrap_or(0) < put_seq);
        assert_eq!(
            lookup_metric(
                &metrics_recorder,
                crate::wal_buffer_stats::WAL_BUFFER_FLUSHES
            )
            .unwrap(),
            0
        );

        // Simulate a failed state (e.g. fenced).
        db.inner
            .status_manager
            .write_result(Err(SlateDBError::Fenced));

        // close() should succeed but not flush when failed.
        db.close().await.unwrap();

        let wal_status = db.inner.wal_observer.status().unwrap_err();
        assert!(matches!(wal_status.closed_reason, Some(WalError::Fenced)));
        assert!(wal_status.last_flushed_seq.unwrap_or(0) < put_seq);
        assert_eq!(
            lookup_metric(
                &metrics_recorder,
                crate::wal_buffer_stats::WAL_BUFFER_FLUSHES
            )
            .unwrap(),
            0
        );
        let status = db.status();
        assert_eq!(status.close_reason, Some(CloseReason::Fenced));
    }

    #[tokio::test]
    async fn test_clean_close_flushes_pending_wal() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db_options_no_flush_interval = {
            let mut db_options = test_db_options(0, 1024, None);
            db_options.flush_interval = None;
            db_options
        };
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder("/tmp/test_clean_close_flushes_pending_wal", object_store)
            .with_settings(db_options_no_flush_interval)
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();

        let put_seq = db
            .put_with_options(
                b"test_key",
                b"test_value",
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .seq;

        let wal_status = db.inner.wal_observer.status().unwrap();
        assert_eq!(wal_status.buffered_wal_entries_count, 1);
        assert!(wal_status.last_flushed_seq.unwrap_or(0) < put_seq);
        assert_eq!(
            lookup_metric(
                &metrics_recorder,
                crate::wal_buffer_stats::WAL_BUFFER_FLUSHES
            )
            .unwrap(),
            0
        );

        db.close().await.unwrap();

        let wal_status = db.inner.wal_observer.status().unwrap_err();
        assert!(matches!(wal_status.closed_reason, Some(WalError::Closed)));
        assert_eq!(wal_status.last_flushed_seq, Some(put_seq));
        assert_eq!(
            lookup_metric(
                &metrics_recorder,
                crate::wal_buffer_stats::WAL_BUFFER_FLUSHES
            )
            .unwrap(),
            1
        );
        let status = db.status();
        assert_eq!(status.close_reason, Some(CloseReason::Clean));
    }

    #[tokio::test]
    async fn test_close_flushes_memtables_to_l0() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db_options = {
            let mut db_options = test_db_options(0, 1024, None);
            db_options.flush_interval = None;
            db_options
        };
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder("/tmp/test_close_flushes_memtables_to_l0", object_store)
            .with_settings(db_options)
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();

        db.put_with_options(
            b"test_key",
            b"test_value",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        // No L0 flushes should have happened yet.
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::L0_FLUSH_BYTES).unwrap_or(0),
            0
        );

        db.close().await.unwrap();

        // close() should have flushed memtables to L0.
        assert!(
            lookup_metric(&metrics_recorder, crate::db_stats::L0_FLUSH_BYTES).unwrap() > 0,
            "expected L0 flush during close"
        );
    }

    #[tokio::test]
    async fn test_close_with_options_default_flushes_final_memtable() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let mut settings = test_db_options(0, 1024, None);
        settings.flush_interval = None;
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder(
            "/tmp/test_close_with_options_default_flushes_final_memtable",
            object_store,
        )
        .with_settings(settings)
        .with_metrics_recorder(metrics_recorder.clone())
        .build()
        .await
        .unwrap();

        db.put(b"test_key", b"test_value").await.unwrap();

        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::L0_FLUSH_BYTES).unwrap_or(0),
            0
        );

        db.close_with_options(CloseOptions::default())
            .await
            .unwrap();

        assert!(
            lookup_metric(&metrics_recorder, crate::db_stats::L0_FLUSH_BYTES).unwrap() > 0,
            "expected L0 flush during close with default options"
        );
    }

    #[tokio::test]
    async fn test_close_with_options_flushes_wal_only() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let mut settings = test_db_options(0, 1024, None);
        settings.flush_interval = None;
        let path = "/tmp/test_close_with_options_flushes_wal_only";
        let db = Db::builder(path, object_store.clone())
            .with_settings(settings.clone())
            .build()
            .await
            .unwrap();

        db.put(b"test_key", b"test_value").await.unwrap();

        db.close_with_options(CloseOptions::default().with_flush_type(Some(FlushType::Wal)))
            .await
            .unwrap();

        let manifest = db.manifest();
        assert!(
            manifest.manifest.core.replay_after_wal_id + 1 < manifest.manifest.core.next_wal_sst_id,
            "expected a data WAL after the replay watermark"
        );
        assert!(
            manifest.manifest.core.tree.l0.is_empty(),
            "expected no flushed memtables in the manifest"
        );

        let reopened = Db::builder(path, object_store)
            .with_settings(settings)
            .build()
            .await
            .unwrap();
        assert_eq!(
            reopened.get(b"test_key").await.unwrap(),
            Some(Bytes::from_static(b"test_value"))
        );
        reopened.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_close_with_options_skips_final_flush() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let mut settings = test_db_options(0, 1024, None);
        settings.flush_interval = None;
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder(
            "/tmp/test_close_with_options_skips_final_flush",
            object_store,
        )
        .with_settings(settings)
        .with_metrics_recorder(metrics_recorder.clone())
        .build()
        .await
        .unwrap();

        db.put(b"test_key", b"test_value").await.unwrap();

        db.close_with_options(CloseOptions::default().with_flush_type(None))
            .await
            .unwrap();

        assert_eq!(
            lookup_metric(&metrics_recorder, crate::wal_buffer_stats::WAL_FLUSH_BYTES,)
                .unwrap_or(0),
            0
        );
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::L0_FLUSH_BYTES).unwrap_or(0),
            0
        );
    }

    #[cfg(feature = "wal_disable")]
    #[tokio::test]
    async fn test_close_without_flush_fails_pending_durability_wait() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let mut settings = test_db_options(0, 1024, None);
        settings.flush_interval = None;
        settings.wal_enabled = false;
        let db = Db::builder(
            "/tmp/test_close_without_flush_fails_pending_durability_wait",
            object_store,
        )
        .with_settings(settings)
        .build()
        .await
        .unwrap();

        let handle = db.put(b"key", b"value").await.unwrap();

        db.close_with_options(CloseOptions::default().with_flush_type(None))
            .await
            .unwrap();

        let error = tokio::time::timeout(Duration::from_secs(5), handle.await_durable())
            .await
            .expect("durability wait remained blocked after close")
            .expect_err("discarded write should not become durable");
        assert!(matches!(
            error.kind(),
            crate::ErrorKind::Closed(CloseReason::Clean)
        ));
    }

    #[tokio::test]
    async fn test_memtable_write_bytes_matches_batch_payload() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder(
            "/tmp/test_memtable_write_bytes_matches_batch_payload",
            object_store,
        )
        .with_settings(test_db_options(0, 1024 * 1024, None))
        .with_metrics_recorder(metrics_recorder.clone())
        .build()
        .await
        .unwrap();

        db.put(b"hello", b"world!").await.unwrap();
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::MEMTABLE_WRITE_BYTES).unwrap(),
            (b"hello".len() + b"world!".len()) as i64,
        );

        db.put(b"k2", b"v2").await.unwrap();
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::MEMTABLE_WRITE_BYTES).unwrap(),
            (b"hello".len() + b"world!".len() + b"k2".len() + b"v2".len()) as i64,
        );

        // Deletes count only the key length.
        db.delete(b"k3").await.unwrap();
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::MEMTABLE_WRITE_BYTES).unwrap(),
            (b"hello".len() + b"world!".len() + b"k2".len() + b"v2".len() + b"k3".len()) as i64,
        );

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_memtable_write_bytes_matches_batch_payload_with_merges() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder(
            "/tmp/test_memtable_write_bytes_matches_batch_payload_with_merges",
            object_store,
        )
        .with_settings(test_db_options(0, 1024 * 1024, None))
        .with_merge_operator(Arc::new(StringConcatMergeOperator {}))
        .with_metrics_recorder(metrics_recorder.clone())
        .build()
        .await
        .unwrap();

        // A single merge per key is not folded by the merge iterator, so the
        // tracked size matches key + value.
        db.merge(b"k1", b"a").await.unwrap();
        let mut expected = (b"k1".len() + b"a".len()) as i64;
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::MEMTABLE_WRITE_BYTES).unwrap(),
            expected,
        );

        // Multiple merges for the same key in a single batch are folded by the
        // merge iterator before counting. memtable_write_bytes reflects the merged
        // output (key + "abc"), not the sum of raw inputs (3 * key + "a" + "b" + "c").
        let mut batch = WriteBatch::new();
        batch.merge(b"k2", b"a");
        batch.merge(b"k2", b"b");
        batch.merge(b"k2", b"c");
        db.write(batch).await.unwrap();
        expected += (b"k2".len() + b"abc".len()) as i64;
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::MEMTABLE_WRITE_BYTES).unwrap(),
            expected,
        );

        // Merges to distinct keys in one batch are not folded, so each entry
        // contributes its raw key + value.
        let mut batch = WriteBatch::new();
        batch.merge(b"k3", b"x");
        batch.merge(b"k4", b"yy");
        db.write(batch).await.unwrap();
        expected += (b"k3".len() + b"x".len() + b"k4".len() + b"yy".len()) as i64;
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::MEMTABLE_WRITE_BYTES).unwrap(),
            expected,
        );

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_wal_flush_bytes_after_explicit_flush() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db_options = {
            let mut db_options = test_db_options(0, 1024 * 1024, None);
            db_options.flush_interval = None;
            db_options
        };
        let db = Db::builder(
            "/tmp/test_wal_flush_bytes_after_explicit_flush",
            object_store,
        )
        .with_settings(db_options)
        .with_metrics_recorder(metrics_recorder.clone())
        .build()
        .await
        .unwrap();

        db.put_with_options(
            b"hello",
            b"world",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::wal_buffer_stats::WAL_FLUSH_BYTES,)
                .unwrap_or(0),
            0,
        );

        db.flush().await.unwrap();

        let wal_bytes =
            lookup_metric(&metrics_recorder, crate::wal_buffer_stats::WAL_FLUSH_BYTES).unwrap();
        let memtable_bytes =
            lookup_metric(&metrics_recorder, crate::db_stats::MEMTABLE_WRITE_BYTES).unwrap();
        // WAL SST framing/footer makes the encoded payload at least as large as
        // the memtable payload if no compression is used.
        assert!(
            wal_bytes >= memtable_bytes,
            "wal_bytes={wal_bytes} memtable_bytes={memtable_bytes}",
        );
        db.close().await.unwrap();
    }

    #[tokio::test]
    #[cfg(feature = "wal_disable")]
    async fn test_get_with_durability_level_when_wal_disabled() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let mut options = test_db_options(0, 1024 * 1024, None);
        options.wal_enabled = false;
        let db = Db::builder("/tmp/test_kv_store", object_store)
            .with_settings(options)
            .build()
            .await
            .unwrap();
        tokio::time::timeout(
            Duration::from_secs(1),
            db.task_executor
                .join_task(crate::wal::slatedb::writer::WAL_BUFFER_TASK_NAME),
        )
        .await
        .expect("native WAL task should not run when the WAL is disabled")
        .unwrap();
        let put_options = PutOptions::default();
        let write_options = WriteOptions {
            ..Default::default()
        };
        let get_memory_options = ReadOptions::new().with_durability_filter(Memory);
        let get_remote_options = ReadOptions::new().with_durability_filter(Remote);

        db.put_with_options(b"foo", b"bar", &put_options, &write_options)
            .await
            .unwrap();
        let val_bytes = Bytes::copy_from_slice(b"bar");
        assert_eq!(
            None,
            db.get_with_options(b"foo", &get_remote_options)
                .await
                .unwrap()
        );
        assert_eq!(
            Some(val_bytes.clone()),
            db.get_with_options(b"foo", &get_memory_options)
                .await
                .unwrap()
        );
        db.flush().await.unwrap();
        assert_eq!(
            Some(val_bytes.clone()),
            db.get_with_options(b"foo", &get_remote_options)
                .await
                .unwrap()
        );
        assert_eq!(
            Some(val_bytes.clone()),
            db.get_with_options(b"foo", &get_memory_options)
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    #[cfg(feature = "wal_disable")]
    async fn test_find_with_multiple_repeated_keys() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let mut options = test_db_options(0, 1024 * 1024, None);
        options.wal_enabled = false;
        let db = Db::builder("/tmp/test_kv_store", object_store)
            .with_settings(options)
            .build()
            .await
            .unwrap();

        // write enough rows with the same key that we yield an L0 SST with multiple blocks
        let mut last_val: String = "foo".to_string();
        for x in 0..4096 {
            let val = format!("val{}", x);
            db.put_with_options(
                b"key",
                val.as_bytes(),
                &PutOptions {
                    ttl: Default::default(),
                },
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            last_val = val;
            if db
                .inner
                .state
                .write()
                .memtable()
                .metadata()
                .entries_size_in_bytes
                > (SsTableFormat::default().block_size * 3)
            {
                break;
            }
        }
        assert_eq!(
            Some(Bytes::copy_from_slice(last_val.as_bytes())),
            db.get_with_options(b"key", &ReadOptions::new().with_durability_filter(Memory))
                .await
                .unwrap()
        );
        db.flush().await.unwrap();

        let state = db.inner.state.read().view();
        assert_eq!(1, state.state.manifest.value.core.tree.l0.len());
        let view = state.state.manifest.value.core.tree.l0.front().unwrap();
        let index = db
            .inner
            .table_store
            .read_index(
                &view.sst,
                true,
                Some(Bytes::new()),
                &crate::reader::ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        assert!(!index.borrow().block_meta().is_empty());
        assert_eq!(
            Some(Bytes::copy_from_slice(last_val.as_bytes())),
            db.get(b"key").await.unwrap()
        );
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_db_records_main_and_wal_object_store_requests_separately() {
        // given:
        let main_object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let wal_object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let path = "/tmp/test_db_records_main_and_wal_object_store_requests_separately";
        let db = Db::builder(path, main_object_store)
            .with_settings(test_db_options(0, 1024, None))
            .with_wal_object_store(wal_object_store)
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();

        let wal_before =
            lookup_object_store_op_request_count(&metrics_recorder, "db", "wal", "put");
        let main_before =
            lookup_object_store_op_request_count(&metrics_recorder, "db", "main", "put");
        let wal_hist_before =
            lookup_object_store_op_histogram_count(&metrics_recorder, "db", "wal", "put");
        let main_hist_before =
            lookup_object_store_op_histogram_count(&metrics_recorder, "db", "main", "put");

        // when:
        db.put(b"key", b"value").await.unwrap();
        db.flush().await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        let wal_after = lookup_object_store_op_request_count(&metrics_recorder, "db", "wal", "put");
        let main_after =
            lookup_object_store_op_request_count(&metrics_recorder, "db", "main", "put");
        let wal_hist_after =
            lookup_object_store_op_histogram_count(&metrics_recorder, "db", "wal", "put");
        let main_hist_after =
            lookup_object_store_op_histogram_count(&metrics_recorder, "db", "main", "put");

        // then:
        assert!(wal_after > wal_before);
        assert!(main_after > main_before);
        assert!(wal_hist_after > wal_hist_before);
        assert!(main_hist_after > main_hist_before);
        db.close().await.unwrap();
    }

    async fn build_database_from_table(
        table: &BTreeMap<Bytes, Bytes>,
        db_options: Settings,
        wait_for_durability: bool,
    ) -> Db {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_kv_store", object_store)
            .with_settings(db_options)
            .build()
            .await
            .unwrap();

        test_utils::seed_database(&db, table, false).await.unwrap();

        if wait_for_durability {
            db.flush().await.unwrap();
        }

        db
    }

    #[tokio::test]
    async fn test_should_allow_iterating_behind_box_dyn() {
        #[async_trait]
        trait IteratorSupplier {
            async fn iterator(&self) -> Box<dyn IteratorTrait>;
        }

        struct DbHolder {
            db: Db,
        }

        #[async_trait]
        impl IteratorSupplier for DbHolder {
            async fn iterator(&self) -> Box<dyn IteratorTrait> {
                let range = BytesRange::new_empty();
                let iter = self
                    .db
                    .inner
                    .scan_with_options(range, &ScanOptions::default(), None)
                    .await
                    .unwrap();
                Box::new(iter)
            }
        }

        #[async_trait]
        trait IteratorTrait {
            async fn next(&mut self) -> Result<Option<KeyValue>, crate::Error>;
        }

        #[async_trait]
        impl IteratorTrait for DbIterator {
            async fn next(&mut self) -> Result<Option<KeyValue>, crate::Error> {
                DbIterator::next(self).await
            }
        }

        let db_options = test_db_options(0, 1024, None);
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_kv_store", object_store)
            .with_settings(db_options)
            .build()
            .await
            .unwrap();
        let db_holder = DbHolder { db };
        let mut boxed = db_holder.iterator().await;
        let next = boxed.next().await;
        assert_eq!(next.unwrap(), None);
    }

    async fn assert_records_in_range(
        table: &BTreeMap<Bytes, Bytes>,
        db: &Db,
        scan_options: &ScanOptions,
        range: BytesRange,
    ) {
        let mut iter = db
            .inner
            .scan_with_options(range.clone(), scan_options, None)
            .await
            .unwrap();
        test_utils::assert_ranged_db_scan(table, range, IterationOrder::Ascending, &mut iter).await;
    }

    #[test]
    fn test_scan_returns_records_in_range() {
        let mut runner = new_proptest_runner(None);
        let table = sample::table(runner.rng(), 1000, 5);

        let runtime = Runtime::new().unwrap();
        let db_options = test_db_options(0, 1024, None);
        let db = runtime.block_on(build_database_from_table(&table, db_options, true));

        runner
            .run(&arbitrary::nonempty_range(10), |range| {
                runtime.block_on(assert_records_in_range(
                    &table,
                    &db,
                    &ScanOptions::default(),
                    range,
                ));
                Ok(())
            })
            .unwrap();
    }

    fn new_proptest_runner(rng_seed: Option<[u8; 32]>) -> TestRunner {
        proptest_util::runner::new(file!(), rng_seed)
    }

    #[test]
    fn test_scan_returns_uncommitted_records_if_read_level_uncommitted() {
        let mut runner = new_proptest_runner(None);
        let table = sample::table(runner.rng(), 1000, 5);

        let runtime = Runtime::new().unwrap();
        let mut db_options = test_db_options(0, 1024, None);
        db_options.flush_interval = Some(Duration::from_secs(5));
        let db = runtime.block_on(build_database_from_table(&table, db_options, false));

        runner
            .run(&arbitrary::nonempty_range(10), |range| {
                let scan_options = ScanOptions {
                    durability_filter: Memory,
                    ..ScanOptions::default()
                };
                runtime.block_on(assert_records_in_range(&table, &db, &scan_options, range));
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn test_seek_outside_of_range_returns_invalid_argument() {
        let mut runner = new_proptest_runner(None);
        let table = sample::table(runner.rng(), 1000, 10);

        let runtime = Runtime::new().unwrap();
        let db_options = test_db_options(0, 1024, None);
        let db = runtime.block_on(build_database_from_table(&table, db_options, true));

        runner
            .run(
                &(arbitrary::nonempty_bytes(10), arbitrary::rng()),
                |(arbitrary_key, mut rng)| {
                    runtime.block_on(assert_out_of_bound_seek_returns_invalid_argument(
                        &db,
                        &mut rng,
                        arbitrary_key,
                    ));
                    Ok(())
                },
            )
            .unwrap();

        async fn assert_out_of_bound_seek_returns_invalid_argument(
            db: &Db,
            rng: &mut TestRng,
            arbitrary_key: Bytes,
        ) {
            let mut iter = db
                .scan_with_options(..arbitrary_key.clone(), &ScanOptions::default())
                .await
                .unwrap();

            let lower_bounded_range = BytesRange::from(arbitrary_key.clone()..);
            let value = sample::bytes_in_range(rng, &lower_bounded_range);
            let err = iter.seek(value.clone()).await.unwrap_err();
            assert!(
                err.to_string()
                    .contains("cannot seek to a key outside the iterator range"),
                "{}",
                err
            );

            let mut iter = db
                .scan_with_options(arbitrary_key.clone().., &ScanOptions::default())
                .await
                .unwrap();

            let upper_bounded_range = BytesRange::from(..arbitrary_key.clone());
            let value = sample::bytes_in_range(rng, &upper_bounded_range);
            let err = iter.seek(value.clone()).await.unwrap_err();
            assert!(
                err.to_string()
                    .contains("cannot seek to a key outside the iterator range"),
                "{}",
                err
            );
        }
    }

    #[test]
    fn test_seek_fast_forwards_iterator() {
        let mut runner = new_proptest_runner(None);
        let table = sample::table(runner.rng(), 1000, 10);

        let runtime = Runtime::new().unwrap();
        let db_options = test_db_options(0, 1024, None);
        let db = runtime.block_on(build_database_from_table(&table, db_options, true));

        runner
            .run(
                &(arbitrary::nonempty_range(5), arbitrary::rng()),
                |(range, mut rng)| {
                    runtime.block_on(assert_seek_fast_forwards_iterator(
                        &table, &db, &range, &mut rng,
                    ));
                    Ok(())
                },
            )
            .unwrap();

        async fn assert_seek_fast_forwards_iterator(
            table: &BTreeMap<Bytes, Bytes>,
            db: &Db,
            scan_range: &BytesRange,
            rng: &mut TestRng,
        ) {
            let mut iter = db
                .inner
                .scan_with_options(scan_range.clone(), &ScanOptions::default(), None)
                .await
                .unwrap();

            let seek_key = sample::bytes_in_range(rng, scan_range);
            iter.seek(seek_key.clone()).await.unwrap();

            let seek_range = BytesRange::new(
                Included(seek_key),
                std::ops::RangeBounds::end_bound(scan_range).cloned(),
            );
            test_utils::assert_ranged_db_scan(
                table,
                seek_range,
                IterationOrder::Ascending,
                &mut iter,
            )
            .await;
        }
    }

    #[tokio::test]
    async fn test_write_batch() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let kv_store = Db::builder("/tmp/test_kv_store", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        // Create a new WriteBatch
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.put(b"key2", b"value2");
        batch.delete(b"key1");

        // Write the batch
        kv_store.write(batch).await.expect("write batch failed");

        // Read back keys
        assert_eq!(kv_store.get(b"key1").await.unwrap(), None);
        assert_eq!(
            kv_store.get(b"key2").await.unwrap(),
            Some(Bytes::from_static(b"value2"))
        );

        kv_store.close().await.unwrap();
    }

    #[cfg(feature = "wal_disable")]
    #[tokio::test]
    async fn test_write_batch_without_wal() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        // Use a very small l0 size to force flushes so await is notified
        let mut options = test_db_options(0, 8, None);

        // Disable WAL
        options.wal_enabled = false;

        let kv_store = Db::builder("/tmp/test_kv_store_without_wal", object_store.clone())
            .with_settings(options)
            .build()
            .await
            .unwrap();

        // Create a new WriteBatch
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.put(b"key2", b"value2");
        batch.delete(b"key1");

        // Write the batch
        kv_store.write(batch).await.expect("write batch failed");

        // Read back keys
        assert_eq!(kv_store.get(b"key1").await.unwrap(), None);
        assert_eq!(kv_store.get(b"key2").await.unwrap(), Some("value2".into()));

        kv_store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_write_batch_with_empty_key() {
        let mut batch = WriteBatch::new();
        let result = std::panic::catch_unwind(move || {
            batch.put(b"", b"value");
        });
        assert!(
            result.is_err(),
            "Expected panic when using empty key in put operation"
        );

        let mut batch = WriteBatch::new();
        let result = std::panic::catch_unwind(move || {
            batch.delete(b"");
        });
        assert!(
            result.is_err(),
            "Expected panic when using empty key in delete operation"
        );
    }

    /// Test that batch writes are atomic. Test does the following:
    ///
    /// - A set of, say 100 keys, 1-100
    /// - Two tasks writing, one writing value to be same key, and other
    ///   writing it to be key*2.
    /// - We wait for both to complete.
    /// - Assert that either all values are same as key, or all values key*2.
    /// - Repeat above loop few times.
    ///
    /// _Note: This test is non-deterministic because it depends on the async
    /// runtime to schedule the tasks in a way that the writes are concurrent._
    #[tokio::test]
    async fn test_concurrent_batch_writes_consistency() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let kv_store = Arc::new(
            Db::builder("/tmp/test_concurrent_kv_store", object_store)
                .with_settings(test_db_options(
                    0,
                    1024,
                    // Enable compactor to prevent l0 from filling up and
                    // applying backpressure indefinitely.
                    Some(CompactorOptions {
                        poll_interval: Duration::from_millis(100),
                        max_concurrent_compactions: 1,
                        manifest_update_timeout: Duration::from_secs(300),
                        worker: Some(CompactionWorkerOptions {
                            max_sst_size: 256,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                ))
                .build()
                .await
                .unwrap(),
        );

        const NUM_KEYS: usize = 100;
        const NUM_ROUNDS: usize = 20;

        for _ in 0..NUM_ROUNDS {
            // Write two tasks that write to the same keys
            let task1 = {
                let store = kv_store.clone();
                tokio::spawn(async move {
                    let mut batch = WriteBatch::new();
                    for key in 1..=NUM_KEYS {
                        batch.put(key.to_be_bytes(), key.to_be_bytes());
                    }
                    store.write(batch).await.expect("write batch failed");
                })
            };

            let task2 = {
                let store = kv_store.clone();
                tokio::spawn(async move {
                    let mut batch = WriteBatch::new();
                    for key in 1..=NUM_KEYS {
                        let value = (key * 2).to_be_bytes();
                        batch.put(key.to_be_bytes(), value);
                    }
                    store.write(batch).await.expect("write batch failed");
                })
            };

            // Wait for both tasks to complete
            join_all(vec![task1, task2]).await;

            // Ensure consistency: all values must be either key or key * 2
            let mut all_key = true;
            let mut all_key2 = true;

            for key in 1..=NUM_KEYS {
                let value = kv_store.get(key.to_be_bytes()).await.unwrap();
                let value = value.expect("Value should exist");

                if value.as_ref() != key.to_be_bytes() {
                    all_key = false;
                }
                if value.as_ref() != (key * 2).to_be_bytes() {
                    all_key2 = false;
                }
            }

            // Assert that the result is consistent: either all key or all key * 2
            assert!(
                all_key || all_key2,
                "Inconsistent state: not all values match either key or key * 2"
            );
        }

        kv_store.close().await.unwrap();
    }

    #[tokio::test]
    #[cfg(feature = "wal_disable")]
    async fn test_disable_wal_after_wal_enabled() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        // open a db and write a wal entry
        let options = test_db_options(0, 32, None);
        let db = Db::builder(path, object_store.clone())
            .with_settings(options)
            .build()
            .await
            .unwrap();
        db.put(&[b'a'; 4], &[b'j'; 4]).await.unwrap();
        db.put(&[b'b'; 4], &[b'k'; 4]).await.unwrap();
        db.close().await.unwrap();

        // open a db with wal disabled and write a memtable
        let mut options = test_db_options(0, 32, None);
        options.wal_enabled = false;
        let db = Db::builder(path, object_store.clone())
            .with_settings(options.clone())
            .build()
            .await
            .unwrap();
        db.delete_with_options(
            &[b'b'; 4],
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put(&[b'a'; 4], &[b'z'; 64]).await.unwrap();
        db.close().await.unwrap();

        // ensure we don't overwrite the values we just put on a reload
        let db = Db::builder(path, object_store.clone())
            .with_settings(options.clone())
            .build()
            .await
            .unwrap();
        let val = db.get(&[b'a'; 4]).await.unwrap();
        assert_eq!(val.unwrap(), Bytes::copy_from_slice(&[b'z'; 64]));
        let val = db.get(&[b'b'; 4]).await.unwrap();
        assert!(val.is_none());
    }

    #[cfg(feature = "wal_disable")]
    #[tokio::test]
    async fn test_wal_disabled() {
        use crate::{test_utils::assert_iterator, types::RowEntry};

        let clock = Arc::new(MockSystemClock::new());
        let mut options = test_db_options(0, 350, None);
        options.wal_enabled = false;
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let sst_format = SsTableFormat::default();
        let table_store = Arc::new(TableStore::new(
            object_store.clone(),
            sst_format,
            path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let db = Db::builder(path.clone(), object_store.clone())
            .with_settings(options)
            .with_system_clock(clock.clone())
            .build()
            .await
            .unwrap();
        let manifest_store = Arc::new(ManifestStore::new(&path, object_store.clone()));
        let mut stored_manifest =
            StoredManifest::load(manifest_store.clone(), Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let write_options = WriteOptions {
            ..Default::default()
        };

        db.put_with_options(
            &[b'a'; 32],
            &[b'j'; 32],
            &PutOptions::default(),
            &write_options,
        )
        .await
        .unwrap();
        db.delete_with_options(&[b'b'; 31], &write_options)
            .await
            .unwrap();

        // ensure the memtable's size is greater than l0_sst_size_bytes, or
        // the memtable will not be flushed to l0, and the test will hang
        // at this put_with_options call.
        let write_options = WriteOptions {
            ..Default::default()
        };
        clock.set(10);
        db.put_with_options(
            &[b'c'; 32],
            &[b'l'; 32],
            &PutOptions::default(),
            &write_options,
        )
        .await
        .unwrap()
        .await_durable()
        .await
        .unwrap();

        let state = wait_for_manifest_condition(
            &mut stored_manifest,
            |s| !s.tree.l0.is_empty(),
            Duration::from_secs(30),
        )
        .await;
        assert_eq!(state.tree.l0.len(), 1);

        let l0 = state.tree.l0.front().unwrap();
        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            l0,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");
        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_value(&[b'a'; 32], &[b'j'; 32], 1).with_create_ts(0),
                RowEntry::new_tombstone(&[b'b'; 31], 2).with_create_ts(0),
                RowEntry::new_value(&[b'c'; 32], &[b'l'; 32], 3).with_create_ts(10),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_put_flushes_memtable() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let kv_store = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 320, None))
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();
        let manifest_store = Arc::new(ManifestStore::new(&Path::from(path), object_store.clone()));
        let mut stored_manifest =
            StoredManifest::load(manifest_store.clone(), Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let sst_format = SsTableFormat {
            min_filter_keys: 10,
            ..SsTableFormat::default()
        };
        let table_store = Arc::new(TableStore::new(
            object_store.clone(),
            sst_format,
            path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));

        // Write data a few times such that each loop results in a memtable flush
        let mut last_wal_id = 0;
        for i in 0..3 {
            let key = [b'a' + i; 16];
            let value = [b'b' + i; 50];
            kv_store
                .put(&key, &value)
                .await
                .unwrap()
                .await_durable()
                .await
                .unwrap();
            let key = [b'j' + i; 16];
            let value = [b'k' + i; 50];
            kv_store
                .put(&key, &value)
                .await
                .unwrap()
                .await_durable()
                .await
                .unwrap();
            let db_state = wait_for_manifest_condition(
                &mut stored_manifest,
                |s| s.replay_after_wal_id > last_wal_id,
                Duration::from_secs(30),
            )
            .await;

            // 2 wal per iteration.
            assert_eq!(db_state.replay_after_wal_id, (i as u64) * 2 + 2);
            last_wal_id = db_state.replay_after_wal_id
        }

        let manifest = stored_manifest.refresh().await.unwrap();
        let l0 = &manifest.core.tree.l0;
        assert_eq!(l0.len(), 3);
        let sst_iter_options = SstIteratorOptions::default();

        for i in 0u8..3u8 {
            let sst1 = l0.get(2 - i as usize).unwrap();
            let mut iter = SstIterator::new_borrowed_initialized(
                ..,
                sst1,
                table_store.clone(),
                sst_iter_options.clone(),
            )
            .await
            .unwrap()
            .expect("Expected Some(iter) but got None");
            let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
            assert_eq!(kv.key.as_ref(), [b'a' + i; 16]);
            assert_eq!(kv.value.as_ref(), [b'b' + i; 50]);
            let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
            assert_eq!(kv.key.as_ref(), [b'j' + i; 16]);
            assert_eq!(kv.value.as_ref(), [b'k' + i; 50]);
            let kv = iter.next().await.unwrap().map(KeyValue::from);
            assert!(kv.is_none());
        }
        assert!(lookup_metric(&metrics_recorder, IMMUTABLE_MEMTABLE_FLUSHES).is_some_and(|v| v > 0));
    }

    #[tokio::test]
    async fn test_put_flushes_memtable_after_max_wal_flushes() {
        const MAX_WAL_FLUSHES_BEFORE_L0_FLUSH: u64 = 4096;

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_flush_memtable_max_wal_flushes";

        let mut settings = test_db_options(0, 64 * 1024 * 1024, None);
        settings.flush_interval = None; // Disable flushing
        settings.max_wal_flushes_before_l0_flush = MAX_WAL_FLUSHES_BEFORE_L0_FLUSH;

        let kv_store = Db::builder(path, object_store.clone())
            .with_settings(settings)
            .build()
            .await
            .unwrap();

        let manifest_store = Arc::new(ManifestStore::new(&Path::from(path), object_store.clone()));
        let mut stored_manifest =
            StoredManifest::load(manifest_store.clone(), Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();

        let write_options: WriteOptions = WriteOptions {
            ..Default::default()
        };
        let put_options = PutOptions::default();

        for i in 0..(MAX_WAL_FLUSHES_BEFORE_L0_FLUSH - 1) {
            let key = format!("key{:08}", i);
            kv_store
                .put_with_options(key.as_bytes(), b"v", &put_options, &write_options)
                .await
                .unwrap();
            kv_store.flush().await.unwrap();
        }

        // Verify WALs flushes.
        let wal_id = kv_store
            .inner
            .wal_observer
            .status()
            .unwrap()
            .last_flushed_wal_id;
        assert_eq!(wal_id, MAX_WAL_FLUSHES_BEFORE_L0_FLUSH); // account for the empty WAL written for fencing

        // Verify no memtable was frozen or L0 flush happened.
        {
            let guard = kv_store.inner.state.read();
            assert!(guard.state().imm_memtable.is_empty());
            assert_eq!(guard.state().core().tree.l0.len(), 0);
        }

        // This put() triggers a freeze.
        let key = format!("key{:08}", MAX_WAL_FLUSHES_BEFORE_L0_FLUSH - 1);
        kv_store
            .put_with_options(key.as_bytes(), b"v", &put_options, &write_options)
            .await
            .unwrap();
        // Flush the WAL so the manifest writer can proceed (flush_interval is
        // disabled in this test, so there is no periodic WAL flush).
        kv_store.flush().await.unwrap();

        // Verify that the WAL count threshold triggered a memtable freeze and L0 flush.
        // replay_after_wal_id should have advanced to the threshold, and there should
        // be exactly one L0 SST.
        let db_state = wait_for_manifest_condition(
            &mut stored_manifest,
            |s| s.replay_after_wal_id == MAX_WAL_FLUSHES_BEFORE_L0_FLUSH,
            Duration::from_secs(30),
        )
        .await;
        assert_eq!(db_state.tree.l0.len(), 1);

        // Run MAX_WAL_FLUSHES_BEFORE_L0_FLUSH more put()/flush() cycles
        // and see if the threshold triggers again.
        for i in 0..(MAX_WAL_FLUSHES_BEFORE_L0_FLUSH - 1) {
            let key = format!("key{:08}", i);
            kv_store
                .put_with_options(key.as_bytes(), b"v", &put_options, &write_options)
                .await
                .unwrap();
            kv_store.flush().await.unwrap();
        }

        // Verify no more memtables were frozen or L0 flush happened.
        {
            let guard = kv_store.inner.state.read();
            assert_eq!(guard.state().core().tree.l0.len(), 1);
        }

        // This put() triggers a freeze.
        let key = format!("key{:08}", MAX_WAL_FLUSHES_BEFORE_L0_FLUSH);
        kv_store
            .put_with_options(key.as_bytes(), b"v", &put_options, &write_options)
            .await
            .unwrap();
        kv_store.flush().await.unwrap();

        // Wait for the flush to happen.
        let db_state = wait_for_manifest_condition(
            &mut stored_manifest,
            |s| s.replay_after_wal_id == MAX_WAL_FLUSHES_BEFORE_L0_FLUSH * 2,
            Duration::from_secs(30),
        )
        .await;
        assert_eq!(db_state.tree.l0.len(), 2); // We should have two L0 flushes.

        kv_store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_flush_memtable_with_wal_enabled() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_flush_with_options";
        let mut options = test_db_options(0, 256, None);
        options.flush_interval = Some(Duration::from_secs(u64::MAX));
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let kv_store = Db::builder(path, object_store.clone())
            .with_settings(options)
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();

        let manifest_store = Arc::new(ManifestStore::new(&Path::from(path), object_store.clone()));
        let mut stored_manifest =
            StoredManifest::load(manifest_store.clone(), Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let sst_format = SsTableFormat {
            min_filter_keys: 10,
            ..SsTableFormat::default()
        };
        let table_store = Arc::new(TableStore::new(
            object_store.clone(),
            sst_format,
            path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));

        // Write some data to populate the memtable
        let key1 = b"test_key_1";
        let value1 = b"test_value_1";
        kv_store
            .put_with_options(
                key1,
                value1,
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let key2 = b"test_key_2";
        let value2 = b"test_value_2";
        kv_store
            .put_with_options(
                key2,
                value2,
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Get initial state
        let initial_manifest = stored_manifest.refresh().await.unwrap();
        let initial_l0_count = initial_manifest.core.tree.l0.len();

        let initial_flush_count =
            lookup_metric(&metrics_recorder, IMMUTABLE_MEMTABLE_FLUSHES).unwrap();

        // Flush memtable using flush_with_options
        kv_store
            .flush_with_options(FlushOptions {
                flush_type: FlushType::MemTable,
            })
            .await
            .unwrap();

        // Wait for the flush to complete and manifest to be updated
        let db_state = wait_for_manifest_condition(
            &mut stored_manifest,
            |s| s.tree.l0.len() > initial_l0_count,
            Duration::from_secs(30),
        )
        .await;

        // Verify that a new SST was created in L0
        assert_eq!(db_state.tree.l0.len(), initial_l0_count + 1);

        // Verify that the flush metrics were updated
        let final_flush_count =
            lookup_metric(&metrics_recorder, IMMUTABLE_MEMTABLE_FLUSHES).unwrap();
        assert!(final_flush_count > initial_flush_count);

        // Verify that the WAL was also flushed since we guarantee
        // memtable data is persisted in the WAL prior to L0 flush.
        let recent_flushed_wal_id = kv_store
            .inner
            .wal_observer
            .status()
            .unwrap()
            .last_flushed_wal_id;
        assert_eq!(recent_flushed_wal_id, 2);

        // Verify that the data is still accessible after flush
        let retrieved_value1 = kv_store.get(key1).await.unwrap().unwrap();
        assert_eq!(retrieved_value1.as_ref(), value1);

        let retrieved_value2 = kv_store.get(key2).await.unwrap().unwrap();
        assert_eq!(retrieved_value2.as_ref(), value2);

        // Verify the data exists in the newly created SST
        let latest_sst = db_state.tree.l0.back().unwrap();
        let sst_iter_options = SstIteratorOptions::default();
        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            latest_sst,
            table_store.clone(),
            sst_iter_options,
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // Collect all key-value pairs from the SST
        let mut found_keys = std::collections::HashSet::new();
        while let Some(kv) = iter.next().await.unwrap().map(KeyValue::from) {
            found_keys.insert(kv.key.to_vec());
        }

        // Verify our keys are in the SST
        assert!(found_keys.contains(key1.as_slice()));
        assert!(found_keys.contains(key2.as_slice()));
    }

    #[tokio::test]
    async fn test_memtable_flush_also_flushes_wal() {
        let main_object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let wal_object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_memtable_flush_also_flushes_wal";
        let mut settings = test_db_options(0, 1024, None);
        settings.flush_interval = None;

        let kv_store = Db::builder(path, Arc::clone(&main_object_store))
            .with_settings(settings)
            .with_wal_object_store(wal_object_store.clone())
            .build()
            .await
            .unwrap();

        let key = b"wal_flush_key";
        let value = b"wal_flush_value";
        kv_store
            .put_with_options(
                key,
                value,
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(
            kv_store
                .inner
                .wal_observer
                .status()
                .unwrap()
                .buffered_wal_entries_count,
            1
        );

        kv_store
            .flush_with_options(FlushOptions {
                flush_type: FlushType::MemTable,
            })
            .await
            .unwrap();

        assert_eq!(
            kv_store
                .inner
                .wal_observer
                .status()
                .unwrap()
                .buffered_wal_entries_count,
            0
        );

        let wal_reader = SlateDbWalReaderBuilder::new()
            .with_object_store(main_object_store)
            .with_wal_object_store(wal_object_store)
            .with_path(Path::from(path))
            .build()
            .unwrap();
        let tail = wal_reader.last_wal_file_id(0).await.unwrap();
        assert_eq!(tail, 2); // first file is the fencing operation
        let mut rows = Vec::new();
        let mut wal_iter = wal_reader
            .iterator((1..tail.checked_add(1).unwrap()).into())
            .await
            .expect("expected successful WAL iterator call");
        while let Some(batch) = wal_iter
            .next()
            .await
            .expect("expected successful WAL rows read")
        {
            rows.extend(batch.rows);
        }
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.key.as_ref(), key);
        assert_eq!(
            row.value.as_bytes().expect("expected bytes").as_ref(),
            value
        );
        assert_eq!(row.seq, 1);
    }

    async fn test_sequence_tracker_persisted_across_flush_and_reload_impl(wal_enabled: bool) {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_sequence_tracker_flush";
        let mut settings = test_db_options(0, 256, None);
        settings.flush_interval = None;
        #[cfg(feature = "wal_disable")]
        {
            settings.wal_enabled = wal_enabled;
        }
        #[cfg(not(feature = "wal_disable"))]
        let _ = wal_enabled;
        let system_clock = Arc::new(MockSystemClock::new());

        let kv_store = Db::builder(path, object_store.clone())
            .with_settings(settings.clone())
            .with_system_clock(system_clock.clone())
            .build()
            .await
            .unwrap();

        let manifest_store = Arc::new(ManifestStore::new(&Path::from(path), object_store.clone()));
        let mut stored_manifest =
            StoredManifest::load(manifest_store.clone(), Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let write_options = WriteOptions {
            ..Default::default()
        };
        let put_options = PutOptions::default();

        let timestamps_ms = [0_i64, 60_000, 120_000];
        for (idx, ts) in timestamps_ms.iter().enumerate() {
            system_clock.set(*ts);
            let key = format!("key-{idx}").into_bytes();
            let value = format!("value-{idx}").into_bytes();
            kv_store
                .put_with_options(&key, &value, &put_options, &write_options)
                .await
                .unwrap();
        }

        kv_store
            .flush_with_options(FlushOptions {
                flush_type: FlushType::MemTable,
            })
            .await
            .unwrap();

        let target_ts = Utc.timestamp_opt(120, 0).single().unwrap();
        let persisted_state = wait_for_manifest_condition(
            &mut stored_manifest,
            move |core| {
                core.sequence_tracker
                    .find_seq(target_ts, FindOption::RoundDown)
                    == Some(3)
            },
            Duration::from_secs(5),
        )
        .await;

        let tracker = persisted_state.sequence_tracker.clone();
        let live_tracker = kv_store
            .inner
            .state
            .read()
            .state()
            .core()
            .sequence_tracker
            .clone();
        assert_eq!(tracker, live_tracker);

        let seq1_ts = tracker.find_ts(1, FindOption::RoundDown).unwrap();
        assert_eq!(seq1_ts.timestamp(), 0);

        let seq2_ts = tracker.find_ts(2, FindOption::RoundDown).unwrap();
        assert_eq!(seq2_ts.timestamp(), 60);

        let seq3_ts = tracker.find_ts(3, FindOption::RoundDown).unwrap();
        assert_eq!(seq3_ts.timestamp(), 120);

        let ts_lookup = Utc.timestamp_opt(60, 0).single().unwrap();
        assert_eq!(tracker.find_seq(ts_lookup, FindOption::RoundDown), Some(2));

        kv_store.close().await.unwrap();

        let reopened = Db::builder(path, object_store.clone())
            .with_settings(settings)
            .with_system_clock(system_clock.clone())
            .build()
            .await
            .unwrap();

        let reopened_tracker = reopened
            .inner
            .state
            .read()
            .state()
            .core()
            .sequence_tracker
            .clone();
        assert_eq!(tracker, reopened_tracker);

        reopened.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_sequence_tracker_persisted_across_flush_and_reload_wal_enabled() {
        test_sequence_tracker_persisted_across_flush_and_reload_impl(true).await;
    }

    #[tokio::test]
    #[cfg(feature = "wal_disable")]
    async fn test_sequence_tracker_persisted_across_flush_and_reload_wal_disabled() {
        test_sequence_tracker_persisted_across_flush_and_reload_impl(false).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_sequence_tracker_not_ahead_of_last_l0_seq_when_flush_races_with_writes() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let mut settings = test_db_options(0, 2048, None);
        settings.flush_interval = None;
        // Don't trigger `flush_and_record` unless we explicitly ask for it.
        settings.manifest_poll_interval = Duration::from_secs(60 * 60);

        let system_clock = Arc::new(MockSystemClock::new());
        let db = Db::builder(
            "/tmp/test_sequence_tracker_flush_race",
            object_store.clone(),
        )
        .with_settings(settings)
        .with_system_clock(system_clock.clone())
        .with_fp_registry(fp_registry.clone())
        .build()
        .await
        .unwrap();

        let write_options = WriteOptions {
            ..Default::default()
        };

        async fn put_with_timestamp(
            db: &Db,
            system_clock: &Arc<MockSystemClock>,
            idx: usize,
            write_options: &WriteOptions,
        ) {
            system_clock.set((idx as i64) * 60_000);
            let key = format!("race-key-{idx}").into_bytes();
            let value = format!("race-value-{idx}").into_bytes();
            db.put_with_options(&key, &value, &PutOptions::default(), write_options)
                .await
                .unwrap();
        }

        // These are the entries that should make it into the first L0 flush.
        put_with_timestamp(&db, &system_clock, 0, &write_options).await;
        put_with_timestamp(&db, &system_clock, 1, &write_options).await;

        // Pause after the immutable memtable has been written to an L0 SST but before the manifest
        // is updated. That gives us a precise window where later writes can race with manifest state.
        fail_parallel::cfg(
            fp_registry.clone(),
            "after-flush-imm-to-l0-before-manifest",
            "pause",
        )
        .unwrap();

        // Kick off the flush in the background so the test can interleave more writes while the
        // flusher is paused at the failpoint above.
        let flush_handle = {
            let inner = Arc::clone(&db.inner);
            tokio::spawn(async move { inner.flush_memtables(FlushTarget::All).await })
        };

        let mut wrote_l0_sst = false;
        for _ in 0..6000 {
            // Waiting for the SST itself is more precise than watching imm_memtable state. We only
            // continue once the flusher has definitely crossed the "write SST, not manifest" boundary.
            let ssts = db.inner.table_store.list_compacted_ssts(..).await.unwrap();
            if !ssts.is_empty() {
                wrote_l0_sst = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            wrote_l0_sst,
            "L0 SST was not written before manifest update pause"
        );

        // These writes land in the new active memtable while the first flush is paused.
        // They should not leak into the persisted sequence tracker state for the earlier flush.
        put_with_timestamp(&db, &system_clock, 2, &write_options).await;
        put_with_timestamp(&db, &system_clock, 3, &write_options).await;

        // Let the original flush finish publishing its manifest update.
        fail_parallel::cfg(
            fp_registry.clone(),
            "after-flush-imm-to-l0-before-manifest",
            "off",
        )
        .unwrap();

        flush_handle.await.unwrap().unwrap();

        {
            let guard = db.inner.state.read();
            // The background flush should have drained the single immutable memtable we created.
            assert!(guard.state().imm_memtable.is_empty());
        }

        let manifest_state = {
            let guard = db.inner.state.read();
            guard.state().manifest.value.core.clone()
        };
        let last_l0_seq = manifest_state.last_l0_seq;
        assert!(
            last_l0_seq >= 2,
            "expected flushed memtable to advance last_l0_seq"
        );

        // The core invariant: once the first flush publishes last_l0_seq, the persisted tracker
        // must not contain timestamps for later sequence numbers from the second memtable.
        assert!(
            manifest_state
                .sequence_tracker
                .find_ts(last_l0_seq + 1, FindOption::RoundUp)
                .is_none(),
            "sequence tracker should not advance beyond last_l0_seq (last_l0_seq={})",
            last_l0_seq
        );

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_flush_with_options_wal() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_flush_with_options_wal";
        let mut options = test_db_options(0, 1024, None);
        // Larger memtable to avoid memtable flushes
        options.flush_interval = Some(Duration::from_secs(u64::MAX));
        // Fail all memtable writes before the DB starts, so we can be sure that
        // only the WAL is flushed.
        fail_parallel::cfg(
            fp_registry.clone(),
            "write-compacted-sst-io-error",
            "return",
        )
        .unwrap();
        let kv_store = Db::builder(path, object_store.clone())
            .with_settings(options)
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();

        // Write some data to populate the WAL buffer
        let key1 = b"wal_test_key_1";
        let value1 = b"wal_test_value_1";
        kv_store
            .put_with_options(
                key1,
                value1,
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let key2 = b"wal_test_key_2";
        let value2 = b"wal_test_value_2";
        kv_store
            .put_with_options(
                key2,
                value2,
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Get initial WAL ID to verify flush occurred
        let initial_wal_id = kv_store
            .inner
            .wal_observer
            .status()
            .unwrap()
            .last_flushed_wal_id;

        // Flush WAL using flush_with_options - this should succeed without error
        let flush_result = kv_store
            .flush_with_options(FlushOptions {
                flush_type: FlushType::Wal,
            })
            .await;

        // Verify the flush operation completed successfully
        assert!(flush_result.is_ok(), "WAL flush should succeed");

        // Verify that the data is still accessible after WAL flush
        let retrieved_value1 = kv_store.get(key1).await.unwrap().unwrap();
        assert_eq!(retrieved_value1.as_ref(), value1);

        let retrieved_value2 = kv_store.get(key2).await.unwrap().unwrap();
        assert_eq!(retrieved_value2.as_ref(), value2);

        // Verify that the WAL buffer is in a consistent state after flush
        // The recent_flushed_wal_id should be at least as high as before
        let final_wal_id = kv_store
            .inner
            .wal_observer
            .status()
            .unwrap()
            .last_flushed_wal_id;
        assert!(
            final_wal_id >= initial_wal_id,
            "WAL ID should not decrease after flush"
        );

        // Verify that the memtable has not been flushed by checking the db for error state
        assert!(
            kv_store.inner.status().close_reason.is_none(),
            "DB should not have an error state"
        );
    }

    #[tokio::test]
    #[cfg(feature = "wal_disable")]
    async fn test_flush_with_options_wal_disabled_error() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_flush_with_options_wal_disabled";
        let mut options = test_db_options(0, 1024, None);
        options.wal_enabled = false; // Disable WAL
        let kv_store = Db::builder(path, object_store.clone())
            .with_settings(options)
            .build()
            .await
            .unwrap();

        // Write some data to the database
        let key1 = b"test_key_1";
        let value1 = b"test_value_1";
        kv_store
            .put_with_options(
                key1,
                value1,
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Attempt to flush WAL on a WAL-disabled database
        let flush_result = kv_store
            .flush_with_options(FlushOptions {
                flush_type: FlushType::Wal,
            })
            .await;

        // Verify that we get the WalDisabled error
        assert!(flush_result.is_err(), "Expected WalDisabled error");
        let error = flush_result.unwrap_err();

        assert!(
            error
                .to_string()
                .contains("attempted a WAL operation when the WAL is disabled"),
            "Expected WalDisabled error message, got: {}",
            error
        );

        // Verify that memtable flush still works when WAL is disabled
        let memtable_flush_result = kv_store
            .flush_with_options(FlushOptions {
                flush_type: FlushType::MemTable,
            })
            .await;
        assert!(
            memtable_flush_result.is_ok(),
            "Memtable flush should work even when WAL is disabled"
        );

        // Verify that the data is still accessible
        let retrieved_value1 = kv_store.get(key1).await.unwrap().unwrap();
        assert_eq!(retrieved_value1.as_ref(), value1);
    }

    // 2 threads so we can can wait on the write_with_options (main) thread
    // while the write_batch (background) thread is blocked on writing the
    // WAL SST.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_apply_wal_memory_backpressure() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let mut options = test_db_options(0, 1, None);
        let first_entry = RowEntry::new_value(b"key1", b"val1", 1).with_create_ts(0);
        let sst_format = SsTableFormat {
            min_filter_keys: options.min_filter_keys,
            ..SsTableFormat::default()
        };
        let first_memtable_bytes =
            sst_format.estimate_encoded_size_compacted(1, first_entry.estimated_size());
        // Keep the memtable alone below the limit so this test only applies
        // backpressure when the WAL estimate is included.
        options.max_unflushed_bytes = first_memtable_bytes.saturating_add(1);
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder(path, object_store.clone())
            .with_settings(options)
            .with_fp_registry(fp_registry.clone())
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();
        let metrics_recorder_clone = metrics_recorder.clone();
        let write_opts = WriteOptions {
            ..Default::default()
        };

        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "pause").unwrap();

        // Helper function to wait for a condition to be true.
        let wait_for = async move |condition: Box<dyn Fn() -> bool>| {
            for _ in 0..3000 {
                if condition() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        };

        // 1 wal entry in memory
        db.put_with_options(b"key1", b"val1", &PutOptions::default(), &write_opts)
            .await
            .unwrap();

        // Wait for put to end up in the WAL buffer
        let this_wal_buffer = db.inner.wal_observer.clone();
        wait_for(Box::new(move || {
            this_wal_buffer.status().unwrap().buffered_wal_entries_count > 0
        }))
        .await;

        // Verify that there is now 1 WAL entry in memory.
        let wal_status = db.inner.wal_observer.status().unwrap();
        assert_eq!(wal_status.buffered_wal_entries_count, 1);

        let (active_memtable_size_bytes, imm_memtable_size_bytes) = {
            let guard = db.inner.state.read();
            let estimate = |metadata: KVTableMetadata| {
                db.inner.table_store.estimate_encoded_size_compacted(
                    metadata.entry_num,
                    metadata.entries_size_in_bytes,
                )
            };
            let active_memtable_size_bytes = estimate(guard.memtable().table().metadata());
            let imm_memtable_size_bytes = guard
                .state()
                .imm_memtable
                .iter()
                .map(|imm| estimate(imm.table().metadata()))
                .fold(0usize, |total, size| total.saturating_add(size));
            (active_memtable_size_bytes, imm_memtable_size_bytes)
        };
        let memtable_size_bytes =
            active_memtable_size_bytes.saturating_add(imm_memtable_size_bytes);
        let total_mem_size_bytes = memtable_size_bytes.saturating_add(wal_status.estimated_bytes);
        assert!(
            memtable_size_bytes < db.inner.settings.max_unflushed_bytes,
            "test requires memtable bytes ({memtable_size_bytes}) to remain below \
             max_unflushed_bytes ({})",
            db.inner.settings.max_unflushed_bytes
        );
        assert!(
            total_mem_size_bytes >= db.inner.settings.max_unflushed_bytes,
            "test requires memtable plus WAL bytes ({total_mem_size_bytes}) to reach \
             max_unflushed_bytes ({})",
            db.inner.settings.max_unflushed_bytes
        );

        // Put another WAL entry, which should trigger backpressure. Do this in a separate
        // task since the put() is blocked until the WAL is flushed, which isn't happening
        // due to the fail point.
        let join_handle = tokio::spawn(async move {
            db.put_with_options(b"key2", b"val2", &PutOptions::default(), &write_opts)
                .await
                .unwrap();
        });

        let this_recorder = metrics_recorder_clone.clone();
        // Wait up to 30s for backpressure to be applied to the second write.
        wait_for(Box::new(move || {
            lookup_metric(&this_recorder, crate::db_stats::BACKPRESSURE_COUNT)
                .is_some_and(|v| v > 0)
        }))
        .await;

        // Verify that backpressure is applied.
        assert!(
            lookup_metric(&metrics_recorder_clone, crate::db_stats::BACKPRESSURE_COUNT).unwrap()
                >= 1
        );

        // Unblock so put_with_options in join_handle can complete and join_handle.await returns
        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "off").unwrap();

        // Shutdown the background task
        join_handle.abort();
        let _ = join_handle.await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_backpressure_waiter_exits_when_db_is_fenced() {
        // Pause the L0 upload so a frozen memtable can't drain, keeping unflushed
        // bytes above the backpressure threshold indefinitely.
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let fp_registry = Arc::new(FailPointRegistry::new());
        fail_parallel::cfg(fp_registry.clone(), "write-compacted-sst-io-error", "pause").unwrap();

        let mut options = test_db_options(0, 4 * 1024, None);
        options.flush_interval = None;
        options.max_unflushed_bytes = 8 * 1024;

        // Use a metrics recorder so the test can observe when the spawned task
        // has actually entered maybe_apply_backpressure().
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder(
            "/tmp/test_backpressure_waiter_exits_when_db_is_fenced",
            object_store,
        )
        .with_settings(options)
        .with_fp_registry(fp_registry.clone())
        .with_metrics_recorder(metrics_recorder.clone())
        .build()
        .await
        .unwrap();
        let write_opts = WriteOptions {
            ..Default::default()
        };

        let large_value = vec![b'x'; 16 * 1024];
        db.put_with_options(b"key1", &large_value, &PutOptions::default(), &write_opts)
            .await
            .unwrap();
        assert!(!db.inner.state.read().state().imm_memtable.is_empty());

        // Start backpressure on a cloned inner handle. This parks the task on
        // the same wait path used by writers before they enqueue a batch.
        let inner = db.inner.clone();
        let mut backpressure_task =
            tokio::spawn(async move { inner.maybe_apply_backpressure().await });

        // Wait until the task has observed the unflushed memtable and incremented
        // the backpressure counter, proving it is inside the wait path.
        tokio::time::timeout(Duration::from_secs(60), async {
            loop {
                if lookup_metric(&metrics_recorder, crate::db_stats::BACKPRESSURE_COUNT)
                    .is_some_and(|v| v > 0)
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("timed out waiting for backpressure to be applied");

        // Simulate the DB being fenced while the writer is already parked in
        // backpressure.
        db.inner
            .status_manager
            .write_result(Err(SlateDBError::Fenced));

        // The lifecycle signal should wake the waiter promptly even though no
        // WAL flush or memtable upload will notify it.
        let result = tokio::time::timeout(Duration::from_secs(5), &mut backpressure_task).await;
        if result.is_err() {
            backpressure_task.abort();
            let _ = backpressure_task.await;
        }

        // Resume the L0 upload so the pending memtable can drain and close can
        // complete cleanly.
        fail_parallel::cfg(fp_registry.clone(), "write-compacted-sst-io-error", "off").unwrap();
        let _ = db.close().await;

        // Assert that the waiter exits with the terminal fenced error, not a
        // successful write path or some unrelated task failure.
        let backpressure_result = result
            .expect("backpressure waiter did not exit after DB was fenced")
            .expect("backpressure task panicked");
        assert!(
            matches!(backpressure_result, Err(SlateDBError::Fenced)),
            "expected fenced error, got {:?}",
            backpressure_result
        );
    }

    #[tokio::test]
    async fn test_apply_backpressure_to_memtable_flush() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let mut options = test_db_options(0, 1, None);
        options.l0_max_ssts = 4;
        let db = Db::builder("/tmp/test_kv_store", object_store.clone())
            .with_settings(options)
            .build()
            .await
            .unwrap();
        db.put(b"key1", b"val1").await.unwrap();
        db.put(b"key2", b"val2").await.unwrap();
        db.put(b"key3", b"val3").await.unwrap();
        db.put(b"key4", b"val4").await.unwrap();
        db.put(b"key5", b"val5").await.unwrap();

        db.flush().await.unwrap();

        let db_state = db.inner.state.read().view();
        assert_eq!(db_state.state.imm_memtable.len(), 1);
    }

    #[tokio::test]
    async fn test_put_empty_value() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let kv_store = Db::builder("/tmp/test_kv_store", object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();
        let key = b"test_key";
        let value = b"";
        kv_store.put(key, value).await.unwrap();
        kv_store.flush().await.unwrap();

        assert_eq!(
            kv_store.get(key).await.unwrap(),
            Some(Bytes::from_static(value))
        );
    }

    #[tokio::test]
    async fn test_flush_while_iterating() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let kv_store = Db::builder("/tmp/test_kv_store", object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_system_clock(Arc::new(MockSystemClock::new()))
            .build()
            .await
            .unwrap();

        let memtable = {
            let lock = kv_store.inner.state.read();
            lock.memtable()
                .put(RowEntry::new_value(b"abc1111", b"value1111", 1));
            lock.memtable()
                .put(RowEntry::new_value(b"abc2222", b"value2222", 2));
            lock.memtable()
                .put(RowEntry::new_value(b"abc3333", b"value3333", 3));
            lock.memtable().table().clone()
        };

        let mut iter = memtable.iter();
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        assert_eq!(kv.key, b"abc1111".as_slice());

        kv_store.flush().await.unwrap();

        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        assert_eq!(kv.key, b"abc2222".as_slice());

        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        assert_eq!(kv.key, b"abc3333".as_slice());
    }

    #[tokio::test]
    async fn test_basic_restore() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let mut next_wal_id = 1;
        let kv_store = Db::builder(path, object_store.clone())
            // with l0_sst_size_bytes = 600 all large puts should be flushed in one L0 SST
            .with_settings(test_db_options(0, 600, None))
            .with_system_clock(Arc::new(MockSystemClock::new()))
            .build()
            .await
            .unwrap();
        // increment wal id for the empty wal
        next_wal_id += 1;

        // do all flushes manually
        let write_opts = WriteOptions {
            ..Default::default()
        };

        // do a few writes that will result in l0 flushes
        let l0_count: u64 = 3;
        for i in 0..l0_count {
            kv_store
                .put_with_options(
                    &[b'a' + i as u8; 16],
                    &[b'b' + i as u8; 48],
                    &PutOptions::default(),
                    &write_opts,
                )
                .await
                .unwrap();
            kv_store.flush().await.unwrap();
            kv_store
                .put_with_options(
                    &[b'j' + i as u8; 16],
                    &[b'k' + i as u8; 48],
                    &PutOptions::default(),
                    &write_opts,
                )
                .await
                .unwrap();
            kv_store.flush().await.unwrap();
            next_wal_id += 2;
        }

        // write some smaller keys so that we populate wal without flushing to l0
        let sst_count: u64 = 5;
        for i in 0..sst_count {
            kv_store
                .put_with_options(
                    &i.to_be_bytes(),
                    &i.to_be_bytes(),
                    &PutOptions::default(),
                    &write_opts,
                )
                .await
                .unwrap();
            kv_store.flush().await.unwrap();
            next_wal_id += 1;
        }

        kv_store.close().await.unwrap();

        // recover and validate that sst files are loaded on recovery.
        let kv_store_restored = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 128, None))
            .with_system_clock(Arc::new(MockSystemClock::new()))
            .build()
            .await
            .unwrap();
        // increment wal id for the empty wal
        next_wal_id += 1;

        for i in 0..l0_count {
            let val = kv_store_restored.get([b'a' + i as u8; 16]).await.unwrap();
            assert_eq!(val, Some(Bytes::copy_from_slice(&[b'b' + i as u8; 48])));
            let val = kv_store_restored.get([b'j' + i as u8; 16]).await.unwrap();
            assert_eq!(val, Some(Bytes::copy_from_slice(&[b'k' + i as u8; 48])));
        }
        for i in 0..sst_count {
            let val = kv_store_restored.get(i.to_be_bytes()).await.unwrap();
            assert_eq!(val, Some(Bytes::copy_from_slice(&i.to_be_bytes())));
        }
        kv_store_restored.close().await.unwrap();

        // validate that the manifest file exists.
        let manifest_store = Arc::new(ManifestStore::new(&Path::from(path), object_store.clone()));
        let stored_manifest =
            StoredManifest::load(manifest_store, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let db_state = stored_manifest.db_state();
        assert_eq!(db_state.next_wal_sst_id, next_wal_id);
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_restore_seq_number() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        // Block L0 uploads so the data remains only in the WAL. The
        // uploader gives up on shutdown when the WAL is enabled, so
        // close() will complete without flushing memtables to L0.
        fail_parallel::cfg(
            fp_registry.clone(),
            "write-compacted-sst-io-error",
            "return",
        )
        .unwrap();
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 512, None))
            .with_system_clock(Arc::new(MockSystemClock::new()))
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();

        db.put_with_options(
            b"key1",
            b"val1",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            b"key2",
            b"val2",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            b"key3",
            b"val3",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.flush().await.unwrap();
        // expect to fail as l0 upload is blocked
        assert!(db.close().await.is_err());

        // Disable the failpoint so the restored DB can flush normally.
        fail_parallel::cfg(fp_registry.clone(), "write-compacted-sst-io-error", "off").unwrap();

        let db_restored = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 512, None))
            .with_system_clock(Arc::new(MockSystemClock::new()))
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();

        let state = db_restored.inner.state.read();
        let memtable = state.memtable();
        let mut iter = memtable.table().iter();
        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_value(b"key1", b"val1", 1).with_create_ts(0),
                RowEntry::new_value(b"key2", b"val2", 2).with_create_ts(0),
                RowEntry::new_value(b"key3", b"val3", 3).with_create_ts(0),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_read_merges_from_snapshot_across_compaction() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/testdb";
        let should_compact_l0 = Arc::new(AtomicBool::new(false));
        let this_should_compact_l0 = should_compact_l0.clone();
        let compaction_scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            move |_state| this_should_compact_l0.swap(false, Ordering::SeqCst),
        )));
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 1024 * 1024, None))
            .with_merge_operator(Arc::new(StringConcatMergeOperator {}))
            .with_compactor_builder(
                CompactorBuilder::new(path, object_store.clone())
                    .with_scheduler_supplier(compaction_scheduler.clone())
                    .with_options(fast_compactor_options()),
            )
            .build()
            .await
            .unwrap();
        let db = Arc::new(db);

        db.merge(b"foo", b"0").await.unwrap();
        let snapshot = db.snapshot().await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();
        db.merge(b"foo", b"1").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // await a compaction
        should_compact_l0.store(true, Ordering::SeqCst);
        let db_poll = db.clone();
        tokio::time::timeout(Duration::from_secs(10), async move {
            loop {
                {
                    let db_state = db_poll.inner.state.read();
                    if !db_state.state().core().tree.compacted.is_empty() {
                        return;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();

        let result = snapshot.get(b"foo").await.unwrap();
        assert_eq!(result, Some(Bytes::copy_from_slice(b"0")));
        let result = db.get(b"foo").await.unwrap();
        assert_eq!(result, Some(Bytes::copy_from_slice(b"01")));
    }

    #[tokio::test]
    async fn test_all_kv_seq_num_are_greater_than_0() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store_seq_num";
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 1024 * 1024, None))
            .build()
            .await
            .unwrap();

        // Write some data to memtable
        db.put(b"key1", b"value1").await.unwrap();

        let val = db.get(b"key1").await.unwrap();
        assert_eq!(val, Some(Bytes::from_static(b"value1")));

        let state = db.inner.state.read();
        let memtable = state.memtable();
        assert_eq!(memtable.table().last_seq(), Some(1));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_should_read_uncommitted_data_if_read_level_uncommitted() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let kv_store = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();

        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "pause").unwrap();
        kv_store
            .put_with_options(
                "foo".as_bytes(),
                "bar".as_bytes(),
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Validate uncommitted read
        let val = kv_store
            .get_with_options(
                "foo".as_bytes(),
                &ReadOptions::new().with_durability_filter(Memory),
            )
            .await
            .unwrap();
        assert_eq!(val, Some("bar".into()));

        // Validate committed read should still return None
        let val = kv_store
            .get_with_options(
                "foo".as_bytes(),
                &ReadOptions::new().with_durability_filter(Remote),
            )
            .await
            .unwrap();
        assert_eq!(val, None);

        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "off").unwrap();
        kv_store.close().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_should_read_only_committed_data() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let kv_store = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();

        kv_store
            .put("foo".as_bytes(), "bar".as_bytes())
            .await
            .unwrap()
            .await_durable()
            .await
            .unwrap();
        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "pause").unwrap();
        kv_store
            .put_with_options(
                "foo".as_bytes(),
                "bla".as_bytes(),
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let val = kv_store
            .get_with_options(
                "foo".as_bytes(),
                &ReadOptions::new().with_durability_filter(Remote),
            )
            .await
            .unwrap();
        assert_eq!(val, Some("bar".into()));
        let val = kv_store
            .get_with_options(
                "foo".as_bytes(),
                &ReadOptions::new().with_durability_filter(Memory),
            )
            .await
            .unwrap();
        assert_eq!(val, Some("bla".into()));

        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "off").unwrap();
        kv_store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_should_delete_without_awaiting_flush() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let kv_store = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();

        kv_store
            .put("foo".as_bytes(), "bar".as_bytes())
            .await
            .unwrap()
            .await_durable()
            .await
            .unwrap();
        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "pause").unwrap();
        kv_store
            .delete_with_options(
                "foo".as_bytes(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let val = kv_store
            .get_with_options(
                "foo".as_bytes(),
                &ReadOptions::new().with_durability_filter(Remote),
            )
            .await
            .unwrap();
        assert_eq!(val, Some("bar".into()));
        let val = kv_store
            .get_with_options(
                "foo".as_bytes(),
                &ReadOptions::new().with_durability_filter(Memory),
            )
            .await
            .unwrap();
        assert_eq!(val, None);

        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "off").unwrap();
        kv_store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_should_read_only_committed_data() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let kv_store = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();

        // Write and commit some initial data
        kv_store
            .put("key1".as_bytes(), "committed1".as_bytes())
            .await
            .unwrap();
        kv_store
            .put("key2".as_bytes(), "committed2".as_bytes())
            .await
            .unwrap();
        kv_store
            .put("key3".as_bytes(), "committed3".as_bytes())
            .await
            .unwrap()
            .await_durable()
            .await
            .unwrap();

        // Pause WAL writes to prevent new writes from being committed
        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "pause").unwrap();

        // Write uncommitted data
        kv_store
            .put_with_options(
                "key2".as_bytes(),
                "uncommitted2".as_bytes(),
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        kv_store
            .put_with_options(
                "key4".as_bytes(),
                "uncommitted4".as_bytes(),
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Scan with Remote filter should only see committed data
        let mut iter = kv_store
            .scan_with_options(
                "key1".as_bytes().."key5".as_bytes(),
                &ScanOptions::new().with_durability_filter(Remote),
            )
            .await
            .unwrap();

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key1");
        assert_eq!(kv.value.as_ref(), b"committed1");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key2");
        assert_eq!(kv.value.as_ref(), b"committed2"); // Old committed value

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key3");
        assert_eq!(kv.value.as_ref(), b"committed3");

        // key4 should not be visible with Remote filter
        assert_eq!(iter.next().await.unwrap(), None);

        // Scan with Memory filter should see uncommitted data
        let mut iter = kv_store
            .scan_with_options(
                "key1".as_bytes().."key5".as_bytes(),
                &ScanOptions::new().with_durability_filter(Memory),
            )
            .await
            .unwrap();

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key1");
        assert_eq!(kv.value.as_ref(), b"committed1");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key2");
        assert_eq!(kv.value.as_ref(), b"uncommitted2"); // New uncommitted value

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key3");
        assert_eq!(kv.value.as_ref(), b"committed3");

        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key.as_ref(), b"key4");
        assert_eq!(kv.value.as_ref(), b"uncommitted4"); // Uncommitted key visible

        assert_eq!(iter.next().await.unwrap(), None);

        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "off").unwrap();
        kv_store.close().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_should_recover_imm_from_wal() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        fail_parallel::cfg(fp_registry.clone(), "write-compacted-sst-io-error", "pause").unwrap();

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let mut next_wal_id = 1;
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 128, None))
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();
        next_wal_id += 1;

        // subscribe to status manager to get notified when db is closed
        let mut rx = db.inner.status_manager.subscribe();

        // write a few keys that will result in memtable flushes
        let key1 = [b'a'; 32];
        let value1 = [b'b'; 96];
        db.put(key1, value1)
            .await
            .unwrap()
            .await_durable()
            .await
            .unwrap();
        next_wal_id += 1;
        let key2 = [b'c'; 32];
        let value2 = [b'd'; 96];
        db.put(key2, value2)
            .await
            .unwrap()
            .await_durable()
            .await
            .unwrap();
        next_wal_id += 1;

        let reader = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 128, None))
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();

        // increment wal id for the empty wal
        next_wal_id += 1;

        // verify that we reload imm
        let db_state = reader.inner.state.read().view();
        assert_eq!(db_state.state.imm_memtable.len(), 2);

        // one empty wal and two wals for the puts
        assert_eq!(
            db_state
                .state
                .imm_memtable
                .front()
                .unwrap()
                .recent_flushed_wal_id(),
            1 + 2
        );
        assert_eq!(
            db_state
                .state
                .imm_memtable
                .get(1)
                .unwrap()
                .recent_flushed_wal_id(),
            2
        );
        assert_eq!(db_state.state.core().next_wal_sst_id, next_wal_id);
        assert_eq!(
            reader.get(key1).await.unwrap(),
            Some(Bytes::copy_from_slice(&value1))
        );
        assert_eq!(
            reader.get(key2).await.unwrap(),
            Some(Bytes::copy_from_slice(&value2))
        );

        fail_parallel::cfg(fp_registry.clone(), "write-compacted-sst-io-error", "off").unwrap();

        // wait for the background task to report the Fenced error
        rx.wait_for(|status| status.close_reason.is_some())
            .await
            .unwrap();
        assert_eq!(
            db.inner.status_manager.status().close_reason,
            Some(crate::error::CloseReason::Fenced)
        );

        db.close().await.unwrap();
        reader.close().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_should_recover_imm_from_wal_after_flush_error() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        fail_parallel::cfg(
            fp_registry.clone(),
            "write-compacted-sst-io-error",
            "return",
        )
        .unwrap();
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 4096, None))
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();

        // write data to the WAL, but not enough to trigger a memtable flush
        let key1 = [b'a'; 32];
        let value1 = [b'b'; 96];
        let result = db.put(&key1, &value1).await;
        assert!(result.is_ok(), "Failed to write key1");
        result.unwrap().await_durable().await.unwrap();
        assert_eq!(
            db.inner.wal_observer.status().unwrap().last_flushed_wal_id,
            2
        );

        // Let background flush attempts fail while WAL durability preserves recovery.
        // expect to fail as l0 upload is blocked
        assert!(db.close().await.is_err());

        // pause write-compacted-sst-io-error to prevent immutable tables
        // from being flushed, so we can snapshot the state when there is
        // an immutable table to verify its contents.
        fail_parallel::cfg(fp_registry.clone(), "write-compacted-sst-io-error", "pause").unwrap();

        // reload the db
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 128, None))
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();

        let db_state = db.inner.state.read().view();

        // resume write-compacted-sst-io-error since we got a snapshot and
        // want to let the test finish.
        fail_parallel::cfg(fp_registry.clone(), "write-compacted-sst-io-error", "off").unwrap();

        // verify that we reload imm
        assert_eq!(db_state.state.imm_memtable.len(), 1);

        // verify that we have no L0 SSTs because memtables should have failed to flush
        assert_eq!(db_state.state.core().tree.l0.len(), 0);
        assert_eq!(db_state.state.core().tree.compacted.len(), 0);

        // one empty wal and one wal for the first put
        assert_eq!(
            db_state
                .state
                .imm_memtable
                .front()
                .unwrap()
                .recent_flushed_wal_id(),
            1 + 1
        );
        assert!(db_state.state.imm_memtable.get(1).is_none());

        assert_eq!(db_state.state.core().next_wal_sst_id, 4);
        assert_eq!(
            db.get(key1).await.unwrap(),
            Some(Bytes::copy_from_slice(&value1))
        );
    }

    #[tokio::test]
    async fn test_should_fail_write_if_wal_flush_task_panics() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let db = Arc::new(
            Db::builder(path, object_store.clone())
                .with_settings(test_db_options(0, 128, None))
                .with_fp_registry(fp_registry.clone())
                .build()
                .await
                .unwrap(),
        );

        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "panic").unwrap();
        let result = db
            .put(b"foo", b"bar")
            .await
            .unwrap()
            .await_durable()
            .await
            .unwrap_err();
        assert!(result.to_string().contains("background task panicked"));
    }

    #[tokio::test]
    async fn test_wal_id_last_seen_should_only_reflect_flushed_wals() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let db = Arc::new(
            Db::builder(path, object_store.clone())
                .with_settings(test_db_options(0, 128, None))
                .with_fp_registry(fp_registry.clone())
                .build()
                .await
                .unwrap(),
        );
        // Trigger a WAL write and block until durable so WAL is written
        db.put(b"foo", b"bar")
            .await
            .unwrap()
            .await_durable()
            .await
            .unwrap();

        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "panic").unwrap();

        // Trigger a WAL write, which should not advance the manifest WAL ID
        let result = db
            .put(b"foo", b"bar")
            .await
            .unwrap()
            .await_durable()
            .await
            .unwrap_err();
        assert_eq!(result.kind(), crate::ErrorKind::Closed(CloseReason::Panic));
        assert!(result
            .to_string()
            .contains("background task panicked. name=`wal_writer`"));

        // Close, which flushes the latest manifest to the object store
        // TODO: it might make sense to return an error if there're unflushed wals in memory
        // on close().
        db.close().await.unwrap();

        let manifest_store = ManifestStore::new(&Path::from(path), object_store.clone());
        let wal_store = WalTableStore::new(
            object_store.clone(),
            SsTableFormat::default(),
            path,
            TableStoreKind::Main,
        );

        // Get the next WAL SST ID based on what's currently in the object store
        let next_wal_sst_id = wal_store.next_wal_sst_id(0.into()).await.unwrap();

        // Get the latest manifest
        let manifest = manifest_store.read_latest_manifest().await.unwrap();

        // Assert that the manifest reflects only the flushed WAL
        assert_eq!(
            manifest.manifest.core.next_wal_sst_id,
            next_wal_sst_id.value()
        );
    }

    #[tokio::test]
    async fn test_close_should_return_error_if_wal_flush_fails() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";

        let mut settings = test_db_options(0, 256, None);
        // Disable automatic flush
        settings.flush_interval = None;

        let db = Db::builder(path, object_store.clone())
            .with_settings(settings)
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();

        // Turn on the io error failpoint for WAL
        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "return").unwrap();

        // Write data without awaiting durable so it goes into the WAL buffer
        db.put_with_options(
            b"foo",
            b"bar",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        // Close triggers the WAL flush, which should fail due to the io error
        db.close()
            .await
            .expect_err("close should error out due to WAL IO error");
    }

    #[tokio::test]
    async fn test_constructed_write_handle_uses_durability_waiter() {
        let waiter_called = Arc::new(AtomicBool::new(false));
        let waiter_called_clone = waiter_called.clone();
        let handle = WriteHandle::new(1, 0, move || {
            let waiter_called = waiter_called_clone.clone();
            async move {
                waiter_called.store(true, Ordering::SeqCst);
                Ok(())
            }
        });

        handle.await_durable().await.unwrap();

        assert!(waiter_called.load(Ordering::SeqCst));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_write_handle_await_durable_waits_for_flush() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let mut settings = test_db_options(0, 1024, None);
        settings.flush_interval = None;
        let db = Db::builder(
            "/tmp/test_write_handle_await_durable_waits_for_flush",
            object_store,
        )
        .with_settings(settings)
        .build()
        .await
        .unwrap();

        let handle = db.put(b"foo", b"bar").await.unwrap();
        let durability_wait = tokio::spawn(async move { handle.await_durable().await });
        tokio::task::yield_now().await;
        assert!(!durability_wait.is_finished());

        db.flush().await.unwrap();
        durability_wait.await.unwrap().unwrap();
        db.close().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_write_handle_await_durable_returns_error_if_db_closes_first() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let mut settings = test_db_options(0, 1024, None);
        settings.flush_interval = None;
        let db = Arc::new(
            Db::builder(
                "/tmp/test_write_handle_await_durable_returns_error_if_db_closes_first",
                object_store,
            )
            .with_settings(settings)
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap(),
        );
        // pause writes so that we can force the write to fail on the close status before the
        // final flush causes the write to become durable
        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "pause").unwrap();
        let write_db = db.clone();
        let write_task = tokio::spawn(async move {
            let handle = write_db
                .put_with_options(
                    b"foo",
                    b"bar",
                    &PutOptions::default(),
                    &WriteOptions::default(),
                )
                .await?;
            handle.await_durable().await
        });
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if db
                    .inner
                    .wal_observer
                    .status()
                    .unwrap()
                    .buffered_wal_entries_count
                    == 1
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("write was not buffered");
        let close_db = db.clone();
        let close_task = tokio::spawn(async move { close_db.close().await });

        let write_error = tokio::time::timeout(Duration::from_secs(10), write_task)
            .await
            .expect("timed out waiting for write")
            .expect("write task panicked")
            .expect_err("write unexpectedly reported success");

        assert_eq!(
            write_error.kind(),
            crate::ErrorKind::Closed(CloseReason::Clean)
        );
        fail_parallel::cfg(fp_registry, "write-wal-sst-io-error", "off").unwrap();
        let _ = close_task.await.unwrap();
    }

    async fn do_test_should_read_compacted_db(mut options: Settings) {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let should_compact_l0 = Arc::new(AtomicBool::new(false));
        let this_should_compact_l0 = should_compact_l0.clone();
        let compaction_scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            move |_state| this_should_compact_l0.swap(false, Ordering::SeqCst),
        )));

        let compactor_options = options.compactor_options.take();
        let db = Db::builder(path, object_store.clone())
            .with_settings(options)
            .with_compactor_builder(
                CompactorBuilder::new(path, object_store.clone())
                    .with_scheduler_supplier(compaction_scheduler.clone())
                    .with_options(compactor_options.unwrap()),
            )
            .build()
            .await
            .unwrap();
        let ms = ManifestStore::new(&Path::from(path), object_store.clone());
        let mut sm = StoredManifest::load(Arc::new(ms), Arc::new(DefaultSystemClock::new()))
            .await
            .unwrap();

        // write enough to fill up a few l0 SSTs
        for i in 0..4 {
            db.put(&[b'a' + i; 32], &[1u8 + i; 32]).await.unwrap();
            db.put(&[b'm' + i; 32], &[13u8 + i; 32]).await.unwrap();
        }
        // wait for compactor to compact them
        wait_for_manifest_condition(
            &mut sm,
            |s| {
                // compact after writing values. include in loop since the on demand scheduler
                // only runs once per `should_compact`, and memtables might still be getting
                // flushed (the put() calls above return before the writes become durable).
                should_compact_l0.store(true, Ordering::SeqCst);
                s.tree.last_compacted_l0_sst_view_id.is_some() && s.tree.l0.is_empty()
            },
            Duration::from_secs(10),
        )
        .await;
        let manifest = db.manifest();
        info!(
            "1 l0: {} {}",
            manifest.manifest.core.tree.l0.len(),
            manifest.manifest.core.tree.compacted.len()
        );

        // write more l0s and wait for compaction
        for i in 0..4 {
            db.put(&[b'f' + i; 32], &[6u8 + i; 32]).await.unwrap();
            db.put(&[b's' + i; 32], &[19u8 + i; 32]).await.unwrap();
        }
        // wait for compactor to compact them
        wait_for_manifest_condition(
            &mut sm,
            |s| {
                // compact after writing values. include in loop since the on demand scheduler
                // only runs once per `should_compact`, and memtables might still be getting
                // flushed (the put() calls above return before the writes become durable).
                should_compact_l0.store(true, Ordering::SeqCst);
                s.tree.last_compacted_l0_sst_view_id.is_some() && s.tree.l0.is_empty()
            },
            Duration::from_secs(10),
        )
        .await;
        let manifest = db.manifest();
        info!(
            "2 l0: {} {}",
            manifest.manifest.core.tree.l0.len(),
            manifest.manifest.core.tree.compacted.len()
        );
        // write another l0
        db.put(&[b'a'; 32], &[128u8; 32]).await.unwrap();
        db.put(&[b'm'; 32], &[129u8; 32]).await.unwrap();

        let val = db.get([b'a'; 32]).await.unwrap();
        assert_eq!(val, Some(Bytes::copy_from_slice(&[128u8; 32])));
        let val = db.get([b'm'; 32]).await.unwrap();
        assert_eq!(val, Some(Bytes::copy_from_slice(&[129u8; 32])));
        for i in 1..4 {
            let manifest = db.manifest();
            info!(
                "3 l0: {} {}",
                manifest.manifest.core.tree.l0.len(),
                manifest.manifest.core.tree.compacted.len()
            );
            let val = db.get([b'a' + i; 32]).await.unwrap();
            assert_eq!(val, Some(Bytes::copy_from_slice(&[1u8 + i; 32])));
            let val = db.get([b'm' + i; 32]).await.unwrap();
            assert_eq!(val, Some(Bytes::copy_from_slice(&[13u8 + i; 32])));
        }
        for i in 0..4 {
            let val = db.get([b'f' + i; 32]).await.unwrap();
            assert_eq!(val, Some(Bytes::copy_from_slice(&[6u8 + i; 32])));
            let val = db.get([b's' + i; 32]).await.unwrap();
            assert_eq!(val, Some(Bytes::copy_from_slice(&[19u8 + i; 32])));
        }
        let neg_lookup = db.get(b"abc").await;
        assert!(neg_lookup.unwrap().is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_should_read_from_compacted_db() {
        do_test_should_read_compacted_db(test_db_options(
            0,
            127,
            Some(CompactorOptions {
                poll_interval: Duration::from_millis(100),
                max_concurrent_compactions: 1,
                manifest_update_timeout: Duration::from_secs(300),
                worker: Some(CompactionWorkerOptions {
                    max_sst_size: 256,
                    compactions_poll_interval: Duration::from_millis(100),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        ))
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_should_read_from_compacted_db_no_filters() {
        do_test_should_read_compacted_db(test_db_options(
            u32::MAX,
            127,
            Some(CompactorOptions {
                poll_interval: Duration::from_millis(100),
                manifest_update_timeout: Duration::from_secs(300),
                max_concurrent_compactions: 1,
                worker: Some(CompactionWorkerOptions {
                    max_sst_size: 256,
                    compactions_poll_interval: Duration::from_millis(100),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        ))
        .await
    }

    #[tokio::test]
    async fn test_db_open_should_write_empty_wal() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        // assert that open db writes an empty wal.
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 128, None))
            .build()
            .await
            .unwrap();
        assert_eq!(db.inner.state.read().state().core().next_wal_sst_id, 2);
        let wal_store = WalTableStore::new(
            object_store.clone(),
            SsTableFormat::default(),
            path,
            TableStoreKind::Main,
        );
        let wal_ssts = wal_store.list_wal_ssts(..).await.unwrap();
        assert_eq!(wal_ssts.len(), 1);
        assert_eq!(wal_ssts[0].metadata.size, 0);
        db.put(b"1", b"1").await.unwrap();
        // assert that second open writes another empty wal.
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 128, None))
            .build()
            .await
            .unwrap();
        assert_eq!(db.inner.state.read().state().core().next_wal_sst_id, 4);
    }

    #[tokio::test]
    async fn test_empty_wal_should_fence_old_writer() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";

        async fn do_put(db: &Db, key: &[u8], val: &[u8]) -> Result<WriteHandle, crate::Error> {
            let handle = db
                .put_with_options(key, val, &PutOptions::default(), &WriteOptions::default())
                .await?;
            handle.await_durable().await?;
            Ok(handle)
        }

        // open db1 and assert that it can write.
        let db1 = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 128, None))
            .build()
            .await
            .unwrap();
        do_put(&db1, b"1", b"1").await.unwrap();

        // open db2, causing it to write an empty wal and fence db1.
        let db2 = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 128, None))
            .build()
            .await
            .unwrap();

        // assert that db1 can no longer write.
        let err = do_put(&db1, b"1", b"1").await.unwrap_err();
        assert_eq!(err.to_string(), "Closed error: detected newer DB client");

        do_put(&db2, b"2", b"2").await.unwrap();
        assert_eq!(db2.inner.state.read().state().core().next_wal_sst_id, 5);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_writer_paused_in_replay_wal_should_be_fenced_by_concurrent_open() {
        // Race we're trying to reproduce:
        // - W1 starts opening: claims writer_epoch=1, writes its fence WAL, then
        //   enters replay_wal. We pause it inside replay_wal.
        // - W2 starts opening: claims writer_epoch=2, writes its own fence WAL
        //   (above W1's), replays, and finishes init.
        // - W1 unpauses, finishes replay_wal, and returns its Db handle.
        // - W1 issues a put. This put should fail because W1's next WAL id is taken.
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_writer_paused_in_replay_wal_race";
        let fp_registry = Arc::new(FailPointRegistry::new());

        // Pause replay_wal for any writer that holds writer_epoch == 1
        // (the condition is enforced inside the fail point body).
        fail_parallel::cfg(fp_registry.clone(), "replay-wal-pause", "pause").unwrap();

        // Kick off W1 in the background. It will park inside replay_wal.
        // Use a long manifest poll interval on W1 so its background poller
        // doesn't independently observe W2's epoch bump and trip the
        // closed/fenced check while replay is paused.
        let w1_settings = {
            let mut s = test_db_options(0, 128, None);
            s.manifest_poll_interval = Duration::from_secs(600);
            s
        };
        let w1_handle = {
            let object_store = object_store.clone();
            let fp_registry = fp_registry.clone();
            tokio::spawn(async move {
                Db::builder(path, object_store)
                    .with_settings(w1_settings)
                    .with_fp_registry(fp_registry)
                    .build()
                    .await
            })
        };

        // Wait for W1 to write its fence WAL — at that point W1 has finished
        // fence_writers and has either entered or is about to enter the paused
        // replay_wal call.
        let probe_wal_store = WalTableStore::new(
            object_store.clone(),
            SsTableFormat::default(),
            path,
            TableStoreKind::Main,
        );
        let mut w1_paused = false;
        for _ in 0..600 {
            let wals = probe_wal_store.list_wal_ssts(..).await.unwrap();
            if !wals.is_empty() {
                w1_paused = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(w1_paused, "W1 did not reach replay_wal pause in time");
        // Small additional wait for W1 to transition from fence_writers into
        // the paused replay_wal block.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // While W1 is paused, open W2. W2's epoch is 2, so replay_wal is not
        // paused for W2. W2 writes its own fence WAL above W1's and finishes
        // init.
        let db2 = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 128, None))
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();
        assert_eq!(
            db2.inner.state.read().state().manifest.value.writer_epoch,
            2
        );

        // Release W1. It finishes replay_wal and returns a Db handle.
        fail_parallel::cfg(fp_registry.clone(), "replay-wal-pause", "off").unwrap();
        let db1 = w1_handle.await.unwrap().unwrap();
        assert_eq!(
            db1.inner.state.read().state().manifest.value.writer_epoch,
            1
        );

        // W1's put should fail because its now fenced
        let result = db1
            .put_with_options(
                b"w1",
                b"value",
                &PutOptions::default(),
                &WriteOptions::default(),
            )
            .await;
        if let Ok(handle) = result {
            assert!(handle.await_durable().await.is_err());
        }
    }

    async fn wait_for_wal_sst_count(wal_store: &WalTableStore, min_count: usize, context: &str) {
        for _ in 0..6000 {
            let wals = wal_store.list_wal_ssts(..).await.unwrap();
            if wals.len() >= min_count {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("{context}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn wal_replay_not_found_should_be_fenced_when_writer_epoch_advanced() {
        let base_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let gated_store = Arc::new(GatedObjectStore::new(base_store.clone()));
        let gated_object_store: Arc<dyn ObjectStore> = gated_store.clone();
        let path = "/tmp/wal_replay_not_found_should_be_fenced_when_writer_epoch_advanced";
        let fp_registry = Arc::new(FailPointRegistry::new());

        fail_parallel::cfg(fp_registry.clone(), "replay-wal-pause", "pause").unwrap();

        let w1_settings = {
            let mut s = test_db_options(0, 128, None);
            s.manifest_poll_interval = Duration::from_secs(600);
            s
        };
        let w1_handle = {
            let object_store = gated_object_store.clone();
            let fp_registry = fp_registry.clone();
            tokio::spawn(async move {
                Db::builder(path, object_store)
                    .with_settings(w1_settings)
                    .with_fp_registry(fp_registry)
                    .build()
                    .await
            })
        };

        let probe_wal_store = WalTableStore::new(
            base_store.clone(),
            SsTableFormat::default(),
            path,
            TableStoreKind::Main,
        );
        wait_for_wal_sst_count(
            &probe_wal_store,
            1,
            "W1 did not write its fence WAL in time",
        )
        .await;

        let head_arrivals_before = gated_store.head_gate.arrivals();
        gated_store.head_gate.close();
        fail_parallel::cfg(fp_registry.clone(), "replay-wal-pause", "off").unwrap();
        gated_store
            .head_gate
            .wait_for_arrivals(head_arrivals_before + 1)
            .await;

        let db2 = Db::builder(path, base_store.clone())
            .with_settings(test_db_options(0, 128, None))
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();
        assert_eq!(
            db2.inner.state.read().state().manifest.value.writer_epoch,
            2
        );

        probe_wal_store.delete_sst(1.into()).await.unwrap();
        gated_store.head_gate.release();

        let err = match w1_handle.await.unwrap() {
            Ok(_) => panic!("expected W1 open to fail"),
            Err(err) => err,
        };
        assert!(
            matches!(err.kind(), crate::ErrorKind::Closed(CloseReason::Fenced)),
            "expected fenced error, got {err:?}"
        );

        db2.close().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn wal_replay_not_found_should_remain_not_found_when_writer_epoch_unchanged() {
        let base_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let gated_store = Arc::new(GatedObjectStore::new(base_store.clone()));
        let gated_object_store: Arc<dyn ObjectStore> = gated_store.clone();
        let path = "/tmp/wal_replay_not_found_should_remain_not_found_when_writer_epoch_unchanged";
        let fp_registry = Arc::new(FailPointRegistry::new());

        fail_parallel::cfg(fp_registry.clone(), "replay-wal-pause", "pause").unwrap();

        let w1_settings = {
            let mut s = test_db_options(0, 128, None);
            s.manifest_poll_interval = Duration::from_secs(600);
            s
        };
        let w1_handle = {
            let object_store = gated_object_store.clone();
            let fp_registry = fp_registry.clone();
            tokio::spawn(async move {
                Db::builder(path, object_store)
                    .with_settings(w1_settings)
                    .with_fp_registry(fp_registry)
                    .build()
                    .await
            })
        };

        let probe_wal_store = WalTableStore::new(
            base_store.clone(),
            SsTableFormat::default(),
            path,
            TableStoreKind::Main,
        );
        wait_for_wal_sst_count(
            &probe_wal_store,
            1,
            "W1 did not write its fence WAL in time",
        )
        .await;

        let head_arrivals_before = gated_store.head_gate.arrivals();
        gated_store.head_gate.close();
        fail_parallel::cfg(fp_registry.clone(), "replay-wal-pause", "off").unwrap();
        gated_store
            .head_gate
            .wait_for_arrivals(head_arrivals_before + 1)
            .await;

        probe_wal_store.delete_sst(1.into()).await.unwrap();
        gated_store.head_gate.release();

        let err = match w1_handle.await.unwrap() {
            Ok(_) => panic!("expected W1 open to fail"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), crate::ErrorKind::Data);
    }

    #[tokio::test]
    async fn test_invalid_clock_progression() {
        // Given:
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";

        let clock = Arc::new(MockSystemClock::new());
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 128, None))
            .with_system_clock(clock.clone())
            .build()
            .await
            .unwrap();

        // When:
        // put with time = 10
        clock.set(10);
        db.put_with_options(
            b"1",
            b"1",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        // Then:
        // put with time goes backwards, should fail
        clock.set(5);
        match db
            .put_with_options(
                b"1",
                b"1",
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
        {
            Ok(_) => panic!("expected an error on inserting backwards time"),
            Err(e) => assert_eq!(e.to_string(), "Invalid error: invalid clock tick, must be monotonic. last_tick=`10`, next_tick=`5`"),
        }
    }

    #[tokio::test]
    async fn test_invalid_clock_progression_across_db_instances() {
        // Given:
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";

        let clock = Arc::new(MockSystemClock::new());
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 128, None))
            .with_system_clock(clock.clone())
            .build()
            .await
            .unwrap();

        // When:
        // put with time = 10
        clock.set(10);
        db.put_with_options(
            b"1",
            b"1",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.flush().await.unwrap();

        let db2 = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 128, None))
            .with_system_clock(clock.clone())
            .build()
            .await
            .unwrap();
        clock.set(5);
        match db2
            .put_with_options(
                b"1",
                b"1",
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
        {
            Ok(_) => panic!("expected an error on inserting backwards time"),
            Err(e) => assert_eq!(e.to_string(), "Invalid error: invalid clock tick, must be monotonic. last_tick=`10`, next_tick=`5`"),
        }
    }

    #[tokio::test]
    #[cfg(feature = "wal_disable")]
    async fn should_flush_all_memtables_when_wal_disabled() {
        // Given:
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";

        let db_options = Settings {
            wal_enabled: false,
            flush_interval: Some(Duration::from_secs(10)),
            ..Settings::default()
        };

        let db = Db::builder(path, object_store.clone())
            .with_settings(db_options.clone())
            .build()
            .await
            .unwrap();

        let mut rng = proptest_util::rng::new_test_rng(None);
        let table = sample::table(&mut rng, 1000, 5);
        test_utils::seed_database(&db, &table, false).await.unwrap();
        db.flush().await.unwrap();

        // When: reopen the database without closing the old instance
        let reopened_db = Db::builder(path, object_store.clone())
            .with_settings(db_options.clone())
            .build()
            .await
            .unwrap();

        // Then:
        assert_records_in_range(
            &table,
            &reopened_db,
            &ScanOptions::default(),
            BytesRange::from(..),
        )
        .await
    }

    #[tokio::test]
    async fn test_recover_clock_tick_from_wal() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        // Block L0 uploads so the data remains only in the WAL. The
        // uploader gives up on shutdown when the WAL is enabled, so
        // close() will complete without flushing memtables to L0.
        fail_parallel::cfg(
            fp_registry.clone(),
            "write-compacted-sst-io-error",
            "return",
        )
        .unwrap();
        let clock = Arc::new(MockSystemClock::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";

        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_system_clock(clock.clone())
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();

        clock.set(10);
        db.put_with_options(
            &[b'a'; 4],
            &[b'j'; 8],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .expect("write batch failed");
        clock.set(11);
        db.put_with_options(
            &[b'b'; 4],
            &[b'k'; 8],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .expect("write batch failed");

        db.flush().await.unwrap();
        // expect to fail as l0 upload is blocked
        assert!(db.close().await.is_err());

        // check the last_l0_clock_tick persisted in the manifest, it should be
        // i64::MIN because no WAL SST has yet made its way into L0
        let manifest_store = Arc::new(ManifestStore::new(&Path::from(path), object_store.clone()));
        let stored_manifest =
            StoredManifest::load(manifest_store, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let db_state = stored_manifest.db_state();
        let last_clock_tick = db_state.last_l0_clock_tick;
        assert_eq!(last_clock_tick, i64::MIN);

        // Disable the failpoint so the restored DB can flush normally.
        fail_parallel::cfg(fp_registry.clone(), "write-compacted-sst-io-error", "off").unwrap();

        let clock = Arc::new(MockSystemClock::new());
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_system_clock(clock.clone())
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();

        assert_eq!(db.inner.mono_clock.last_tick.load(Ordering::SeqCst), 11);
    }

    #[tokio::test]
    async fn test_should_update_manifest_clock_tick_on_l0_flush() {
        let clock = Arc::new(MockSystemClock::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";

        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 32, None))
            .with_system_clock(clock.clone())
            .build()
            .await
            .unwrap();

        // this will exceed the l0_sst_size_bytes, meaning a clean shutdown
        // will update the manifest
        clock.set(10);
        db.put(&[b'a'; 4], &[b'j'; 8])
            .await
            .expect("write batch failed");
        clock.set(11);
        db.put(&[b'b'; 4], &[b'k'; 8])
            .await
            .expect("write batch failed");

        // close the db to flush the manifest
        db.flush().await.unwrap();
        db.close().await.unwrap();

        // check the last_clock_tick persisted in the manifest, it should be
        // i64::MIN because no WAL SST has yet made its way into L0
        let manifest_store = Arc::new(ManifestStore::new(&Path::from(path), object_store.clone()));
        let stored_manifest =
            StoredManifest::load(manifest_store, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let db_state = stored_manifest.db_state();
        let last_clock_tick = db_state.last_l0_clock_tick;
        assert_eq!(last_clock_tick, 11);
    }

    #[tokio::test]
    #[cfg(feature = "wal_disable")]
    async fn test_recover_clock_tick_from_manifest() {
        let clock = Arc::new(MockSystemClock::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_kv_store";
        let mut options = test_db_options(0, 32, None);
        options.wal_enabled = false;

        let db = Db::builder(path, object_store.clone())
            .with_settings(options)
            .with_system_clock(clock.clone())
            .build()
            .await
            .unwrap();

        clock.set(10);
        db.put(&[b'a'; 4], &[b'j'; 28])
            .await
            .expect("write batch failed");
        clock.set(11);
        db.put(&[b'b'; 4], &[b'k'; 28])
            .await
            .expect("write batch failed");

        // close the db to flush the manifest
        db.flush().await.unwrap();
        db.close().await.unwrap();

        let clock = Arc::new(MockSystemClock::new());
        let mut options = test_db_options(0, 32, None);
        options.wal_enabled = false;
        let db = Db::builder(path, object_store.clone())
            .with_settings(options)
            .with_system_clock(clock.clone())
            .build()
            .await
            .unwrap();

        assert_eq!(db.inner.mono_clock.last_tick.load(Ordering::SeqCst), 11);
    }

    #[tokio::test]
    async fn test_put_get_reopen_delete_with_separate_wal_store() {
        async fn count_ssts_in(store: &Arc<InMemory>) -> usize {
            store
                .list(None)
                .filter(|r| {
                    future::ready(
                        r.as_ref()
                            .unwrap()
                            .location
                            .extension()
                            .unwrap()
                            .to_lowercase()
                            == "sst",
                    )
                })
                .count()
                .await
        }

        let fp_registry = Arc::new(FailPointRegistry::new());

        let main_object_store = Arc::new(InMemory::new());
        let wal_object_store = Arc::new(InMemory::new());
        let kv_store = Db::builder("/tmp/test_kv_store", main_object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_wal_object_store(wal_object_store.clone())
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();
        assert_eq!(count_ssts_in(&main_object_store).await, 0);
        assert_eq!(count_ssts_in(&wal_object_store).await, 1);

        let key = b"test_key";
        let value = b"test_value";

        // pause memtable flushes
        fail_parallel::cfg(fp_registry.clone(), "write-compacted-sst-io-error", "pause").unwrap();
        kv_store.put(key, value).await.unwrap();
        kv_store.flush().await.unwrap();
        assert_eq!(count_ssts_in(&main_object_store).await, 0);
        assert_eq!(count_ssts_in(&wal_object_store).await, 2);
        assert_eq!(
            kv_store.get(key).await.unwrap(),
            Some(Bytes::from_static(value))
        );
        // resume memtable flushes
        fail_parallel::cfg(fp_registry.clone(), "write-compacted-sst-io-error", "off").unwrap();

        // write some data to force L0 SST creation
        let mut batch = WriteBatch::default();
        for i in 0u32..128 {
            batch.put(i.to_be_bytes(), i.to_be_bytes());
        }
        kv_store.write(batch).await.unwrap();
        kv_store.flush().await.unwrap();
        assert_eq!(count_ssts_in(&main_object_store).await, 1);
        assert_eq!(count_ssts_in(&wal_object_store).await, 3);
        assert_eq!(
            kv_store.get(key).await.unwrap(),
            Some(Bytes::from_static(value))
        );

        kv_store.close().await.unwrap();
        assert_eq!(count_ssts_in(&wal_object_store).await, 3);

        let kv_store = Db::builder("/tmp/test_kv_store", main_object_store)
            .with_settings(test_db_options(0, 1024, None))
            .with_wal_object_store(wal_object_store.clone())
            .build()
            .await
            .unwrap();

        assert_eq!(
            kv_store.get(key).await.unwrap(),
            Some(Bytes::from_static(value))
        );

        kv_store.delete(key).await.unwrap();
        assert_eq!(None, kv_store.get(key).await.unwrap());

        kv_store.close().await.unwrap();
    }

    /// https://github.com/slatedb/slatedb/issues/2055: manifests are now
    /// written as V2 universally (RFC-0004 Phase 2), which drops
    /// `wal_object_store_uri` entirely (PR #1473). So a DB created with
    /// `with_wal_object_store()` can be reopened without it configured;
    /// the WAL-store reconfiguration check is a no-op once the persisted
    /// manifest is V2.
    #[tokio::test]
    async fn test_wal_store_reconfiguration_allowed_after_v2_manifest() {
        let object_store = Arc::new(InMemory::new());
        let wal_object_store = Arc::new(InMemory::new());

        let kv_store = Db::builder("/tmp/test_kv_store", object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_wal_object_store(wal_object_store.clone())
            .build()
            .await
            .unwrap();
        kv_store.close().await.unwrap();

        let reopened = Db::builder("/tmp/test_kv_store", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();
        reopened.close().await.unwrap();
    }

    #[tokio::test]
    #[cfg(feature = "zstd")]
    async fn test_compression_overflow_bug() {
        // This test reproduces the bug reported in https://github.com/slatedb/slatedb/issues/555
        // where re-opening a DB using zstd compression causes "attempt to subtract with overflow"
        // error in Block::decode

        use crate::config::CompressionCodec;
        use std::str::FromStr;

        // Create and load initial database
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let compress = CompressionCodec::from_str("zstd").unwrap();
        let db_builder = Db::builder("/tmp/test_kv_store", os.clone()).with_settings(Settings {
            compression_codec: Some(compress),
            ..Settings::default()
        });
        let db = db_builder.build().await.unwrap();

        for i in 0..1000 {
            let key = format!("k{}", i);
            let value = format!("{}{}", "v".repeat(i), i);
            let put_option = PutOptions::default();
            let write_option = WriteOptions {
                ..Default::default()
            };
            db.put_with_options(key.as_bytes(), value.clone(), &put_option, &write_option)
                .await
                .expect("failed to put");
        }
        db.flush().await.expect("flush failed");
        db.close().await.expect("failed to close db");

        // Reload DB and read a value to trigger error
        let db_builder = Db::builder("/tmp/test_kv_store", os.clone()).with_settings(Settings {
            compression_codec: Some(compress),
            ..Settings::default()
        });
        let db = db_builder.build().await.unwrap();
        let v = db.get("k1").await.expect("get failed").unwrap();
        assert_eq!(v.as_ref(), b"v1");

        db.close().await.expect("failed to close db");
    }

    async fn wait_for_manifest_condition(
        sm: &mut StoredManifest,
        cond: impl Fn(&ManifestCore) -> bool,
        timeout: Duration,
    ) -> ManifestCore {
        let start = tokio::time::Instant::now();
        while start.elapsed() < timeout {
            let manifest = sm.refresh().await.unwrap();
            if cond(&manifest.core) {
                return manifest.core.clone();
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("manifest condition took longer than timeout")
    }

    fn test_db_options(
        min_filter_keys: u32,
        l0_sst_size_bytes: usize,
        compactor_options: Option<CompactorOptions>,
    ) -> Settings {
        test_db_options_with_ttl(min_filter_keys, l0_sst_size_bytes, compactor_options, None)
    }

    /// Compactor options with fast poll intervals. With the defaults (5s
    /// coordinator poll + 5s worker claim poll, jittered up to 1.5x), a
    /// compaction can take longer to land in the manifest than the 10s the
    /// tests using this helper wait for one.
    fn fast_compactor_options() -> CompactorOptions {
        CompactorOptions {
            poll_interval: Duration::from_millis(100),
            worker: Some(CompactionWorkerOptions {
                compactions_poll_interval: Duration::from_millis(100),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn test_db_options_with_ttl(
        min_filter_keys: u32,
        l0_sst_size_bytes: usize,
        compactor_options: Option<CompactorOptions>,
        default_ttl_millis: Option<u64>,
    ) -> Settings {
        Settings {
            flush_interval: Some(Duration::from_millis(100)),
            #[cfg(feature = "wal_disable")]
            wal_enabled: true,
            manifest_poll_interval: Duration::from_millis(100),
            manifest_update_timeout: Duration::from_secs(300),
            max_unflushed_bytes: 134_217_728,
            l0_max_ssts: 8,
            l0_max_ssts_per_key: 8,
            l0_flush_parallelism: 1,
            min_filter_keys,
            l0_sst_size_bytes,
            max_wal_flushes_before_l0_flush: 4096,
            compactor_options,
            compression_codec: None,
            object_store_cache_options: ObjectStoreCacheOptions::default(),
            garbage_collector_options: None,
            metric_level: MetricLevel::default(),
            default_ttl_millis,
            object_store_max_retries: None,
            block_format: None,
        }
    }

    #[tokio::test]
    async fn test_snapshot_basic_functionality() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::open("test_db", object_store).await.unwrap();

        // Write some data
        db.put(b"key1", b"value1").await.unwrap();
        db.put(b"key2", b"value2").await.unwrap();

        // Create a snapshot
        let snapshot = db.snapshot().await.unwrap();

        // Verify snapshot can read the data
        assert_eq!(
            snapshot.get(b"key1").await.unwrap(),
            Some(Bytes::from(b"value1".as_ref()))
        );
        assert_eq!(
            snapshot.get(b"key2").await.unwrap(),
            Some(Bytes::from(b"value2".as_ref()))
        );

        // Write more data to the original database
        db.put(b"key3", b"value3").await.unwrap();

        // Snapshot should not see the new data
        assert_eq!(snapshot.get(b"key3").await.unwrap(), None);

        // Original database should see the new data
        assert_eq!(
            db.get(b"key3").await.unwrap(),
            Some(Bytes::from(b"value3".as_ref()))
        );
    }

    #[tokio::test]
    async fn test_recent_snapshot_min_seq_monotonic() {
        use crate::oracle::Oracle;

        let path = "/tmp/test_recent_snapshot_min_seq_monotonic";
        let object_store = Arc::new(InMemory::new());
        let settings = Settings {
            l0_sst_size_bytes: 2 * 1024,   // Smaller to trigger flush more easily
            max_unflushed_bytes: 4 * 1024, // Smaller to trigger flush more easily
            min_filter_keys: 0,
            flush_interval: Some(Duration::from_millis(100)),
            ..Default::default()
        };

        let db = Db::builder(path, object_store)
            .with_settings(settings)
            .build()
            .await
            .unwrap();

        // Initial state: recent_snapshot_min_seq should be 0
        {
            let state = db.inner.state.read();
            assert_eq!(state.state().core().recent_snapshot_min_seq, 0);
        }

        // Test 1: Force memtable flush to update recent_snapshot_min_seq
        db.put(b"key1", b"value1").await.unwrap();
        db.inner.flush_memtables(FlushTarget::All).await.unwrap();

        {
            let state = db.inner.state.read();
            let recent_min_seq = state.state().core().recent_snapshot_min_seq;
            // After flush, recent_snapshot_min_seq should be updated (no active snapshots)
            assert!(
                recent_min_seq > 0,
                "recent_snapshot_min_seq should be > 0 after flush"
            );
        }

        // Test 2: With active snapshots
        let _snapshot = db.snapshot().await.unwrap();
        let snapshot_seq = db.inner.oracle.last_committed_seq();

        // Write more data and force flush
        db.put(b"key2", b"value2").await.unwrap();
        db.inner.flush_memtables(FlushTarget::All).await.unwrap();

        // Verify that snapshot_manager.min_active_seq() returns the snapshot seq
        let min_active_seq = db.inner.snapshot_manager.min_active_seq();
        assert!(min_active_seq.is_some());
        assert_eq!(min_active_seq.unwrap(), snapshot_seq);

        {
            let state = db.inner.state.read();
            let recent_min_seq = state.state().core().recent_snapshot_min_seq;
            assert_eq!(
                recent_min_seq,
                min_active_seq.unwrap(),
                "recent_snapshot_min_seq should equal snapshot_manager.min_active_seq() after flush"
            );
        }

        // Test 3: Drop snapshot and check update
        drop(_snapshot);

        // Write more data and flush to trigger update
        db.put(b"key3", b"value3").await.unwrap();
        db.inner.flush_memtables(FlushTarget::All).await.unwrap();

        // Now recent_snapshot_min_seq should be updated to higher value (no active snapshots)
        {
            let state = db.inner.state.read();
            let recent_min_seq = state.state().core().recent_snapshot_min_seq;
            let last_l0_seq = state.state().core().last_l0_seq;

            // Should be updated to last_l0_seq since no active snapshots
            assert_eq!(
                recent_min_seq, last_l0_seq,
                "recent_snapshot_min_seq should equal last_l0_seq when no active snapshots"
            );
            assert!(recent_min_seq > snapshot_seq);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_compaction_resume_loses_merge_operands_after_snapshot_retention_advances() {
        let path =
            "/tmp/test_compaction_resume_loses_merge_operands_after_snapshot_retention_advances";
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let fp_registry = Arc::new(FailPointRegistry::new());
        let should_compact = Arc::new(AtomicBool::new(false));
        let compactor_options = CompactorOptions {
            poll_interval: Duration::from_millis(10),
            commit_compacted_interval: Duration::from_millis(10),
            worker: Some(CompactionWorkerOptions {
                compactions_poll_interval: Duration::from_millis(10),
                heartbeat_interval: Duration::from_millis(10),
                max_sst_size: 1,
                ..Default::default()
            }),
            ..Default::default()
        };
        let settings_without_compactor = test_db_options(0, 1024, None);

        // The compactor's SST I/O runs through a gated store so the test can
        // freeze the job after its first output SSTs upload. Manifest and
        // `.compactions` I/O use the ungated store passed to `Db::builder`,
        // so the worker's heartbeats keep flowing while the job is frozen.
        let gated = Arc::new(GatedObjectStore::new(object_store.clone()));
        let gated_store: Arc<dyn ObjectStore> = gated.clone();

        let db = Db::builder(path, object_store.clone())
            .with_settings(settings_without_compactor.clone())
            .with_sst_block_size(SstBlockSize::Other(1))
            .with_fp_registry(fp_registry.clone())
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .with_compactor_builder(
                CompactorBuilder::new(path, gated_store)
                    .with_options(compactor_options.clone())
                    .with_scheduler_supplier(Arc::new(OnDemandCompactionSchedulerSupplier::new({
                        let should_compact = should_compact.clone();
                        Arc::new(move |_| should_compact.load(Ordering::SeqCst))
                    }))),
            )
            .build()
            .await
            .unwrap();

        db.merge(b"k", b"1").await.unwrap();
        let snapshot = db.snapshot().await.unwrap();
        let snapshot_seq = db.inner.oracle.last_committed_seq();
        db.flush().await.unwrap();

        for operand in [b"2", b"3", b"4", b"5", b"6"] {
            db.merge(b"k", operand).await.unwrap();
            db.flush().await.unwrap();
        }

        let manifest_store = Arc::new(ManifestStore::new(&Path::from(path), object_store.clone()));
        let mut stored_manifest =
            StoredManifest::load(manifest_store.clone(), Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let staged = wait_for_manifest_condition(
            &mut stored_manifest,
            |core| core.last_l0_seq >= 6 && core.recent_snapshot_min_seq == snapshot_seq,
            Duration::from_secs(10),
        )
        .await;
        assert!(
            staged.tree.l0.len() > 1,
            "the test requires multiple L0s so compaction has work to resume"
        );

        // Fence the worker on the first heartbeat that publishes progress
        // carrying an output SST. This leaves a partially-complete compaction
        // (its first output SST plus the retention_min_seq captured while
        // the snapshot was live) persisted for the resumed attempt.
        fail_parallel::cfg(
            fp_registry.clone(),
            "compactor-heartbeat-after-output-sst",
            "return",
        )
        .unwrap();

        let mut status_rx = db.subscribe();

        // Admit exactly one output SST upload through the closed gate, so the
        // job reports progress with one SST and then freezes on its next
        // upload — it cannot finish before the fencing heartbeat observes
        // that progress.
        let baseline_puts = gated.put_opts_gate.arrivals();
        gated.put_opts_gate.close();
        should_compact.store(true, Ordering::SeqCst);
        tokio::time::timeout(
            Duration::from_secs(10),
            gated.put_opts_gate.wait_for_arrivals(baseline_puts + 1),
        )
        .await
        .expect("compaction should upload output SSTs");
        gated.put_opts_gate.admit(1);

        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                match status_rx.borrow().close_reason {
                    Some(CloseReason::Fenced) => break,
                    Some(reason) => {
                        panic!("expected compactor failpoint to fence DB, got {reason:?}")
                    }
                    None => {}
                }
                status_rx.changed().await.expect("db status channel closed");
            }
        })
        .await
        .expect("compactor did not hit the failpoint");

        drop(snapshot);
        gated.put_opts_gate.release();
        db.close().await.unwrap();
        fail_parallel::cfg(
            fp_registry.clone(),
            "compactor-heartbeat-after-output-sst",
            "off",
        )
        .unwrap();

        let db = Db::builder(path, object_store.clone())
            .with_settings(settings_without_compactor.clone())
            .with_sst_block_size(SstBlockSize::Other(1))
            .with_fp_registry(fp_registry.clone())
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();
        db.put(b"zz-advance-retention", b"x").await.unwrap();
        db.flush().await.unwrap();
        wait_for_manifest_condition(
            &mut stored_manifest,
            |core| core.last_l0_seq >= 7 && core.recent_snapshot_min_seq > snapshot_seq,
            Duration::from_secs(10),
        )
        .await;
        db.close().await.unwrap();

        let db = Db::builder(path, object_store.clone())
            .with_settings(settings_without_compactor)
            .with_sst_block_size(SstBlockSize::Other(1))
            .with_fp_registry(fp_registry)
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .with_compactor_builder(
                CompactorBuilder::new(path, object_store.clone())
                    .with_options(compactor_options)
                    .with_scheduler_supplier(Arc::new(OnDemandCompactionSchedulerSupplier::new(
                        Arc::new(|_| false),
                    ))),
            )
            .build()
            .await
            .unwrap();

        let manifest_store = Arc::new(ManifestStore::new(&Path::from(path), object_store.clone()));
        let mut stored_manifest =
            StoredManifest::load(manifest_store, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        wait_for_manifest_condition(
            &mut stored_manifest,
            |core| !core.tree.compacted.is_empty(),
            Duration::from_secs(10),
        )
        .await;
        db.refresh_manifest().await.unwrap();

        let actual = db.get(b"k").await.unwrap();
        db.close().await.unwrap();

        assert_eq!(actual, Some(Bytes::from_static(b"123456")));
    }

    #[tokio::test]
    async fn test_recent_snapshot_min_seq_uses_transaction_seq() {
        let path = "/tmp/test_recent_snapshot_min_seq_uses_transaction_seq";
        let object_store = Arc::new(InMemory::new());
        let db = Db::builder(path, object_store).build().await.unwrap();

        {
            let state = db.inner.state.read();
            assert_eq!(state.state().core().recent_snapshot_min_seq, 0);
        }

        db.put(b"key1", b"value1").await.unwrap();
        db.inner.flush_memtables(FlushTarget::All).await.unwrap();

        let txn = db.begin(IsolationLevel::Snapshot).await.unwrap();
        let txn_seq = txn.seqnum();

        db.put(b"key2", b"value2").await.unwrap();
        db.inner.flush_memtables(FlushTarget::All).await.unwrap();

        let min_active_seq = db.inner.txn_manager.min_active_seq();
        assert_eq!(min_active_seq, Some(txn_seq));

        {
            let state = db.inner.state.read();
            let recent_min_seq = state.state().core().recent_snapshot_min_seq;
            assert_eq!(
                recent_min_seq, txn_seq,
                "recent_snapshot_min_seq should equal txn_manager.min_active_seq() after flush"
            );
        }

        drop(txn);

        db.put(b"key3", b"value3").await.unwrap();
        db.inner.flush_memtables(FlushTarget::All).await.unwrap();

        {
            let state = db.inner.state.read();
            let recent_min_seq = state.state().core().recent_snapshot_min_seq;
            let last_l0_seq = state.state().core().last_l0_seq;
            assert_eq!(
                recent_min_seq, last_l0_seq,
                "recent_snapshot_min_seq should equal last_l0_seq when no active transactions"
            );
            assert!(recent_min_seq > txn_seq);
        }
    }

    #[tokio::test]
    async fn test_recent_snapshot_min_seq_prefers_snapshot_when_snapshot_seq_is_lower() {
        let path = "/tmp/test_recent_snapshot_min_seq_prefers_snapshot_when_snapshot_seq_is_lower";
        let object_store = Arc::new(InMemory::new());
        let db = Db::builder(path, object_store).build().await.unwrap();

        db.put(b"key1", b"value1").await.unwrap();
        db.inner.flush_memtables(FlushTarget::All).await.unwrap();

        let snapshot = db.snapshot().await.unwrap();
        let snapshot_seq = snapshot.seq();

        db.put(b"key2", b"value2").await.unwrap();
        db.inner.flush_memtables(FlushTarget::All).await.unwrap();

        let txn = db.begin(IsolationLevel::Snapshot).await.unwrap();
        let txn_seq = txn.seqnum();

        assert_eq!(
            db.inner.snapshot_manager.min_active_seq(),
            Some(snapshot_seq)
        );
        assert_eq!(db.inner.txn_manager.min_active_seq(), Some(txn_seq));
        assert!(snapshot_seq < txn_seq);

        db.put(b"key3", b"value3").await.unwrap();
        db.inner.flush_memtables(FlushTarget::All).await.unwrap();

        {
            let state = db.inner.state.read();
            let recent_min_seq = state.state().core().recent_snapshot_min_seq;
            assert_eq!(
                recent_min_seq,
                snapshot_seq,
                "recent_snapshot_min_seq should use the snapshot seq when it is smaller than the transaction seq"
            );
        }

        drop(snapshot);

        db.put(b"key4", b"value4").await.unwrap();
        db.inner.flush_memtables(FlushTarget::All).await.unwrap();

        assert_eq!(db.inner.snapshot_manager.min_active_seq(), None);
        assert_eq!(db.inner.txn_manager.min_active_seq(), Some(txn_seq));

        {
            let state = db.inner.state.read();
            let recent_min_seq = state.state().core().recent_snapshot_min_seq;
            assert_eq!(
                recent_min_seq,
                txn_seq,
                "recent_snapshot_min_seq should move to the transaction seq after the snapshot is dropped"
            );
        }
    }

    #[tokio::test]
    async fn test_recent_snapshot_min_seq_prefers_transaction_when_transaction_seq_is_lower() {
        let path =
            "/tmp/test_recent_snapshot_min_seq_prefers_transaction_when_transaction_seq_is_lower";
        let object_store = Arc::new(InMemory::new());
        let db = Db::builder(path, object_store).build().await.unwrap();

        db.put(b"key1", b"value1").await.unwrap();
        db.inner.flush_memtables(FlushTarget::All).await.unwrap();

        let txn = db.begin(IsolationLevel::Snapshot).await.unwrap();
        let txn_seq = txn.seqnum();

        db.put(b"key2", b"value2").await.unwrap();
        db.inner.flush_memtables(FlushTarget::All).await.unwrap();

        let snapshot = db.snapshot().await.unwrap();
        let snapshot_seq = snapshot.seq();

        assert_eq!(db.inner.txn_manager.min_active_seq(), Some(txn_seq));
        assert_eq!(
            db.inner.snapshot_manager.min_active_seq(),
            Some(snapshot_seq)
        );
        assert!(txn_seq < snapshot_seq);

        db.put(b"key3", b"value3").await.unwrap();
        db.inner.flush_memtables(FlushTarget::All).await.unwrap();

        {
            let state = db.inner.state.read();
            let recent_min_seq = state.state().core().recent_snapshot_min_seq;
            assert_eq!(
                recent_min_seq,
                txn_seq,
                "recent_snapshot_min_seq should use the transaction seq when it is smaller than the snapshot seq"
            );
        }

        drop(txn);

        db.put(b"key4", b"value4").await.unwrap();
        db.inner.flush_memtables(FlushTarget::All).await.unwrap();

        assert_eq!(db.inner.txn_manager.min_active_seq(), None);
        assert_eq!(
            db.inner.snapshot_manager.min_active_seq(),
            Some(snapshot_seq)
        );

        {
            let state = db.inner.state.read();
            let recent_min_seq = state.state().core().recent_snapshot_min_seq;
            assert_eq!(
                recent_min_seq,
                snapshot_seq,
                "recent_snapshot_min_seq should move to the snapshot seq after the transaction is dropped"
            );
        }
    }

    #[tokio::test]
    async fn test_memtable_flush_updates_last_remote_persisted_seq() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test";
        let mut opts = test_db_options(0, 256, None);
        opts.flush_interval = Some(Duration::MAX);
        let db = Db::builder(path, object_store.clone())
            .with_settings(opts)
            .build()
            .await
            .unwrap();

        // do a write and flush memtable only (not wal)
        let write_opts = WriteOptions {
            ..Default::default()
        };
        db.put_with_options(&b"foo", &b"bar", &PutOptions::default(), &write_opts)
            .await
            .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // check that read with durability level remote returns value
        let v = db
            .get_with_options(&b"foo", &ReadOptions::new().with_durability_filter(Memory))
            .await
            .unwrap();
        assert_eq!(v, Some(Bytes::from(b"bar".as_ref())));
        let v = db
            .get_with_options(&b"foo", &ReadOptions::new().with_durability_filter(Remote))
            .await
            .unwrap();
        assert_eq!(v, Some(Bytes::from(b"bar".as_ref())));
    }

    #[tokio::test]
    async fn should_merge_operand_into_empty_key() {
        // Given: Database with merge operator, empty key
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_merge_1", object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        // When: Merging a value
        db.merge(b"key1", b"value1").await.unwrap();

        // Then: Value is stored and retrievable
        let result = db.get(b"key1").await.unwrap();
        assert_eq!(result, Some(Bytes::from("value1")));
    }

    #[tokio::test]
    async fn should_merge_multiple_operands_into_same_key() {
        // Given: Database with merge operator, key with initial merge
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_merge_2", object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        // When: Merging multiple operands to the same key
        db.merge(b"key1", b"a").await.unwrap();
        db.merge(b"key1", b"b").await.unwrap();
        db.merge(b"key1", b"c").await.unwrap();

        // Then: All operands are merged correctly when read
        let result = db.get(b"key1").await.unwrap();
        assert_eq!(result, Some(Bytes::from("abc")));
    }

    #[tokio::test]
    async fn should_persist_merge_operands_across_flush() {
        // Given: Database with merge operator, merge operands in memtable
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_merge_3", object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        db.merge(b"key1", b"a").await.unwrap();
        db.merge(b"key1", b"b").await.unwrap();

        // When: Flushing memtable and reading
        db.flush().await.unwrap();

        // Then: Merged result is correct after flush
        let result = db.get(b"key1").await.unwrap();
        assert_eq!(result, Some(Bytes::from("ab")));

        // Verify it still works after additional merges post-flush
        db.merge(b"key1", b"c").await.unwrap();
        let result = db.get(b"key1").await.unwrap();
        assert_eq!(result, Some(Bytes::from("abc")));
    }

    #[tokio::test]
    async fn should_error_when_merging_without_merge_operator() {
        // Given: Database with merge operator, merge operands written
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_merge_4";
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        db.merge(b"key1", b"value1").await.unwrap();
        db.flush().await.unwrap();
        db.close().await.unwrap();

        // When: Reopening the DB without a merge operator and then reading
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        // Then: Reading should fail because merge operands require a merge operator
        let err = db.get(b"key1").await.unwrap_err();
        assert_eq!(err.kind(), crate::ErrorKind::Invalid);
    }

    #[tokio::test]
    async fn should_error_when_writing_merge_without_merge_operator() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_merge_4_write_fails", object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        let err = db.merge(b"key1", b"value1").await.unwrap_err();
        assert_eq!(err.kind(), crate::ErrorKind::Invalid);

        let result = db.get(b"key1").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn should_error_when_writing_batch_with_merge_without_merge_operator() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_merge_4_batch_write_fails", object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.merge(b"key2", b"value2");

        let err = db.write(batch).await.unwrap_err();
        assert_eq!(err.kind(), crate::ErrorKind::Invalid);

        assert_eq!(db.get(b"key1").await.unwrap(), None);
        assert_eq!(db.get(b"key2").await.unwrap(), None);
    }

    #[tokio::test]
    async fn should_merge_operands_after_reopen() {
        // Given: Database with merge operator, merge operands written
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_merge_5";
        let db = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        db.merge(b"key1", b"a").await.unwrap();
        db.merge(b"key1", b"b").await.unwrap();
        db.flush().await.unwrap();
        db.close().await.unwrap();

        // When: Closing and reopening database, then reading
        let db_reopened = Db::builder(path, object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        // Then: Merged result is correct after reopen
        let result = db_reopened.get(b"key1").await.unwrap();
        assert_eq!(result, Some(Bytes::from("ab")));

        // Verify additional merges work after reopen
        db_reopened.merge(b"key1", b"c").await.unwrap();
        let result = db_reopened.get(b"key1").await.unwrap();
        assert_eq!(result, Some(Bytes::from("abc")));
    }

    /// Reproduces a race where GC can delete an L0 SST before the manifest
    /// is updated to reference it at the DB level:
    /// 1. New L0 is written
    /// 2. 100ms passes
    /// 3. GC lists SSTs
    /// 4. GC sees L0 SST from (1)
    /// 5. GC deletes L0 SST from (1) (it is > min_age=100ms old and is not in any manifests)
    /// 6. L0 is added to in-memory manifest
    /// 7. Manifest is written to object storage
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_gc_race_deletes_l0_before_manifest_update() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_gc_race_deletes_l0_before_manifest_update");

        let mut settings = test_db_options(0, 1024, None);
        settings.flush_interval = None;

        let db = Db::builder(path.clone(), object_store.clone())
            .with_settings(settings)
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .expect("failed to build DB");
        let db = Arc::new(db);

        // Pause after the L0 SST is written but before the manifest is updated.
        fail_parallel::cfg(
            fp_registry.clone(),
            "after-flush-imm-to-l0-before-manifest",
            "pause",
        )
        .expect("failed to set failpoint");

        // Write some data so we have an immutable memtable to flush to L0.
        db.put_with_options(
            b"key1",
            b"value1",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .expect("failed to put");

        // Trigger a memtable flush in the background; it will block at the failpoint.
        let this_db = db.clone();
        let flush_handle = tokio::spawn(async move {
            this_db
                .flush_with_options(FlushOptions {
                    flush_type: FlushType::MemTable,
                })
                .await
        });

        // Wait for the L0 SST to appear in the table store, indicating it has been written.
        let mut ssts = Vec::new();
        for _ in 0..200 {
            ssts = db
                .inner
                .table_store
                .list_compacted_ssts(..)
                .await
                .expect("failed to list compacted ssts");
            if !ssts.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            ssts.len(),
            1,
            "expected exactly one L0 SST after GC, but found {:?}",
            ssts.iter().map(|sst| sst.id).collect::<Vec<_>>()
        );

        // Run a manual GC with aggressive settings to delete the L0 SST while
        // the manifest is still not updated.
        let gc_options = GarbageCollectorOptions {
            wal_options: Some(GarbageCollectorDirectoryOptions {
                interval: None,
                min_age: Duration::from_millis(0),
                dry_run: false,
            }),
            wal_fence_options: None,
            manifest_options: Some(GarbageCollectorDirectoryOptions {
                interval: None,
                min_age: Duration::from_millis(0),
                dry_run: false,
            }),
            compacted_options: Some(GarbageCollectorDirectoryOptions {
                interval: None,
                min_age: Duration::from_millis(0),
                dry_run: false,
            }),
            compactions_options: Some(GarbageCollectorDirectoryOptions {
                interval: None,
                min_age: Duration::from_millis(0),
                dry_run: false,
            }),
            detach_options: None,
            metric_level: None,
            boundary_files_enabled: true,
            object_store_max_retries: None,
        };

        let gc = GarbageCollectorBuilder::new(path.clone(), object_store.clone())
            .with_options(gc_options)
            .with_system_clock(db.inner.system_clock.clone())
            .build();

        // Run the GC a few times so it sees the L0 SST (and hopefully doesn't delete it)
        for _ in 0..5 {
            gc.run_gc_once().await;
            ssts = db
                .inner
                .table_store
                .list_compacted_ssts(..)
                .await
                .expect("failed to list compacted ssts after manual GC");
            if ssts.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            ssts.len(),
            1,
            "expected exactly one L0 SST after GC, but found {:?}",
            ssts.iter().map(|sst| sst.id).collect::<Vec<_>>()
        );

        // Now allow the memtable flush to resume and persist the manifest referencing the deleted SST.
        fail_parallel::cfg(
            fp_registry.clone(),
            "after-flush-imm-to-l0-before-manifest",
            "off",
        )
        .expect("failed to set failpoint");
        flush_handle
            .await
            .expect("failed to join flush handle")
            .expect("flush failed");

        // Read the latest manifest and verify it references the L0 SST.
        let manifest_store = ManifestStore::new(&path, object_store.clone());
        let manifest = manifest_store
            .read_latest_manifest()
            .await
            .expect("failed to read latest manifest");
        assert_eq!(
            manifest.manifest.core.tree.l0.len(),
            1,
            "expected exactly one L0 SST in manifest"
        );
        let l0_id = manifest.manifest.core.tree.l0[0].sst.id;
        assert_eq!(
            l0_id, ssts[0].id,
            "expected SST {:?} but found SST {:?}",
            ssts[0].id, l0_id,
        );

        // Build a read-only TableStore sharing the same underlying object store
        // and assert that the referenced L0 SST still exists.
        let table_store = TableStore::new(
            object_store.clone(),
            SsTableFormat::default(),
            path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let compacted_ssts = table_store
            .list_compacted_ssts(..)
            .await
            .expect("failed to list compacted ssts");
        let still_exists = compacted_ssts.iter().any(|m| m.id == l0_id);
        assert!(
            still_exists,
            "manifest references L0 SST {:?} that GC has already deleted",
            l0_id
        );

        db.close().await.expect("failed to close DB");
    }

    #[tokio::test]
    async fn test_write_handle() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_write_handle_db";
        let clock = Arc::new(MockSystemClock::new());
        let db = Db::builder(path, object_store)
            .with_settings(test_db_options(0, 1024, None))
            .with_system_clock(clock.clone())
            .build()
            .await
            .unwrap();

        // Put
        let key = b"key1";
        let value = b"value1";
        clock.set(100);
        let handle = db
            .put_with_options(
                key,
                value,
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(handle.seqnum(), 1);
        assert_eq!(handle.create_ts(), 100);

        // Put with options (TTL)
        clock.set(200);
        let put_opts = PutOptions {
            ttl: Ttl::ExpireAfterMillis(1000),
        };
        let handle = db
            .put_with_options(
                b"key2",
                b"value2",
                &put_opts,
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(handle.seqnum(), 2);
        assert_eq!(handle.create_ts(), 200);

        // Delete
        clock.set(300);
        let handle = db
            .delete_with_options(
                b"key1",
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(handle.seqnum(), 3);
        assert_eq!(handle.create_ts(), 300);

        // Write Batch
        clock.set(400);
        let mut batch = WriteBatch::new();
        batch.put(b"key3", b"value3");
        batch.delete(b"key2");
        let handle = db
            .write_with_options(
                batch,
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(handle.seqnum(), 4);
        assert_eq!(handle.create_ts(), 400);
    }

    #[tokio::test]
    async fn test_write_handle_with_batch() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_write_batch_handle";
        let clock = Arc::new(MockSystemClock::new());
        let db = Db::builder(path, object_store)
            .with_settings(test_db_options(0, 1024, None))
            .with_system_clock(clock.clone())
            .build()
            .await
            .unwrap();

        // Write Batch 1
        clock.set(100);
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.delete(b"key2");
        let handle = db
            .write_with_options(
                batch,
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(handle.seqnum(), 1);
        assert_eq!(handle.create_ts(), 100);

        // Write Batch 2
        clock.set(200);
        let mut batch = WriteBatch::new();
        batch.put(b"key3", b"value3");
        batch.put(b"key4", b"value4");
        let handle = db
            .write_with_options(
                batch,
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(handle.seqnum(), 2);
        assert_eq!(handle.create_ts(), 200);

        // Write Batch 3
        clock.set(300);
        let mut batch = WriteBatch::new();
        batch.delete(b"key1");
        let handle = db
            .write_with_options(
                batch,
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(handle.seqnum(), 3);
        assert_eq!(handle.create_ts(), 300);
    }

    #[tokio::test]
    async fn test_write_with_options_empty_batch_returns_empty_batch_error() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_write_with_options_empty_batch", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();

        let err = db
            .inner
            .write_with_options(
                WriteBatch::new(),
                &WriteOptions {
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, SlateDBError::EmptyBatch));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_txn_conflict_when_first_commit_paused_post_commit() {
        // This test reproduces the error in #1301. Befor the fix, the commited seqnum
        // in the oracle was advanced outside the commit lock. This caused a race where
        // another transaction could start, see the original seqnum (pre-commit), but not
        // see conflicts. See #1301 for more details.
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_txn_conflict_post_commit_pause", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();

        // 1-2. Create txn1 and write k1=v1.
        let txn1 = db
            .begin(IsolationLevel::SerializableSnapshot)
            .await
            .unwrap();
        txn1.put(b"k1", b"v1").unwrap();

        // 3. Pause on write-batch-post-commit so txn1 blocks after conflict metadata is tracked.
        fail_parallel::cfg(fp_registry.clone(), "write-batch-post-commit", "pause").unwrap();

        let txn1_start_seq = txn1.seqnum();

        // 4. Commit txn1 in the background; it should pause at write-batch-post-commit.
        let txn1_commit_task = tokio::spawn(async move { txn1.commit().await });

        // 5. Wait until txn1 reaches post-commit pause:
        // - txn1 is no longer active in txn_manager
        let pause_reached = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let txn1_removed_from_active = db.inner.txn_manager.min_active_seq().is_none();
                if txn1_removed_from_active {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .is_ok();
        if !pause_reached {
            fail_parallel::cfg(fp_registry.clone(), "write-batch-post-commit", "off").unwrap();
            let _ = txn1_commit_task.await;
            panic!("txn1 did not pause at write-batch-post-commit");
        }

        // 5.1. Add/drop txn to trigger a recycle that removes txn1 from recent commits.
        let txn_dropped = db
            .begin(IsolationLevel::SerializableSnapshot)
            .await
            .unwrap();
        drop(txn_dropped);

        // 6. Create txn2 after txn1 is committed but before batch_write is complete.
        // The seqnum should advance transactionally with the commit, so txn2 should
        // see txn1's post-write seqnum.
        let txn2 = db
            .begin(IsolationLevel::SerializableSnapshot)
            .await
            .unwrap();

        // 6.1. txn2 should see k1=v1 since it started after txn1's commit, even though the
        // batch write is not fully complete until after txn2 starts.
        assert_eq!(
            txn2.get(b"k1").await.unwrap(),
            Some(Bytes::from_static(b"v1"))
        );

        // 7. Unpause write-batch-post-commit, advance seqnum.
        fail_parallel::cfg(fp_registry.clone(), "write-batch-post-commit", "off").unwrap();

        // 8. Wait for txn1 to finish committing. txn1 is dropped when this finishes.
        let _ = txn1_commit_task
            .await
            .expect("failed to join txn1 commit task")
            .expect("txn1 commit should succeed");
        assert_eq!(
            txn2.seqnum(),
            txn1_start_seq + 1, // 1 row was written
            "txn2 should see the commit seqnum after txn1's commit"
        );

        // 9-10. txn2 writes k1=v2 then attempts to commit (should not conflict).
        txn2.put(b"k1", b"v2").unwrap();
        txn2.put(b"k2", b"v2").unwrap();
        assert!(txn2.commit().await.is_ok());

        // 11. txn2 committed, so the db should show it.
        assert_eq!(
            db.get(b"k1").await.unwrap(),
            Some(Bytes::from_static(b"v2"))
        );

        db.close().await.unwrap();
    }

    /// Regression test: cancelling a commit future after the write batch has been
    /// sent to the writer (but before it is processed) drops the DbTransaction,
    /// which removes it from active_txns. The writer then:
    ///   1. Skips conflict detection (check_has_conflict returns false for
    ///      missing txn_ids)
    ///   2. Skips tracking the committed state (track_recent_committed_txn
    ///      silently no-ops)
    ///
    /// This allows a second transaction writing the same key to commit without
    /// detecting a write-write conflict — a lost update anomaly.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_commit_future_cancel_bypasses_conflict_detection() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder(
            "/tmp/test_commit_future_cancel_bypasses_conflict_detection",
            object_store,
        )
        .with_fp_registry(fp_registry.clone())
        .build()
        .await
        .unwrap();

        // Write initial data.
        db.put(b"x", b"v0").await.unwrap();
        let initial_last_seq = db.inner.oracle.last_seq();

        // Start txn1 and buffer a write to key "x".
        let txn1 = db.begin(IsolationLevel::Snapshot).await.unwrap();
        let txn1_started_seq = txn1.seqnum();
        txn1.put(b"x", b"txn1_value").unwrap();

        // Pause the writer *before* it tracks the committed transaction state.
        // At the pause point the WAL + memtable writes have already happened,
        // but track_recent_committed_txn has not been called yet.
        fail_parallel::cfg(fp_registry.clone(), "write-batch-pre-commit", "pause").unwrap();

        // Commit txn1 in a background task. It will send the batch to the
        // writer, which will process it up to the pause point and block.
        let txn1_commit = tokio::spawn(async move { txn1.commit().await });

        // Wait until the writer has started processing txn1's batch.
        // oracle.last_seq() advances at the very start of write_batch. No
        // further synchronization is needed: the failpoint guarantees the
        // writer cannot advance past pre-commit before we cancel below.
        let reached = tokio::time::timeout(Duration::from_secs(30), async {
            while db.inner.oracle.last_seq() <= initial_last_seq {
                tokio::task::yield_now().await;
            }
        })
        .await;
        assert!(reached.is_ok(), "writer did not start processing txn1");

        // Cancel txn1's commit future. The DbTransaction was moved into the
        // writer's queue message at commit time, so cancelling the future does
        // NOT drop it on the caller side and therefore cannot call drop_txn.
        txn1_commit.abort();
        let _ = txn1_commit.await;

        // txn1 stays in active_txns (the writer now owns it, not the caller).
        assert!(
            db.inner.txn_manager.min_active_seq().is_some(),
            "txn1 should still be in active_txns (writer owns the in-flight txn)"
        );

        // Start txn2 while the writer is still paused (committed_seq has not
        // been advanced yet), so txn2 starts at the same snapshot as txn1.
        let txn2 = db.begin(IsolationLevel::Snapshot).await.unwrap();
        assert_eq!(
            txn2.seqnum(),
            txn1_started_seq,
            "txn2 should start at the same seq as txn1 (committed_seq not yet advanced)"
        );
        txn2.put(b"x", b"txn2_value").unwrap();

        // Un-pause the writer. It will track txn1's committed state properly
        // because txn1 is still in active_txns.
        fail_parallel::cfg(fp_registry.clone(), "write-batch-pre-commit", "off").unwrap();

        // Commit txn2. Its batch is enqueued behind txn1's, so the writer
        // finishes tracking txn1's committed state before it runs conflict
        // detection for txn2.
        // Both txn1 and txn2 wrote key "x" from the same snapshot.
        // A correct implementation detects a write-write conflict here.
        let result = txn2.commit().await;
        assert!(
            result.is_err(),
            "txn2 should detect a WW conflict with txn1"
        );

        db.close().await.unwrap();
    }

    /// Verify that the writer cleans up active_txns when a cancelled commit's
    /// batch fails (e.g., TransactionConflict). Because commit moves the
    /// DbTransaction into the writer's queue message, cancelling the future does
    /// not drop it caller-side; the writer owns it and drops it (running drop_txn)
    /// when it finishes processing the message. Without that ownership transfer
    /// the txn would leak in active_txns, pinning min_active_seq and blocking
    /// compaction.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_cancelled_commit_writer_error_cleans_up_active_txn() {
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder(
            "/tmp/test_cancelled_commit_writer_error_cleans_up_active_txn",
            object_store,
        )
        .with_fp_registry(fp_registry.clone())
        .build()
        .await
        .unwrap();

        // Start txn_a and txn_b at the same snapshot. Both write the same key,
        // so txn_b will hit a WW conflict once txn_a's commit is tracked.
        let txn_a = db.begin(IsolationLevel::Snapshot).await.unwrap();
        let txn_b = db.begin(IsolationLevel::Snapshot).await.unwrap();
        txn_a.put(b"y", b"txn_a_value").unwrap();
        txn_b.put(b"y", b"txn_b_value").unwrap();

        // Commit txn_a fully so its committed state is tracked. From here on,
        // txn_b is the only active transaction.
        txn_a.commit().await.expect("txn_a commit should succeed");
        let seq_after_a = db.inner.oracle.last_seq();

        // Park the writer on a filler write: pause at post-commit and wait for
        // the filler's processing to start (oracle.last_seq() advances at the
        // very start of write_batch). With the writer held, txn_b's commit
        // below cannot complete before we cancel it.
        fail_parallel::cfg(fp_registry.clone(), "write-batch-post-commit", "pause").unwrap();
        let filler_db = db.clone();
        let filler = tokio::spawn(async move { filler_db.put(b"z", b"filler_value").await });
        let reached = tokio::time::timeout(Duration::from_secs(30), async {
            while db.inner.oracle.last_seq() <= seq_after_a {
                tokio::task::yield_now().await;
            }
        })
        .await;
        assert!(
            reached.is_ok(),
            "writer did not start processing the filler"
        );
        let seq_after_filler = db.inner.oracle.last_seq();

        // Start committing txn_b, then cancel the commit. The first poll of the
        // commit future enqueues the batch — moving the DbTransaction into the
        // queued message — and then parks waiting on the (paused) writer, so
        // now_or_never() drops the future exactly like a cancelled in-flight
        // commit.
        assert!(
            txn_b.commit().now_or_never().is_none(),
            "commit should be pending while the writer is paused"
        );

        // txn_b should still be in active_txns: ownership moved to the writer
        // at enqueue, so cancelling the future must not run drop_txn.
        assert!(
            db.inner.txn_manager.min_active_seq().is_some(),
            "txn_b should still be in active_txns (writer owns the in-flight txn)"
        );

        // Un-pause the writer. It finishes the filler, then processes txn_b's
        // batch, which is rejected with a WW conflict; dropping the rejected
        // message drops the owned DbTransaction, which runs drop_txn.
        fail_parallel::cfg(fp_registry.clone(), "write-batch-post-commit", "off").unwrap();
        filler
            .await
            .expect("failed to join filler task")
            .expect("filler write should succeed");

        // The writer picks up txn_b's batch (last_seq advances at the start of
        // write_batch, before conflict detection rejects it)...
        let processed = tokio::time::timeout(Duration::from_secs(30), async {
            while db.inner.oracle.last_seq() <= seq_after_filler {
                tokio::task::yield_now().await;
            }
        })
        .await;
        assert!(processed.is_ok(), "writer did not start processing txn_b");

        // ...and cleans it up from active_txns.
        let cleaned = tokio::time::timeout(Duration::from_secs(30), async {
            while db.inner.txn_manager.min_active_seq().is_some() {
                tokio::task::yield_now().await;
            }
        })
        .await;
        assert!(
            cleaned.is_ok(),
            "txn_b should have been cleaned up from active_txns by the writer"
        );

        // txn_b was rejected, so its write must not be visible.
        assert_eq!(
            db.get(b"y").await.unwrap(),
            Some(Bytes::from_static(b"txn_a_value"))
        );

        db.close().await.unwrap();
    }

    /// When the commit's enqueue fails (e.g. the writer channel is closed), the
    /// DbTransaction was moved into `enqueue_write_batch` but never made it onto
    /// the queue. Ownership therefore stays on the caller side: the moved-in txn
    /// drops inside the failing enqueue and runs drop_txn, so the txn must not
    /// leak in active_txns and commit must surface the error.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_commit_enqueue_failure_cleans_up_active_txn() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder(
            "/tmp/test_commit_enqueue_failure_cleans_up_active_txn",
            object_store,
        )
        .build()
        .await
        .unwrap();

        // Capture a handle to the txn manager before closing the db so we can
        // assert on active_txns afterwards.
        let txn_manager = db.inner.txn_manager.clone();

        let txn = db.begin(IsolationLevel::Snapshot).await.unwrap();
        txn.put(b"k", b"v").unwrap();
        assert!(
            txn_manager.min_active_seq().is_some(),
            "txn should be registered in active_txns before commit"
        );

        // Close the db, shutting down the writer and its channel. The txn keeps
        // its own Arc<DbInner>, so it remains usable for the commit attempt.
        db.close().await.unwrap();

        // Commit now fails to enqueue. The moved-in DbTransaction drops on the
        // caller side and cleans itself up.
        let result = txn.commit().await;
        assert!(
            result.is_err(),
            "commit should fail once the writer is closed"
        );
        assert!(
            txn_manager.min_active_seq().is_none(),
            "txn must be cleaned up on the caller side when enqueue fails"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_txn_conflict_commit_seq_gap_does_not_block_l0_retirement() {
        const REPRO_SEED: u64 = 2_985_011_763_506_195_159;
        const MAX_UNFLUSHED_BYTES: usize = 8 * 1024;
        const LARGE_VALUE_BYTES: usize = 16 * 1024;

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let mut options = test_db_options(0, 1024, None);
        options.flush_interval = None;
        options.manifest_poll_interval = Duration::from_millis(10);
        options.max_unflushed_bytes = MAX_UNFLUSHED_BYTES;
        options.l0_max_ssts = 16;

        let db = Db::builder(
            "/tmp/test_txn_conflict_commit_seq_gap_blocks_l0_manifest_retirement",
            object_store,
        )
        .with_seed(REPRO_SEED)
        .with_settings(options)
        .build()
        .await
        .unwrap();
        let write_opts = WriteOptions {
            ..Default::default()
        };

        // Establish and flush seq=1 so the L0 manifest writer has a known
        // contiguous frontier before the conflict sequence gap is created.
        db.put_with_options(b"conflict-key", b"v1", &PutOptions::default(), &write_opts)
            .await
            .unwrap();
        tokio::time::timeout(
            Duration::from_secs(5),
            db.flush_with_options(FlushOptions {
                flush_type: FlushType::MemTable,
            }),
        )
        .await
        .expect("timed out flushing base memtable")
        .expect("base memtable flush should succeed");

        // Start a serializable transaction at seq=1 and record a read on
        // conflict-key. The next committed write to that key will conflict
        // with this transaction when it tries to commit.
        let txn = db
            .begin(IsolationLevel::SerializableSnapshot)
            .await
            .unwrap();
        assert_eq!(
            txn.get(b"conflict-key").await.unwrap(),
            Some(Bytes::from_static(b"v1"))
        );

        // Commit and flush seq=2 for the same key. This advances the L0
        // manifest writer through seq=2 and creates the transaction conflict.
        db.put_with_options(b"conflict-key", b"v2", &PutOptions::default(), &write_opts)
            .await
            .unwrap();
        tokio::time::timeout(
            Duration::from_secs(5),
            db.flush_with_options(FlushOptions {
                flush_type: FlushType::MemTable,
            }),
        )
        .await
        .expect("timed out flushing conflict-producing write")
        .expect("conflict-producing write flush should succeed");

        // The transaction commit must fail, but the bug is that write_batch
        // allocates commit_seq=3 before conflict detection and never writes it
        // into a memtable. This leaves a durable-memtable sequence hole.
        txn.put(b"txn-write", b"will-conflict").unwrap();
        let err = txn.commit().await.expect_err("transaction should conflict");
        assert_eq!(err.kind(), crate::ErrorKind::Transaction);

        // Write enough data to freeze an immutable memtable. Its first sequence
        // is 4 because seq=3 was consumed by the failed transaction commit.
        let large_value = vec![b'x'; LARGE_VALUE_BYTES];
        let large_handle = db
            .put_with_options(
                b"large-after-conflict",
                &large_value,
                &PutOptions::default(),
                &write_opts,
            )
            .await
            .expect("large write after conflict should be accepted");
        assert_eq!(
            large_handle.seqnum(),
            4,
            "the conflicted commit should have consumed commit_seq=3"
        );
        tokio::time::timeout(
            Duration::from_secs(5),
            db.flush_with_options(FlushOptions {
                flush_type: FlushType::Wal,
            }),
        )
        .await
        .expect("timed out flushing WAL after large write")
        .expect("WAL flush after large write should succeed");

        // Wait until the large write either reaches L0 or is visible as an
        // immutable memtable. On the buggy path it is uploaded, but cannot be
        // retired because the manifest writer is still waiting for seq=3.
        let large_seq = large_handle.seqnum();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let (last_l0_seq, has_immutable_memtable) = {
                    let guard = db.inner.state.read();
                    (
                        guard.state().core().last_l0_seq,
                        !guard.state().imm_memtable.is_empty(),
                    )
                };
                if last_l0_seq >= large_seq || has_immutable_memtable {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("large write never froze or flushed");

        // This write enters backpressure because the stuck immutable memtable
        // remains charged against max_unflushed_bytes. The expected failure is
        // this timeout, which proves the hang without letting the test run
        // forever.
        tokio::time::timeout(
            Duration::from_secs(5),
            db.put_with_options(
                b"write-after-gap",
                b"v",
                &PutOptions::default(),
                &write_opts,
            ),
        )
        .await
        .expect("timed out waiting for write after conflicted transaction consumed commit_seq=3")
        .expect("write after conflicted commit sequence gap should succeed");

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn should_notify_seq_watcher_on_wal_flush() {
        // Given: a DB with WAL enabled and a seq watcher
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_watch_wal", object_store)
            .build()
            .await
            .unwrap();
        let mut watcher = db.subscribe();

        // When: writing multiple keys and flushing the WAL
        db.put(b"key1", b"value1").await.unwrap();
        db.put(b"key2", b"value2").await.unwrap();
        db.put(b"key3", b"value3").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::Wal,
        })
        .await
        .unwrap();

        // Then: the watcher should report durable_seq >= 3
        let status = tokio::time::timeout(
            Duration::from_secs(10),
            watcher.wait_for(|s| s.durable_seq >= 3),
        )
        .await
        .expect("timed out waiting for seq update")
        .expect("watch channel closed")
        .clone();
        assert!(
            status.durable_seq >= 3,
            "expected durable seq >= 3, got {}",
            status.durable_seq
        );

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn should_subscribe_to_current_manifest_updates_after_flush() {
        // Given: a DB with a watcher and manifest access
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_watch_current_manifest");
        let db = Db::builder(path.clone(), object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();
        let mut watcher = db.subscribe();
        let manifest_store = Arc::new(ManifestStore::new(&path, object_store));
        let mut stored_manifest =
            StoredManifest::load(manifest_store, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();

        assert_eq!(watcher.borrow().current_manifest, db.manifest());

        // When: writes are flushed to an L0 and the manifest is updated
        db.put(b"key1", b"value1").await.unwrap();
        db.put(b"key2", b"value2").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();
        wait_for_manifest_condition(
            &mut stored_manifest,
            |manifest| manifest.last_l0_seq >= 2,
            Duration::from_secs(10),
        )
        .await;

        // Then: subscribe reports the updated manifest and durability frontier
        let status = tokio::time::timeout(
            Duration::from_secs(10),
            watcher.wait_for(|s| {
                s.current_manifest.manifest.core.last_l0_seq >= 2 && s.durable_seq >= 2
            }),
        )
        .await
        .expect("timed out waiting for manifest update")
        .expect("watch channel closed")
        .clone();
        assert!(status.durable_seq >= 2);
        assert_eq!(status.current_manifest.manifest.core.last_l0_seq, 2);

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn should_publish_remote_manifest_updates_via_poll() {
        // Given: a DB with a watcher
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_watch_remote_manifest");
        let db = Db::builder(path.clone(), object_store.clone())
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();
        let mut watcher = db.subscribe();
        let initial_checkpoint_count = watcher
            .borrow()
            .current_manifest
            .manifest
            .core
            .checkpoints
            .len();

        let manifest_store = Arc::new(ManifestStore::new(&path, object_store));
        let mut stored_manifest =
            StoredManifest::load(manifest_store, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();

        // When: another writer updates the manifest
        stored_manifest
            .write_checkpoint(uuid::Uuid::new_v4(), &CheckpointOptions::default())
            .await
            .unwrap();
        wait_for_manifest_condition(
            &mut stored_manifest,
            |manifest| manifest.checkpoints.len() > initial_checkpoint_count,
            Duration::from_secs(10),
        )
        .await;

        // Then: subscribe eventually reports the merged manifest
        let status = tokio::time::timeout(
            Duration::from_secs(10),
            watcher.wait_for(|s| {
                s.current_manifest.manifest.core.checkpoints.len() > initial_checkpoint_count
            }),
        )
        .await
        .expect("timed out waiting for remote manifest update")
        .expect("watch channel closed")
        .clone();
        assert_eq!(
            status.current_manifest.manifest.core.checkpoints.len(),
            initial_checkpoint_count + 1
        );

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn should_close_watcher_on_db_drop() {
        // Given: a DB with a watcher
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_watch_drop", object_store)
            .build()
            .await
            .unwrap();
        let mut watcher = db.subscribe();

        // When: the DB is closed
        db.close().await.unwrap();

        // Then: the watcher should report close_reason = Clean
        let status = watcher
            .wait_for(|s| s.close_reason.is_some())
            .await
            .expect("watch channel closed")
            .clone();
        assert_eq!(
            status.close_reason,
            Some(CloseReason::Clean),
            "expected close_reason = Clean after db close",
        );

        // When: the DB is dropped (drops the watch sender)
        drop(db);

        // Then: the watcher's changed() should return Err (channel closed)
        let result = watcher.changed().await;
        assert!(
            result.is_err(),
            "expected watch channel closed after db drop, got Ok",
        );
    }

    #[tokio::test]
    async fn should_report_close_reason_clean_on_db_close() {
        // Given: a DB with a watcher
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_close_reason_clean", object_store)
            .build()
            .await
            .unwrap();
        let mut watcher = db.subscribe();

        // When: the DB is closed cleanly
        db.close().await.unwrap();

        // Then: the watcher should report close_reason = Clean
        let status = watcher
            .wait_for(|s| s.close_reason.is_some())
            .await
            .expect("watch channel closed")
            .clone();
        assert_eq!(status.close_reason, Some(CloseReason::Clean));
    }

    #[tokio::test]
    async fn should_report_close_reason_panic_on_background_task_failure() {
        // Given: a DB with a failpoint on WAL flush and a watcher
        let fp_registry = Arc::new(FailPointRegistry::new());
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_close_reason_panic", object_store)
            .with_settings(test_db_options(0, 128, None))
            .with_fp_registry(fp_registry.clone())
            .build()
            .await
            .unwrap();
        let mut watcher = db.subscribe();

        // When: a background task panics
        fail_parallel::cfg(fp_registry.clone(), "write-wal-sst-io-error", "panic").unwrap();
        let _ = db.put(b"foo", b"bar").await;

        // Then: the watcher should report close_reason = Panic
        let status = tokio::time::timeout(
            Duration::from_secs(10),
            watcher.wait_for(|s| s.close_reason.is_some()),
        )
        .await
        .expect("timed out waiting for close reason")
        .expect("watch channel closed")
        .clone();
        assert_eq!(status.close_reason, Some(CloseReason::Panic));
    }

    #[tokio::test]
    async fn should_report_close_reason_fenced_on_fenced_error() {
        // Given: a DB with a watcher
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_close_reason_fenced", object_store)
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();
        let mut watcher = db.subscribe();

        // When: the DB is fenced (simulated via closed_result)
        db.inner
            .status_manager
            .write_result(Err(SlateDBError::Fenced));

        // Then: the watcher should report close_reason = Fenced
        let status = tokio::time::timeout(
            Duration::from_secs(10),
            watcher.wait_for(|s| s.close_reason.is_some()),
        )
        .await
        .expect("timed out waiting for close reason")
        .expect("watch channel closed")
        .clone();
        assert_eq!(status.close_reason, Some(CloseReason::Fenced));
    }

    #[cfg(feature = "wal_disable")]
    #[tokio::test]
    async fn should_notify_seq_watcher_on_l0_flush_when_wal_disabled() {
        // Given: a DB with WAL disabled and a seq watcher
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let mut options = test_db_options(0, 256, None);
        options.wal_enabled = false;
        let db = Db::builder("/tmp/test_watch_l0", object_store)
            .with_settings(options)
            .build()
            .await
            .unwrap();
        let mut watcher = db.subscribe();

        // When: writing multiple keys and flushing the memtable to L0
        db.put_with_options(
            b"key1",
            b"value1",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            b"key2",
            b"value2",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Then: the watcher should report durable_seq >= 2
        let status = tokio::time::timeout(
            Duration::from_secs(10),
            watcher.wait_for(|s| s.durable_seq >= 2),
        )
        .await
        .expect("timed out waiting for seq update")
        .expect("watch channel closed")
        .clone();
        assert!(
            status.durable_seq >= 2,
            "expected durable seq >= 2, got {}",
            status.durable_seq
        );

        db.close().await.unwrap();
    }

    #[cfg(feature = "wal_disable")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn reads_succeed_when_compacted_sr_splits_same_key_across_ssts() {
        use crate::SstBlockSize;
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_merge_split_sr_repro";
        let should_compact = Arc::new(AtomicBool::new(false));
        let should_compact2 = should_compact.clone();
        let compaction_scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            move |_state| {
                let result = should_compact2
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                    .unwrap_or(false);
                if result {
                    info!("TRIGGER COMPACT");
                }
                result
            },
        )));

        // Force frequent L0 flushes and tiny compacted SSTs so a single SR ends up with multiple SSTs.
        let mut settings = test_db_options(
            0,
            128,
            Some(CompactorOptions {
                poll_interval: Duration::from_millis(20),
                max_concurrent_compactions: 1,
                manifest_update_timeout: Duration::from_secs(300),
                worker: Some(CompactionWorkerOptions {
                    max_sst_size: 128,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        );
        settings.l0_max_ssts = 10_000;
        settings.l0_max_ssts_per_key = 10_000;
        settings.flush_interval = None;
        settings.wal_enabled = false;

        let compactor_options = settings.compactor_options.take().unwrap();
        let db = Db::builder(path, object_store.clone())
            .with_settings(settings)
            .with_sst_block_size(SstBlockSize::Other(64))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .with_compactor_builder(
                CompactorBuilder::new(path, object_store.clone())
                    .with_scheduler_supplier(compaction_scheduler)
                    .with_options(compactor_options),
            )
            .build()
            .await
            .unwrap();

        // Write a base value and keep a snapshot alive so later versions are retained by compaction.
        db.put_with_options(
            b"k",
            b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let _snapshot = db.snapshot().await.unwrap();

        // Write many merge operands for the same key, each forced into L0.
        for i in 0..16u16 {
            let val = format!("{}{}", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", i + 1);
            db.put_with_options(
                b"k",
                val.as_bytes(),
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }

        // Flush all pending writes to L0 before triggering compaction.
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Compact until we observe a sorted run where a single logical key spans
        // multiple SSTs. Re-arm should_compact each iteration so partial compactions
        // can be followed by additional rounds.
        tokio::time::timeout(Duration::from_secs(60), async {
            loop {
                should_compact.store(true, Ordering::SeqCst);
                {
                    let state = db.inner.state.read();
                    info!(
                        "l0: {:?}",
                        state
                            .state()
                            .core()
                            .tree
                            .l0
                            .iter()
                            .map(|t| t.estimate_size())
                            .collect::<Vec<_>>()
                    );
                    info!(
                        "compacted: {:?}",
                        state
                            .state()
                            .core()
                            .tree
                            .compacted
                            .iter()
                            .map(|t| t.estimate_size())
                            .collect::<Vec<_>>()
                    );
                    if state
                        .state()
                        .core()
                        .tree
                        .compacted
                        .first()
                        .is_some_and(|sr| sr.sst_views().len() > 1)
                    {
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(3000)).await;
            }
        })
        .await
        .expect("timed out waiting for compacted SR where one key spans multiple SSTs");

        let data = db.get(b"k").await.unwrap().unwrap();
        let expected = Bytes::from(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa16".as_ref());
        let data_scan = db
            .scan(b"k".as_slice()..)
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .unwrap();
        // A descending scan visits the SR's SSTs back to front, so the key's
        // versions arrive out of sequence order unless the SR iterator
        // reassembles them across the boundary.
        let desc_options = ScanOptions::default().with_order(IterationOrder::Descending);
        let data_scan_desc = db
            .scan_with_options(b"k".as_slice().., &desc_options)
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .unwrap();
        info!("data: {:?}", data);
        info!("data (scan): {:?}", data_scan.value);
        assert_eq!(data, expected);
        assert_eq!(data_scan.value, expected);
        assert_eq!(data_scan_desc.value, expected);
    }

    #[cfg(feature = "wal_disable")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn reads_succeed_when_compacted_sr_splits_same_merge_key_across_ssts() {
        use crate::SstBlockSize;
        use bytes::{BufMut as _, BytesMut};
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_merge_split_sr_repro";
        let should_compact = Arc::new(AtomicBool::new(false));
        let should_compact2 = should_compact.clone();
        let compaction_scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            move |_state| {
                let result = should_compact2
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                    .unwrap_or(false);
                if result {
                    info!("TRIGGER COMPACT");
                }
                result
            },
        )));

        // Force frequent L0 flushes and tiny compacted SSTs so a single SR ends up with multiple SSTs.
        let mut settings = test_db_options(
            0,
            128,
            Some(CompactorOptions {
                poll_interval: Duration::from_millis(20),
                max_concurrent_compactions: 1,
                manifest_update_timeout: Duration::from_secs(300),
                worker: Some(CompactionWorkerOptions {
                    max_sst_size: 128,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        );
        settings.l0_max_ssts = 10_000;
        settings.l0_max_ssts_per_key = 10_000;
        settings.flush_interval = None;
        settings.wal_enabled = false;

        let compactor_options = settings.compactor_options.take().unwrap();
        let db = Db::builder(path, object_store.clone())
            .with_settings(settings)
            .with_sst_block_size(SstBlockSize::Other(64))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .with_compactor_builder(
                CompactorBuilder::new(path, object_store.clone())
                    .with_scheduler_supplier(compaction_scheduler)
                    .with_options(compactor_options),
            )
            .build()
            .await
            .unwrap();

        // Write a base value and keep a snapshot alive so later versions are retained by compaction.
        let mut expected = BytesMut::new();
        db.put_with_options(
            b"k",
            b"base",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        expected.put(b"base".as_slice());
        let _snapshot = db.snapshot().await.unwrap();

        // Write distinct merge operands so the final value also verifies operand ordering.
        for i in 0..16u8 {
            let operand = vec![b'a' + i; 32];
            expected.put(operand.as_slice());
            db.merge_with_options(
                b"k",
                operand,
                &MergeOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }

        // Flush all pending writes to L0 before triggering compaction.
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Compact until we observe a sorted run where a single logical key spans
        // multiple SSTs. Re-arm should_compact each iteration so partial compactions
        // can be followed by additional rounds.
        tokio::time::timeout(Duration::from_secs(60), async {
            loop {
                should_compact.store(true, Ordering::SeqCst);
                {
                    let state = db.inner.state.read();
                    info!(
                        "l0: {:?}",
                        state
                            .state()
                            .core()
                            .tree
                            .l0
                            .iter()
                            .map(|t| t.estimate_size())
                            .collect::<Vec<_>>()
                    );
                    info!(
                        "compacted: {:?}",
                        state
                            .state()
                            .core()
                            .tree
                            .compacted
                            .iter()
                            .map(|t| t.estimate_size())
                            .collect::<Vec<_>>()
                    );
                    if state
                        .state()
                        .core()
                        .tree
                        .compacted
                        .first()
                        .is_some_and(|sr| sr.sst_views().len() > 1)
                    {
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(3000)).await;
            }
        })
        .await
        .expect("timed out waiting for compacted SR where one key spans multiple SSTs");

        let data = db.get(b"k").await.unwrap().unwrap();
        let expected = expected.freeze();
        let data_scan = db
            .scan(b"k".as_slice()..)
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .unwrap();
        // Same key, scanned the other way: the merge operands must still be
        // applied newest last, which requires the SR iterator to pull the
        // key's earlier SST before emitting anything.
        let desc_options = ScanOptions::default().with_order(IterationOrder::Descending);
        let data_scan_desc = db
            .scan_with_options(b"k".as_slice().., &desc_options)
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .unwrap();
        info!("data: {:?}", data);
        info!("data (scan): {:?}", data_scan.value);
        assert_eq!(data, expected);
        assert_eq!(data_scan.value, expected);
        assert_eq!(data_scan_desc.value, expected);
    }

    #[tokio::test]
    async fn test_get_key_value() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_get_key_value";
        let clock = Arc::new(MockSystemClock::new());
        let db = Db::builder(path, object_store)
            .with_settings(test_db_options(0, 1024, None))
            .with_system_clock(clock.clone())
            .build()
            .await
            .unwrap();

        clock.set(100);
        let key = b"key1";
        let value = b"value1";
        db.put_with_options(
            key,
            value,
            &PutOptions {
                ttl: Ttl::ExpireAfterMillis(50),
            },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let kv = db.get_key_value(key).await.unwrap().unwrap();
        assert_eq!(kv.key, Bytes::from_static(key));
        assert_eq!(kv.value, Bytes::from_static(value));
        assert_eq!(kv.seq, 1);
        assert_eq!(kv.create_ts, 100);
        assert_eq!(kv.expire_ts, Some(150));
    }

    #[tokio::test]
    async fn test_scan_row_entry() {
        use crate::types::ValueDeletable;

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_scan_row_entry";
        let clock = Arc::new(MockSystemClock::new());
        let db = Db::builder(path, object_store)
            .with_settings(test_db_options(0, 1024, None))
            .with_system_clock(clock.clone())
            .build()
            .await
            .unwrap();

        let put_opts = PutOptions {
            ttl: Ttl::ExpireAfterMillis(50),
        };
        let write_opts = WriteOptions {
            ..Default::default()
        };

        clock.set(100);
        db.put_with_options(b"key1", b"value1", &put_opts, &write_opts)
            .await
            .unwrap();

        clock.set(110);
        db.put_with_options(b"key2", b"value2", &put_opts, &write_opts)
            .await
            .unwrap();

        clock.set(120);
        db.put_with_options(b"key3", b"value3", &put_opts, &write_opts)
            .await
            .unwrap();

        let mut iter = db.scan(..).await.unwrap();

        let row_entry1 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(row_entry1.key, Bytes::from_static(b"key1"));
        assert_eq!(
            row_entry1.value,
            ValueDeletable::Value(Bytes::from_static(b"value1"))
        );
        assert_eq!(row_entry1.seq, 1);
        assert_eq!(row_entry1.create_ts, Some(100));
        assert_eq!(row_entry1.expire_ts, Some(150));

        let row_entry2 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(row_entry2.key, Bytes::from_static(b"key2"));
        assert_eq!(
            row_entry2.value,
            ValueDeletable::Value(Bytes::from_static(b"value2"))
        );
        assert_eq!(row_entry2.seq, 2);
        assert_eq!(row_entry2.create_ts, Some(110));
        assert_eq!(row_entry2.expire_ts, Some(160));

        let row_entry3 = iter.next_entry().await.unwrap().unwrap();
        assert_eq!(row_entry3.key, Bytes::from_static(b"key3"));
        assert_eq!(
            row_entry3.value,
            ValueDeletable::Value(Bytes::from_static(b"value3"))
        );
        assert_eq!(row_entry3.seq, 3);
        assert_eq!(row_entry3.create_ts, Some(120));
        assert_eq!(row_entry3.expire_ts, Some(170));

        assert!(iter.next_entry().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_get_key_value_with_expire_at() {
        // given
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_get_key_value_expire_at";
        let clock = Arc::new(MockSystemClock::new());
        let db = Db::builder(path, object_store)
            .with_settings(test_db_options(0, 1024, None))
            .with_system_clock(clock.clone())
            .build()
            .await
            .unwrap();

        // when: write with ExpireAtMillis at different clock times
        clock.set(100);
        db.put_with_options(
            b"key1",
            b"value1",
            &PutOptions {
                ttl: Ttl::ExpireAtMillis(500),
            },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        clock.set(200);
        db.put_with_options(
            b"key2",
            b"value2",
            &PutOptions {
                ttl: Ttl::ExpireAtMillis(500),
            },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        // then: both keys have the same expire_ts regardless of write time
        let kv1 = db.get_key_value(b"key1").await.unwrap().unwrap();
        assert_eq!(kv1.expire_ts, Some(500));
        assert_eq!(kv1.create_ts, 100);

        let kv2 = db.get_key_value(b"key2").await.unwrap().unwrap();
        assert_eq!(kv2.expire_ts, Some(500));
        assert_eq!(kv2.create_ts, 200);
    }

    #[tokio::test]
    async fn test_should_record_scan_request_count() {
        // given:
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder("/tmp/test_should_record_scan_request_count", object_store)
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();
        db.put(b"k1", b"v1").await.unwrap();

        // when:
        let mut iter = db.scan(..).await.unwrap();
        let _ = iter.next().await;

        // then:
        assert_eq!(
            lookup_metric_with_labels(
                &metrics_recorder,
                crate::db_stats::REQUEST_COUNT,
                &[("op", "scan")]
            ),
            Some(1)
        );
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_should_record_flush_request_count() {
        // given:
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder("/tmp/test_should_record_flush_request_count", object_store)
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();
        db.put(b"k1", b"v1").await.unwrap();

        // when:
        db.flush().await.unwrap();

        // then:
        assert_eq!(
            lookup_metric_with_labels(
                &metrics_recorder,
                crate::db_stats::REQUEST_COUNT,
                &[("op", "flush")]
            ),
            Some(1)
        );
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_should_record_write_ops_and_batch_count() {
        // given:
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder(
            "/tmp/test_should_record_write_ops_and_batch_count",
            object_store,
        )
        .with_metrics_recorder(metrics_recorder.clone())
        .build()
        .await
        .unwrap();

        // when:
        db.put(b"k1", b"v1").await.unwrap();
        db.put(b"k2", b"v2").await.unwrap();

        // then:
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::WRITE_OPS),
            Some(2)
        );
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::WRITE_BATCH_COUNT),
            Some(2)
        );
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_should_record_merge_operator_operands_on_flush_path_during_batch_write() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let path =
            "/tmp/test_should_record_merge_operator_operands_on_flush_path_during_batch_write";
        let mut options = test_db_options(0, 1024, None);
        options.flush_interval = None;
        options.max_unflushed_bytes = 1024 * 1024;
        let db = Db::builder(path, object_store.clone())
            .with_settings(options)
            .with_metrics_recorder(metrics_recorder.clone())
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        let mut batch = WriteBatch::new();
        batch.merge(b"key1", b"a");
        batch.merge(b"key1", b"b");
        db.write_with_options(
            batch,
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(
            lookup_merge_operator_operands(&metrics_recorder, MERGE_OPERATOR_READ_PATH),
            Some(0)
        );
        assert_eq!(
            lookup_merge_operator_operands(&metrics_recorder, MERGE_OPERATOR_FLUSH_PATH),
            Some(3)
        );
        assert!(
            lookup_merge_operator_operands(&metrics_recorder, MERGE_OPERATOR_COMPACT_PATH)
                .is_none_or(|value| value == 0)
        );

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn should_reject_batch_local_merges_across_differing_ttls() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder(
            "/tmp/test_reject_batch_local_merges_across_differing_ttls",
            object_store,
        )
        .with_settings(test_db_options(0, 1024, None))
        .with_merge_operator(Arc::new(StringConcatMergeOperator))
        .build()
        .await
        .unwrap();

        let mut batch = WriteBatch::new();
        batch.merge_with_options(
            b"key1",
            b"a",
            &MergeOptions {
                ttl: Ttl::ExpireAfterMillis(3600),
            },
        );
        batch.merge_with_options(
            b"key1",
            b"b",
            &MergeOptions {
                ttl: Ttl::ExpireAfterMillis(7200),
            },
        );

        let err = db.write(batch).await.unwrap_err();
        assert_eq!(err.kind(), crate::ErrorKind::Invalid);
        assert!(
            err.to_string()
                .contains("only one merge TTL per-key allowed"),
            "unexpected error: {err}"
        );
        assert_eq!(db.get(b"key1").await.unwrap(), None);

        db.close().await.unwrap();
    }

    #[cfg(feature = "wal_disable")]
    #[tokio::test]
    async fn test_should_record_total_mem_size_bytes_with_wal_disabled() {
        // given: WAL disabled, so writes land only in the active memtable.
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let mut opts = test_db_options(0, 1024, None);
        opts.flush_interval = None;
        opts.max_unflushed_bytes = 1024 * 1024;
        opts.wal_enabled = false;
        let db = Db::builder(
            "/tmp/test_should_record_total_mem_size_bytes_with_wal_disabled",
            object_store,
        )
        .with_settings(opts)
        .with_metrics_recorder(metrics_recorder.clone())
        .build()
        .await
        .unwrap();

        // when: two writes (the second triggers maybe_apply_backpressure for the first's bytes)
        let write_opts = WriteOptions {
            ..Default::default()
        };
        db.put_with_options(b"k1", b"v1", &PutOptions::default(), &write_opts)
            .await
            .unwrap();
        db.put_with_options(b"k2", b"v2", &PutOptions::default(), &write_opts)
            .await
            .unwrap();

        // then: total_mem_size_bytes reflects the active memtable even with the WAL off
        let mem_size = lookup_metric(&metrics_recorder, crate::db_stats::TOTAL_MEM_SIZE_BYTES);
        assert!(
            mem_size.is_some_and(|v| v > 0),
            "expected total_mem_size_bytes > 0 with WAL disabled, got {:?}",
            mem_size
        );
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_should_record_total_mem_size_bytes() {
        // given:
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let mut opts = test_db_options(0, 1024, None);
        opts.flush_interval = None;
        opts.max_unflushed_bytes = 1024 * 1024;
        let db = Db::builder("/tmp/test_should_record_total_mem_size_bytes", object_store)
            .with_settings(opts)
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();

        // when: write without awaiting durability so data stays in WAL buffer
        db.put_with_options(
            b"k1",
            b"v1",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        // Second write so maybe_apply_backpressure sees the first write's bytes
        db.put_with_options(
            b"k2",
            b"v2",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        // then: total_mem_size_bytes is updated via maybe_apply_backpressure
        let mem_size = lookup_metric(&metrics_recorder, crate::db_stats::TOTAL_MEM_SIZE_BYTES);
        assert!(
            mem_size.is_some_and(|v| v > 0),
            "expected total_mem_size_bytes > 0, got {:?}",
            mem_size
        );
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_should_record_wal_buffer_estimated_bytes() {
        // given:
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let mut opts = test_db_options(0, 1024, None);
        opts.flush_interval = None;
        let db = Db::builder(
            "/tmp/test_should_record_wal_buffer_estimated_bytes",
            object_store,
        )
        .with_settings(opts)
        .with_metrics_recorder(metrics_recorder.clone())
        .build()
        .await
        .unwrap();

        // when:
        db.put_with_options(
            b"k1",
            b"v1",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        // then:
        let estimated = lookup_metric(
            &metrics_recorder,
            crate::wal_buffer_stats::WAL_BUFFER_ESTIMATED_BYTES,
        );
        assert!(
            estimated.is_some_and(|v| v > 0),
            "expected wal_buffer_estimated_bytes > 0, got {:?}",
            estimated
        );
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_should_record_manifest_structural_counts() {
        // given:
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder(
            "/tmp/test_should_record_manifest_structural_counts",
            object_store,
        )
        .with_settings(test_db_options(0, 1024, None))
        .with_metrics_recorder(metrics_recorder.clone())
        .build()
        .await
        .unwrap();

        // when: write data and flush memtable to L0
        db.put(b"k1", b"v1").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Wait for manifest poll to update l0_sst_count (poll interval is 100ms)
        tokio::time::sleep(Duration::from_millis(500)).await;

        // then:
        let l0_count = lookup_metric(&metrics_recorder, crate::db_stats::L0_SST_COUNT);
        assert!(
            l0_count.is_some_and(|v| v > 0),
            "expected l0_sst_count > 0, got {:?}",
            l0_count
        );
        // No segment extractor configured → root is the only tree, so the
        // per-tree max equals the total. Both gauges are updated at the
        // same call site in `merge_remote_manifest`.
        let segment_max =
            lookup_metric(&metrics_recorder, crate::db_stats::SEGMENT_MAX_L0_SST_COUNT);
        assert_eq!(
            segment_max, l0_count,
            "expected segment_max_l0_sst_count == l0_sst_count for an unsegmented DB"
        );

        // The single flushed L0 SST shows up as one SST view, with no sorted
        // runs and no external DBs. These gauges are set at the same call site.
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::SST_VIEW_COUNT),
            Some(1),
            "expected sst_view_count == 1 for a single flushed L0 SST"
        );
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::SST_COUNT),
            Some(1),
            "expected sst_count == 1 (one distinct physical SST) for a single flushed L0 SST"
        );
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::SORTED_RUN_COUNT),
            Some(0),
            "expected sorted_run_count == 0 before any compaction"
        );
        assert_eq!(
            lookup_metric(&metrics_recorder, crate::db_stats::EXTERNAL_DB_COUNT),
            Some(0),
            "expected external_db_count == 0 for a standalone DB"
        );
        db.close().await.unwrap();
    }

    struct GaugeBlockControl {
        target: &'static str,
        armed: AtomicBool,
        tripped: AtomicBool,
        release: AtomicBool,
        entered_tx: tokio::sync::mpsc::UnboundedSender<()>,
    }

    struct BlockableGauge {
        name: String,
        control: Arc<GaugeBlockControl>,
    }

    impl GaugeFn for BlockableGauge {
        fn set(&self, _value: i64) {
            if self.name != self.control.target {
                return;
            }
            if !self.control.armed.load(Ordering::SeqCst) {
                return;
            }
            if self.control.tripped.swap(true, Ordering::SeqCst) {
                return;
            }
            let _ = self.control.entered_tx.send(());
            while !self.control.release.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }

    struct BlockingGaugeRecorder {
        control: Arc<GaugeBlockControl>,
    }

    impl MetricsRecorder for BlockingGaugeRecorder {
        fn register_counter(
            &self,
            name: &str,
            description: &str,
            labels: &[(&str, &str)],
        ) -> Arc<dyn CounterFn> {
            NoopMetricsRecorder.register_counter(name, description, labels)
        }

        fn register_gauge(
            &self,
            name: &str,
            _description: &str,
            _labels: &[(&str, &str)],
        ) -> Arc<dyn GaugeFn> {
            Arc::new(BlockableGauge {
                name: name.to_string(),
                control: self.control.clone(),
            })
        }

        fn register_up_down_counter(
            &self,
            name: &str,
            description: &str,
            labels: &[(&str, &str)],
        ) -> Arc<dyn UpDownCounterFn> {
            NoopMetricsRecorder.register_up_down_counter(name, description, labels)
        }

        fn register_histogram(
            &self,
            name: &str,
            description: &str,
            labels: &[(&str, &str)],
            boundaries: &[f64],
        ) -> Arc<dyn HistogramFn> {
            NoopMetricsRecorder.register_histogram(name, description, labels, boundaries)
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_should_not_hold_state_lock_during_metrics_callbacks() {
        // User metrics callbacks must not run under the `db.state` write
        // lock. Block a gauge callback and assert `get()` still completes.
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let (entered_tx, mut entered_rx) = tokio::sync::mpsc::unbounded_channel();
        let control = Arc::new(GaugeBlockControl {
            target: crate::db_stats::L0_SST_COUNT,
            armed: AtomicBool::new(false),
            tripped: AtomicBool::new(false),
            release: AtomicBool::new(false),
            entered_tx,
        });
        let recorder = Arc::new(BlockingGaugeRecorder {
            control: control.clone(),
        });
        let db = Db::builder(
            "/tmp/test_should_not_hold_state_lock_during_metrics_callbacks",
            object_store,
        )
        .with_settings(test_db_options(0, 1024, None))
        .with_metrics_recorder(recorder)
        .build()
        .await
        .unwrap();

        control.armed.store(true, Ordering::SeqCst);
        tokio::time::timeout(Duration::from_secs(10), entered_rx.recv())
            .await
            .expect("manifest stats gauge was never set")
            .unwrap();

        let read_task = tokio::spawn({
            let db = db.clone();
            async move { db.get(b"key").await }
        });
        let read_result = tokio::time::timeout(Duration::from_secs(5), read_task).await;
        control.release.store(true, Ordering::SeqCst);
        let value = read_result
            .expect("db.get() deadlocked while a metrics callback was in flight")
            .unwrap()
            .unwrap();
        assert_eq!(value, None);

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_should_record_segment_max_l0_sst_count_with_extractor() {
        // With a segment extractor configured, `l0_sst_count` sums L0 SSTs
        // across every tree (root + each named segment) while
        // `segment_max_l0_sst_count` reports the largest single tree. The
        // two values diverge whenever segments accumulate L0 SSTs unevenly
        // — the case the new gauge exists to catch, since `l0_max_ssts`
        // backpressure is enforced per-tree.
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let extractor = Arc::new(test_utils::FixedThreeBytePrefixExtractor);
        let db = Db::builder(
            "/tmp/test_should_record_segment_max_l0_sst_count_with_extractor",
            object_store,
        )
        .with_settings(test_db_options(0, 1024, None))
        .with_segment_extractor(extractor)
        .with_metrics_recorder(metrics_recorder.clone())
        .build()
        .await
        .unwrap();

        // Flush 1: two prefixes → each segment tree gets one L0 SST.
        db.put(b"aaa-1", b"v").await.unwrap();
        db.put(b"bbb-1", b"v").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Flush 2: only "aaa" → that tree grows to 2 L0 SSTs while "bbb"
        // stays at 1. Final state: total = 3, per-tree max = 2.
        db.put(b"aaa-2", b"v").await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Wait for the manifest poll to refresh both gauges (interval 100ms).
        tokio::time::sleep(Duration::from_millis(500)).await;

        let total = lookup_metric(&metrics_recorder, crate::db_stats::L0_SST_COUNT);
        let segment_max =
            lookup_metric(&metrics_recorder, crate::db_stats::SEGMENT_MAX_L0_SST_COUNT);
        assert_eq!(
            total,
            Some(3),
            "expected l0_sst_count to sum across trees, got {:?}",
            total
        );
        assert_eq!(
            segment_max,
            Some(2),
            "expected segment_max_l0_sst_count to track the largest tree, got {:?}",
            segment_max
        );
        db.close().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wal_replay_l0_boundary_does_not_skip_unflushed_replay_batches() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_wal_replay_l0_boundary_does_not_skip_unflushed_replay_batches";

        // Start with a large L0/WAL size so the source writer only creates WAL
        // files. The data stays out of L0 until recovery replays it.
        let mut source_settings = test_db_options(0, 16 * 1024, None);
        source_settings.flush_interval = None;

        let source = Db::builder(path, object_store.clone())
            .with_settings(source_settings)
            .build()
            .await
            .unwrap();

        let write_opts = WriteOptions {
            ..Default::default()
        };

        // Write two records, each flushed into its own WAL. With the smaller
        // replay settings below, these WALs are replayed into the first
        // memtable, and that memtable is the only one that reaches L0 before
        // the second simulated crash.
        let l0_flushed_records = [
            (b"l0-flushed-replay-batch-1".as_slice(), vec![b'a'; 128]),
            (b"l0-flushed-replay-batch-2".as_slice(), vec![b'b'; 1024]),
        ];
        let mut l0_flushed_seq = 0;
        let mut first_l0_flushed_wal_id = 0;
        for (i, (key, value)) in l0_flushed_records.iter().enumerate() {
            let write = source
                .put_with_options(*key, value, &PutOptions::default(), &write_opts)
                .await
                .unwrap();
            source
                .flush_with_options(FlushOptions {
                    flush_type: FlushType::Wal,
                })
                .await
                .unwrap();
            if i == 0 {
                first_l0_flushed_wal_id = source
                    .inner
                    .wal_observer
                    .status()
                    .unwrap()
                    .last_flushed_wal_id;
            }
            l0_flushed_seq = write.seqnum();
        }
        let l0_flushed_boundary_wal_id = source
            .inner
            .wal_observer
            .status()
            .unwrap()
            .last_flushed_wal_id;
        assert!(l0_flushed_boundary_wal_id > first_l0_flushed_wal_id);

        // Write several smaller records, each flushed into a separate WAL. On
        // replay, these WALs remain after the first replayed memtable's WAL
        // boundary, so they must still be eligible for replay after the next
        // reopen.
        let unflushed_replay_records = [
            (b"unflushed-replay-batch-1".as_slice(), vec![b'c'; 128]),
            (b"unflushed-replay-batch-2".as_slice(), vec![b'd'; 128]),
            (b"unflushed-replay-batch-3".as_slice(), vec![b'e'; 128]),
        ];
        for (key, value) in &unflushed_replay_records {
            source
                .put_with_options(*key, value, &PutOptions::default(), &write_opts)
                .await
                .unwrap();
            source
                .flush_with_options(FlushOptions {
                    flush_type: FlushType::Wal,
                })
                .await
                .unwrap();
        }
        let final_source_wal_id = source
            .inner
            .wal_observer
            .status()
            .unwrap()
            .last_flushed_wal_id;
        assert!(final_source_wal_id >= l0_flushed_boundary_wal_id + 2);

        // Recover with a much smaller replay target so WAL replay splits into
        // multiple replayed memtables. Limit L0 to one table so only the first
        // replayed memtable can be flushed before the next reopen.
        let mut replay_settings = test_db_options(0, 512, None);
        replay_settings.flush_interval = None;
        replay_settings.l0_max_ssts = 1;
        replay_settings.l0_max_ssts_per_key = 1;

        // First recovery: replay the WAL and allow the first replayed memtable
        // to publish to L0.
        let _first_recovery = Db::builder(path, object_store.clone())
            .with_settings(replay_settings.clone())
            .build()
            .await
            .unwrap();

        // Wait until the manifest has durably published that first replayed
        // memtable. The manifest now has one L0 with last_l0_seq and
        // replay_after_wal_id matching the first replayed memtable's actual
        // sequence and WAL boundaries.
        let manifest_store = Arc::new(ManifestStore::new(&Path::from(path), object_store.clone()));
        let mut stored_manifest =
            StoredManifest::load(manifest_store, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let first_l0_manifest = wait_for_manifest_condition(
            &mut stored_manifest,
            |state| !state.tree.l0.is_empty() && state.last_l0_seq >= l0_flushed_seq,
            Duration::from_secs(60),
        )
        .await;
        assert_eq!(first_l0_manifest.last_l0_seq, l0_flushed_seq);
        assert_eq!(
            first_l0_manifest.replay_after_wal_id,
            l0_flushed_boundary_wal_id
        );
        assert!(final_source_wal_id >= first_l0_manifest.replay_after_wal_id + 2);
        assert_eq!(first_l0_manifest.tree.l0.len(), 1);

        // Second recovery: simulate crashing after only the first replayed
        // memtable reached L0. Recovery must resume after that first
        // memtable's actual WAL boundary, not after the later replay batch's
        // boundary.
        let recovered = Db::builder(path, object_store.clone())
            .with_settings(replay_settings)
            .build()
            .await
            .unwrap();

        // The L0-flushed keys are present from L0. The later keys must still
        // come from WAL replay because they were not covered by the published
        // L0 boundary.
        for (key, value) in &l0_flushed_records {
            assert_eq!(
                recovered.get(*key).await.unwrap(),
                Some(Bytes::copy_from_slice(value))
            );
        }
        for (key, value) in &unflushed_replay_records {
            assert_eq!(
                recovered.get(*key).await.unwrap(),
                Some(Bytes::copy_from_slice(value))
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_wal_replay_flushes_oversized_active_memtable_before_backpressure() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_wal_replay_flushes_oversized_active_memtable";

        // Leave a single WAL SST whose replayed table is smaller than the
        // source's freeze threshold. Keeping the source open simulates a crash:
        // the recovery writer fences it without first flushing its memtable to L0.
        let mut source_settings = test_db_options(0, 64 * 1024, None);
        source_settings.flush_interval = None;
        let source = Db::builder(path, object_store.clone())
            .with_settings(source_settings)
            .build()
            .await
            .unwrap();
        let value = vec![b'x'; 16 * 1024];
        source
            .put_with_options(
                b"oversized-replay-value",
                &value,
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        source
            .flush_with_options(FlushOptions {
                flush_type: FlushType::Wal,
            })
            .await
            .unwrap();

        // On recovery, one complete WAL SST exceeds both thresholds. Replay
        // must freeze it before backpressure; otherwise open spins forever with
        // an oversized active memtable and nothing for the flusher to drain.
        let mut replay_settings = test_db_options(0, 1024, None);
        replay_settings.flush_interval = None;
        replay_settings.max_unflushed_bytes = 2 * 1024;
        let recovered = tokio::time::timeout(
            Duration::from_secs(5),
            Db::builder(path, object_store)
                .with_settings(replay_settings)
                .build(),
        )
        .await
        .expect("WAL replay deadlocked on an oversized active memtable")
        .expect("failed to recover database");

        assert_eq!(
            recovered.get(b"oversized-replay-value").await.unwrap(),
            Some(Bytes::from(value))
        );
        recovered
            .put_with_options(
                b"write-after-replay",
                b"value",
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .expect("write after oversized WAL replay should succeed");

        recovered.close().await.unwrap();
    }

    /// RFC-0024: WAL replay through a conforming extractor preserves the
    /// keys and lets segment-aware writes resume after the next open.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wal_replay_with_extractor_preserves_keys() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_wal_replay_with_extractor_preserves_keys";

        let extractor = Arc::new(test_utils::FixedThreeBytePrefixExtractor);
        let mut settings = test_db_options(0, 16 * 1024, None);
        settings.flush_interval = None;

        let source = Db::builder(path, object_store.clone())
            .with_settings(settings.clone())
            .with_segment_extractor(extractor.clone())
            .build()
            .await
            .unwrap();
        let write_opts = WriteOptions {
            ..Default::default()
        };
        for (key, value) in [
            (b"aaa-1".as_slice(), b"v1".as_slice()),
            (b"bbb-1".as_slice(), b"v2".as_slice()),
        ] {
            source
                .put_with_options(key, value, &PutOptions::default(), &write_opts)
                .await
                .unwrap();
        }
        source
            .flush_with_options(FlushOptions {
                flush_type: FlushType::Wal,
            })
            .await
            .unwrap();
        // Drop without close so writes remain in WAL only.
        drop(source);

        let recovered = Db::builder(path, object_store.clone())
            .with_settings(settings)
            .with_segment_extractor(extractor)
            .build()
            .await
            .unwrap();
        assert_eq!(
            recovered.get(b"aaa-1").await.unwrap().unwrap().as_ref(),
            b"v1"
        );
        assert_eq!(
            recovered.get(b"bbb-1").await.unwrap().unwrap().as_ref(),
            b"v2"
        );
        recovered.close().await.unwrap();
    }

    /// RFC-0024: WAL replay rejects entries whose prefix under the
    /// current extractor would be empty. We reach this path via the
    /// silent-swap pattern: the source writes through a conforming
    /// fixed-3 extractor, then a reopen substitutes an aliased
    /// `fixed-3` extractor whose `prefix_len` always returns `Some(0)`.
    /// The open-time name check passes; replay catches the empty
    /// prefix as `EmptySegmentPrefix`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wal_replay_rejects_empty_extractor_prefix() {
        #[derive(Debug)]
        struct AliasedAlwaysEmptyExtractor;
        impl PrefixExtractor for AliasedAlwaysEmptyExtractor {
            fn name(&self) -> &str {
                "fixed-3"
            }
            fn prefix_len(&self, _target: &crate::PrefixTarget) -> Option<usize> {
                Some(0)
            }
        }

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_wal_replay_rejects_empty_extractor_prefix";

        // Source with a conforming extractor — keys get a 3-byte
        // prefix and reach the WAL cleanly.
        let mut source_settings = test_db_options(0, 16 * 1024, None);
        source_settings.flush_interval = None;
        let source = Db::builder(path, object_store.clone())
            .with_settings(source_settings)
            .with_segment_extractor(Arc::new(test_utils::FixedThreeBytePrefixExtractor))
            .build()
            .await
            .unwrap();
        let write_opts = WriteOptions {
            ..Default::default()
        };
        source
            .put_with_options(b"abc-1", b"v1", &PutOptions::default(), &write_opts)
            .await
            .unwrap();
        source
            .flush_with_options(FlushOptions {
                flush_type: FlushType::Wal,
            })
            .await
            .unwrap();
        drop(source);

        // Reopen with the same `name()`, but the swapped extractor's
        // logic returns `Some(0)` for every key — replay must reject.
        let result = Db::builder(path, object_store)
            .with_settings(test_db_options(0, 16 * 1024, None))
            .with_segment_extractor(Arc::new(AliasedAlwaysEmptyExtractor))
            .build()
            .await;
        let err = match result {
            Ok(_) => panic!("expected empty-prefix rejection at replay, got Ok"),
            Err(e) => e,
        };
        assert!(matches!(err.kind(), crate::error::ErrorKind::Invalid));
        assert!(
            err.to_string().contains("empty prefix"),
            "expected empty-prefix error, got: {err}"
        );
    }

    #[tokio::test]
    async fn should_report_new_memtable_segments_in_subscription() {
        // given
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_subscribe_reports_memtable_segments";
        let mut settings = test_db_options(0, 16 * 1024, None);
        settings.flush_interval = None;
        let db = Db::builder(path, object_store.clone())
            .with_settings(settings)
            .with_segment_extractor(Arc::new(test_utils::FixedThreeBytePrefixExtractor))
            .build()
            .await
            .unwrap();
        let mut rx = db.subscribe();
        assert!(rx.borrow_and_update().list_segments().is_empty());
        let write_opts = WriteOptions {
            ..Default::default()
        };

        // when
        db.put_with_options(b"abc-1", b"v1", &PutOptions::default(), &write_opts)
            .await
            .unwrap();

        // then
        rx.wait_for(|s| {
            s.list_segments()
                .iter()
                .any(|seg| seg.prefix.as_ref() == b"abc")
        })
        .await
        .unwrap();

        // when
        db.put_with_options(b"xyz-1", b"v2", &PutOptions::default(), &write_opts)
            .await
            .unwrap();

        // then
        rx.wait_for(|s| {
            s.list_segments()
                .into_iter()
                .map(|seg| seg.prefix)
                .collect::<Vec<_>>()
                == vec![Bytes::from_static(b"abc"), Bytes::from_static(b"xyz")]
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn should_report_segments_in_manifest_after_flush() {
        // given
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_report_segments_in_manifest_after_flush";
        let mut settings = test_db_options(0, 16 * 1024, None);
        settings.flush_interval = None;
        let db = Db::builder(path, object_store.clone())
            .with_settings(settings)
            .with_segment_extractor(Arc::new(test_utils::FixedThreeBytePrefixExtractor))
            .build()
            .await
            .unwrap();
        let mut rx = db.subscribe();
        rx.borrow_and_update();

        // when
        db.put_with_options(
            b"abc-1",
            b"v1",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        // then
        rx.wait_for(|s| {
            s.list_segments()
                .iter()
                .any(|seg| seg.prefix.as_ref() == b"abc")
        })
        .await
        .unwrap();

        // when
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // then
        rx.wait_for(|s| {
            s.list_segments()
                .into_iter()
                .map(|seg| seg.prefix)
                .collect::<Vec<_>>()
                == vec![Bytes::from_static(b"abc")]
        })
        .await
        .unwrap();

        // when
        // the segments are deleted from the manifest (as a full compaction would)
        db.inner
            .state
            .write()
            .modify(|m| m.state.manifest.value.core.segments.clear());
        let manifest = db.inner.state.read().state().manifest.clone();
        db.inner.status_manager.report_manifest(manifest.into());

        // then
        assert!(db.status().list_segments().is_empty());
    }

    #[tokio::test]
    async fn should_not_report_segments_without_extractor() {
        // given
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_no_segments_without_extractor";
        let mut settings = test_db_options(0, 16 * 1024, None);
        settings.flush_interval = None;
        let db = Db::builder(path, object_store.clone())
            .with_settings(settings)
            .build()
            .await
            .unwrap();
        let mut rx = db.subscribe();
        assert!(rx.borrow_and_update().list_segments().is_empty());

        // when
        db.put_with_options(
            b"abc-1",
            b"v1",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        // the flush drains the write path and folds the memtable into the
        // manifest, so both reporting paths have run by the time it returns.
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // then
        assert!(db.status().list_segments().is_empty());
    }

    #[derive(Clone, Copy, Debug)]
    enum ExtractorConfig {
        None,
        Fixed3,
        Other,
    }

    impl ExtractorConfig {
        fn to_extractor(self) -> Option<Arc<dyn PrefixExtractor>> {
            #[derive(Debug)]
            struct OtherExtractor;
            impl PrefixExtractor for OtherExtractor {
                fn name(&self) -> &str {
                    "other"
                }
                fn prefix_len(&self, _target: &crate::PrefixTarget) -> Option<usize> {
                    Some(3)
                }
            }
            match self {
                ExtractorConfig::None => None,
                ExtractorConfig::Fixed3 => {
                    Some(Arc::new(test_utils::FixedThreeBytePrefixExtractor))
                }
                ExtractorConfig::Other => Some(Arc::new(OtherExtractor)),
            }
        }
    }

    /// RFC-0024 open-time reconciliation: the configured extractor on
    /// reopen must agree with what was persisted at creation.
    /// `(persisted, configured) → outcome` for every combination of
    /// `None`, the conforming `Fixed3` extractor, and a differently-named
    /// `Other` extractor.
    #[rstest::rstest]
    #[case::no_extractor_round_trip(ExtractorConfig::None, ExtractorConfig::None, true)]
    #[case::same_extractor_round_trip(ExtractorConfig::Fixed3, ExtractorConfig::Fixed3, true)]
    #[case::name_mismatch(ExtractorConfig::Fixed3, ExtractorConfig::Other, false)]
    #[case::removed(ExtractorConfig::Fixed3, ExtractorConfig::None, false)]
    #[case::added(ExtractorConfig::None, ExtractorConfig::Fixed3, false)]
    #[tokio::test]
    async fn test_open_extractor_reconciliation(
        #[case] initial: ExtractorConfig,
        #[case] reopen: ExtractorConfig,
        #[case] expect_ok: bool,
    ) {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = format!("/tmp/test_open_extractor_reconciliation_{initial:?}_{reopen:?}");

        let mut builder = Db::builder(path.clone(), object_store.clone());
        if let Some(extractor) = initial.to_extractor() {
            builder = builder.with_segment_extractor(extractor);
        }
        builder.build().await.unwrap().close().await.unwrap();

        let mut builder = Db::builder(path, object_store);
        if let Some(extractor) = reopen.to_extractor() {
            builder = builder.with_segment_extractor(extractor);
        }
        match (expect_ok, builder.build().await) {
            (true, Ok(reopened)) => reopened.close().await.unwrap(),
            (true, Err(err)) => panic!("expected reopen to succeed, got {err:?}"),
            (false, Ok(_)) => panic!("expected reopen to fail"),
            (false, Err(err)) => {
                assert!(matches!(err.kind(), crate::error::ErrorKind::Invalid));
            }
        }
    }

    /// RFC-0024 open-time per-segment check: every persisted segment
    /// prefix `p` must satisfy `prefix_len(Prefix(p)) == Some(p.len())`
    /// under the configured extractor. This catches a silent-swap case
    /// the name check misses — same `name()`, but the new logic no
    /// longer treats an existing prefix as a complete segment boundary.
    #[tokio::test]
    async fn test_open_rejects_when_segment_prefix_unrecognized() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_open_rejects_unrecognized_segment_prefix";

        let db = Db::builder(path, object_store.clone())
            .with_segment_extractor(Arc::new(test_utils::FixedThreeBytePrefixExtractor))
            .build()
            .await
            .unwrap();
        // Persist segments "abc" and "ab-": both produce 3-byte
        // prefixes under fixed-3 and route to disjoint segments. The
        // close() call flushes the memtable to L0, which is what
        // creates the persisted segment entries.
        db.put(b"abc-1", b"v1").await.unwrap();
        db.put(b"ab--1", b"v2").await.unwrap();
        db.close().await.unwrap();

        // Reopen with an aliased extractor — same name, different
        // logic. Under the swapped logic, `Prefix("ab-")` returns
        // Some(2) (prefix "ab"), which does not equal `"ab-".len()`.
        // The per-segment check must reject.
        let result = Db::builder(path, object_store)
            .with_segment_extractor(Arc::new(test_utils::AliasedFixed3PrefixExtractor))
            .build()
            .await;
        let err = match result {
            Ok(_) => panic!("expected unrecognized-prefix rejection, got Ok"),
            Err(e) => e,
        };
        assert!(matches!(err.kind(), crate::error::ErrorKind::Invalid));
        assert!(
            err.to_string().contains("not recognized"),
            "expected error to mention recognition, got: {err}"
        );
    }

    /// Helper: collect the segment prefix list from `db`'s in-memory
    /// manifest snapshot.
    fn segment_prefixes(db: &Db) -> Vec<Bytes> {
        let guard = db.inner.state.read();
        let cow = guard.state();
        cow.core()
            .segments
            .iter()
            .map(|s| s.prefix.clone())
            .collect()
    }

    /// RFC-0024: a DB created with both an extractor and a separate
    /// WAL object store writes a V2 manifest, which intentionally
    /// drops `wal_object_store_uri` (commit 52cead43). The reopen
    /// path must skip the WAL-store reconfiguration check when the
    /// persisted URI is absent, so a matching configuration on
    /// reopen still succeeds.
    #[tokio::test]
    async fn test_open_with_extractor_and_wal_store_round_trips() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let wal_object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_open_extractor_with_wal_store";
        let extractor = Arc::new(test_utils::FixedThreeBytePrefixExtractor);

        let db = Db::builder(path, object_store.clone())
            .with_segment_extractor(extractor.clone())
            .with_wal_object_store(wal_object_store.clone())
            .build()
            .await
            .unwrap();
        db.close().await.unwrap();

        let reopened = Db::builder(path, object_store)
            .with_segment_extractor(extractor)
            .with_wal_object_store(wal_object_store)
            .build()
            .await
            .unwrap();
        reopened.close().await.unwrap();
    }

    /// End-to-end: write to multiple segments, flush, close, reopen,
    /// read every key back. The manifest carries one segment per
    /// touched prefix and survives the encode/decode cycle.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_segments_round_trip_through_flush_and_reopen() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_segments_round_trip";
        let extractor = Arc::new(test_utils::FixedThreeBytePrefixExtractor);

        let db = Db::builder(path, object_store.clone())
            .with_segment_extractor(extractor.clone())
            .build()
            .await
            .unwrap();

        let entries: &[(&[u8], &[u8])] = &[
            (b"aaa-1", b"v1"),
            (b"aaa-2", b"v2"),
            (b"bbb-1", b"v3"),
            (b"ccc-1", b"v4"),
            (b"ccc-2", b"v5"),
        ];
        for (k, v) in entries {
            db.put(*k, *v).await.unwrap();
        }
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        let prefixes = segment_prefixes(&db);
        assert_eq!(
            prefixes,
            vec![
                Bytes::from_static(b"aaa"),
                Bytes::from_static(b"bbb"),
                Bytes::from_static(b"ccc"),
            ],
            "expected one segment per touched prefix, in sorted order"
        );
        db.close().await.unwrap();

        let reopened = Db::builder(path, object_store)
            .with_segment_extractor(extractor)
            .build()
            .await
            .unwrap();
        for (k, v) in entries {
            assert_eq!(
                reopened.get(*k).await.unwrap().unwrap().as_ref(),
                *v,
                "round-trip mismatch for key {:?}",
                k
            );
        }
        // Segments survived encode/decode.
        assert_eq!(
            segment_prefixes(&reopened),
            vec![
                Bytes::from_static(b"aaa"),
                Bytes::from_static(b"bbb"),
                Bytes::from_static(b"ccc"),
            ]
        );
        reopened.close().await.unwrap();
    }

    /// One batch covering N segments must reach the manifest atomically:
    /// after the flush, every touched segment has exactly one L0 SST
    /// from this flush — no partial visibility.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mixed_batch_publishes_atomically() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_mixed_batch_atomic";

        let db = Db::builder(path, object_store)
            .with_segment_extractor(Arc::new(test_utils::FixedThreeBytePrefixExtractor))
            .build()
            .await
            .unwrap();
        let mut batch = WriteBatch::new();
        batch.put(b"aaa-1", b"v1");
        batch.put(b"bbb-1", b"v2");
        batch.put(b"ccc-1", b"v3");
        db.write(batch).await.unwrap();

        // Pre-flush: segments are still empty (data is in memtable / WAL).
        assert!(segment_prefixes(&db).is_empty());

        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // Scope the read guard so it is dropped before the `await` below.
        {
            let guard = db.inner.state.read();
            let cow = guard.state();
            let core = cow.core();
            assert_eq!(core.segments.len(), 3);
            for segment in &core.segments {
                assert_eq!(
                    segment.tree.l0.len(),
                    1,
                    "expected exactly one L0 SST in segment {:?}, got {}",
                    segment.prefix,
                    segment.tree.l0.len()
                );
                assert!(
                    segment.tree.compacted.is_empty(),
                    "no compaction expected yet"
                );
            }
        }
        db.close().await.unwrap();
    }

    /// WAL replay reconstructs segment state when the source process
    /// dropped before flushing. After reopen, replayed entries land in
    /// the memtable; a subsequent flush then publishes them as
    /// per-segment L0 SSTs.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_replay_reconstructs_segments_from_wal() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_replay_reconstructs_segments";
        let extractor = Arc::new(test_utils::FixedThreeBytePrefixExtractor);

        let mut settings = test_db_options(0, 16 * 1024, None);
        settings.flush_interval = None;

        let source = Db::builder(path, object_store.clone())
            .with_settings(settings.clone())
            .with_segment_extractor(extractor.clone())
            .build()
            .await
            .unwrap();
        let write_opts = WriteOptions {
            ..Default::default()
        };
        for (k, v) in [(b"aaa-1".as_slice(), b"v1"), (b"bbb-1".as_slice(), b"v2")] {
            source
                .put_with_options(k, v, &PutOptions::default(), &write_opts)
                .await
                .unwrap();
        }
        source
            .flush_with_options(FlushOptions {
                flush_type: FlushType::Wal,
            })
            .await
            .unwrap();
        // Drop without close so entries stay in WAL only.
        drop(source);

        // Reopen — the manifest still has no segments at this point.
        let recovered = Db::builder(path, object_store)
            .with_settings(settings)
            .with_segment_extractor(extractor)
            .build()
            .await
            .unwrap();
        assert!(
            segment_prefixes(&recovered).is_empty(),
            "replay should not stamp segments until the memtable flushes"
        );
        // Reads see the replayed data via the memtable.
        assert_eq!(
            recovered.get(b"aaa-1").await.unwrap().unwrap().as_ref(),
            b"v1"
        );
        assert_eq!(
            recovered.get(b"bbb-1").await.unwrap().unwrap().as_ref(),
            b"v2"
        );

        // Force the memtable through, segments should appear.
        recovered
            .flush_with_options(FlushOptions {
                flush_type: FlushType::MemTable,
            })
            .await
            .unwrap();
        assert_eq!(
            segment_prefixes(&recovered),
            vec![Bytes::from_static(b"aaa"), Bytes::from_static(b"bbb")]
        );
        recovered.close().await.unwrap();
    }

    /// A timeseries-style workload: hot writes to a "current" segment
    /// interleaved with occasional backfill into an "older" one. Both
    /// segments end up correctly populated and reads succeed.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_backfill_alongside_active_segment() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_backfill_active";

        let db = Db::builder(path, object_store)
            .with_segment_extractor(Arc::new(test_utils::FixedThreeBytePrefixExtractor))
            .build()
            .await
            .unwrap();

        // Active segment "cur" sees most writes; segment "old" gets a
        // sprinkle of backfill. Interleaved on purpose so several
        // batches touch both.
        let mut expected: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for i in 0..20u32 {
            let cur_key = format!("cur-{i:03}").into_bytes();
            let cur_val = format!("c{i}").into_bytes();
            db.put(&cur_key, &cur_val).await.unwrap();
            expected.push((cur_key, cur_val));
            if i % 7 == 0 {
                let old_key = format!("old-{i:03}").into_bytes();
                let old_val = format!("o{i}").into_bytes();
                db.put(&old_key, &old_val).await.unwrap();
                expected.push((old_key, old_val));
            }
        }
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        assert_eq!(
            segment_prefixes(&db),
            vec![Bytes::from_static(b"cur"), Bytes::from_static(b"old")]
        );
        for (k, v) in &expected {
            assert_eq!(
                db.get(k).await.unwrap().unwrap().as_ref(),
                v.as_slice(),
                "missing value for {:?}",
                k
            );
        }
        db.close().await.unwrap();
    }

    /// After a clean restart, writes resume into existing segments and
    /// both the prior (in compacted/L0) and new (in fresh L0) entries
    /// remain readable.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_restart_resumes_writes_into_existing_segment() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_restart_resume";
        let extractor = Arc::new(test_utils::FixedThreeBytePrefixExtractor);

        let initial = Db::builder(path, object_store.clone())
            .with_segment_extractor(extractor.clone())
            .build()
            .await
            .unwrap();
        initial.put(b"aaa-1", b"v1").await.unwrap();
        initial.put(b"aaa-2", b"v2").await.unwrap();
        initial.close().await.unwrap();

        let reopened = Db::builder(path, object_store)
            .with_segment_extractor(extractor)
            .build()
            .await
            .unwrap();
        // Prior writes still readable after reopen.
        assert_eq!(
            reopened.get(b"aaa-1").await.unwrap().unwrap().as_ref(),
            b"v1"
        );
        assert_eq!(
            reopened.get(b"aaa-2").await.unwrap().unwrap().as_ref(),
            b"v2"
        );

        // Resume writes into the same segment.
        reopened.put(b"aaa-3", b"v3").await.unwrap();
        reopened.put(b"aaa-4", b"v4").await.unwrap();
        reopened
            .flush_with_options(FlushOptions {
                flush_type: FlushType::MemTable,
            })
            .await
            .unwrap();

        // Still exactly the one "aaa" segment, now with two L0 SSTs.
        let prefixes = segment_prefixes(&reopened);
        assert_eq!(prefixes, vec![Bytes::from_static(b"aaa")]);
        {
            let guard = reopened.inner.state.read();
            let cow = guard.state();
            let segment = &cow.core().segments[0];
            assert_eq!(
                segment.tree.l0.len(),
                2,
                "expected two L0 SSTs after the second flush, got {}",
                segment.tree.l0.len()
            );
        }

        for (k, v) in [
            (b"aaa-1".as_slice(), b"v1".as_slice()),
            (b"aaa-2".as_slice(), b"v2".as_slice()),
            (b"aaa-3".as_slice(), b"v3".as_slice()),
            (b"aaa-4".as_slice(), b"v4".as_slice()),
        ] {
            assert_eq!(
                reopened.get(k).await.unwrap().unwrap().as_ref(),
                v,
                "missing value for {:?}",
                k
            );
        }
        reopened.close().await.unwrap();
    }

    async fn create_segmented_scan_fixture(
        path: &str,
        object_store: Arc<dyn ObjectStore>,
    ) -> (Db, BTreeMap<Bytes, Bytes>) {
        let db = Db::builder(path, object_store)
            .with_segment_extractor(Arc::new(test_utils::FixedThreeBytePrefixExtractor))
            .build()
            .await
            .unwrap();

        let table = BTreeMap::from([
            (Bytes::from_static(b"aaa-001"), Bytes::from_static(b"v1")),
            (Bytes::from_static(b"aaa-003"), Bytes::from_static(b"v2")),
            (Bytes::from_static(b"bbb-001"), Bytes::from_static(b"v3")),
            (Bytes::from_static(b"bbb-002"), Bytes::from_static(b"v4")),
            (Bytes::from_static(b"ddd-001"), Bytes::from_static(b"v5")),
            (Bytes::from_static(b"ddd-004"), Bytes::from_static(b"v6")),
        ]);
        test_utils::seed_database(&db, &table, true).await.unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        (db, table)
    }

    async fn assert_segmented_scan_matrix<R>(reader: &R, table: &BTreeMap<Bytes, Bytes>)
    where
        R: DbReadOps + Sync,
    {
        let mut prefix_iter = reader.scan_prefix(b"bbb", ..).await.unwrap();
        test_utils::assert_ranged_db_scan(
            table,
            Bytes::from_static(b"bbb")..Bytes::from_static(b"bbc"),
            IterationOrder::Ascending,
            &mut prefix_iter,
        )
        .await;

        // Bounded subranges compose with the prefix on every read surface:
        // a start bound that excludes earlier suffixes...
        let mut subrange_iter = reader
            .scan_prefix(b"bbb", b"-002".as_slice()..)
            .await
            .unwrap();
        test_utils::assert_ranged_db_scan(
            table,
            Bytes::from_static(b"bbb-002")..Bytes::from_static(b"bbc"),
            IterationOrder::Ascending,
            &mut subrange_iter,
        )
        .await;

        // ...and an end bound that excludes later suffixes.
        let mut subrange_iter = reader
            .scan_prefix(b"ddd", b"-001".as_slice()..b"-004".as_slice())
            .await
            .unwrap();
        test_utils::assert_ranged_db_scan(
            table,
            Bytes::from_static(b"ddd-001")..Bytes::from_static(b"ddd-004"),
            IterationOrder::Ascending,
            &mut subrange_iter,
        )
        .await;

        let mut asc_iter = reader
            .scan(b"aaa".to_vec()..=b"ddd-999".to_vec())
            .await
            .unwrap();
        test_utils::assert_ranged_db_scan(
            table,
            Bytes::from_static(b"aaa")..=Bytes::from_static(b"ddd-999"),
            IterationOrder::Ascending,
            &mut asc_iter,
        )
        .await;

        let desc_options = ScanOptions::default().with_order(IterationOrder::Descending);
        let mut desc_iter = reader
            .scan_with_options(b"aaa".to_vec()..=b"ddd-999".to_vec(), &desc_options)
            .await
            .unwrap();
        test_utils::assert_ranged_db_scan(
            table,
            Bytes::from_static(b"aaa")..=Bytes::from_static(b"ddd-999"),
            IterationOrder::Descending,
            &mut desc_iter,
        )
        .await;

        let mut gap_iter = reader
            .scan(b"bbc".to_vec()..=b"ddd-002".to_vec())
            .await
            .unwrap();
        test_utils::assert_ranged_db_scan(
            table,
            Bytes::from_static(b"bbc")..=Bytes::from_static(b"ddd-002"),
            IterationOrder::Ascending,
            &mut gap_iter,
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_segmented_scans_on_db() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let (db, table) =
            create_segmented_scan_fixture("/tmp/test_segmented_scans_on_db", object_store).await;

        assert_segmented_scan_matrix(&db, &table).await;
        db.close().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_segmented_scans_on_snapshot() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let (db, table) =
            create_segmented_scan_fixture("/tmp/test_segmented_scans_on_snapshot", object_store)
                .await;

        let snapshot = db.snapshot().await.unwrap();
        assert_segmented_scan_matrix(snapshot.as_ref(), &table).await;
        drop(snapshot);
        db.close().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_segmented_scans_on_db_reader() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = "/tmp/test_segmented_scans_on_db_reader";
        let (db, table) = create_segmented_scan_fixture(path, object_store.clone()).await;
        db.close().await.unwrap();

        let reader = DbReaderBuilder::new(path, object_store)
            .with_segment_extractor(Arc::new(test_utils::FixedThreeBytePrefixExtractor))
            .build()
            .await
            .unwrap();
        assert_segmented_scan_matrix(&reader, &table).await;
    }

    #[tokio::test]
    async fn test_db_reader_cache_scoping() {
        use crate::db_cache::{DbCache, SplitCache};

        // Create two separate databases
        let object_store_a: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let object_store_b: Arc<dyn ObjectStore> = Arc::new(InMemory::new());

        // Write different data to each database
        let db_a = Db::builder("/tmp/test_reader_cache_a", object_store_a.clone())
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();
        db_a.put(b"key1", b"value_from_db_a").await.unwrap();
        db_a.flush().await.unwrap();
        db_a.close().await.unwrap();

        let db_b = Db::builder("/tmp/test_reader_cache_b", object_store_b.clone())
            .with_settings(test_db_options(0, 1024, None))
            .build()
            .await
            .unwrap();
        db_b.put(b"key1", b"value_from_db_b").await.unwrap();
        db_b.flush().await.unwrap();
        db_b.close().await.unwrap();

        // Create a shared cache
        let shared_cache: Arc<dyn DbCache> = Arc::new(SplitCache::new().build());

        // Open both databases as readers with the shared cache
        let reader_a = DbReaderBuilder::new("/tmp/test_reader_cache_a", object_store_a)
            .with_db_cache(shared_cache.clone(), 1)
            .build()
            .await
            .unwrap();

        let reader_b = DbReaderBuilder::new("/tmp/test_reader_cache_b", object_store_b)
            .with_db_cache(shared_cache.clone(), 2)
            .build()
            .await
            .unwrap();

        // Verify each reader returns its own data, not the other's
        let value_a = reader_a.get(b"key1").await.unwrap();
        assert_eq!(value_a, Some(Bytes::from("value_from_db_a")));

        let value_b = reader_b.get(b"key1").await.unwrap();
        assert_eq!(value_b, Some(Bytes::from("value_from_db_b")));

        // Read again to exercise cached paths
        let value_a_cached = reader_a.get(b"key1").await.unwrap();
        assert_eq!(value_a_cached, Some(Bytes::from("value_from_db_a")));

        // Close reader_a; the shared cache must remain usable for reader_b.
        reader_a.close().await.unwrap();

        let value_b_cached = reader_b.get(b"key1").await.unwrap();
        assert_eq!(value_b_cached, Some(Bytes::from("value_from_db_b")));

        reader_b.close().await.unwrap();
    }

    #[cfg(feature = "foyer")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_close_does_not_kill_shared_hybrid_cache() {
        use crate::db_cache::foyer_hybrid::FoyerHybridCache;
        use crate::db_cache::{CachedEntry, CachedKey, DbCache};
        use crate::db_state::SsTableId;
        use crate::format::sst::BlockBuilder;
        use foyer::{
            BlockEngineConfig, DeviceBuilder, FsDeviceBuilder, HybridCacheBuilder,
            PsyncIoEngineConfig,
        };

        fn probe_entry() -> CachedEntry {
            use rand::RngCore;
            let mut rng = rand::rng();
            let mut builder = BlockBuilder::new_latest(1024);
            loop {
                let mut k = vec![0u8; 32];
                rng.fill_bytes(&mut k);
                let mut v = vec![0u8; 128];
                rng.fill_bytes(&mut v);
                if builder.add_value(&k, &v, None, None) {
                    break;
                }
            }
            CachedEntry::with_block(Arc::new(builder.build().unwrap()))
        }

        async fn open_shared_cache(path: &std::path::Path) -> Arc<dyn DbCache> {
            let hybrid = HybridCacheBuilder::new()
                .with_name("shared_hybrid")
                .memory(1024 * 1024)
                .with_weighter(|_, v: &CachedEntry| v.size())
                .storage()
                .with_io_engine_config(PsyncIoEngineConfig::new())
                .with_engine_config(
                    BlockEngineConfig::new(
                        FsDeviceBuilder::new(path)
                            .with_capacity(4 * 1024 * 1024)
                            .build()
                            .unwrap(),
                    )
                    .with_block_size(64 * 1024),
                )
                .build()
                .await
                .unwrap();
            Arc::new(FoyerHybridCache::new_with_cache(hybrid))
        }

        let cache_dir = tempfile::tempdir().unwrap();
        let shared_cache = open_shared_cache(cache_dir.path()).await;

        let object_store_a: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let object_store_b: Arc<dyn ObjectStore> = Arc::new(InMemory::new());

        let db_a = Db::builder("/tmp/test_shared_hybrid_a", object_store_a)
            .with_settings(test_db_options(0, 1024, None))
            .with_db_cache(shared_cache.clone(), 1)
            .build()
            .await
            .unwrap();
        let db_b = Db::builder("/tmp/test_shared_hybrid_b", object_store_b)
            .with_settings(test_db_options(0, 1024, None))
            .with_db_cache(shared_cache.clone(), 2)
            .build()
            .await
            .unwrap();

        db_a.put(b"key1", b"value_from_db_a").await.unwrap();
        db_a.flush().await.unwrap();
        db_b.put(b"key1", b"value_from_db_b").await.unwrap();
        db_b.flush().await.unwrap();

        // Close db_a; the shared cache must remain usable for db_b.
        db_a.close().await.unwrap();

        // db_b reads still succeed (they can fall back to the object store)...
        let value_b = db_b.get(b"key1").await.unwrap();
        assert_eq!(value_b, Some(Bytes::from("value_from_db_b")));

        // ...and entries cached on behalf of db_b after db_a closed should
        // still persist across the caller's own graceful shutdown sequence
        // (close all DBs, then close the cache we own) + cache reopen, exactly
        // like `should_persist_blocks_to_disk_on_close` proves for a cache
        // that nobody else closed. If db_a's close had closed the shared
        // cache, these entries could never reach disk: the disk engine drops
        // post-close writes and the final close below becomes a no-op.
        let mut keys = Vec::new();
        let sst_id = SsTableId::from(ulid::Ulid::new());
        for b in 0u64..64 {
            let k = CachedKey::from((sst_id, b));
            shared_cache.insert(k.clone(), probe_entry()).await;
            keys.push(k);
        }

        db_b.close().await.unwrap();
        // The caller owns the injected cache and closes it once all DBs are
        // closed; this flushes the memory tier to disk.
        shared_cache.close().await.unwrap();
        drop(db_a);
        drop(db_b);
        drop(shared_cache);

        let reopened = open_shared_cache(cache_dir.path()).await;
        let mut found = 0;
        for k in &keys {
            if reopened.get_block(k).await.unwrap().is_some() {
                found += 1;
            }
        }
        assert_eq!(
            found,
            keys.len(),
            "entries cached after another DB closed the shared cache were lost \
             ({}/{} survived cache close + reopen)",
            found,
            keys.len()
        );
    }

    mod object_store_cache {
        use super::*;
        use crate::cached_object_store::stats::{PART_ACCESS_COUNT, PART_HIT_COUNT};
        use crate::cached_object_store::CachedObjectStore;
        use object_store::ObjectStoreExt;

        /// Fixture for the object store cache tests.
        struct ObjectStoreCacheTest {
            db: Db,
            upstream: Arc<dyn ObjectStore>,
            /// The typed handle to the cache passed to the db as its object
            /// store; `None` when built `without_object_store_cache`.
            cache: Option<Arc<CachedObjectStore>>,
            cache_root: std::path::PathBuf,
            db_path: String,
            should_compact: Option<Arc<AtomicBool>>,
        }

        /// Builder for [`ObjectStoreCacheTest`]. Defaults: 1 KiB cache parts, a
        /// 1 KiB L0 size, both write sources uncached, and no compactor.
        struct ObjectStoreCacheTestBuilder {
            db_path: String,
            object_store_cache: bool,
            cache_on_flush: bool,
            cache_on_compaction: bool,
            part_size: usize,
            l0_sst_size_bytes: usize,
            on_demand_compactor: bool,
            custom_compactor_store: bool,
            metrics_recorder: Option<Arc<DefaultMetricsRecorder>>,
        }

        impl ObjectStoreCacheTestBuilder {
            fn new(db_path: &str) -> Self {
                Self {
                    db_path: db_path.to_string(),
                    object_store_cache: true,
                    cache_on_flush: false,
                    cache_on_compaction: false,
                    part_size: 1024,
                    l0_sst_size_bytes: 1024,
                    on_demand_compactor: false,
                    custom_compactor_store: false,
                    metrics_recorder: None,
                }
            }

            /// Leaves the object store cache unconfigured (no root folder).
            fn without_object_store_cache(mut self) -> Self {
                self.object_store_cache = false;
                self
            }

            fn metrics_recorder(mut self, recorder: Arc<DefaultMetricsRecorder>) -> Self {
                self.metrics_recorder = Some(recorder);
                self
            }

            fn cache_on_flush(mut self) -> Self {
                self.cache_on_flush = true;
                self
            }

            fn cache_on_compaction(mut self) -> Self {
                self.cache_on_compaction = true;
                self
            }

            fn part_size(mut self, part_size: usize) -> Self {
                self.part_size = part_size;
                self
            }

            fn l0_sst_size_bytes(mut self, bytes: usize) -> Self {
                self.l0_sst_size_bytes = bytes;
                self
            }

            /// Adds an embedded compactor that compacts once each time
            /// [`ObjectStoreCacheTest::compact_and_wait`] is called.
            fn on_demand_compactor(mut self) -> Self {
                self.on_demand_compactor = true;
                self
            }

            /// Like `on_demand_compactor`, but the compactor holds its own
            /// handle to upstream, so the db builder keeps it off the cached store.
            fn on_demand_compactor_with_custom_store(mut self) -> Self {
                self.on_demand_compactor = true;
                self.custom_compactor_store = true;
                self
            }

            async fn build(self) -> ObjectStoreCacheTest {
                let Self {
                    db_path,
                    object_store_cache,
                    cache_on_flush,
                    cache_on_compaction,
                    part_size,
                    l0_sst_size_bytes,
                    on_demand_compactor,
                    custom_compactor_store,
                    metrics_recorder,
                } = self;

                let upstream: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
                let temp_dir = tempfile::Builder::new()
                    .prefix("objstore_cache_test_")
                    .tempdir()
                    .unwrap();
                let cache_root = temp_dir.keep();

                let opts = test_db_options(0, l0_sst_size_bytes, None);

                // The cache is user-constructed and passed to the db as the
                // object store itself.
                let cache = if object_store_cache {
                    Some(
                        CachedObjectStore::builder(cache_root.clone(), upstream.clone())
                            .with_part_size_bytes(part_size)
                            .with_cache_on_flush(cache_on_flush)
                            .with_cache_on_compaction(cache_on_compaction)
                            .build()
                            .await
                            .unwrap(),
                    )
                } else {
                    None
                };
                let main_store: Arc<dyn ObjectStore> = match &cache {
                    Some(cache) => cache.clone(),
                    None => upstream.clone(),
                };

                let mut builder =
                    Db::builder(db_path.as_str(), main_store.clone()).with_settings(opts);
                if let Some(recorder) = metrics_recorder {
                    builder = builder.with_metrics_recorder(recorder);
                }
                let should_compact = if on_demand_compactor {
                    let flag = Arc::new(AtomicBool::new(false));
                    let flag_clone = flag.clone();
                    let scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
                        move |_state| flag_clone.swap(false, Ordering::SeqCst),
                    )));
                    // A custom compactor store bypasses the cache entirely;
                    // otherwise the compactor shares the db's (possibly
                    // cached) store. Open gates make GatedObjectStore a
                    // pass-through.
                    let compactor_store: Arc<dyn ObjectStore> = if custom_compactor_store {
                        Arc::new(GatedObjectStore::new(upstream.clone()))
                    } else {
                        main_store.clone()
                    };
                    // One subcompaction writes one output SST, keeping exact
                    // part counts deterministic.
                    let mut compactor_options = fast_compactor_options();
                    if let Some(worker) = compactor_options.worker.as_mut() {
                        worker.max_subcompactions = 1;
                    }
                    builder = builder.with_compactor_builder(
                        CompactorBuilder::new(db_path.as_str(), compactor_store)
                            .with_scheduler_supplier(scheduler)
                            .with_options(compactor_options),
                    );
                    Some(flag)
                } else {
                    None
                };
                let db = builder.build().await.unwrap();

                ObjectStoreCacheTest {
                    db,
                    upstream,
                    cache,
                    cache_root,
                    db_path,
                    should_compact,
                }
            }
        }

        impl ObjectStoreCacheTest {
            fn builder(db_path: &str) -> ObjectStoreCacheTestBuilder {
                ObjectStoreCacheTestBuilder::new(db_path)
            }

            fn db(&self) -> &Db {
                &self.db
            }

            /// A path under the db root, e.g. `sub_path("wal/00..002.sst")`.
            fn sub_path(&self, suffix: &str) -> Path {
                Path::from(format!("{}/{}", self.db_path, suffix))
            }

            /// Number of cached part files for an object.
            fn cached_part_count(&self, path: &Path) -> usize {
                let dir = self.cache_root.join(path.to_string());
                let Ok(entries) = std::fs::read_dir(dir) else {
                    return 0;
                };
                entries
                    .filter(|e| {
                        e.as_ref()
                            .unwrap()
                            .file_name()
                            .to_string_lossy()
                            .starts_with("_part")
                    })
                    .count()
            }

            fn assert_cached(&self, path: &Path, expected_parts: usize) {
                assert_eq!(
                    self.cached_part_count(path),
                    expected_parts,
                    "expected {path} to be cached as {expected_parts} part(s)"
                );
            }

            /// Asserts each of `suffixes` (relative to the db root) is uncached.
            fn assert_uncached(&self, suffixes: &[&str]) {
                for suffix in suffixes {
                    let path = self.sub_path(suffix);
                    assert_eq!(
                        self.cached_part_count(&path),
                        0,
                        "expected {suffix} to be uncached"
                    );
                }
            }

            /// Lists the compacted SSTs currently in the object store.
            async fn compacted_locations(&self) -> Vec<Path> {
                let prefix = self.sub_path("compacted");
                self.upstream
                    .list(Some(&prefix))
                    .map(|meta| meta.unwrap().location)
                    .collect()
                    .await
            }

            /// The size of an object as stored upstream, in bytes.
            async fn object_size(&self, path: &Path) -> u64 {
                self.upstream.head(path).await.unwrap().size
            }

            /// The upstream path of a compacted SST id.
            fn compacted_sst_path(&self, id: &SsTableId) -> Path {
                PathResolver::from_root(self.db_path.as_str()).sst_path(id)
            }

            fn l0_ids(&self) -> Vec<SsTableId> {
                self.db.manifest().l0().iter().map(|v| v.sst.id).collect()
            }

            /// Triggers one on-demand compaction and waits for a sorted run to
            /// land in the manifest. Requires `on_demand_compactor`.
            async fn compact_and_wait(&self) {
                self.should_compact
                    .as_ref()
                    .expect("fixture built without on_demand_compactor")
                    .store(true, Ordering::SeqCst);
                tokio::time::timeout(Duration::from_secs(30), async {
                    loop {
                        if !self.db.manifest().compacted().is_empty() {
                            return;
                        }
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                })
                .await
                .expect("compaction did not land within timeout");
            }

            /// The SSTs the compaction wrote: sorted run members that were not
            /// among the flushed L0s.
            fn compaction_output_ids(&self, l0_ids: &[SsTableId]) -> Vec<SsTableId> {
                self.db
                    .manifest()
                    .compacted()
                    .iter()
                    .flat_map(|sr| sr.sst_views().iter())
                    .map(|v| v.sst.id)
                    .filter(|id| !l0_ids.contains(id))
                    .collect()
            }

            async fn close(self) {
                self.db.close().await.unwrap();
            }
        }

        #[tokio::test]
        async fn test_get_with_object_store_cache_metrics() {
            let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
            let mut opts = test_db_options(0, 1024, None);
            let temp_dir = tempfile::Builder::new()
                .prefix("objstore_cache_test_")
                .tempdir()
                .unwrap();

            opts.object_store_cache_options.root_folder = Some(temp_dir.keep());
            opts.object_store_cache_options.part_size_bytes = 1024;
            let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
            let kv_store = Db::builder(
                "/tmp/test_kv_store_with_cache_metrics",
                object_store.clone(),
            )
            .with_settings(opts)
            .with_db_cache_disabled()
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();

            let access_count0 = lookup_metric(&metrics_recorder, PART_ACCESS_COUNT).unwrap();
            let key = b"test_key";
            let value = b"test_value";
            kv_store.put(key, value).await.unwrap();
            kv_store
                .flush_with_options(FlushOptions {
                    flush_type: FlushType::MemTable,
                })
                .await
                .unwrap();

            // First (cold) get. cache_on_flush is off, so the SST is not cached on the
            // write. The whole SST is a single cache part, read as three sub-ranges
            // (index, filter and block). The first is a cold read that fetches and
            // caches the part (a miss) and the next two are served from the cache
            // (hits). So three accesses, two hits.
            let val = kv_store.get(key).await.unwrap();
            assert_eq!(val, Some(Bytes::from_static(value)));
            let access_count1 = lookup_metric(&metrics_recorder, PART_ACCESS_COUNT).unwrap();
            let hit_count1 = lookup_metric(&metrics_recorder, PART_HIT_COUNT).unwrap();
            assert_eq!(
                access_count1 - access_count0,
                3,
                "one point get reads the single-part SST in three sub-ranges"
            );
            assert_eq!(
                hit_count1, 2,
                "the cold read is a miss; the next two reads hit the warm cache"
            );

            // Second (warm) get: the object is fully cached, so all three reads hit.
            let got = kv_store.get(key).await.unwrap();
            assert_eq!(got, Some(Bytes::from_static(value)));
            let access_count2 = lookup_metric(&metrics_recorder, PART_ACCESS_COUNT).unwrap();
            let hit_count2 = lookup_metric(&metrics_recorder, PART_HIT_COUNT).unwrap();
            assert_eq!(
                access_count2 - access_count1,
                3,
                "the second get reads the same three sub-ranges"
            );
            assert_eq!(
                hit_count2 - hit_count1,
                3,
                "every read in the warm get is a cache hit"
            );
        }

        #[tokio::test]
        async fn test_db_records_read_calls_into_cached_object_store() {
            let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
            let mut opts = test_db_options(0, 1024, None);
            let temp_dir = tempfile::Builder::new()
                .prefix("objstore_metrics_test_")
                .tempdir()
                .unwrap();

            opts.manifest_poll_interval = Duration::from_secs(3600);
            let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
            let path = "/tmp/test_db_records_read_calls_into_cached_object_store";
            let cached_store = CachedObjectStore::builder(temp_dir.keep(), object_store)
                .with_part_size_bytes(1024)
                .build()
                .await
                .unwrap();
            // Disable the in-memory block cache so reads reach the object store
            // cache layer (the subject of this test) instead of being served from
            // decoded blocks in memory.
            let kv_store = Db::builder(path, cached_store)
                .with_settings(opts)
                .with_db_cache_disabled()
                .with_metrics_recorder(metrics_recorder.clone())
                .build()
                .await
                .unwrap();

            kv_store.put(b"test_key", b"test_value").await.unwrap();
            kv_store.flush().await.unwrap();
            kv_store
                .flush_with_options(FlushOptions {
                    flush_type: FlushType::MemTable,
                })
                .await
                .unwrap();

            let requests_before = lookup_object_store_api_request_count(
                &metrics_recorder,
                "db",
                "main",
                "get",
                "get_range",
            );
            let histograms_before = lookup_object_store_api_histogram_count(
                &metrics_recorder,
                "db",
                "main",
                "get",
                "get_range",
            );
            let _val = kv_store.get(b"test_key").await.unwrap();
            let requests_after_first = lookup_object_store_api_request_count(
                &metrics_recorder,
                "db",
                "main",
                "get",
                "get_range",
            );
            let got = kv_store.get(b"test_key").await.unwrap();
            let requests_after_second = lookup_object_store_api_request_count(
                &metrics_recorder,
                "db",
                "main",
                "get",
                "get_range",
            );
            let histograms_after_second = lookup_object_store_api_histogram_count(
                &metrics_recorder,
                "db",
                "main",
                "get",
                "get_range",
            );

            // The instrumented store sits above the object store cache and counts
            // logical read calls whether they are served from the cache or the
            // remote store. Restrict the assertion to range reads so background
            // manifest polling, which uses plain get calls, cannot affect it.
            // A point get reads the single-part SST in three sub-ranges (index,
            // filter and block).
            assert_eq!(requests_after_first, requests_before + 3);
            assert_eq!(got, Some(Bytes::from_static(b"test_value")));
            assert_eq!(requests_after_second, requests_after_first + 3);
            assert_eq!(
                histograms_after_second - histograms_before,
                (requests_after_second - requests_before) as u64
            );
            kv_store.close().await.unwrap();
        }

        /// Warming a disk cache by enumerating SSTs from the manifest,
        /// resolving their paths with `PathResolver`, and loading their raw
        /// bytes with `load_files_to_cache`.
        #[tokio::test]
        async fn test_preload_disk_cache_from_manifest() {
            let fixture =
                ObjectStoreCacheTest::builder("/tmp/test_preload_disk_cache_from_manifest")
                    .build()
                    .await;

            // Two flushed L0 SSTs, not admitted on write.
            for (key, value) in [(b"k1", b"v1"), (b"k2", b"v2")] {
                fixture.db().put(key, value).await.unwrap();
                fixture.db().flush().await.unwrap();
                fixture
                    .db()
                    .flush_with_options(FlushOptions {
                        flush_type: FlushType::MemTable,
                    })
                    .await
                    .unwrap();
            }

            let ids = fixture.l0_ids();
            assert_eq!(ids.len(), 2);
            let paths: Vec<_> = ids
                .iter()
                .map(|id| fixture.compacted_sst_path(id))
                .collect();
            for path in &paths {
                fixture.assert_cached(path, 0);
            }

            let cache = fixture.cache.as_ref().unwrap();
            cache
                .load_files_to_cache(paths.clone(), usize::MAX)
                .await
                .unwrap();

            // Each small SST fits in a single 1 KiB part.
            for path in &paths {
                fixture.assert_cached(path, 1);
            }
            fixture.close().await;
        }

        /// A flushed L0 SST is a compacted SST written by the main store, so
        /// cache_on_flush admits it. The manifest (untagged) and the WAL
        /// (skipped by policy) are never cached.
        #[tokio::test]
        async fn test_object_store_cache_caches_flushed_sst_only() {
            let fixture = ObjectStoreCacheTest::builder("/tmp/test_object_store_cache_flush_only")
                .cache_on_flush()
                .build()
                .await;

            // A foreground write plus a memtable flush produces a WAL entry, a
            // manifest, and an L0 SST.
            fixture.db().put(b"test_key", b"test_value").await.unwrap();
            fixture.db().flush().await.unwrap();
            fixture
                .db()
                .flush_with_options(FlushOptions {
                    flush_type: FlushType::MemTable,
                })
                .await
                .unwrap();

            fixture.assert_uncached(&[
                "manifest/00000000000000000001.manifest",
                "manifest/00000000000000000002.manifest",
                "wal/00000000000000000001.sst",
                "wal/00000000000000000002.sst",
            ]);

            // The single explicit memtable flush produces one L0 SST, cached as one
            // part (the key/value is well under the 1 KiB part size).
            let compacted = fixture.compacted_locations().await;
            assert_eq!(compacted.len(), 1, "expected exactly one flushed SST");
            fixture.assert_cached(&compacted[0], 1);
            fixture.close().await;
        }

        /// A flushed SST above the 10 MiB multipart threshold is written with a
        /// multipart upload, whose parts are now mirrored into the cache, so the
        /// whole SST is cached (one part per part_size chunk).
        #[tokio::test]
        async fn test_object_store_cache_caches_large_multipart_flush() {
            const MIB: usize = 1024 * 1024;

            let fixture = ObjectStoreCacheTest::builder("/tmp/test_object_store_cache_large_flush")
                .cache_on_flush()
                .part_size(MIB)
                // Large enough that the whole write flushes as a single L0 SST.
                .l0_sst_size_bytes(64 * MIB)
                .build()
                .await;

            // ~20 MiB in one memtable, flushed as a single L0 SST above the 10 MiB
            // multipart threshold.
            for i in 0..20u32 {
                let key = format!("k{:04}", i);
                fixture
                    .db()
                    .put(key.as_bytes(), &vec![i as u8; MIB])
                    .await
                    .unwrap();
            }
            fixture
                .db()
                .flush_with_options(FlushOptions {
                    flush_type: FlushType::MemTable,
                })
                .await
                .unwrap();

            let compacted = fixture.compacted_locations().await;
            assert_eq!(compacted.len(), 1, "expected exactly one flushed SST");
            // The SST is written with a multipart upload (each 1 MiB part teed into
            // the cache), so every part of the SST is cached.
            let expected_parts = (fixture.object_size(&compacted[0]).await as usize).div_ceil(MIB);
            assert!(
                expected_parts > 10,
                "expected a large multipart SST, got {expected_parts} part(s)"
            );
            fixture.assert_cached(&compacted[0], expected_parts);
            fixture.close().await;
        }

        /// cache_on_compaction admits the embedded compactor's output; with
        /// cache_on_flush off, the flushed L0 inputs stay uncached.
        #[tokio::test]
        async fn test_object_store_cache_caches_compaction_output() {
            let t = ObjectStoreCacheTest::builder("/tmp/test_object_store_cache_compaction_output")
                .cache_on_compaction()
                .on_demand_compactor()
                .build()
                .await;

            for i in 0..2u32 {
                let key = format!("key{:04}", i);
                t.db().put(key.as_bytes(), &[b'v'; 64]).await.unwrap();
                t.db()
                    .flush_with_options(FlushOptions {
                        flush_type: FlushType::MemTable,
                    })
                    .await
                    .unwrap();
            }
            let l0_ids = t.l0_ids();
            assert_eq!(l0_ids.len(), 2);

            t.compact_and_wait().await;

            for id in &l0_ids {
                t.assert_cached(&t.compacted_sst_path(id), 0);
            }

            let output_ids = t.compaction_output_ids(&l0_ids);
            assert!(!output_ids.is_empty(), "expected compaction output SSTs");
            for id in &output_ids {
                let path = t.compacted_sst_path(id);
                assert!(
                    t.cached_part_count(&path) > 0,
                    "expected compaction output {path} to be cached"
                );
            }
            t.close().await;
        }

        /// Compaction output above the multipart threshold is cached in full.
        #[tokio::test]
        async fn test_object_store_cache_caches_large_multipart_compaction_output() {
            const MIB: usize = 1024 * 1024;

            let t = ObjectStoreCacheTest::builder(
                "/tmp/test_object_store_cache_large_compaction_output",
            )
            .cache_on_compaction()
            .on_demand_compactor()
            .part_size(MIB)
            // Large enough that each write batch flushes as a single L0 SST.
            .l0_sst_size_bytes(64 * MIB)
            .build()
            .await;

            // Two ~10 MiB L0s; the ~20 MiB output crosses the multipart threshold.
            for sst in 0..2u32 {
                for i in 0..10u32 {
                    let key = format!("k{:04}", sst * 10 + i);
                    t.db()
                        .put(key.as_bytes(), &vec![i as u8; MIB])
                        .await
                        .unwrap();
                }
                t.db()
                    .flush_with_options(FlushOptions {
                        flush_type: FlushType::MemTable,
                    })
                    .await
                    .unwrap();
            }
            let l0_ids = t.l0_ids();
            assert_eq!(l0_ids.len(), 2);

            t.compact_and_wait().await;

            let output_ids = t.compaction_output_ids(&l0_ids);
            assert_eq!(output_ids.len(), 1, "expected one output SST");
            let path = t.compacted_sst_path(&output_ids[0]);
            let expected_parts = (t.object_size(&path).await as usize).div_ceil(MIB);
            assert_eq!(
                expected_parts, 21,
                "update this count if an SST encoding change shifts the size"
            );
            t.assert_cached(&path, expected_parts);
            t.close().await;
        }

        /// A compactor builder with its own object store stays cacheless:
        /// output is not admitted even with cache_on_compaction on.
        #[tokio::test]
        async fn test_object_store_cache_skips_compaction_output_from_custom_store() {
            let t = ObjectStoreCacheTest::builder(
                "/tmp/test_object_store_cache_custom_compactor_store",
            )
            .cache_on_compaction()
            .on_demand_compactor_with_custom_store()
            .build()
            .await;

            for i in 0..2u32 {
                let key = format!("key{:04}", i);
                t.db().put(key.as_bytes(), &[b'v'; 64]).await.unwrap();
                t.db()
                    .flush_with_options(FlushOptions {
                        flush_type: FlushType::MemTable,
                    })
                    .await
                    .unwrap();
            }
            let l0_ids = t.l0_ids();
            assert_eq!(l0_ids.len(), 2);

            t.compact_and_wait().await;

            let output_ids = t.compaction_output_ids(&l0_ids);
            assert!(!output_ids.is_empty(), "expected compaction output SSTs");
            for id in &output_ids {
                let path = t.compacted_sst_path(id);
                assert_eq!(
                    t.cached_part_count(&path),
                    0,
                    "expected compaction output {path} to stay uncached"
                );
            }
            t.close().await;
        }

        /// An embedded compactor on the DB's own store records its object
        /// store I/O under the compactor component, with and without the
        /// object store cache.
        #[tokio::test]
        async fn test_embedded_compactor_io_recorded_under_compactor_component() {
            for object_store_cache in [true, false] {
                let recorder = Arc::new(DefaultMetricsRecorder::new());
                let mut builder =
                    ObjectStoreCacheTest::builder("/tmp/test_compactor_component_metrics")
                        .cache_on_compaction()
                        .on_demand_compactor()
                        .metrics_recorder(recorder.clone());
                if !object_store_cache {
                    builder = builder.without_object_store_cache();
                }
                let t = builder.build().await;

                for i in 0..2u32 {
                    let key = format!("key{:04}", i);
                    t.db().put(key.as_bytes(), &[b'v'; 64]).await.unwrap();
                    t.db()
                        .flush_with_options(FlushOptions {
                            flush_type: FlushType::MemTable,
                        })
                        .await
                        .unwrap();
                }
                t.compact_and_wait().await;

                let gets =
                    lookup_object_store_op_request_count(&recorder, "compactor", "main", "get");
                let puts =
                    lookup_object_store_op_request_count(&recorder, "compactor", "main", "put");
                assert!(gets > 0, "no compactor gets [cache={object_store_cache}]");
                assert!(puts > 0, "no compactor puts [cache={object_store_cache}]");
                t.close().await;
            }
        }
    }
}
