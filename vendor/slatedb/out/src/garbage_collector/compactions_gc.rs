//! Garbage collection for `.compactions` files.
//!
//! The compactions store is a versioned log of compactor state snapshots. GC can
//! safely delete older versions because only the latest record is required for:
//! - establishing the compaction low-watermark (to prevent deleting in-flight outputs),
//! - fencing semantics (epoch checks on the newest record).
//!
//! Policy:
//! - Always retain the most recent `.compactions` file.
//! - Only delete files older than the configured `min_age`.
//!
//! Safety:
//! - Deleting old versions does not affect recovery because the newest record
//!   contains the authoritative compactor epoch and retained compaction state.
//! - This task does not inspect compaction contents; it is purely time-based and
//!   version-aware (keeps the latest).
//!
//! Errors are logged and the task continues; stats are updated only on successful
//! deletes.

use crate::{
    compactions_store::CompactionsStore, config::GarbageCollectorDirectoryOptions,
    error::SlateDBError,
};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use log::error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::filter::retain_allowed_by_gc_filter;
use super::{GcFilter, GcStats, GcTask, GC_DELETE_CONCURRENCY};

#[derive(Clone)]
pub(crate) struct CompactionsGcTask {
    compactions_store: Arc<CompactionsStore>,
    stats: Arc<GcStats>,
    compactions_options: GarbageCollectorDirectoryOptions,
    gc_filter: Option<Arc<dyn GcFilter>>,
    boundary_files_enabled: bool,
}

impl std::fmt::Debug for CompactionsGcTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompactionsGcTask")
            .field("compactions_options", &self.compactions_options)
            .field("boundary_files_enabled", &self.boundary_files_enabled)
            .finish()
    }
}

impl CompactionsGcTask {
    pub(super) fn new(
        compactions_store: Arc<CompactionsStore>,
        stats: Arc<GcStats>,
        compactions_options: GarbageCollectorDirectoryOptions,
        gc_filter: Option<Arc<dyn GcFilter>>,
        boundary_files_enabled: bool,
    ) -> Self {
        Self {
            compactions_store,
            stats,
            compactions_options,
            gc_filter,
            boundary_files_enabled,
        }
    }

    fn compactions_min_age(&self) -> chrono::Duration {
        chrono::Duration::from_std(self.compactions_options.min_age).expect("invalid duration")
    }

    /// Deletes the given compactions files from the compactions store.
    ///
    /// In case of dryrun, the actual deletion doesn't happen.
    ///
    /// Returns the number of compactions files actually deleted.
    async fn maybe_delete_compactions(&self, compactions_ids: Vec<u64>) -> u64 {
        if self.compactions_options.dry_run {
            if !compactions_ids.is_empty() {
                log::info!(
                    "dry run: skipping compactions deletion [count={}]",
                    compactions_ids.len()
                );
            }
            for id in compactions_ids {
                log::debug!(
                    "dry run: would delete compactions but skipped [id={:?}]",
                    id
                );
            }
            return 0;
        }

        let deleted_count = AtomicU64::new(0);
        futures::stream::iter(compactions_ids)
            .for_each_concurrent(GC_DELETE_CONCURRENCY, |id| {
                let deleted_count = &deleted_count;
                async move {
                    if let Err(e) = self
                        .compactions_store
                        .delete_compactions_unchecked(id)
                        .await
                    {
                        error!("error deleting compactions [id={:?}, error={}]", id, e);
                    } else {
                        self.stats.gc_compactions_count.increment(1);
                        deleted_count.fetch_add(1, Ordering::Relaxed);
                    }
                }
            })
            .await;
        deleted_count.load(Ordering::Relaxed)
    }
}

