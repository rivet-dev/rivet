use crate::{
    config::GarbageCollectorDirectoryOptions, error::SlateDBError, manifest::store::ManifestStore,
};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use log::error;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::filter::retain_allowed_by_gc_filter;
use super::{GcFilter, GcStats, GcTask, GC_DELETE_CONCURRENCY};

#[derive(Clone)]
pub(crate) struct ManifestGcTask {
    manifest_store: Arc<ManifestStore>,
    stats: Arc<GcStats>,
    manifest_options: GarbageCollectorDirectoryOptions,
    gc_filter: Option<Arc<dyn GcFilter>>,
    boundary_files_enabled: bool,
}

impl std::fmt::Debug for ManifestGcTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManifestGcTask")
            .field("manifest_options", &self.manifest_options)
            .field("boundary_files_enabled", &self.boundary_files_enabled)
            .finish()
    }
}

impl ManifestGcTask {
    pub(super) fn new(
        manifest_store: Arc<ManifestStore>,
        stats: Arc<GcStats>,
        manifest_options: GarbageCollectorDirectoryOptions,
        gc_filter: Option<Arc<dyn GcFilter>>,
        boundary_files_enabled: bool,
    ) -> Self {
        ManifestGcTask {
            manifest_store,
            stats,
            manifest_options,
            gc_filter,
            boundary_files_enabled,
        }
    }

    fn manifest_min_age(&self) -> chrono::Duration {
        chrono::Duration::from_std(self.manifest_options.min_age).expect("invalid duration")
    }

    /// Deletes the given manifests from the manifest store.
    ///
    /// In case of dryrun, the actual deletion doesn't happen.
    ///
    /// Returns the number of manifests actually deleted.
    async fn maybe_delete_manifests(&self, manifest_ids: Vec<u64>) -> u64 {
        if self.manifest_options.dry_run {
            if !manifest_ids.is_empty() {
                log::info!(
                    "dry run: skipping manifest deletion [count={}]",
                    manifest_ids.len()
                );
            }
            for id in manifest_ids {
                log::debug!("dry run: would delete manifest but skipped [id={:?}]", id);
            }
            return 0;
        }

        let deleted_count = AtomicU64::new(0);
        futures::stream::iter(manifest_ids)
            .for_each_concurrent(GC_DELETE_CONCURRENCY, |id| {
                let deleted_count = &deleted_count;
                async move {
                    if let Err(e) = self.manifest_store.delete_manifest_unchecked(id).await {
                        error!("error deleting manifest [id={:?}, error={}]", id, e);
                    } else {
                        self.stats.gc_manifest_count.increment(1);
                        deleted_count.fetch_add(1, Ordering::Relaxed);
                    }
                }
            })
            .await;
        deleted_count.load(Ordering::Relaxed)
    }
}

