use crate::{
    config::GarbageCollectorScheduleOptions,
    error::SlateDBError,
    manifest::{
        store::{ManifestStore, StoredManifest},
        ExternalDb, Manifest,
    },
};
use chrono::{DateTime, Utc};
use log::{error, info};
use object_store::ObjectStore;
use slatedb_common::clock::SystemClock;
use slatedb_txn_obj::TransactionalObject;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

use super::{GcStats, GcTask};

/// Detaches a clone from its parent database(s) once the clone no longer references
/// any of the parent's SSTs.
///
/// A clone references its parent via an entry in `manifest.external_dbs`. The entry
/// holds a list of parent SST ids the clone may read (`sst_ids`) and a
/// `final_checkpoint_id` that pins a checkpoint in the parent's manifest so the
/// parent's GC does not collect those SSTs.
///
/// Compaction on the clone shrinks `sst_ids` as parent SSTs are rewritten into
/// clone-owned SSTs (see `Manifest::prune_external_sst_ids`). Once `sst_ids` is
/// empty in the current manifest AND in every manifest version referenced by a live
/// checkpoint, the clone no longer needs the parent — and this task detaches it:
///
/// 1. Delete the pinning `final_checkpoint_id` on the parent (idempotent).
/// 2. Remove the `ExternalDb` entry from the clone's manifest.
///
/// Parent-first order is chosen deliberately: a crash between the two leaves the
/// clone with a stale `ExternalDb` whose `sst_ids` is empty and whose
/// `final_checkpoint_id` points to a missing checkpoint — the next tick retries
/// step 1 as a no-op and completes step 2. The reverse order would leak the
/// parent's checkpoint forever on a crash.
#[derive(Clone)]
pub(crate) struct DetachGcTask {
    manifest_store: Arc<ManifestStore>,
    object_store: Arc<dyn ObjectStore>,
    system_clock: Arc<dyn SystemClock>,
    stats: Arc<GcStats>,
    detach_options: GarbageCollectorScheduleOptions,
}

impl std::fmt::Debug for DetachGcTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetachGcTask")
            .field("detach_options", &self.detach_options)
            .finish()
    }
}

impl DetachGcTask {
    pub(super) fn new(
        manifest_store: Arc<ManifestStore>,
        object_store: Arc<dyn ObjectStore>,
        system_clock: Arc<dyn SystemClock>,
        stats: Arc<GcStats>,
        detach_options: GarbageCollectorScheduleOptions,
    ) -> Self {
        Self {
            manifest_store,
            object_store,
            system_clock,
            stats,
            detach_options,
        }
    }

    async fn find_detachable(
        &self,
        manifest_id: u64,
        manifest: &Manifest,
    ) -> Result<Vec<ExternalDb>, SlateDBError> {
        if manifest.external_dbs.is_empty() {
            return Ok(Vec::new());
        }

        let candidates: Vec<&ExternalDb> = manifest
            .external_dbs
            .iter()
            .filter(|e| e.sst_ids.is_empty() && e.final_checkpoint_id.is_some())
            .collect();

        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let referenced = self
            .manifest_store
            .read_referenced_manifests(manifest_id, manifest)
            .await?;

        let detachable: Vec<ExternalDb> = candidates
            .into_iter()
            .filter(|candidate| {
                let final_id = candidate
                    .final_checkpoint_id
                    .expect("candidates must have final_checkpoint_id");
                referenced.values().all(|other_manifest| {
                    other_manifest.external_dbs.iter().all(|other| {
                        other.final_checkpoint_id != Some(final_id) || other.sst_ids.is_empty()
                    })
                })
            })
            .cloned()
            .collect();

        Ok(detachable)
    }