impl GcTask for CompactionsGcTask {
    /// Collect garbage from the compactions store. This will delete any compactions files
    /// that are older than the minimum age specified in the options, excluding the latest
    /// compactions file.
    async fn collect(&self, utc_now: DateTime<Utc>) -> Result<(), SlateDBError> {
        let min_age = self.compactions_min_age();
        let mut compactions_metadata_list = self.compactions_store.list_compactions(..).await?;
        let pre_gc_count = compactions_metadata_list.len() as u64;

        // Remove the last element so we never delete the latest compactions file
        compactions_metadata_list.pop();

        // Delete compactions files older than min_age
        let compactions_to_delete = compactions_metadata_list
            .into_iter()
            .filter(|compactions_metadata| {
                utc_now.signed_duration_since(compactions_metadata.metadata.last_modified) > min_age
            })
            .collect::<Vec<_>>();

        self.stats.gc_compactions_versions.set(pre_gc_count as i64);

        // Advance the boundary to the latest compactions file selected by the GC model. The
        // optional GC filter only gates the final deletion pass.
        if self.boundary_files_enabled {
            if let Some(boundary) = compactions_to_delete
                .iter()
                .map(|compactions_metadata| compactions_metadata.id)
                .max()
            {
                self.compactions_store.advance_boundary(boundary).await?;
            }
        }
        let compactions_to_delete =
            retain_allowed_by_gc_filter(&self.gc_filter, compactions_to_delete).await;
        let compactions_ids_to_delete = compactions_to_delete
            .into_iter()
            .map(|compactions_metadata| compactions_metadata.id)
            .collect::<Vec<_>>();

        let deleted_count = self
            .maybe_delete_compactions(compactions_ids_to_delete)
            .await;

        if deleted_count > 0 {
            self.stats
                .gc_compactions_versions
                .set((pre_gc_count - deleted_count) as i64);
        }

        Ok(())
    }