impl GcTask for ManifestGcTask {
    /// Collect garbage from the manifest store. This will delete any manifests
    /// that are older than the minimum age specified in the options.
    async fn collect(&self, utc_now: DateTime<Utc>) -> Result<(), SlateDBError> {
        let min_age = self.manifest_min_age();
        let mut manifest_metadata_list = self.manifest_store.list_manifests(..).await?;

        // Remove the last element so we never delete the latest manifest
        let latest_manifest = if let Some(manifest_metadata) = manifest_metadata_list.pop() {
            self.manifest_store
                .read_manifest(manifest_metadata.id)
                .await?
        } else {
            return Err(SlateDBError::LatestTransactionalObjectVersionMissing);
        };

        // Do not delete manifests which are still referenced by active checkpoints
        let active_manifest_ids: HashSet<_> = latest_manifest
            .core
            .checkpoints
            .iter()
            .map(|checkpoint| checkpoint.manifest_id)
            .collect();

        // Delete manifests older than min_age
        // Capture length before into_iter() consumes the list; +1 re-adds the popped latest.
        let pre_gc_count = manifest_metadata_list.len() as u64 + 1;
        let manifests_to_delete = manifest_metadata_list
            .into_iter()
            .filter(|manifest_metadata| {
                let is_active = active_manifest_ids.contains(&manifest_metadata.id);
                !is_active
                    && utc_now.signed_duration_since(manifest_metadata.metadata.last_modified)
                        > min_age
            })
            .collect::<Vec<_>>();

        self.stats.gc_manifest_versions.set(pre_gc_count as i64);

        // Advance the boundary to the latest manifest selected by the GC model. The optional GC
        // filter only gates the final deletion pass.
        if self.boundary_files_enabled {
            if let Some(boundary) = manifests_to_delete
                .iter()
                .map(|manifest_metadata| manifest_metadata.id)
                .max()
            {
                self.manifest_store.advance_boundary(boundary).await?;
            }
        }
        let manifests_to_delete =
            retain_allowed_by_gc_filter(&self.gc_filter, manifests_to_delete).await;
        let manifest_ids_to_delete = manifests_to_delete
            .into_iter()
            .map(|manifest_metadata| manifest_metadata.id)
            .collect::<Vec<_>>();

        let deleted_count = self.maybe_delete_manifests(manifest_ids_to_delete).await;

        if deleted_count > 0 {
            self.stats
                .gc_manifest_versions
                .set((pre_gc_count - deleted_count) as i64);
        }

        Ok(())
    }

    fn resource(&self) -> &str {
        "Manifest"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{
        store::{ManifestStore, StoredManifest},
        ManifestCore,
    };
    use async_trait::async_trait;
    use chrono::TimeDelta;
    use object_store::{local::LocalFileSystem, memory::InMemory, path::Path, ObjectStoreExt};
    use slatedb_common::clock::DefaultSystemClock;
    use slatedb_common::metrics::{
        lookup_metric_with_labels, DefaultMetricsRecorder, MetricsRecorderHelper,
    };
    use slatedb_common::ObjectMetadata;
    use std::time::Duration;

    struct DenyAllGcFilter;

    #[async_trait]
    impl GcFilter for DenyAllGcFilter {
        async fn filter(&self, _candidates: HashSet<ObjectMetadata>) -> HashSet<ObjectMetadata> {
            HashSet::new()
        }
    }

    #[tokio::test]
    async fn test_collect_advances_boundary_for_old_manifest_files() {
        let object_store = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from("/root"),
            object_store.clone(),
        ));
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
        stored_manifest
            .update(stored_manifest.prepare_dirty().unwrap())
            .await
            .unwrap();