    async fn detach_from_parent(&self, external_db: &ExternalDb) -> Result<(), SlateDBError> {
        let final_id = external_db
            .final_checkpoint_id
            .expect("detachable entries must have final_checkpoint_id");

        let parent_store = Arc::new(ManifestStore::new(
            &external_db.path.clone().into(),
            self.object_store.clone(),
        ));
        let mut parent_manifest =
            StoredManifest::load(parent_store, self.system_clock.clone()).await?;
        parent_manifest.delete_checkpoint(final_id).await
    }
}

impl GcTask for DetachGcTask {
    async fn collect(&self, _utc_now: DateTime<Utc>) -> Result<(), SlateDBError> {
        let mut stored_manifest =
            StoredManifest::load(self.manifest_store.clone(), self.system_clock.clone()).await?;
        let manifest_id = stored_manifest.id();
        let manifest_snapshot = stored_manifest.manifest().clone();

        let detachable = self
            .find_detachable(manifest_id, &manifest_snapshot)
            .await?;
        if detachable.is_empty() {
            return Ok(());
        }

        let mut detached_final_ids: HashSet<Uuid> = HashSet::new();
        for external_db in &detachable {
            match self.detach_from_parent(external_db).await {
                Ok(()) => {
                    let final_id = external_db
                        .final_checkpoint_id
                        .expect("detachable entries must have final_checkpoint_id");
                    detached_final_ids.insert(final_id);
                    info!(
                        "detached clone from parent [parent_path={}, final_checkpoint_id={}]",
                        external_db.path, final_id
                    );
                }
                Err(e) => {
                    error!(
                        "failed to delete pinning checkpoint on parent [parent_path={}, error={}]",
                        external_db.path, e
                    );
                }
            }
        }

        if detached_final_ids.is_empty() {
            return Ok(());
        }

        let detached_count = detached_final_ids.len() as u64;

        stored_manifest
            .maybe_apply_update(|sr| {
                let current = sr.object();
                let retained: Vec<ExternalDb> = current
                    .external_dbs
                    .iter()
                    .filter(|e| {
                        e.final_checkpoint_id
                            .map(|id| !detached_final_ids.contains(&id))
                            .unwrap_or(true)
                    })
                    .cloned()
                    .collect();
                if retained.len() == current.external_dbs.len() {
                    Ok(None)
                } else {
                    let mut dirty = sr.prepare_dirty()?;
                    dirty.value.external_dbs = retained;
                    Ok(Some(dirty))
                }
            })
            .await?;

        for _ in 0..detached_count {
            self.stats.gc_detach_count.increment(1);
        }

        Ok(())
    }

    fn resource(&self) -> &str {
        "Clone detach"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CheckpointOptions;
    use crate::db_state::SsTableId;
    use crate::manifest::ManifestCore;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use slatedb_common::clock::DefaultSystemClock;
    use slatedb_txn_obj::TransactionalObject;
    use ulid::Ulid;

    const PARENT_PATH: &str = "/parent";
    const CLONE_PATH: &str = "/clone";

    /// Materialize a test setup: shared object store with an initialized parent and an
    /// initialized clone manifest. Returns (object_store, parent_store, clone_store).
    async fn build_setup() -> (Arc<dyn ObjectStore>, Arc<ManifestStore>, Arc<ManifestStore>) {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let parent_store = Arc::new(ManifestStore::new(
            &Path::from(PARENT_PATH),
            object_store.clone(),
        ));
        let clone_store = Arc::new(ManifestStore::new(
            &Path::from(CLONE_PATH),
            object_store.clone(),
        ));
        let clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());
        StoredManifest::create_new_db(parent_store.clone(), ManifestCore::new(), clock.clone())
            .await
            .unwrap();
        StoredManifest::create_new_db(clone_store.clone(), ManifestCore::new(), clock.clone())
            .await
            .unwrap();
        (object_store, parent_store, clone_store)
    }

    fn clock() -> Arc<dyn SystemClock> {
        Arc::new(DefaultSystemClock::new())
    }