    fn resource(&self) -> &str {
        "Compactions"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compactions_store::{CompactionsStore, StoredCompactions};
    use async_trait::async_trait;
    use chrono::TimeDelta;
    use object_store::{local::LocalFileSystem, memory::InMemory, path::Path, ObjectStoreExt};
    use slatedb_common::metrics::{
        lookup_metric_with_labels, DefaultMetricsRecorder, MetricsRecorderHelper,
    };
    use slatedb_common::ObjectMetadata;
    use std::collections::HashSet;
    use std::time::Duration;

    struct DenyAllGcFilter;

    #[async_trait]
    impl GcFilter for DenyAllGcFilter {
        async fn filter(&self, _candidates: HashSet<ObjectMetadata>) -> HashSet<ObjectMetadata> {
            HashSet::new()
        }
    }

    #[tokio::test]
    async fn test_collect_advances_boundary_for_old_compactions_files() {
        let object_store = Arc::new(InMemory::new());
        let compactions_store = Arc::new(CompactionsStore::new(
            &Path::from("/root"),
            object_store.clone(),
        ));
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

        let recorder = MetricsRecorderHelper::noop();
        let task = CompactionsGcTask::new(
            compactions_store.clone(),
            Arc::new(GcStats::new(&recorder)),
            GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(1),
                interval: None,
                dry_run: false,
            },
            None,
            true,
        );
        task.collect(Utc::now() + TimeDelta::hours(1))
            .await
            .unwrap();

        let raw_boundary = object_store
            .get(&Path::from("/root/gc/compactions.boundary"))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!("2", std::str::from_utf8(&raw_boundary).unwrap());

        let compactions = compactions_store.list_compactions(..).await.unwrap();
        assert_eq!(
            vec![3],
            compactions
                .iter()
                .map(|compactions| compactions.id)
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn test_collect_without_boundary_advancement_deletes_and_preserves_boundary() {
        let object_store = Arc::new(InMemory::new());
        let compactions_store = Arc::new(CompactionsStore::new(
            &Path::from("/root"),
            object_store.clone(),
        ));
        let mut stored_compactions = StoredCompactions::create(compactions_store.clone(), 0)
            .await
            .unwrap();
        stored_compactions
            .update(stored_compactions.prepare_dirty().unwrap())
            .await
            .unwrap();
        compactions_store.advance_boundary(1).await.unwrap();
        stored_compactions
            .update(stored_compactions.prepare_dirty().unwrap())
            .await
            .unwrap();

        let recorder = MetricsRecorderHelper::noop();
        let task = CompactionsGcTask::new(
            compactions_store.clone(),
            Arc::new(GcStats::new(&recorder)),
            GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(1),
                interval: None,
                dry_run: false,
            },
            None,
            false,
        );
        task.collect(Utc::now() + TimeDelta::hours(1))
            .await
            .unwrap();

        let raw_boundary = object_store
            .get(&Path::from("/root/gc/compactions.boundary"))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!("1", std::str::from_utf8(&raw_boundary).unwrap());
        let compactions = compactions_store.list_compactions(..).await.unwrap();
        assert_eq!(
            vec![3],
            compactions
                .iter()
                .map(|compactions| compactions.id)
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn test_collect_advances_boundary_before_filtering_compactions_files() {
        let object_store = Arc::new(InMemory::new());
        let compactions_store = Arc::new(CompactionsStore::new(
            &Path::from("/root"),
            object_store.clone(),
        ));
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

        let recorder = MetricsRecorderHelper::noop();
        let task = CompactionsGcTask::new(
            compactions_store.clone(),
            Arc::new(GcStats::new(&recorder)),
            GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(1),
                interval: None,
                dry_run: false,
            },
            Some(Arc::new(DenyAllGcFilter) as Arc<dyn GcFilter>),
            true,
        );
        task.collect(Utc::now() + TimeDelta::hours(1))
            .await
            .unwrap();

        let raw_boundary = object_store
            .get(&Path::from("/root/gc/compactions.boundary"))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!("2", std::str::from_utf8(&raw_boundary).unwrap());

        assert!(compactions_store
            .try_read_compactions(1)
            .await
            .unwrap()
            .is_some());
        assert!(compactions_store
            .try_read_compactions(2)
            .await
            .unwrap()
            .is_some());
    }

    async fn make_compactions_store() -> (Arc<CompactionsStore>, StoredCompactions) {
        let object_store = Arc::new(InMemory::new());
        let compactions_store = Arc::new(CompactionsStore::new(&Path::from("/root"), object_store));
        let stored = StoredCompactions::create(compactions_store.clone(), 0)
            .await
            .unwrap();
        (compactions_store, stored)
    }

    #[tokio::test]
    async fn test_version_count_after_gc_deletes_old_compactions() {
        let (compactions_store, mut stored) = make_compactions_store().await;
        // Write two more compactions files: ids 1 (create), 2, 3
        stored
            .update(stored.prepare_dirty().unwrap())
            .await
            .unwrap();
        stored
            .update(stored.prepare_dirty().unwrap())
            .await
            .unwrap();

        let metrics = Arc::new(DefaultMetricsRecorder::new());
        let recorder = MetricsRecorderHelper::new(metrics.clone(), Default::default());
        let task = CompactionsGcTask::new(
            compactions_store.clone(),
            Arc::new(GcStats::new(&recorder)),
            GarbageCollectorDirectoryOptions {
                min_age: Duration::ZERO,
                interval: None,
                dry_run: false,
            },
            None,
            true,
        );

        task.collect(Utc::now() + TimeDelta::hours(1))
            .await
            .unwrap();

        // GC deletes compactions 1 and 2 (older than min_age=0); compactions 3 survives as latest.
        assert_eq!(
            lookup_metric_with_labels(
                &metrics,
                crate::garbage_collector::stats::VERSION_COUNT,
                &[("resource", "compactions")]
            ),
            Some(1),
            "expected 1 surviving compactions file after GC"
        );
    }

    #[tokio::test]
    async fn test_version_count_when_nothing_to_delete() {
        let (compactions_store, mut stored) = make_compactions_store().await;
        stored
            .update(stored.prepare_dirty().unwrap())
            .await
            .unwrap();
        stored
            .update(stored.prepare_dirty().unwrap())
            .await
            .unwrap();

        let metrics = Arc::new(DefaultMetricsRecorder::new());
        let recorder = MetricsRecorderHelper::new(metrics.clone(), Default::default());
        let task = CompactionsGcTask::new(
            compactions_store.clone(),
            Arc::new(GcStats::new(&recorder)),
            GarbageCollectorDirectoryOptions {
                min_age: Duration::from_secs(3600), // too new to delete
                interval: None,
                dry_run: false,
            },
            None,
            true,
        );

        task.collect(Utc::now()).await.unwrap();

        // Nothing deleted — all 3 compactions files survive.
        assert_eq!(
            lookup_metric_with_labels(
                &metrics,
                crate::garbage_collector::stats::VERSION_COUNT,
                &[("resource", "compactions")]
            ),
            Some(3),
            "expected all 3 compactions files when nothing qualifies for deletion"
        );
    }

    #[tokio::test]
    async fn test_version_count_refreshes_when_boundary_advance_fails() {
        let tempdir = tempfile::tempdir().unwrap();
        let object_store = Arc::new(LocalFileSystem::new_with_prefix(tempdir.path()).unwrap());
        let compactions_store = Arc::new(CompactionsStore::new(
            &Path::from("/root"),
            object_store.clone(),
        ));
        let mut stored = StoredCompactions::create(compactions_store.clone(), 0)
            .await
            .unwrap();
        stored
            .update(stored.prepare_dirty().unwrap())
            .await
            .unwrap();
        stored
            .update(stored.prepare_dirty().unwrap())
            .await
            .unwrap();
        compactions_store.advance_boundary(1).await.unwrap();

        let metrics = Arc::new(DefaultMetricsRecorder::new());
        let recorder = MetricsRecorderHelper::new(metrics.clone(), Default::default());
        let stats = Arc::new(GcStats::new(&recorder));
        stats.gc_compactions_versions.set(-1);
        let task = CompactionsGcTask::new(
            compactions_store.clone(),
            stats,
            GarbageCollectorDirectoryOptions {
                min_age: Duration::ZERO,
                interval: None,
                dry_run: false,
            },
            None,
            true,
        );

        let error = task
            .collect(Utc::now() + TimeDelta::hours(1))
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            SlateDBError::ObjectStoreError(error)
                if matches!(error.as_ref(), object_store::Error::NotImplemented { .. })
        ));

        let raw_boundary = object_store
            .get(&Path::from("/root/gc/compactions.boundary"))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!("1", std::str::from_utf8(&raw_boundary).unwrap());
        assert_eq!(
            compactions_store
                .list_compactions(..)
                .await
                .unwrap()
                .iter()
                .map(|compactions| compactions.id)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            lookup_metric_with_labels(
                &metrics,
                crate::garbage_collector::stats::VERSION_COUNT,
                &[("resource", "compactions")]
            ),
            Some(3)
        );
    }

