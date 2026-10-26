//! This module contains SlateDB's garbage collector.
//!
//! The garbage collector is responsible for removing obsolete data from SlateDB storage:
//! - Write-ahead log (WAL) SSTs that are no longer referenced by active manifests or
//!   checkpoints
//! - Compacted SSTs that are no longer referenced by active manifests or checkpoints
//! - Old compactions files that are no longer needed
//! - Old manifests that are not needed for recovery or checkpoints
//!
//! The garbage collector runs periodically in the background, with configurable intervals
//! and minimum age thresholds for each type of data. This ensures that recently created
//! data isn't immediately deleted, which would risk removing files that might still be
//! referenced by in-flight operations.

use crate::checkpoint::Checkpoint;
use crate::compactions_store::CompactionsStore;
use crate::config::GarbageCollectorOptions;
pub use crate::db::builder::GarbageCollectorBuilder;
use crate::db_status::ClosedResultWriter;
use crate::dispatcher::{MessageHandler, MessageHandlerExecutor, MessageTickerDef};
use crate::error::SlateDBError;
use crate::garbage_collector::stats::GcStats;
use crate::manifest::store::{ManifestStore, StoredManifest};
use crate::manifest::Manifest;
use crate::tablestore::TableStore;
use crate::utils::WatchableOnceCell;
use crate::wal::slatedb::gc::{SlateDbWalGc, WalGcMode};
use crate::wal::slatedb::store::WalTableStore;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use compacted_gc::CompactedGcTask;
use compactions_gc::CompactionsGcTask;
use detach_gc::DetachGcTask;
use futures::stream::BoxStream;
use log::{error, info};
use manifest_gc::ManifestGcTask;
use object_store::ObjectStore;
use slatedb_common::clock::SystemClock;
use slatedb_common::metrics::MetricsRecorderHelper;
use slatedb_txn_obj::{DirtyObject, SimpleTransactionalObject, TransactionalObject};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;
use tracing::instrument;
use wal_gc::WalGcTask;

mod compacted_gc;
mod compactions_gc;
mod detach_gc;
mod filter;
mod manifest_gc;
pub mod stats;
mod wal_gc;

use crate::wal::WalGc;
pub(crate) use filter::retain_allowed_by_gc_filter;
pub use filter::GcFilter;

pub(crate) const DEFAULT_MIN_AGE: Duration = Duration::from_secs(300);
pub(crate) const DEFAULT_INTERVAL: Duration = Duration::from_secs(600);
pub(crate) const GC_TASK_NAME: &str = "garbage_collector";
/// Maximum number of concurrent object-store deletes issued by a GC task's
/// deletion pass. Deletes are independent single-object operations, so a small
/// bounded fan-out keeps large backlogs tractable without overwhelming the
/// object store.
pub(crate) const GC_DELETE_CONCURRENCY: usize = 8;

trait GcTask {
    fn resource(&self) -> &str;
    async fn collect(&self, now: DateTime<Utc>) -> Result<(), SlateDBError>;
}

#[derive(Debug)]
pub(crate) enum GcMessage {
    Wal,
    WalFence,
    Compacted,
    Compactions,
    Manifest,
    Detach,
}

/// SlateDB's garbage collector.
///
/// The `GarbageCollector` is responsible for cleaning up obsolete data in SlateDB:
///
/// - Write-ahead log (WAL) SSTs that are no longer referenced by active manifests or checkpoints
/// - Compacted SSTs that are no longer referenced by active manifests or checkpoints
/// - Old manifests that are not needed for recovery or checkpoints
///
/// The garbage collector can run in two modes:
///
/// - As an async task with [`run`](GarbageCollector::run)
/// - As a one-time operation with [`run_gc_once`](GarbageCollector::run_gc_once)
///
/// The garbage collector uses configurable intervals and minimum age thresholds for each
/// type of data to ensure that recently created data isn't immediately deleted, which
/// helps prevent removing files that might still be referenced by in-flight operations.
#[derive(Clone)]
pub struct GarbageCollector {
    manifest_store: Arc<ManifestStore>,
    options: GarbageCollectorOptions,
    stats: Arc<GcStats>,
    system_clock: Arc<dyn SystemClock>,
    task_executor: Arc<MessageHandlerExecutor>,
    manifest_gc_task: Option<ManifestGcTask>,
    wal_gc_task: Option<WalGcTask>,
    wal_fence_gc_task: Option<WalGcTask>,
    compacted_gc_task: Option<CompactedGcTask>,
    compactions_gc_task: Option<CompactionsGcTask>,
    detach_gc_task: Option<DetachGcTask>,
}

#[async_trait]
impl MessageHandler<GcMessage> for GarbageCollector {
    fn tickers(&mut self) -> Vec<MessageTickerDef<GcMessage>> {
        let mut tickers: Vec<MessageTickerDef<GcMessage>> = Vec::new();

        if let Some(opts) = self.options.manifest_options {
            tickers.push(MessageTickerDef::new(
                opts.interval.unwrap_or(DEFAULT_INTERVAL),
                Box::new(|| GcMessage::Manifest),
            ));
        }
        if let Some(opts) = self.options.wal_options {
            tickers.push(MessageTickerDef::new(
                opts.interval.unwrap_or(DEFAULT_INTERVAL),
                Box::new(|| GcMessage::Wal),
            ));
        }
        if let Some(opts) = self.options.wal_fence_options {
            tickers.push(MessageTickerDef::new(
                opts.interval.unwrap_or(DEFAULT_INTERVAL),
                Box::new(|| GcMessage::WalFence),
            ));
        }
        if let Some(opts) = self.options.compacted_options {
            tickers.push(MessageTickerDef::new(
                opts.interval.unwrap_or(DEFAULT_INTERVAL),
                Box::new(|| GcMessage::Compacted),
            ));
        }
        if let Some(opts) = self.options.compactions_options {
            tickers.push(MessageTickerDef::new(
                opts.interval.unwrap_or(DEFAULT_INTERVAL),
                Box::new(|| GcMessage::Compactions),
            ));
        }
        if let Some(opts) = self.options.detach_options {
            tickers.push(MessageTickerDef::new(
                opts.interval.unwrap_or(DEFAULT_INTERVAL),
                Box::new(|| GcMessage::Detach),
            ));
        }

        tickers
    }

    async fn handle(&mut self, message: GcMessage) -> Result<(), SlateDBError> {
        match message {
            GcMessage::Manifest => {
                let task = self
                    .manifest_gc_task
                    .as_ref()
                    .expect("got manifest tick with unconfigured manifest task");
                self.run_gc_task(task).await;
            }
            GcMessage::Wal => {
                let task = self
                    .wal_gc_task
                    .as_ref()
                    .expect("got wal tick with unconfigured wal task");
                self.run_gc_task(task).await;
            }
            GcMessage::WalFence => {
                let task = self
                    .wal_fence_gc_task
                    .as_ref()
                    .expect("got wal fence tick with unconfigured wal fence task");
                self.run_gc_task(task).await;
            }
            GcMessage::Compacted => {
                let task = self
                    .compacted_gc_task
                    .as_ref()
                    .expect("got compacted tick with unconfigured compacted task");
                self.run_gc_task(task).await;
            }
            GcMessage::Compactions => {
                let task = self
                    .compactions_gc_task
                    .as_ref()
                    .expect("got compactions tick with unconfigured compactions task");
                self.run_gc_task(task).await;
            }
            GcMessage::Detach => {
                let task = self
                    .detach_gc_task
                    .as_ref()
                    .expect("got detach tick with unconfigured detach task");
                self.run_gc_task(task).await;
            }
        }
        Ok(())
    }

    async fn cleanup(
        &mut self,
        _messages: BoxStream<'async_trait, GcMessage>,
        _result: Result<(), SlateDBError>,
    ) -> Result<(), SlateDBError> {
        info!("garbage collector shutdown");
        Ok(())
    }
}

impl GarbageCollector {
    /// Creates a new `GarbageCollector` instance.
    ///
    /// # Arguments
    ///
    /// * `manifest_store` - The manifest store to use for garbage collection.
    /// * `table_store` - The table store to use for garbage collection.
    /// * `options` - Configuration options for the garbage collector.
    /// * `stat_registry` - Registry for tracking garbage collection metrics.
    /// * `system_clock` - Clock implementation for time-based decisions.
    ///
    /// # Returns
    ///
    /// A new `GarbageCollector` instance configured with the provided components.
    pub(crate) fn new(
        manifest_store: Arc<ManifestStore>,
        compactions_store: Arc<CompactionsStore>,
        table_store: Arc<TableStore>,
        wal_store: Arc<WalTableStore>,
        object_store: Arc<dyn ObjectStore>,
        options: GarbageCollectorOptions,
        recorder: &MetricsRecorderHelper,
        system_clock: Arc<dyn SystemClock>,
        gc_filter: Option<Arc<dyn GcFilter>>,
        wal_gc: Option<Arc<dyn WalGc>>,
    ) -> Self {
        let stats = Arc::new(GcStats::new(recorder));
        // The standalone GC lifecycle does not surface a closed result yet, so the
        // task executor uses a throwaway sink.
        let closed_result: Arc<dyn ClosedResultWriter> = Arc::new(WatchableOnceCell::new());
        let task_executor = Arc::new(MessageHandlerExecutor::new(
            closed_result,
            system_clock.clone(),
        ));
        let wal_gc_task = options.wal_options.map(|wal_options| {
            let wal_gc = wal_gc.unwrap_or_else(|| {
                Arc::new(SlateDbWalGc::new(
                    wal_store.clone(),
                    stats.clone(),
                    WalGcMode::Regular,
                    gc_filter.clone(),
                    system_clock.clone(),
                ))
            });
            WalGcTask::new(
                manifest_store.clone(),
                wal_gc,
                WalGcMode::Regular.resource(),
                wal_options.min_age,
                wal_options.dry_run,
            )
        });
        let wal_fence_gc_task = options.wal_fence_options.map(|wal_fence_options| {
            let wal_gc = Arc::new(SlateDbWalGc::new(
                wal_store,
                stats.clone(),
                WalGcMode::Fence,
                gc_filter.clone(),
                system_clock.clone(),
            ));
            WalGcTask::new(
                manifest_store.clone(),
                wal_gc,
                WalGcMode::Fence.resource(),
                wal_fence_options.min_age,
                wal_fence_options.dry_run,
            )
        });
        let compacted_gc_task = options.compacted_options.map(|compacted_options| {
            CompactedGcTask::new(
                manifest_store.clone(),
                compactions_store.clone(),
                table_store.clone(),
                stats.clone(),
                compacted_options,
                gc_filter.clone(),
            )
        });
        let compactions_gc_task = options.compactions_options.map(|compactions_options| {
            CompactionsGcTask::new(
                compactions_store.clone(),
                stats.clone(),
                compactions_options,
                gc_filter.clone(),
                options.boundary_files_enabled,
            )
        });
        let manifest_gc_task = options.manifest_options.map(|manifest_options| {
            ManifestGcTask::new(
                manifest_store.clone(),
                stats.clone(),
                manifest_options,
                gc_filter.clone(),
                options.boundary_files_enabled,
            )
        });
        let detach_gc_task = options.detach_options.map(|detach_options| {
            DetachGcTask::new(
                manifest_store.clone(),
                object_store,
                system_clock.clone(),
                stats.clone(),
                detach_options,
            )
        });
        Self {
            manifest_store,
            options,
            stats,
            system_clock,
            task_executor,
            manifest_gc_task,
            wal_gc_task,
            wal_fence_gc_task,
            compacted_gc_task,
            compactions_gc_task,
            detach_gc_task,
        }
    }