    fn new_stats() -> Arc<GcStats> {
        let recorder = slatedb_common::metrics::MetricsRecorderHelper::noop();
        Arc::new(GcStats::new(&recorder))
    }

    /// Seed the parent with a checkpoint pinning `final_checkpoint_id`; add a matching
    /// ExternalDb entry to the clone's manifest with the provided sst_ids.
    async fn attach_clone_to_parent(
        parent_store: &Arc<ManifestStore>,
        clone_store: &Arc<ManifestStore>,
        final_checkpoint_id: Uuid,
        sst_ids: Vec<SsTableId>,
    ) {
        let mut parent = StoredManifest::load(parent_store.clone(), clock())
            .await
            .unwrap();
        parent
            .write_checkpoint(
                final_checkpoint_id,
                &CheckpointOptions {
                    checkpoint_id: None,
                    lifetime: None,
                    source: None,
                    name: None,
                },
            )
            .await
            .unwrap();

        let mut clone = StoredManifest::load(clone_store.clone(), clock())
            .await
            .unwrap();
        clone
            .maybe_apply_update(|sr| {
                let mut dirty = sr.prepare_dirty()?;
                dirty.value.external_dbs.push(ExternalDb {
                    path: PARENT_PATH.to_string(),
                    source_checkpoint_id: Uuid::new_v4(),
                    final_checkpoint_id: Some(final_checkpoint_id),
                    sst_ids: sst_ids.clone(),
                });
                Ok(Some(dirty))
            })
            .await
            .unwrap();
    }

    fn make_task(
        clone_store: Arc<ManifestStore>,
        object_store: Arc<dyn ObjectStore>,
    ) -> DetachGcTask {
        DetachGcTask::new(
            clone_store,
            object_store,
            clock(),
            new_stats(),
            GarbageCollectorScheduleOptions { interval: None },
        )
    }

    async fn parent_has_checkpoint(parent_store: &Arc<ManifestStore>, id: Uuid) -> bool {
        let parent = StoredManifest::load(parent_store.clone(), clock())
            .await
            .unwrap();
        parent.db_state().find_checkpoint(id).is_some()
    }

    async fn clone_external_dbs_len(clone_store: &Arc<ManifestStore>) -> usize {
        let clone = StoredManifest::load(clone_store.clone(), clock())
            .await
            .unwrap();
        clone.manifest().external_dbs.len()
    }

    #[tokio::test]
    async fn test_detaches_entry_with_empty_sst_ids() {
        let (object_store, parent_store, clone_store) = build_setup().await;
        let final_id = Uuid::new_v4();
        attach_clone_to_parent(&parent_store, &clone_store, final_id, vec![]).await;
        assert!(parent_has_checkpoint(&parent_store, final_id).await);
        assert_eq!(clone_external_dbs_len(&clone_store).await, 1);

        let task = make_task(clone_store.clone(), object_store);
        task.collect(Utc::now()).await.unwrap();

        assert!(
            !parent_has_checkpoint(&parent_store, final_id).await,
            "parent checkpoint should be deleted"
        );
        assert_eq!(
            clone_external_dbs_len(&clone_store).await,
            0,
            "clone external_dbs entry should be removed"
        );
    }

    #[tokio::test]
    async fn test_skips_entry_with_non_empty_sst_ids() {
        let (object_store, parent_store, clone_store) = build_setup().await;
        let final_id = Uuid::new_v4();
        let live_sst = SsTableId::from(Ulid::new());
        attach_clone_to_parent(&parent_store, &clone_store, final_id, vec![live_sst]).await;

        let task = make_task(clone_store.clone(), object_store);
        task.collect(Utc::now()).await.unwrap();

        assert!(
            parent_has_checkpoint(&parent_store, final_id).await,
            "parent checkpoint must be retained while clone still references it"
        );
        assert_eq!(
            clone_external_dbs_len(&clone_store).await,
            1,
            "clone external_dbs entry must be retained"
        );
    }

