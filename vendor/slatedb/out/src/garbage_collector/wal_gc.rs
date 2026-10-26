use super::GcTask;
use crate::manifest::Manifest;
use crate::{
    error::SlateDBError,
    manifest::store::ManifestStore,
    wal::{WalFileRange, WalGc},
};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::ops::Bound;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct WalGcTask {
    manifest_store: Arc<ManifestStore>,
    wal_gc: Arc<dyn WalGc>,
    resource: &'static str,
    min_age: Duration,
    dry_run: bool,
}

impl std::fmt::Debug for WalGcTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalGcTask")
            .field("resource", &self.resource.to_string())
            .finish()
    }
}

impl WalGcTask {
    pub(super) fn new(
        manifest_store: Arc<ManifestStore>,
        wal_gc: Arc<dyn WalGc>,
        resource: &'static str,
        min_age: Duration,
        dry_run: bool,
    ) -> Self {
        Self {
            manifest_store,
            wal_gc,
            resource,
            min_age,
            dry_run,
        }
    }

    fn referenced_wal_ranges(
        latest_manifest_id: u64,
        active_manifests: &BTreeMap<u64, Manifest>,
    ) -> Vec<WalFileRange> {
        active_manifests
            .iter()
            .map(|(manifest_id, manifest)| {
                if *manifest_id == latest_manifest_id {
                    // Keep the current compaction boundary and everything after it. Retaining the
                    // boundary matches the existing GC protocol and protects concurrent writers.
                    WalFileRange(
                        Bound::Included(manifest.core.replay_after_wal_id),
                        Bound::Unbounded,
                    )
                } else {
                    // A checkpoint only references WALs that must be replayed for its manifest.
                    WalFileRange(
                        Bound::Excluded(manifest.core.replay_after_wal_id),
                        Bound::Excluded(manifest.core.next_wal_sst_id),
                    )
                }
            })
            .collect()
    }
}

impl GcTask for WalGcTask {
    /// Resolve the WAL ranges referenced by the current manifest and active checkpoints, then
    /// delegate collection to the configured WAL implementation.
    async fn collect(&self, _utc_now: DateTime<Utc>) -> Result<(), SlateDBError> {
        let latest_manifest = self.manifest_store.read_latest_manifest().await?;
        let active_manifests = self
            .manifest_store
            .read_referenced_manifests(latest_manifest.id, &latest_manifest.manifest)
            .await?;
        let referenced_ranges = Self::referenced_wal_ranges(latest_manifest.id, &active_manifests);

        self.wal_gc
            .collect(referenced_ranges, self.min_age, self.dry_run)
            .await
            .map_err(Into::into)
    }

    fn resource(&self) -> &str {
        self.resource
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::Checkpoint;
    use crate::manifest::store::StoredManifest;
    use crate::manifest::ManifestCore;
    use crate::wal::WalError;
    use async_trait::async_trait;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use slatedb_common::clock::DefaultSystemClock;
    use std::sync::Mutex;
    use std::time::Duration;
    use uuid::Uuid;

    #[derive(Default)]
    struct RecordingWalGc {
        calls: Mutex<Vec<Vec<WalFileRange>>>,
    }

    impl RecordingWalGc {
        fn calls(&self) -> Vec<Vec<WalFileRange>> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl WalGc for RecordingWalGc {
        async fn collect(
            &self,
            referenced_ranges: Vec<WalFileRange>,
            _min_age: Duration,
            _dry_run: bool,
        ) -> Result<(), WalError> {
            self.calls.lock().unwrap().push(referenced_ranges);
            Ok(())
        }
    }

    #[test]
    fn test_referenced_wal_ranges() {
        let mut checkpoint_core = ManifestCore::new();
        checkpoint_core.replay_after_wal_id = 2;
        checkpoint_core.next_wal_sst_id = 6;

        let mut current_core = ManifestCore::new();
        current_core.replay_after_wal_id = 5;
        current_core.next_wal_sst_id = 8;

        let active_manifests = BTreeMap::from([
            (1, Manifest::initial(checkpoint_core)),
            (2, Manifest::initial(current_core)),
        ]);

        assert_eq!(
            WalGcTask::referenced_wal_ranges(2, &active_manifests),
            vec![
                WalFileRange(Bound::Excluded(2), Bound::Excluded(6)),
                WalFileRange(Bound::Included(5), Bound::Unbounded),
            ]
        );
    }

    #[tokio::test]
    async fn test_collect_calls_wal_gc_with_referenced_ranges() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from("/test/wal-gc-ranges"),
            object_store,
        ));

        let mut checkpoint_core = ManifestCore::new();
        checkpoint_core.replay_after_wal_id = 2;
        checkpoint_core.next_wal_sst_id = 6;
        let mut stored_manifest = StoredManifest::create_new_db(
            manifest_store.clone(),
            checkpoint_core,
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        let checkpoint_manifest_id = stored_manifest.id();

        let mut dirty = stored_manifest.prepare_dirty().unwrap();
        dirty.value.core.replay_after_wal_id = 5;
        dirty.value.core.next_wal_sst_id = 8;
        dirty.value.core.checkpoints.push(Checkpoint {
            id: Uuid::new_v4(),
            manifest_id: checkpoint_manifest_id,
            expire_time: None,
            create_time: Utc::now(),
            name: None,
        });
        stored_manifest.update(dirty).await.unwrap();

        let wal_gc = Arc::new(RecordingWalGc::default());
        let task = WalGcTask::new(manifest_store, wal_gc.clone(), "WAL", Duration::ZERO, false);

        task.collect(Utc::now()).await.unwrap();

        assert_eq!(
            wal_gc.calls(),
            vec![vec![
                WalFileRange(Bound::Excluded(2), Bound::Excluded(6)),
                WalFileRange(Bound::Included(5), Bound::Unbounded),
            ]]
        );
    }
}