    #[tokio::test]
    async fn test_version_count_unchanged_on_dry_run() {
        let (compactions_store, mut stored) = make_compactions_store().await;
        // Write two more compactions files: ids 1 (create), 2, 3
        stored
            .update(stored.prepare_dirty().unwrap())
            .await
            .unwrap();
        stored
            .update(stored.prepare_dirty().unwrap())
            .await
            .unwrap();

        let metrics = Arc::new(DefaultMetricsRecorder::new());
        let recorder = MetricsRecorderHelper::new(metrics.clone(), Default::default());
        let task = CompactionsGcTask::new(
            compactions_store.clone(),
            Arc::new(GcStats::new(&recorder)),
            GarbageCollectorDirectoryOptions {
                min_age: Duration::ZERO,
                interval: None,
                dry_run: true,
            },
            None,
            true,
        );

        task.collect(Utc::now() + TimeDelta::hours(1))
            .await
            .unwrap();

        // Dry run deletes nothing, so all 3 compactions files still exist and the gauge
        // reports the true current count rather than the hypothetical post-deletion count.
        assert_eq!(
            lookup_metric_with_labels(
                &metrics,
                crate::garbage_collector::stats::VERSION_COUNT,
                &[("resource", "compactions")]
            ),
            Some(3),
            "expected dry run to leave the gauge at the true current count"
        );
        let compactions = compactions_store.list_compactions(..).await.unwrap();
        assert_eq!(
            compactions.len(),
            3,
            "dry run should not delete any compactions files"
        );
    }
}