    /// Starts the garbage collector. This method runs the recurring garbage
    /// collection event loop until [`GarbageCollector::stop`] is called.
    ///
    /// ## Returns
    /// - `Ok(())` when the garbage collector exits cleanly, or [`SlateDBError`] on failure.
    pub async fn run(&self) -> Result<(), crate::Error> {
        self.start()?;
        self.join().await
    }

    pub(crate) fn start(&self) -> Result<(), crate::Error> {
        let (_, rx) = async_channel::unbounded();
        self.task_executor
            .add_handler(
                GC_TASK_NAME.to_string(),
                Box::new(self.clone()),
                rx,
                &Handle::current(),
            )
            .map_err(crate::Error::from)?;
        self.task_executor.monitor_on(&Handle::current())?;
        Ok(())
    }

    pub(crate) async fn join(&self) -> Result<(), crate::Error> {
        self.task_executor
            .join_task(GC_TASK_NAME)
            .await
            .map_err(crate::Error::from)
    }

    /// Gracefully stops the garbage collector task and waits for it to finish.
    ///
    /// ## Returns
    /// - `Ok(())` once the task has shut down, or [`SlateDBError`] if shutdown fails.
    pub async fn stop(&self) -> Result<(), crate::Error> {
        self.task_executor
            .shutdown_task(GC_TASK_NAME)
            .await
            .map_err(crate::Error::from)
    }

    /// Run the garbage collector once.
    ///
    /// This method runs all configured garbage collection tasks:
    ///
    /// - WAL SST garbage collection
    /// - Compacted SST garbage collection
    /// - Manifest garbage collection
    /// - Detach clone from parent garbage
    pub async fn run_gc_once(&self) {
        if let Some(task) = &self.manifest_gc_task {
            self.run_gc_task(task).await;
        }
        if let Some(task) = &self.wal_gc_task {
            self.run_gc_task(task).await;
        }
        if let Some(task) = &self.wal_fence_gc_task {
            self.run_gc_task(task).await;
        }
        if let Some(task) = &self.compacted_gc_task {
            self.run_gc_task(task).await;
        }
        if let Some(task) = &self.compactions_gc_task {
            self.run_gc_task(task).await;
        }
        if let Some(task) = &self.detach_gc_task {
            self.run_gc_task(task).await;
        }

        self.stats.gc_count.increment(1);
    }

    #[instrument(level = "debug", skip_all, fields(resource = task.resource()))]
    async fn run_gc_task<T: GcTask + std::fmt::Debug>(&self, task: &T) {
        if let Err(e) = self.remove_expired_checkpoints().await {
            error!(
                "error removing expired checkpoints [resource={}, error={:?}]",
                task.resource(),
                e,
            );
        } else if let Err(e) = task.collect(self.system_clock.now()).await {
            error!(
                "error collecting garbage [resource={}, error={:?}]",
                task.resource(),
                e,
            );
        }
    }

    async fn remove_expired_checkpoints(&self) -> Result<(), SlateDBError> {
        let mut stored_manifest =
            StoredManifest::load(Arc::clone(&self.manifest_store), self.system_clock.clone())
                .await?;

        stored_manifest
            .maybe_apply_update(|manifest| self.filter_expired_checkpoints(manifest))
            .await
    }