    #[tokio::test]
    async fn test_skips_entry_held_by_clone_checkpoint() {
        // The clone had a non-empty sst_ids at a prior manifest version, then compaction
        // emptied it. A live checkpoint on the clone still references the older manifest
        // version where sst_ids was non-empty — detach must NOT fire.
        let (object_store, parent_store, clone_store) = build_setup().await;
        let final_id = Uuid::new_v4();
        let live_sst = SsTableId::from(Ulid::new());
        attach_clone_to_parent(&parent_store, &clone_store, final_id, vec![live_sst]).await;

        // Pin this manifest version (with non-empty sst_ids) via a clone-side checkpoint.
        let mut clone = StoredManifest::load(clone_store.clone(), clock())
            .await
            .unwrap();
        clone
            .write_checkpoint(
                Uuid::new_v4(),
                &CheckpointOptions {
                    checkpoint_id: None,
                    lifetime: None,
                    source: None,
                    name: None,
                },
            )
            .await
            .unwrap();

        // Now empty sst_ids in the current manifest.
        let mut clone = StoredManifest::load(clone_store.clone(), clock())
            .await
            .unwrap();
        clone
            .maybe_apply_update(|sr| {
                let mut dirty = sr.prepare_dirty()?;
                for entry in dirty.value.external_dbs.iter_mut() {
                    entry.sst_ids.clear();
                }
                Ok(Some(dirty))
            })
            .await
            .unwrap();

        let task = make_task(clone_store.clone(), object_store);
        task.collect(Utc::now()).await.unwrap();

        assert!(
            parent_has_checkpoint(&parent_store, final_id).await,
            "parent checkpoint must remain while a clone-side checkpoint still holds non-empty sst_ids"
        );
        assert_eq!(clone_external_dbs_len(&clone_store).await, 1);
    }

    #[tokio::test]
    async fn test_idempotent_when_parent_checkpoint_already_gone() {
        let (object_store, parent_store, clone_store) = build_setup().await;
        let final_id = Uuid::new_v4();
        attach_clone_to_parent(&parent_store, &clone_store, final_id, vec![]).await;

        // Simulate a prior partial detach: parent's checkpoint is already deleted.
        let mut parent = StoredManifest::load(parent_store.clone(), clock())
            .await
            .unwrap();
        parent.delete_checkpoint(final_id).await.unwrap();
        assert!(!parent_has_checkpoint(&parent_store, final_id).await);

        let task = make_task(clone_store.clone(), object_store);
        task.collect(Utc::now()).await.unwrap();

        assert_eq!(
            clone_external_dbs_len(&clone_store).await,
            0,
            "clone-side removal must still complete even though parent checkpoint was already gone"
        );
    }

    #[tokio::test]
    async fn test_detaches_only_matching_entry_among_many() {
        let (object_store, parent_store, clone_store) = build_setup().await;
        let detachable_id = Uuid::new_v4();
        let retained_id = Uuid::new_v4();
        let live_sst = SsTableId::from(Ulid::new());
        attach_clone_to_parent(&parent_store, &clone_store, detachable_id, vec![]).await;
        attach_clone_to_parent(&parent_store, &clone_store, retained_id, vec![live_sst]).await;
        assert_eq!(clone_external_dbs_len(&clone_store).await, 2);

        let task = make_task(clone_store.clone(), object_store);
        task.collect(Utc::now()).await.unwrap();

        assert!(!parent_has_checkpoint(&parent_store, detachable_id).await);
        assert!(parent_has_checkpoint(&parent_store, retained_id).await);
        let clone = StoredManifest::load(clone_store.clone(), clock())
            .await
            .unwrap();
        let external_dbs = &clone.manifest().external_dbs;
        assert_eq!(external_dbs.len(), 1);
        assert_eq!(external_dbs[0].final_checkpoint_id, Some(retained_id));
    }
}