        let recorder = MetricsRecorderHelper::noop();
        let task = ManifestGcTask::new(
            manifest_store.clone(),
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
            .get(&Path::from("/root/gc/manifest.boundary"))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!("2", std::str::from_utf8(&raw_boundary).unwrap());

        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(
            vec![3],
            manifests
                .iter()
                .map(|manifest| manifest.id)
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn test_collect_without_boundary_advancement_deletes_without_creating_boundary() {
        let object_store = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from("/root"),
            object_store.clone(),
        ));
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
        stored_manifest
            .update(stored_manifest.prepare_dirty().unwrap())
            .await
            .unwrap();

        let recorder = MetricsRecorderHelper::noop();
        let task = ManifestGcTask::new(
            manifest_store.clone(),
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

        assert!(matches!(
            object_store
                .get(&Path::from("/root/gc/manifest.boundary"))
                .await,
            Err(object_store::Error::NotFound { .. })
        ));
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(
            vec![3],
            manifests
                .iter()
                .map(|manifest| manifest.id)
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn test_collect_advances_boundary_before_filtering_manifest_files() {
        let object_store = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from("/root"),
            object_store.clone(),
        ));
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
        stored_manifest
            .update(stored_manifest.prepare_dirty().unwrap())
            .await
            .unwrap();

        let recorder = MetricsRecorderHelper::noop();
        let task = ManifestGcTask::new(
            manifest_store.clone(),
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
            .get(&Path::from("/root/gc/manifest.boundary"))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!("2", std::str::from_utf8(&raw_boundary).unwrap());

        assert!(manifest_store.try_read_manifest(1).await.unwrap().is_some());
        assert!(manifest_store.try_read_manifest(2).await.unwrap().is_some());
    }

    async fn make_manifest_store() -> (Arc<ManifestStore>, StoredManifest) {
        let object_store = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(&Path::from("/root"), object_store));
        let stored = StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        (manifest_store, stored)
    }

    #[tokio::test]
    async fn test_version_count_after_gc_deletes_old_manifests() {
        let (manifest_store, mut stored) = make_manifest_store().await;
        // Write two more manifests: ids 1 (create), 2, 3
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
        let task = ManifestGcTask::new(
            manifest_store.clone(),
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

        // GC deletes manifests 1 and 2 (older than min_age=0); manifest 3 survives as latest.
        assert_eq!(
            lookup_metric_with_labels(
                &metrics,
                crate::garbage_collector::stats::VERSION_COUNT,
                &[("resource", "manifest")]
            ),
            Some(1),
            "expected 1 surviving manifest after GC"
        );
    }

    #[tokio::test]
    async fn test_version_count_when_nothing_to_delete() {
        let (manifest_store, mut stored) = make_manifest_store().await;
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
        let task = ManifestGcTask::new(
            manifest_store.clone(),
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

        // Nothing deleted — all 3 manifests survive.
        assert_eq!(
            lookup_metric_with_labels(
                &metrics,
                crate::garbage_collector::stats::VERSION_COUNT,
                &[("resource", "manifest")]
            ),
            Some(3),
            "expected all 3 manifests when nothing qualifies for deletion"
        );
    }

    #[tokio::test]
    async fn test_version_count_refreshes_when_boundary_advance_fails() {
        let tempdir = tempfile::tempdir().unwrap();
        let object_store = Arc::new(LocalFileSystem::new_with_prefix(tempdir.path()).unwrap());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from("/root"),
            object_store.clone(),
        ));
        let mut stored = StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
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
        manifest_store.advance_boundary(1).await.unwrap();

        let metrics = Arc::new(DefaultMetricsRecorder::new());
        let recorder = MetricsRecorderHelper::new(metrics.clone(), Default::default());
        let stats = Arc::new(GcStats::new(&recorder));
        stats.gc_manifest_versions.set(-1);
        let task = ManifestGcTask::new(
            manifest_store.clone(),
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
            .get(&Path::from("/root/gc/manifest.boundary"))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!("1", std::str::from_utf8(&raw_boundary).unwrap());
        assert_eq!(
            manifest_store
                .list_manifests(..)
                .await
                .unwrap()
                .iter()
                .map(|manifest| manifest.id)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            lookup_metric_with_labels(
                &metrics,
                crate::garbage_collector::stats::VERSION_COUNT,
                &[("resource", "manifest")]
            ),
            Some(3)
        );
    }

    #[tokio::test]
    async fn test_version_count_unchanged_on_dry_run() {
        let (manifest_store, mut stored) = make_manifest_store().await;
        // Write two more manifests: ids 1 (create), 2, 3
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
        let task = ManifestGcTask::new(
            manifest_store.clone(),
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

        // Dry run deletes nothing, so all 3 manifests still exist and the gauge reports
        // the true current count rather than the hypothetical post-deletion count.
        assert_eq!(
            lookup_metric_with_labels(
                &metrics,
                crate::garbage_collector::stats::VERSION_COUNT,
                &[("resource", "manifest")]
            ),
            Some(3),
            "expected dry run to leave the gauge at the true current count"
        );
        let manifests = manifest_store.list_manifests(..).await.unwrap();
        assert_eq!(
            manifests.len(),
            3,
            "dry run should not delete any manifests"
        );
    }
}