    fn filter_expired_checkpoints(
        &self,
        manifest: &SimpleTransactionalObject<Manifest>,
    ) -> Result<Option<DirtyObject<Manifest>>, SlateDBError> {
        let utc_now: DateTime<Utc> = self.system_clock.now();
        let mut dirty = manifest.prepare_dirty()?;
        let retained_checkpoints: Vec<Checkpoint> = dirty
            .value
            .core
            .checkpoints
            .iter()
            .filter(|checkpoint| match checkpoint.expire_time {
                Some(expire_time) => expire_time > utc_now,
                None => true,
            })
            .cloned()
            .collect();

        let maybe_dirty = if dirty.value.core.checkpoints.len() != retained_checkpoints.len() {
            dirty.value.core.checkpoints = retained_checkpoints;
            Some(dirty)
        } else {
            None
        };
        Ok(maybe_dirty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_cache_policy::BlockCachePolicy;
    use crate::tablestore::TableStoreKind;
    use crate::wal::slatedb::store::{WalFileId, WalTableStore};
    use bytes::Bytes;

    use std::collections::HashSet;
    use std::{fs::OpenOptions, sync::Arc};

    use chrono::{DateTime, Days, TimeDelta, Utc};
    use object_store::{local::LocalFileSystem, path::Path};
    use uuid::Uuid;

    use crate::checkpoint::Checkpoint;
    use crate::compactions_store::StoredCompactions;
    use crate::compactor_state::{Compaction, CompactionSpec, SourceId};
    use crate::config::{GarbageCollectorDirectoryOptions, GarbageCollectorOptions};
    use crate::error::SlateDBError;
    use crate::paths::PathResolver;
    use crate::types::RowEntry;
    use slatedb_common::clock::DefaultSystemClock;
    use slatedb_common::metrics::{
        lookup_metric_with_labels, DefaultMetricsRecorder, MetricsRecorderHelper,
    };
    use slatedb_common::ObjectMetadata;

    use crate::format::sst::SsTableFormat;
    use crate::{
        db_state::{SortedRun, SsTableHandle, SsTableId, SsTableView},
        manifest::{
            store::{ManifestStore, StoredManifest},
            ManifestCore,
        },
        tablestore::TableStore,
    };

    struct LocationGcFilter {
        allowed_locations: HashSet<Path>,
    }

    #[async_trait]
    impl GcFilter for LocationGcFilter {
        async fn filter(&self, candidates: HashSet<ObjectMetadata>) -> HashSet<ObjectMetadata> {
            candidates
                .into_iter()
                .filter(|metadata| self.allowed_locations.contains(&metadata.location))
                .collect()
        }
    }

    #[tokio::test]
    async fn test_collect_garbage_manifest() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();

        // Create a manifest
        let state = ManifestCore::new();
        let mut stored_manifest = StoredManifest::create_new_db(
            manifest_store.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        // Add a second manifest
        stored_manifest
            .update(stored_manifest.prepare_dirty().unwrap())
            .await
            .unwrap();

        // Set the first manifest file to be a day old
        let now_minus_24h = set_modified(
            local_object_store.clone(),
            &Path::from(format!("manifest/{:020}.{}", 1, "manifest")),
            86400,
        );

        // Verify that the manifests are there as expected
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].id, 1);
        assert_eq!(manifests[1].id, 2);
        assert_eq!(manifests[0].metadata.last_modified, now_minus_24h);

        // Start the garbage collector
        run_gc_once(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            None,
        )
        .await;

        // Verify that the first manifest was deleted
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].id, 2);
    }

    #[tokio::test]
    async fn test_collect_garbage_only_recent_manifests() {
        let (manifest_store, compactions_store, table_store, wal_store, _) = build_objects();

        // Create a manifest
        let mut stored_manifest = StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        // Add a second manifest
        stored_manifest
            .update(stored_manifest.prepare_dirty().unwrap())
            .await
            .unwrap();

        // Verify that the manifests are there as expected
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].id, 1);
        assert_eq!(manifests[1].id, 2);

        // Start the garbage collector
        run_gc_once(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            None,
        )
        .await;

        // Verify that no manifests were deleted
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].id, 1);
        assert_eq!(manifests[1].id, 2);
    }

    #[tokio::test]
    async fn test_collect_garbage_compactions_keeps_latest() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let mut stored_compactions = StoredCompactions::create(compactions_store.clone(), 0)
            .await
            .unwrap();
        // Create two compaction files so we can test that the latest is kept
        stored_compactions
            .update(stored_compactions.prepare_dirty().unwrap())
            .await
            .unwrap();
        stored_compactions
            .update(stored_compactions.prepare_dirty().unwrap())
            .await
            .unwrap();

        let compaction_count = compactions_store
            .list_compactions(..)
            .await
            .expect("should list compactions")
            .len();

        assert_eq!(compaction_count, 3);

        for id in 1..=compaction_count {
            // Mark all compaction files as old enough to delete
            set_modified(
                local_object_store.clone(),
                &Path::from(format!("compactions/{:020}.{}", id, "compactions")),
                86400,
            );
        }

        run_gc_once(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            None,
        )
        .await;

        let compactions = compactions_store.list_compactions(..).await.unwrap();
        assert_eq!(compactions.len(), 1);
        assert_eq!(compactions[0].id, 3);
    }

    #[tokio::test]
    async fn test_collect_garbage_compactions_respects_min_age() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let mut stored_compactions = StoredCompactions::create(compactions_store.clone(), 0)
            .await
            .unwrap();
        stored_compactions
            .update(stored_compactions.prepare_dirty().unwrap())
            .await
            .unwrap();

        let compaction_count = compactions_store
            .list_compactions(..)
            .await
            .expect("should list compactions")
            .len();

        assert_eq!(compaction_count, 2);

        // Mark all compaction files as only 60 seconds old, which is less than the default min age of 300 seconds
        for id in 1..=compaction_count {
            set_modified(
                local_object_store.clone(),
                &Path::from(format!("compactions/{:020}.{}", id, "compactions")),
                60,
            );
        }

        run_gc_once(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            None,
        )
        .await;

        // Verify that no compaction files were deleted
        let compactions = compactions_store.list_compactions(..).await.unwrap();
        assert_eq!(compactions.len(), 2);
        assert_eq!(compactions[0].id, 1);
        assert_eq!(compactions[1].id, 2);
    }

    fn new_checkpoint(manifest_id: u64, expire_time: Option<DateTime<Utc>>) -> Checkpoint {
        Checkpoint {
            id: Uuid::new_v4(),
            manifest_id,
            expire_time,
            create_time: DefaultSystemClock::default().now(),
            name: None,
        }
    }

    async fn checkpoint_current_manifest(
        stored_manifest: &mut StoredManifest,
        expire_time: Option<DateTime<Utc>>,
    ) -> Result<Uuid, SlateDBError> {
        let mut dirty = stored_manifest.prepare_dirty()?;
        let checkpoint = new_checkpoint(stored_manifest.id(), expire_time);
        let checkpoint_id = checkpoint.id;
        dirty.value.core.checkpoints.push(checkpoint);
        stored_manifest.update(dirty).await?;
        Ok(checkpoint_id)
    }

    async fn remove_checkpoint(
        checkpoint_id: Uuid,
        stored_manifest: &mut StoredManifest,
    ) -> Result<(), SlateDBError> {
        let mut dirty = stored_manifest.prepare_dirty()?;
        let updated_checkpoints = dirty
            .value
            .core
            .checkpoints
            .iter()
            .filter(|checkpoint| checkpoint.id != checkpoint_id)
            .cloned()
            .collect();
        dirty.value.core.checkpoints = updated_checkpoints;
        stored_manifest.update(dirty).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_remove_expired_checkpoints() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();

        // Manifest 1
        let state = ManifestCore::new();
        let mut stored_manifest = StoredManifest::create_new_db(
            manifest_store.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        // Manifest 2 (expired_checkpoint_id -> 1)
        let one_day_ago = DefaultSystemClock::default()
            .now()
            .checked_sub_days(Days::new(1))
            .unwrap();
        let _expired_checkpoint_id =
            checkpoint_current_manifest(&mut stored_manifest, Some(one_day_ago))
                .await
                .unwrap();
        // Manifest 3 (expired_checkpoint_id -> 1, unexpired_checkpoint_id -> 2)
        let one_day_ahead = DefaultSystemClock::default()
            .now()
            .checked_add_days(Days::new(1))
            .unwrap();
        let unexpired_checkpoint_id =
            checkpoint_current_manifest(&mut stored_manifest, Some(one_day_ahead))
                .await
                .unwrap();

        // Make all manifests eligible for deletion
        for i in 1..=3 {
            set_modified(
                local_object_store.clone(),
                &Path::from(format!("manifest/{:020}.{}", i, "manifest")),
                86400,
            );
        }

        // Start the garbage collector
        run_gc_once(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            None,
        )
        .await;

        // The GC should create a new manifest version 4 with the expired
        // checkpoint removed.
        let latest_manifest = manifest_store.read_latest_manifest().await.unwrap();
        assert_eq!(4, latest_manifest.id);
        assert_eq!(1, latest_manifest.manifest.core.checkpoints.len());
        assert_eq!(
            unexpired_checkpoint_id,
            latest_manifest.manifest.core.checkpoints[0].id
        );
        assert_eq!(2, latest_manifest.manifest.core.checkpoints[0].manifest_id);

        // Only the latest manifest and the one referenced by the unexpired checkpoint
        // should be retained.
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].id, 2);
        assert_eq!(manifests[1].id, 4);
    }

    #[tokio::test]
    async fn test_collector_should_not_clean_manifests_referenced_by_checkpoints() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();

        // Manifest 1
        let state = ManifestCore::new();
        let mut stored_manifest = StoredManifest::create_new_db(
            manifest_store.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        // Manifest 2 (active_checkpoint_id -> 1)
        let active_checkpoint_id = checkpoint_current_manifest(&mut stored_manifest, None)
            .await
            .unwrap();
        // Manifest 3 (active_checkpoint_id -> 1, inactive_checkpoint_id -> 2)
        let inactive_checkpoint_id = checkpoint_current_manifest(&mut stored_manifest, None)
            .await
            .unwrap();
        // Manifest 4 (active_checkpoint_id -> 1)
        remove_checkpoint(inactive_checkpoint_id, &mut stored_manifest)
            .await
            .unwrap();

        // Set the older manifests to be a day old to make them eligible for deletion
        for i in 1..4 {
            set_modified(
                local_object_store.clone(),
                &Path::from(format!("manifest/{:020}.{}", i, "manifest")),
                86400,
            );
        }

        // Verify that the manifests are there as expected
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 4);

        // Start the garbage collector
        run_gc_once(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            None,
        )
        .await;

        // Verify that the latest manifest version is still 4 with the active checkpoint
        let latest_manifest = manifest_store.read_latest_manifest().await.unwrap();
        assert_eq!(4, latest_manifest.id);
        assert_eq!(1, latest_manifest.manifest.core.checkpoints.len());
        assert_eq!(
            active_checkpoint_id,
            latest_manifest.manifest.core.checkpoints[0].id
        );
        assert_eq!(1, latest_manifest.manifest.core.checkpoints[0].manifest_id);

        // The active manifest and the manifest corresponding to the active
        // checkpoint should be retained. The rest should be deleted.
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].id, 1);
        assert_eq!(manifests[1].id, 4);
    }

    #[tokio::test]
    async fn test_collect_garbage_old_active_manifest() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();

        // Create a manifest
        let mut stored_manifest = StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        // Add a second manifest
        stored_manifest
            .update(stored_manifest.prepare_dirty().unwrap())
            .await
            .unwrap();

        // Set both manifests to be a day old
        let now_minus_24h_1 = set_modified(
            local_object_store.clone(),
            &Path::from(format!("manifest/{:020}.{}", 1, "manifest")),
            86400,
        );
        let now_minus_24h_2 = set_modified(
            local_object_store.clone(),
            &Path::from(format!("manifest/{:020}.{}", 2, "manifest")),
            86400,
        );

        // Verify that the manifests are there as expected
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].id, 1);
        assert_eq!(manifests[1].id, 2);
        assert_eq!(manifests[0].metadata.last_modified, now_minus_24h_1);
        assert_eq!(manifests[1].metadata.last_modified, now_minus_24h_2);

        // Start the garbage collector
        run_gc_once(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            None,
        )
        .await;

        // Verify that the first manifest was deleted, but the second is still safe
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].id, 2);
    }

    async fn write_wal_sst(
        wal_store: Arc<WalTableStore>,
        wal_id: WalFileId,
    ) -> Result<(), SlateDBError> {
        let mut sst = wal_store.table_builder();
        sst.add(RowEntry::new_value(b"key", b"value", 0)).await?;
        let table = sst.build().await?;
        wal_store.write_sst(wal_id, &table).await?;
        Ok(())
    }

    async fn write_wal_fence(wal_store: Arc<WalTableStore>, wal_id: u64) {
        wal_store.write_wal_fence(wal_id.into()).await.unwrap();
    }

    #[tokio::test]
    async fn test_collect_garbage_wal_ssts() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();
        let path_resolver = PathResolver::from_root("/");

        // write a wal sst
        let id1 = WalFileId::from(1);
        write_wal_sst(wal_store.clone(), id1).await.unwrap();

        let id2 = WalFileId::from(2);
        write_wal_sst(wal_store.clone(), id2).await.unwrap();

        // Set the first WAL SST file to be a day old
        let now_minus_24h = set_modified(
            local_object_store.clone(),
            &path_resolver.wal_sst_path(&WalFileId::from(1)),
            86400,
        );

        // Create a manifest
        let mut state = ManifestCore::new();
        state.replay_after_wal_id = id2.value();
        StoredManifest::create_new_db(
            manifest_store.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        // Verify that the WAL SST is there as expected
        let wal_ssts = wal_store.list_wal_ssts(..).await.unwrap();
        assert_eq!(wal_ssts.len(), 2);
        assert_eq!(wal_ssts[0].id, id1);
        assert_eq!(wal_ssts[1].id, id2);
        assert_eq!(wal_ssts[0].metadata.last_modified, now_minus_24h);
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 1);
        let current_manifest = manifest_store.read_latest_manifest().await.unwrap();
        assert_eq!(
            current_manifest.manifest.core.replay_after_wal_id,
            id2.value()
        );

        // Start the garbage collector
        run_gc_once(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            None,
        )
        .await;

        // Verify that the first WAL was deleted and the second is kept
        let wal_ssts = wal_store.list_wal_ssts(..).await.unwrap();
        assert_eq!(wal_ssts.len(), 1);
        assert_eq!(wal_ssts[0].id, id2);
    }

    #[tokio::test]
    async fn test_do_not_remove_wals_referenced_by_active_checkpoints() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();
        let path_resolver = PathResolver::from_root("/");

        let id1 = WalFileId::from(1);
        write_wal_sst(wal_store.clone(), id1).await.unwrap();

        let id2 = WalFileId::from(2);
        write_wal_sst(wal_store.clone(), id2).await.unwrap();

        let id3 = WalFileId::from(3);
        write_wal_sst(wal_store.clone(), id3).await.unwrap();

        // Manifest 1 with table 1 eligible for deletion
        let mut state = ManifestCore::new();
        state.replay_after_wal_id = 1;
        state.next_wal_sst_id = 4;
        let mut stored_manifest = StoredManifest::create_new_db(
            manifest_store.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        assert_eq!(1, stored_manifest.id());

        // Manifest 2 with checkpoint referencing Manifest 1
        let mut dirty = stored_manifest.prepare_dirty().unwrap();
        dirty.value.core.replay_after_wal_id = 3;
        dirty.value.core.next_wal_sst_id = 4;
        dirty.value.core.checkpoints.push(new_checkpoint(1, None));
        stored_manifest.update(dirty).await.unwrap();
        assert_eq!(2, stored_manifest.id());

        // All tables are eligible for deletion
        for i in 1..=3 {
            set_modified(
                local_object_store.clone(),
                &path_resolver.wal_sst_path(&WalFileId::from(i)),
                86400,
            );
        }

        // Start the garbage collector
        run_gc_once(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            None,
        )
        .await;

        // Only the first table is deleted. The second is eligible,
        // but the reference in the checkpoint is still active.
        let wal_ssts = wal_store.list_wal_ssts(..).await.unwrap();
        assert_eq!(wal_ssts.len(), 2);
        assert_eq!(wal_ssts[0].id, id2);
        assert_eq!(wal_ssts[1].id, id3);
    }

    #[tokio::test]
    async fn test_collect_garbage_wal_ssts_and_keep_expired_last_compacted() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();
        let path_resolver = PathResolver::from_root("/");

        // write a wal sst
        let id1 = WalFileId::from(1);
        write_wal_sst(wal_store.clone(), id1).await.unwrap();

        let id2 = WalFileId::from(2);
        write_wal_sst(wal_store.clone(), id2).await.unwrap();

        // Set the both WAL SST file to be a day old
        let now_minus_24h_1 = set_modified(
            local_object_store.clone(),
            &path_resolver.wal_sst_path(&WalFileId::from(1)),
            86400,
        );
        let now_minus_24h_2 = set_modified(
            local_object_store.clone(),
            &path_resolver.wal_sst_path(&WalFileId::from(2)),
            86400,
        );

        // Create a manifest
        let mut state = ManifestCore::new();
        state.replay_after_wal_id = id2.value();
        StoredManifest::create_new_db(
            manifest_store.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        // Verify that the WAL SST is there as expected
        let wal_ssts = wal_store.list_wal_ssts(..).await.unwrap();
        assert_eq!(wal_ssts.len(), 2);
        assert_eq!(wal_ssts[0].id, id1);
        assert_eq!(wal_ssts[1].id, id2);
        assert_eq!(wal_ssts[0].metadata.last_modified, now_minus_24h_1);
        assert_eq!(wal_ssts[1].metadata.last_modified, now_minus_24h_2);
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 1);
        let current_manifest = manifest_store.read_latest_manifest().await.unwrap();
        assert_eq!(
            current_manifest.manifest.core.replay_after_wal_id,
            id2.value()
        );

        // Start the garbage collector
        run_gc_once(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            None,
        )
        .await;

        // Verify that the first WAL was deleted and the second is kept even though it's expired
        let wal_ssts = wal_store.list_wal_ssts(..).await.unwrap();
        assert_eq!(wal_ssts.len(), 1);
        assert_eq!(wal_ssts[0].id, id2);
    }

    #[tokio::test]
    async fn test_regular_wal_gc_does_not_delete_wal_fences() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();
        let path_resolver = PathResolver::from_root("/");

        let fence_id = WalFileId::from(1);
        write_wal_fence(wal_store.clone(), 1).await;

        let regular_wal_id = WalFileId::from(2);
        write_wal_sst(wal_store.clone(), regular_wal_id)
            .await
            .unwrap();

        for id in [fence_id, regular_wal_id] {
            set_modified(
                local_object_store.clone(),
                &path_resolver.wal_sst_path(&id),
                86400,
            );
        }

        let mut state = ManifestCore::new();
        state.replay_after_wal_id = 3;
        StoredManifest::create_new_db(
            manifest_store.clone(),
            state,
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let gc_opts = GarbageCollectorOptions {
            manifest_options: None,
            wal_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            wal_fence_options: None,
            compacted_options: None,
            compactions_options: None,
            detach_options: None,
            metric_level: None,
            boundary_files_enabled: true,
            object_store_max_retries: None,
        };
        let gc = GarbageCollector::new(
            manifest_store.clone(),
            compactions_store,
            table_store.clone(),
            wal_store.clone(),
            Arc::new(object_store::memory::InMemory::new()),
            gc_opts,
            &MetricsRecorderHelper::noop(),
            Arc::new(DefaultSystemClock::default()),
            None,
            None,
        );

        gc.run_gc_once().await;

        let wal_ssts = wal_store.list_wal_ssts(..).await.unwrap();
        assert_eq!(wal_ssts.len(), 1);
        assert_eq!(wal_ssts[0].id, fence_id);
        assert_eq!(wal_ssts[0].metadata.size, 0);
    }

    #[tokio::test]
    async fn test_wal_fence_gc_deletes_old_fences() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();
        let path_resolver = PathResolver::from_root("/");

        let old_fence_id = WalFileId::from(1);
        write_wal_fence(wal_store.clone(), 1).await;

        let regular_wal_id = WalFileId::from(2);
        write_wal_sst(wal_store.clone(), regular_wal_id)
            .await
            .unwrap();

        let newer_fence_id = WalFileId::from(3);
        write_wal_fence(wal_store.clone(), 3).await;

        for id in [old_fence_id, regular_wal_id, newer_fence_id] {
            set_modified(
                local_object_store.clone(),
                &path_resolver.wal_sst_path(&id),
                86400,
            );
        }

        let mut state = ManifestCore::new();
        state.replay_after_wal_id = 4;
        StoredManifest::create_new_db(
            manifest_store.clone(),
            state,
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let gc_opts = GarbageCollectorOptions {
            manifest_options: None,
            wal_options: None,
            wal_fence_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            compacted_options: None,
            compactions_options: None,
            detach_options: None,
            metric_level: None,
            boundary_files_enabled: true,
            object_store_max_retries: None,
        };
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(recorder.clone(), Default::default());
        let gc = GarbageCollector::new(
            manifest_store.clone(),
            compactions_store,
            table_store.clone(),
            wal_store.clone(),
            Arc::new(object_store::memory::InMemory::new()),
            gc_opts,
            &helper,
            Arc::new(DefaultSystemClock::default()),
            None,
            None,
        );

        gc.run_gc_once().await;

        let wal_ssts = wal_store.list_wal_ssts(..).await.unwrap();
        let wal_ids = wal_ssts.iter().map(|sst| sst.id).collect::<Vec<_>>();
        assert_eq!(wal_ids, vec![regular_wal_id]);
        assert!(wal_ssts[0].metadata.size > 0);
        assert_eq!(
            lookup_metric_with_labels(
                &recorder,
                stats::DELETED_COUNT,
                &[("resource", "wal_fence")]
            ),
            Some(2)
        );
    }

    #[tokio::test]
    async fn test_wal_fence_gc_deletes_single_old_fence() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();
        let path_resolver = PathResolver::from_root("/");

        let fence_id = WalFileId::from(1);
        write_wal_fence(wal_store.clone(), 1).await;
        set_modified(
            local_object_store,
            &path_resolver.wal_sst_path(&fence_id),
            86400,
        );

        let mut state = ManifestCore::new();
        state.replay_after_wal_id = 2;
        StoredManifest::create_new_db(
            manifest_store.clone(),
            state,
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let gc_opts = GarbageCollectorOptions {
            manifest_options: None,
            wal_options: None,
            wal_fence_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            compacted_options: None,
            compactions_options: None,
            detach_options: None,
            metric_level: None,
            boundary_files_enabled: true,
            object_store_max_retries: None,
        };
        let gc = GarbageCollector::new(
            manifest_store.clone(),
            compactions_store,
            table_store.clone(),
            wal_store.clone(),
            Arc::new(object_store::memory::InMemory::new()),
            gc_opts,
            &MetricsRecorderHelper::noop(),
            Arc::new(DefaultSystemClock::default()),
            None,
            None,
        );

        gc.run_gc_once().await;

        let wal_ssts = wal_store.list_wal_ssts(..).await.unwrap();
        assert!(wal_ssts.is_empty());
    }

    #[tokio::test]
    async fn test_regular_and_wal_fence_gc_run_independently() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();
        let path_resolver = PathResolver::from_root("/");

        let old_fence_id = WalFileId::from(1);
        write_wal_fence(wal_store.clone(), 1).await;

        let regular_wal_id_1 = WalFileId::from(2);
        write_wal_sst(wal_store.clone(), regular_wal_id_1)
            .await
            .unwrap();

        let newer_fence_id = WalFileId::from(3);
        write_wal_fence(wal_store.clone(), 3).await;

        let regular_wal_id_2 = WalFileId::from(4);
        write_wal_sst(wal_store.clone(), regular_wal_id_2)
            .await
            .unwrap();

        for id in [
            old_fence_id,
            regular_wal_id_1,
            newer_fence_id,
            regular_wal_id_2,
        ] {
            set_modified(
                local_object_store.clone(),
                &path_resolver.wal_sst_path(&id),
                86400,
            );
        }

        let mut state = ManifestCore::new();
        state.replay_after_wal_id = 4;
        StoredManifest::create_new_db(
            manifest_store.clone(),
            state,
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let gc_opts = GarbageCollectorOptions {
            manifest_options: None,
            wal_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            wal_fence_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            compacted_options: None,
            compactions_options: None,
            detach_options: None,
            metric_level: None,
            boundary_files_enabled: true,
            object_store_max_retries: None,
        };
        let gc = GarbageCollector::new(
            manifest_store.clone(),
            compactions_store,
            table_store.clone(),
            wal_store.clone(),
            Arc::new(object_store::memory::InMemory::new()),
            gc_opts,
            &MetricsRecorderHelper::noop(),
            Arc::new(DefaultSystemClock::default()),
            None,
            None,
        );

        gc.run_gc_once().await;

        let wal_ssts = wal_store.list_wal_ssts(..).await.unwrap();
        assert_eq!(wal_ssts.len(), 1);
        assert_eq!(wal_ssts[0].id, regular_wal_id_2);
        assert!(wal_ssts[0].metadata.size > 0);
    }

    /// This test creates eight compacted SSTs:
    /// - One L0 SST
    /// - One active L0 SST that's a day old
    /// - One inactive expired L0 SST
    /// - One inactive unexpired L0 SST
    /// - One active SST
    /// - One active SST that's a day old
    /// - One inactive expired SST
    /// - One inactive unexpired SST
    /// The test then runs the compactor to verify that only the inactive expired SSTs
    /// are deleted.
    #[tokio::test]
    async fn test_collect_garbage_compacted_ssts() {
        let (manifest_store, compactions_store, table_store, wal_store, _local_object_store) =
            build_objects();
        // Use ULID timestamps to model "expired" vs "unexpired" SSTs relative to the
        // compacted GC min_age of 1h.
        let now = DefaultSystemClock::default().now();
        let expired_base_ms = (now - TimeDelta::seconds(7200)).timestamp_millis() as u64; // 2h old
        let unexpired_base_ms = (now - TimeDelta::seconds(1800)).timestamp_millis() as u64; // 30m old

        let l0_sst_handle = create_sst(table_store.clone(), unexpired_base_ms).await;
        let active_expired_l0_sst_handle = create_sst(table_store.clone(), expired_base_ms).await;
        let inactive_expired_l0_sst_handle =
            create_sst(table_store.clone(), expired_base_ms + 1).await;
        let inactive_unexpired_l0_sst_handle =
            create_sst(table_store.clone(), unexpired_base_ms + 1).await;
        let active_sst_handle = create_sst(table_store.clone(), unexpired_base_ms + 2).await;
        let active_expired_sst_handle = create_sst(table_store.clone(), expired_base_ms + 2).await;
        let inactive_expired_sst_handle =
            create_sst(table_store.clone(), expired_base_ms + 3).await;
        let inactive_unexpired_sst_handle =
            create_sst(table_store.clone(), unexpired_base_ms + 3).await;

        // Create a manifest
        let mut state = ManifestCore::new();
        Arc::make_mut(&mut state.tree)
            .l0
            .push_back(SsTableView::identity(l0_sst_handle.clone()));
        Arc::make_mut(&mut state.tree)
            .l0
            .push_back(SsTableView::identity(active_expired_l0_sst_handle.clone()));
        // Dont' push inactive_expired_l0_sst_handle
        Arc::make_mut(&mut state.tree)
            .compacted
            .push(SortedRun::new(
                1,
                // Don't add inactive_expired_sst_handle
                [
                    SsTableView::identity(active_sst_handle.clone()),
                    SsTableView::identity(active_expired_sst_handle.clone()),
                ],
            ));
        StoredManifest::create_new_db(
            manifest_store.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        // Verify that the compacted SSTs are there as expected
        let compacted_ssts = table_store.list_compacted_ssts(..).await.unwrap();
        assert_eq!(compacted_ssts.len(), 8);
        let ids: HashSet<_> = compacted_ssts.iter().map(|m| m.id).collect();
        for expected in [
            l0_sst_handle.id,
            active_expired_l0_sst_handle.id,
            inactive_expired_l0_sst_handle.id,
            inactive_unexpired_l0_sst_handle.id,
            active_sst_handle.id,
            active_expired_sst_handle.id,
            inactive_expired_sst_handle.id,
            inactive_unexpired_sst_handle.id,
        ] {
            assert!(ids.contains(&expected));
        }
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 1);
        let current_manifest = manifest_store.read_latest_manifest().await.unwrap();
        assert_eq!(current_manifest.manifest.core.tree.l0.len(), 2);
        assert_eq!(current_manifest.manifest.core.tree.compacted.len(), 1);
        assert_eq!(
            current_manifest.manifest.core.tree.compacted[0]
                .sst_views()
                .len(),
            2
        );

        // Start the garbage collector
        run_gc_once(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            Some(now),
        )
        .await;

        // Verify that only inactive, expired SSTs were deleted.
        let compacted_ssts = table_store.list_compacted_ssts(..).await.unwrap();
        assert_eq!(compacted_ssts.len(), 6);
        let remaining_ids: HashSet<_> = compacted_ssts.iter().map(|m| m.id).collect();
        // Still-present SSTs
        for expected in [
            l0_sst_handle.id,
            active_expired_l0_sst_handle.id,
            inactive_unexpired_l0_sst_handle.id,
            active_sst_handle.id,
            active_expired_sst_handle.id,
            inactive_unexpired_sst_handle.id,
        ] {
            assert!(remaining_ids.contains(&expected));
        }
        // Deleted SSTs
        assert!(!remaining_ids.contains(&inactive_expired_l0_sst_handle.id));
        assert!(!remaining_ids.contains(&inactive_expired_sst_handle.id));
        let current_manifest = manifest_store.read_latest_manifest().await.unwrap();
        assert_eq!(current_manifest.manifest.core.tree.l0.len(), 2);
        assert_eq!(current_manifest.manifest.core.tree.compacted.len(), 1);
        assert_eq!(
            current_manifest.manifest.core.tree.compacted[0]
                .sst_views()
                .len(),
            2
        );
    }

    /// This test creates six compacted SSTs:
    /// - One L0 SST
    /// - One inactive expired L0 SST (w/ checkpoint)
    /// - One inactive expired L0 SST (w/o checkpoint)
    /// - One active SST
    /// - One inactive expired SST (w/ checkpoint)
    /// - One inactive expired SST (w/o checkpoint)
    /// The test then runs the compactor to verify that only the inactive expired SSTs
    /// are deleted.
    #[tokio::test]
    async fn test_collect_garbage_compacted_ssts_respects_checkpoint_references() {
        let (manifest_store, compactions_store, table_store, wal_store, _local_object_store) =
            build_objects();
        // Make all SSTs "expired" according to ULID timestamp so that min_age
        // does not prevent their collection; checkpoint references will gate GC.
        let now = DefaultSystemClock::default().now();
        let expired_base_ms = (now - TimeDelta::seconds(7200)).timestamp_millis() as u64;

        // Inactive SSTs are strictly older than all active SSTs so that
        // they are eligible for collection when not referenced.
        let active_l0_sst_handle = create_sst(table_store.clone(), expired_base_ms + 5).await;
        let active_checkpoint_l0_sst_handle =
            create_sst(table_store.clone(), expired_base_ms + 4).await;
        let active_sst_handle = create_sst(table_store.clone(), expired_base_ms + 3).await;
        let active_checkpoint_sst_handle =
            create_sst(table_store.clone(), expired_base_ms + 2).await;
        let inactive_l0_sst_handle = create_sst(table_store.clone(), expired_base_ms + 1).await;
        let inactive_sst_handle = create_sst(table_store.clone(), expired_base_ms).await;

        // Create an initial manifest with active and active checkpoint tables
        let mut state = ManifestCore::new();
        Arc::make_mut(&mut state.tree)
            .l0
            .push_back(SsTableView::identity(active_l0_sst_handle.clone()));
        Arc::make_mut(&mut state.tree)
            .l0
            .push_back(SsTableView::identity(
                active_checkpoint_l0_sst_handle.clone(),
            ));
        Arc::make_mut(&mut state.tree)
            .compacted
            .push(SortedRun::new(
                1,
                [SsTableView::identity(active_sst_handle.clone())],
            ));
        Arc::make_mut(&mut state.tree)
            .compacted
            .push(SortedRun::new(
                2,
                [SsTableView::identity(active_checkpoint_sst_handle.clone())],
            ));
        let mut stored_manifest = StoredManifest::create_new_db(
            manifest_store.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let checkpoint_id = checkpoint_current_manifest(&mut stored_manifest, None)
            .await
            .unwrap();

        // Now drop the active tables from the checkpoint
        let mut dirty = stored_manifest.prepare_dirty().unwrap();
        Arc::make_mut(&mut dirty.value.core.tree).l0.truncate(1);
        Arc::make_mut(&mut dirty.value.core.tree)
            .compacted
            .truncate(1);
        stored_manifest.update(dirty).await.unwrap();

        // Start the garbage collector
        run_gc_once(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            Some(now),
        )
        .await;

        // Verify that the active tables are still there
        let compacted_ssts = table_store.list_compacted_ssts(..).await.unwrap();
        assert_eq!(compacted_ssts.len(), 4);
        let remaining_ids: HashSet<_> = compacted_ssts.iter().map(|m| m.id).collect();
        assert!(remaining_ids.contains(&active_l0_sst_handle.id));
        assert!(remaining_ids.contains(&active_checkpoint_l0_sst_handle.id));
        assert!(remaining_ids.contains(&active_sst_handle.id));
        assert!(remaining_ids.contains(&active_checkpoint_sst_handle.id));
        assert!(!remaining_ids.contains(&inactive_l0_sst_handle.id));
        assert!(!remaining_ids.contains(&inactive_sst_handle.id));

        // Drop the checkpoint and run the GC one more time
        remove_checkpoint(checkpoint_id, &mut stored_manifest)
            .await
            .unwrap();

        // Start the garbage collector
        run_gc_once(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            Some(now),
        )
        .await;
        let compacted_ssts = table_store.list_compacted_ssts(..).await.unwrap();
        // After dropping the checkpoint, the L0 and SST that were only kept alive by
        // the checkpoint can be collected.
        let remaining_ids: HashSet<_> = compacted_ssts.iter().map(|m| m.id).collect();
        eprintln!("remaining_ids: {:#?}", remaining_ids);
        assert_eq!(remaining_ids.len(), 2);
        assert!(remaining_ids.contains(&active_l0_sst_handle.id));
        assert!(remaining_ids.contains(&active_sst_handle.id));
        assert!(!remaining_ids.contains(&active_checkpoint_sst_handle.id));
        assert!(!remaining_ids.contains(&active_checkpoint_l0_sst_handle.id));
        assert!(!remaining_ids.contains(&inactive_l0_sst_handle.id));
        assert!(!remaining_ids.contains(&inactive_sst_handle.id));
    }

    /// Builds the objects needed to construct the garbage collector.
    /// # Returns
    /// A tuple containing the manifest store, compactions store, table store,
    /// and local object store
    #[allow(clippy::type_complexity)]
    fn build_objects() -> (
        Arc<ManifestStore>,
        Arc<CompactionsStore>,
        Arc<TableStore>,
        Arc<WalTableStore>,
        Arc<LocalFileSystem>,
    ) {
        let tempdir = tempfile::tempdir().unwrap().keep();
        let local_object_store = Arc::new(
            LocalFileSystem::new_with_prefix(tempdir)
                .unwrap()
                .with_automatic_cleanup(true),
        );
        let path = Path::from("/");
        let manifest_store = Arc::new(ManifestStore::new(&path, local_object_store.clone()));
        let compactions_store = Arc::new(CompactionsStore::new(&path, local_object_store.clone()));
        let sst_format = SsTableFormat::default();
        let table_store = Arc::new(TableStore::new(
            local_object_store.clone(),
            sst_format.clone(),
            path.clone(),
            None,
            TableStoreKind::GC,
            BlockCachePolicy::default(),
        ));
        let wal_store = Arc::new(WalTableStore::new(
            local_object_store.clone(),
            sst_format,
            path,
            TableStoreKind::GC,
        ));

        (
            manifest_store,
            compactions_store,
            table_store,
            wal_store,
            local_object_store,
        )
    }

    /// Create an SSTable with a fixed ULID timestamp and write it to the table store.
    /// # Arguments
    /// * `table_store` - The table store to write the SSTable to
    /// * `ts_ms` - The ULID timestamp in milliseconds since Unix epoch
    /// # Returns
    /// The handle to the SSTable that was created
    async fn create_sst(table_store: Arc<TableStore>, ts_ms: u64) -> SsTableHandle {
        let sst_id = SsTableId::from(ulid::Ulid::from_parts(ts_ms, 0));
        let mut sst = table_store.table_builder();
        sst.add(RowEntry::new_value(b"key", b"value", 0))
            .await
            .unwrap();
        let table = sst.build().await.unwrap();
        table_store
            .write_sst(&sst_id, &table, Some(Bytes::new()))
            .await
            .unwrap()
    }

    /// Set the modified time of a file to be a certain number of seconds ago.
    /// # Arguments
    /// * `local_object_store` - The local object store that is writing files
    /// * `path` - The path to the file to modify (relative to the object store root)
    /// * `seconds_ago` - The number of seconds ago to set the modified time to
    /// # Returns
    /// The new modified time
    fn set_modified(
        local_object_store: Arc<LocalFileSystem>,
        path: &Path,
        seconds_ago: u64,
    ) -> DateTime<Utc> {
        let file = local_object_store.path_to_filesystem(path).unwrap();
        // On Windows, setting file times requires write access to file attributes.
        // Open with write enabled so `set_modified` succeeds across platforms.
        let file = OpenOptions::new().write(true).open(file).unwrap();
        let now_minus_24h = DefaultSystemClock::default().now()
            - TimeDelta::seconds(seconds_ago.try_into().unwrap());
        file.set_modified(now_minus_24h.into()).unwrap();
        now_minus_24h
    }

    async fn assert_no_dangling_references(
        manifest_store: Arc<ManifestStore>,
        table_store: Arc<TableStore>,
        wal_store: Arc<WalTableStore>,
    ) {
        let manifest = manifest_store.read_latest_manifest().await.unwrap();
        let manifests = manifest_store
            .read_referenced_manifests(manifest.id, &manifest.manifest)
            .await
            .unwrap();

        let wal_ssts = wal_store
            .list_wal_ssts(..)
            .await
            .unwrap()
            .iter()
            .map(|sst| sst.id)
            .collect::<HashSet<WalFileId>>();
        let compacted_ssts = table_store
            .list_compacted_ssts(..)
            .await
            .unwrap()
            .iter()
            .map(|sst| sst.id)
            .collect::<HashSet<SsTableId>>();

        for manifest in manifests.values() {
            let wal_sst_start_inclusive = manifest.core.replay_after_wal_id + 1;
            let wal_sst_end_exclusive = manifest.core.next_wal_sst_id;
            for wal_sst_id in wal_sst_start_inclusive..wal_sst_end_exclusive {
                assert!(wal_ssts.contains(&WalFileId::from(wal_sst_id)));
            }

            for view in &manifest.core.tree.l0 {
                assert!(compacted_ssts.contains(&view.sst.id));
            }

            for sr in &manifest.core.tree.compacted {
                for view in sr.sst_views() {
                    assert!(compacted_ssts.contains(&view.sst.id));
                }
            }
        }
    }

    async fn run_gc_once(
        manifest_store: Arc<ManifestStore>,
        compactions_store: Arc<CompactionsStore>,
        table_store: Arc<TableStore>,
        wal_store: Arc<WalTableStore>,
        compaction_low_watermark_dt: Option<DateTime<Utc>>,
    ) {
        run_gc_once_with_recorder(
            manifest_store,
            compactions_store,
            table_store,
            wal_store,
            compaction_low_watermark_dt,
            &MetricsRecorderHelper::noop(),
        )
        .await;
    }

    async fn run_gc_once_with_recorder(
        manifest_store: Arc<ManifestStore>,
        compactions_store: Arc<CompactionsStore>,
        table_store: Arc<TableStore>,
        wal_store: Arc<WalTableStore>,
        compaction_low_watermark_dt: Option<DateTime<Utc>>,
        recorder: &MetricsRecorderHelper,
    ) {
        // Pretend a compaction job has already run with the specified start time
        if let Some(compaction_low_watermark_dt) = compaction_low_watermark_dt {
            // Start by creating an empty .compactions file if it doesn't exist
            let mut stored_compactions = if let Some(stored_compactions) =
                StoredCompactions::try_load(compactions_store.clone())
                    .await
                    .expect("load failed")
            {
                stored_compactions
            } else {
                let manifest = StoredManifest::load(
                    manifest_store.clone(),
                    Arc::new(DefaultSystemClock::default()),
                )
                .await
                .expect("manifest load failed");
                StoredCompactions::create(
                    compactions_store.clone(),
                    manifest.manifest().compactor_epoch,
                )
                .await
                .expect("compactions creation failed")
            };
            // Now insert a compaction with the specified start time
            let mut compactions_dirty = stored_compactions.prepare_dirty().unwrap();
            compactions_dirty.value.insert(Compaction::new(
                ulid::Ulid::from_parts(compaction_low_watermark_dt.timestamp_millis() as u64, 0),
                CompactionSpec::new(vec![SourceId::SortedRun(0)], 0),
            ));
            stored_compactions.update(compactions_dirty).await.unwrap();
        }

        let gc_opts = GarbageCollectorOptions {
            manifest_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            wal_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            wal_fence_options: None,
            compacted_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            compactions_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            detach_options: None,
            metric_level: None,
            boundary_files_enabled: true,
            object_store_max_retries: None,
        };

        let gc = GarbageCollector::new(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            Arc::new(object_store::memory::InMemory::new()),
            gc_opts,
            recorder,
            Arc::new(DefaultSystemClock::default()),
            None,
            None,
        );

        gc.run_gc_once().await;

        // Verify reference integrity
        assert_no_dangling_references(manifest_store, table_store, wal_store).await;
    }

    #[tokio::test]
    async fn test_handle_should_only_run_one_task_per_message() {
        use crate::dispatcher::MessageHandler;

        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();

        // Create two manifests where the first is old enough to GC
        let mut stored_manifest = StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        stored_manifest
            .update(stored_manifest.prepare_dirty().unwrap())
            .await
            .unwrap();

        // Make manifest 1 eligible for deletion
        set_modified(
            local_object_store.clone(),
            &Path::from(format!("manifest/{:020}.{}", 1, "manifest")),
            86400,
        );

        // Sanity check initial manifests
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 2);

        // Build a GC with standard options (1h min_age)
        let recorder = MetricsRecorderHelper::noop();
        let gc_opts = GarbageCollectorOptions {
            manifest_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            wal_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            wal_fence_options: None,
            compacted_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            compactions_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            detach_options: None,
            metric_level: None,
            boundary_files_enabled: true,
            object_store_max_retries: None,
        };

        let mut gc = GarbageCollector::new(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            Arc::new(object_store::memory::InMemory::new()),
            gc_opts,
            &recorder,
            Arc::new(DefaultSystemClock::default()),
            None,
            None,
        );

        // Send a WAL GC message. Correct behavior: only WAL GC runs.
        // Current bug: handle() calls run_gc_once(), running all tasks (including Manifest GC).
        gc.handle(GcMessage::Wal).await.unwrap();

        // Assert that manifests were not collected (should still be 2).
        // With the bug, Manifest GC will have deleted the first manifest and this will fail.
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(
            manifests.len(),
            2,
            "manifest GC should not run on WAL message"
        );
    }

    #[tokio::test]
    async fn test_run_gc_once_skips_disabled_manifest_gc() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();

        let mut stored_manifest = StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        stored_manifest
            .update(stored_manifest.prepare_dirty().unwrap())
            .await
            .unwrap();

        set_modified(
            local_object_store.clone(),
            &Path::from(format!("manifest/{:020}.{}", 1, "manifest")),
            86400,
        );

        let recorder = MetricsRecorderHelper::noop();
        let gc_opts = GarbageCollectorOptions {
            manifest_options: None,
            wal_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            wal_fence_options: None,
            compacted_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            compactions_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            detach_options: None,
            metric_level: None,
            boundary_files_enabled: true,
            object_store_max_retries: None,
        };

        let gc = GarbageCollector::new(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            Arc::new(object_store::memory::InMemory::new()),
            gc_opts,
            &recorder,
            Arc::new(DefaultSystemClock::default()),
            None,
            None,
        );
        gc.run_gc_once().await;

        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(
            manifests.len(),
            2,
            "manifest GC should not run when manifest options are disabled"
        );
    }

    #[tokio::test]
    async fn test_tickers_should_skip_disabled_gc_tasks() {
        use crate::dispatcher::MessageHandler;

        let (manifest_store, compactions_store, table_store, wal_store, _) = build_objects();
        let recorder = MetricsRecorderHelper::noop();

        let gc_opts = GarbageCollectorOptions {
            manifest_options: None,
            wal_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: Some(Duration::from_secs(11)),
                dry_run: false,
            }),
            wal_fence_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: Some(Duration::from_secs(13)),
                dry_run: false,
            }),
            compacted_options: None,
            compactions_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: Some(Duration::from_secs(17)),
                dry_run: false,
            }),
            detach_options: None,
            metric_level: None,
            boundary_files_enabled: true,
            object_store_max_retries: None,
        };

        let mut gc = GarbageCollector::new(
            manifest_store,
            compactions_store,
            table_store,
            wal_store,
            Arc::new(object_store::memory::InMemory::new()),
            gc_opts,
            &recorder,
            Arc::new(DefaultSystemClock::default()),
            None,
            None,
        );

        let intervals: Vec<_> = gc.tickers().into_iter().map(|def| def.interval).collect();
        assert_eq!(
            intervals,
            vec![
                Duration::from_secs(11),
                Duration::from_secs(13),
                Duration::from_secs(17),
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn test_gc_shutdown() {
        let (manifest_store, compactions_store, table_store, wal_store, _) = build_objects();
        let recorder = MetricsRecorderHelper::noop();

        let gc_opts = GarbageCollectorOptions {
            manifest_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: Some(Duration::from_secs(1)),
                dry_run: false,
            }),
            wal_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: Some(Duration::from_secs(1)),
                dry_run: false,
            }),
            wal_fence_options: None,
            compacted_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: Some(Duration::from_secs(1)),
                dry_run: false,
            }),
            compactions_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: Some(Duration::from_secs(1)),
                dry_run: false,
            }),
            detach_options: None,
            metric_level: None,
            boundary_files_enabled: true,
            object_store_max_retries: None,
        };

        let gc = GarbageCollector::new(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            Arc::new(object_store::memory::InMemory::new()),
            gc_opts,
            &recorder,
            Arc::new(DefaultSystemClock::default()),
            None,
            None,
        );
        gc.start().expect("failed to start garbage collector");
        gc.stop().await.expect("failed to stop garbage collector");
    }

    #[tokio::test]
    async fn test_should_record_gc_manifest_deleted_count() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();

        // given: two manifests, first one old enough to GC
        let mut stored_manifest = StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        stored_manifest
            .update(stored_manifest.prepare_dirty().unwrap())
            .await
            .unwrap();
        set_modified(
            local_object_store,
            &Path::from(format!("manifest/{:020}.manifest", 1)),
            86400,
        );

        // when:
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(recorder.clone(), Default::default());
        run_gc_once_with_recorder(
            manifest_store,
            compactions_store,
            table_store,
            wal_store,
            None,
            &helper,
        )
        .await;

        // then:
        assert_eq!(
            lookup_metric_with_labels(&recorder, stats::DELETED_COUNT, &[("resource", "manifest")]),
            Some(1)
        );
    }

    #[tokio::test]
    async fn test_should_record_gc_wal_deleted_count() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();
        let path_resolver = PathResolver::from_root("/");

        // given: two WAL SSTs, first one old enough to GC
        let id1 = WalFileId::from(1);
        write_wal_sst(wal_store.clone(), id1).await.unwrap();
        let id2 = WalFileId::from(2);
        write_wal_sst(wal_store.clone(), id2).await.unwrap();
        set_modified(local_object_store, &path_resolver.wal_sst_path(&id1), 86400);

        let mut state = ManifestCore::new();
        state.replay_after_wal_id = id2.value();
        StoredManifest::create_new_db(
            manifest_store.clone(),
            state,
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        // when:
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(recorder.clone(), Default::default());
        run_gc_once_with_recorder(
            manifest_store,
            compactions_store,
            table_store,
            wal_store,
            None,
            &helper,
        )
        .await;

        // then:
        assert_eq!(
            lookup_metric_with_labels(&recorder, stats::DELETED_COUNT, &[("resource", "wal")]),
            Some(1)
        );
    }

    #[tokio::test]
    async fn test_should_record_gc_compacted_deleted_count() {
        let (manifest_store, compactions_store, table_store, wal_store, _) = build_objects();
        let now = DefaultSystemClock::default().now();
        let expired_ms = (now - TimeDelta::seconds(7200)).timestamp_millis() as u64;
        let unexpired_ms = (now - TimeDelta::seconds(1800)).timestamp_millis() as u64;

        // given: one active L0 SST, one active sorted-run SST, one inactive expired SST
        let active_l0_handle = create_sst(table_store.clone(), unexpired_ms).await;
        let active_handle = create_sst(table_store.clone(), unexpired_ms + 1).await;
        let inactive_expired_handle = create_sst(table_store.clone(), expired_ms).await;

        let mut state = ManifestCore::new();
        Arc::make_mut(&mut state.tree)
            .l0
            .push_back(SsTableView::identity(active_l0_handle));
        Arc::make_mut(&mut state.tree)
            .compacted
            .push(SortedRun::new(1, [SsTableView::identity(active_handle)]));
        // inactive_expired_handle is NOT in manifest -> eligible for GC
        StoredManifest::create_new_db(
            manifest_store.clone(),
            state,
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        // when:
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(recorder.clone(), Default::default());
        run_gc_once_with_recorder(
            manifest_store,
            compactions_store,
            table_store.clone(),
            wal_store,
            Some(now),
            &helper,
        )
        .await;

        // then: the inactive expired SST should be deleted
        let remaining: HashSet<_> = table_store
            .list_compacted_ssts(..)
            .await
            .unwrap()
            .iter()
            .map(|s| s.id)
            .collect();
        assert!(!remaining.contains(&inactive_expired_handle.id));
        assert_eq!(
            lookup_metric_with_labels(
                &recorder,
                stats::DELETED_COUNT,
                &[("resource", "compacted")]
            ),
            Some(1)
        );
    }

    #[tokio::test]
    async fn test_should_record_gc_compactions_deleted_count() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();

        // given: create a manifest and three compaction files, age them all
        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let mut stored_compactions = StoredCompactions::create(compactions_store.clone(), 0)
            .await
            .unwrap();
        stored_compactions
            .update(stored_compactions.prepare_dirty().unwrap())
            .await
            .unwrap();
        stored_compactions
            .update(stored_compactions.prepare_dirty().unwrap())
            .await
            .unwrap();

        for id in 1..=3 {
            set_modified(
                local_object_store.clone(),
                &Path::from(format!("compactions/{:020}.compactions", id)),
                86400,
            );
        }
        assert_eq!(
            compactions_store.list_compactions(..).await.unwrap().len(),
            3
        );

        // when:
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(recorder.clone(), Default::default());
        run_gc_once_with_recorder(
            manifest_store,
            compactions_store.clone(),
            table_store,
            wal_store,
            None,
            &helper,
        )
        .await;

        // then: two old compaction files deleted, latest kept
        assert_eq!(
            compactions_store.list_compactions(..).await.unwrap().len(),
            1
        );
        assert_eq!(
            lookup_metric_with_labels(
                &recorder,
                stats::DELETED_COUNT,
                &[("resource", "compactions")]
            ),
            Some(2)
        );
    }

    #[tokio::test]
    async fn test_gc_filter_can_reject_all_directory_gc_deletes() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();
        let path_resolver = PathResolver::from_root("/");
        let now = DefaultSystemClock::default().now();
        let expired_ms = (now - TimeDelta::seconds(7200)).timestamp_millis() as u64;
        let unexpired_ms = (now - TimeDelta::seconds(1800)).timestamp_millis() as u64;

        // Create regular WALs and a fence WAL, then age the GC-eligible ones.
        let old_wal_id = WalFileId::from(1);
        write_wal_sst(wal_store.clone(), old_wal_id).await.unwrap();
        let recent_wal_id = WalFileId::from(2);
        write_wal_sst(wal_store.clone(), recent_wal_id)
            .await
            .unwrap();
        let old_fence_id = WalFileId::from(3);
        write_wal_fence(wal_store.clone(), 3).await;

        set_modified(
            local_object_store.clone(),
            &path_resolver.wal_sst_path(&old_wal_id),
            86400,
        );
        set_modified(
            local_object_store.clone(),
            &path_resolver.wal_sst_path(&old_fence_id),
            86400,
        );

        // Create one inactive compacted SST that would normally be collected.
        let inactive_expired_handle = create_sst(table_store.clone(), expired_ms).await;
        let active_l0_handle = create_sst(table_store.clone(), unexpired_ms).await;

        // Create two manifests so the older manifest is eligible for manifest GC.
        let mut state = ManifestCore::new();
        state.replay_after_wal_id = 4;
        state.next_wal_sst_id = 5;
        Arc::make_mut(&mut state.tree)
            .l0
            .push_back(SsTableView::identity(active_l0_handle));
        let mut stored_manifest = StoredManifest::create_new_db(
            manifest_store.clone(),
            state,
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        stored_manifest
            .update(stored_manifest.prepare_dirty().unwrap())
            .await
            .unwrap();
        set_modified(
            local_object_store.clone(),
            &Path::from(format!("manifest/{:020}.manifest", 1)),
            86400,
        );

        // Create compactions files and age all of them so every non-latest file is eligible.
        let mut stored_compactions = StoredCompactions::create(
            compactions_store.clone(),
            stored_manifest.manifest().compactor_epoch,
        )
        .await
        .unwrap();
        let mut compactions_dirty = stored_compactions.prepare_dirty().unwrap();
        compactions_dirty.value.insert(Compaction::new(
            ulid::Ulid::from_parts(now.timestamp_millis() as u64, 0),
            CompactionSpec::new(vec![SourceId::SortedRun(0)], 0),
        ));
        stored_compactions.update(compactions_dirty).await.unwrap();

        let compaction_ids = compactions_store
            .list_compactions(..)
            .await
            .unwrap()
            .into_iter()
            .map(|metadata| metadata.id)
            .collect::<Vec<_>>();
        for id in &compaction_ids {
            set_modified(
                local_object_store.clone(),
                &Path::from(format!("compactions/{id:020}.compactions")),
                86400,
            );
        }

        // Use an empty location filter to reject every GC candidate.
        let options = GarbageCollectorDirectoryOptions {
            min_age: Duration::from_secs(3600),
            interval: None,
            dry_run: false,
        };
        let gc_opts = GarbageCollectorOptions {
            manifest_options: Some(options),
            wal_options: Some(options),
            wal_fence_options: Some(options),
            compacted_options: Some(options),
            compactions_options: Some(options),
            detach_options: None,
            metric_level: None,
            boundary_files_enabled: true,
            object_store_max_retries: None,
        };
        let recorder = MetricsRecorderHelper::noop();
        let gc = GarbageCollector::new(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            Arc::new(object_store::memory::InMemory::new()),
            gc_opts,
            &recorder,
            Arc::new(DefaultSystemClock::default()),
            Some(Arc::new(LocationGcFilter {
                allowed_locations: HashSet::new(),
            })),
            None,
        );

        // Run every directory GC task with candidates present for each task type.
        gc.run_gc_once().await;

        // The filter should prevent manifest deletion.
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 2);

        // The filter should prevent both WAL and fence WAL deletion.
        let wal_ids = wal_store
            .list_wal_ssts(..)
            .await
            .unwrap()
            .into_iter()
            .map(|metadata| metadata.id)
            .collect::<HashSet<_>>();
        assert!(wal_ids.contains(&old_wal_id));
        assert!(wal_ids.contains(&recent_wal_id));
        assert!(wal_ids.contains(&old_fence_id));

        // The filter should prevent compacted SST deletion.
        let compacted_ids = table_store
            .list_compacted_ssts(..)
            .await
            .unwrap()
            .into_iter()
            .map(|metadata| metadata.id)
            .collect::<HashSet<_>>();
        assert!(compacted_ids.contains(&inactive_expired_handle.id));

        // The filter should prevent compactions file deletion.
        assert_eq!(
            compactions_store.list_compactions(..).await.unwrap().len(),
            compaction_ids.len()
        );
    }

    #[tokio::test]
    async fn test_gc_filter_allows_subset_and_stats_count_successful_deletes() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();
        let path_resolver = PathResolver::from_root("/");

        // Create three old WALs below the replay boundary so all would be eligible
        // without a filter. The middle one is the only filter-approved delete.
        let rejected_before_wal_id = WalFileId::from(1);
        let allowed_wal_id = WalFileId::from(2);
        let rejected_after_wal_id = WalFileId::from(3);
        for id in [
            rejected_before_wal_id,
            allowed_wal_id,
            rejected_after_wal_id,
        ] {
            write_wal_sst(wal_store.clone(), id).await.unwrap();
            set_modified(
                local_object_store.clone(),
                &path_resolver.wal_sst_path(&id),
                86400,
            );
        }

        // Keep the replay boundary above every test WAL so boundary retention
        // cannot explain any remaining WAL.
        let mut state = ManifestCore::new();
        state.replay_after_wal_id = 4;
        StoredManifest::create_new_db(
            manifest_store.clone(),
            state,
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let gc_opts = GarbageCollectorOptions {
            manifest_options: None,
            wal_options: Some(GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600),
                interval: None,
                dry_run: false,
            }),
            wal_fence_options: None,
            compacted_options: None,
            compactions_options: None,
            detach_options: None,
            metric_level: None,
            boundary_files_enabled: true,
            object_store_max_retries: None,
        };
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(recorder.clone(), Default::default());
        let gc = GarbageCollector::new(
            manifest_store.clone(),
            compactions_store,
            table_store.clone(),
            wal_store.clone(),
            Arc::new(object_store::memory::InMemory::new()),
            gc_opts,
            &helper,
            Arc::new(DefaultSystemClock::default()),
            Some(Arc::new(LocationGcFilter {
                allowed_locations: HashSet::from([path_resolver.wal_sst_path(&allowed_wal_id)]),
            })),
            None,
        );

        // Run WAL GC with a filter that permits only one of the eligible WALs.
        gc.run_gc_once().await;

        // Only the two filter-rejected WALs should remain, and stats should count one delete.
        let wal_ids = wal_store
            .list_wal_ssts(..)
            .await
            .unwrap()
            .into_iter()
            .map(|metadata| metadata.id)
            .collect::<Vec<_>>();
        assert_eq!(wal_ids, vec![rejected_before_wal_id, rejected_after_wal_id]);
        assert_eq!(
            lookup_metric_with_labels(&recorder, stats::DELETED_COUNT, &[("resource", "wal")]),
            Some(1)
        );
    }

    #[tokio::test]
    async fn test_dry_run_skips_directory_gc_deletes() {
        let (manifest_store, compactions_store, table_store, wal_store, local_object_store) =
            build_objects();
        let path_resolver = PathResolver::from_root("/");
        let now = DefaultSystemClock::default().now();
        let expired_ms = (now - TimeDelta::seconds(7200)).timestamp_millis() as u64;
        let unexpired_ms = (now - TimeDelta::seconds(1800)).timestamp_millis() as u64;

        let old_wal_id = WalFileId::from(1);
        write_wal_sst(wal_store.clone(), old_wal_id).await.unwrap();
        let recent_wal_id = WalFileId::from(2);
        write_wal_sst(wal_store.clone(), recent_wal_id)
            .await
            .unwrap();
        let old_fence_id = WalFileId::from(3);
        write_wal_fence(wal_store.clone(), 3).await;

        set_modified(
            local_object_store.clone(),
            &path_resolver.wal_sst_path(&old_wal_id),
            86400,
        );
        set_modified(
            local_object_store.clone(),
            &path_resolver.wal_sst_path(&old_fence_id),
            86400,
        );

        let inactive_expired_handle = create_sst(table_store.clone(), expired_ms).await;
        let active_l0_handle = create_sst(table_store.clone(), unexpired_ms).await;

        let mut state = ManifestCore::new();
        state.replay_after_wal_id = 4;
        state.next_wal_sst_id = 5;
        Arc::make_mut(&mut state.tree)
            .l0
            .push_back(SsTableView::identity(active_l0_handle));
        let mut stored_manifest = StoredManifest::create_new_db(
            manifest_store.clone(),
            state,
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        stored_manifest
            .update(stored_manifest.prepare_dirty().unwrap())
            .await
            .unwrap();
        set_modified(
            local_object_store.clone(),
            &Path::from(format!("manifest/{:020}.manifest", 1)),
            86400,
        );

        let mut stored_compactions = StoredCompactions::create(
            compactions_store.clone(),
            stored_manifest.manifest().compactor_epoch,
        )
        .await
        .unwrap();
        let mut compactions_dirty = stored_compactions.prepare_dirty().unwrap();
        compactions_dirty.value.insert(Compaction::new(
            ulid::Ulid::from_parts(now.timestamp_millis() as u64, 0),
            CompactionSpec::new(vec![SourceId::SortedRun(0)], 0),
        ));
        stored_compactions.update(compactions_dirty).await.unwrap();

        let compaction_ids = compactions_store
            .list_compactions(..)
            .await
            .unwrap()
            .into_iter()
            .map(|metadata| metadata.id)
            .collect::<Vec<_>>();
        for id in &compaction_ids {
            set_modified(
                local_object_store.clone(),
                &Path::from(format!("compactions/{id:020}.compactions")),
                86400,
            );
        }

        let dry_run_options = GarbageCollectorDirectoryOptions {
            min_age: Duration::from_secs(3600),
            interval: None,
            dry_run: true,
        };
        let gc_opts = GarbageCollectorOptions {
            manifest_options: Some(dry_run_options),
            wal_options: Some(dry_run_options),
            wal_fence_options: Some(dry_run_options),
            compacted_options: Some(dry_run_options),
            compactions_options: Some(dry_run_options),
            detach_options: None,
            metric_level: None,
            boundary_files_enabled: true,
            object_store_max_retries: None,
        };
        let recorder = MetricsRecorderHelper::noop();
        let gc = GarbageCollector::new(
            manifest_store.clone(),
            compactions_store.clone(),
            table_store.clone(),
            wal_store.clone(),
            Arc::new(object_store::memory::InMemory::new()),
            gc_opts,
            &recorder,
            Arc::new(DefaultSystemClock::default()),
            None,
            None,
        );

        gc.run_gc_once().await;

        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 2);

        let wal_ids = wal_store
            .list_wal_ssts(..)
            .await
            .unwrap()
            .into_iter()
            .map(|metadata| metadata.id)
            .collect::<HashSet<_>>();
        assert!(wal_ids.contains(&old_wal_id));
        assert!(wal_ids.contains(&recent_wal_id));
        assert!(wal_ids.contains(&old_fence_id));

        let compacted_ids = table_store
            .list_compacted_ssts(..)
            .await
            .unwrap()
            .into_iter()
            .map(|metadata| metadata.id)
            .collect::<HashSet<_>>();
        assert!(compacted_ids.contains(&inactive_expired_handle.id));

        assert_eq!(
            compactions_store.list_compactions(..).await.unwrap().len(),
            compaction_ids.len()
        );
    }
}
