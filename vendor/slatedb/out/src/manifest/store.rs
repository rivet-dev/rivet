// Local leadership/lifecycle modifications; see LEADERSHIP.md in this package.
use crate::sequenced_store::{SequencedStore, BoundaryReuse};
use crate::checkpoint::Checkpoint;
use crate::config::CheckpointOptions;
use crate::error::SlateDBError;
use crate::error::SlateDBError::{
    CheckpointMissing, InvalidDBState, LatestTransactionalObjectVersionMissing, ManifestMissing,
};
use crate::flatbuffer_types::FlatBufferManifestCodec;
use crate::manifest::invariants::manifest_invariants;
use crate::manifest::{Manifest, ManifestCore, VersionedManifest};
use log::debug;
use object_store::path::Path;
use object_store::ObjectStore;
use slatedb_common::clock::SystemClock;
use slatedb_common::object_metadata::IdentifiedObjectMetadata;
use slatedb_txn_obj::object_store::ObjectStoreSequencedStorageProtocol;
use slatedb_txn_obj::{
    DirtyObject, FenceableTransactionalObject, MonotonicId, SequencedStorageProtocol,
    SimpleTransactionalObject, TransactionalObject, TransactionalStorageProtocol,
};
use std::collections::BTreeMap;
use std::ops::RangeBounds;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use uuid::Uuid;

pub(crate) struct FenceableManifest {
    clock: Arc<dyn SystemClock>,
    inner: FenceableTransactionalObject<Manifest>,
    /// Versions written through this database's store in this process.
    writes: watch::Receiver<u64>,
}

// This type wraps StoredManifest, and fences other conflicting writers by incrementing
// the relevant epoch when initialized. It also detects when the current writer has been
// fenced and fails all operations with SlateDBError::Fenced.
impl FenceableManifest {
    pub(crate) async fn init_writer(
        stored_manifest: StoredManifest,
        manifest_update_timeout: Duration,
        system_clock: Arc<dyn SystemClock>,
    ) -> Result<Self, SlateDBError> {
        let clock = system_clock.clone();
        let writes = stored_manifest.writes;
        // Initialize generic fenceable record using writer epoch
        let fr = FenceableTransactionalObject::init(
            stored_manifest.inner,
            manifest_update_timeout,
            system_clock,
            |m: &Manifest| m.writer_epoch,
            |m: &mut Manifest, e: u64| m.writer_epoch = e,
        )
        .await?;
        Ok(Self {
            inner: fr,
            clock,
            writes,
        })
    }

    pub(crate) async fn init_compactor(
        stored_manifest: StoredManifest,
        manifest_update_timeout: Duration,
        system_clock: Arc<dyn SystemClock>,
    ) -> Result<Self, SlateDBError> {
        let clock = system_clock.clone();
        let writes = stored_manifest.writes;
        let fr = FenceableTransactionalObject::init(
            stored_manifest.inner,
            manifest_update_timeout,
            system_clock,
            |m: &Manifest| m.compactor_epoch,
            |m: &mut Manifest, e: u64| m.compactor_epoch = e,
        )
        .await?;
        Ok(Self {
            inner: fr,
            clock,
            writes,
        })
    }

    pub(crate) async fn init_writer_with_epoch(
        stored_manifest: StoredManifest,
        manifest_update_timeout: Duration,
        system_clock: Arc<dyn SystemClock>,
        epoch: std::num::NonZeroU64,
    ) -> Result<Self, SlateDBError> {
        let clock = system_clock.clone();
        let writes = stored_manifest.writes;
        let inner = FenceableTransactionalObject::init_with_epoch(
            stored_manifest.inner,
            manifest_update_timeout,
            system_clock,
            epoch.get(),
            |m: &Manifest| m.writer_epoch,
            |m: &mut Manifest, e| m.writer_epoch = e,
        )
        .await?;
        Ok(Self {
            inner,
            clock,
            writes,
        })
    }

    pub(crate) async fn init_compactor_with_epoch(
        stored_manifest: StoredManifest,
        manifest_update_timeout: Duration,
        system_clock: Arc<dyn SystemClock>,
        epoch: std::num::NonZeroU64,
    ) -> Result<Self, SlateDBError> {
        let clock = system_clock.clone();
        let writes = stored_manifest.writes;
        let inner = FenceableTransactionalObject::init_with_epoch(
            stored_manifest.inner,
            manifest_update_timeout,
            system_clock,
            epoch.get(),
            |m: &Manifest| m.compactor_epoch,
            |m: &mut Manifest, e| m.compactor_epoch = e,
        )
        .await?;
        Ok(Self {
            inner,
            clock,
            writes,
        })
    }

    pub(crate) fn manifest(&self) -> (u64, &Manifest) {
        (self.inner.id().id(), self.inner.object())
    }

    pub(crate) fn local_epoch(&self) -> u64 {
        self.inner.local_epoch()
    }

    /// Follows manifest versions written in this process by any component sharing the store.
    pub(crate) fn subscribe_writes(&self) -> watch::Receiver<u64> {
        self.writes.clone()
    }

    pub(crate) async fn refresh(&mut self) -> Result<&Manifest, SlateDBError> {
        Ok(self.inner.refresh().await?)
    }

    pub(crate) fn prepare_dirty(&self) -> Result<DirtyObject<Manifest>, SlateDBError> {
        Ok(self.inner.prepare_dirty()?)
    }

    pub(crate) async fn update(
        &mut self,
        dirty: DirtyObject<Manifest>,
    ) -> Result<(), SlateDBError> {
        Ok(self.inner.update(dirty).await?)
    }

    pub(crate) fn new_checkpoint(
        &self,
        checkpoint_id: Uuid,
        options: &CheckpointOptions,
    ) -> Result<Checkpoint, SlateDBError> {
        Self::make_new_checkpoint(self.clock.clone(), &self.inner, checkpoint_id, options)
    }

    fn make_new_checkpoint(
        clock: Arc<dyn SystemClock>,
        inner: &FenceableTransactionalObject<Manifest>,
        checkpoint_id: Uuid,
        options: &CheckpointOptions,
    ) -> Result<Checkpoint, SlateDBError> {
        let db_state = &inner.object().core;
        let manifest_id = match options.source {
            Some(source_checkpoint_id) => {
                let Some(source_checkpoint) = db_state.find_checkpoint(source_checkpoint_id) else {
                    return Err(CheckpointMissing(source_checkpoint_id));
                };
                source_checkpoint.manifest_id
            }
            None => {
                if !db_state.initialized {
                    return Err(InvalidDBState);
                }
                inner.id().next().into()
            }
        };
        Ok(Checkpoint {
            id: checkpoint_id,
            manifest_id,
            expire_time: options.lifetime.map(|l| clock.now() + l),
            create_time: clock.now(),
            name: options.name.clone(),
        })
    }

    pub(crate) async fn write_checkpoint(
        &mut self,
        checkpoint_id: Uuid,
        options: &CheckpointOptions,
    ) -> Result<Checkpoint, SlateDBError> {
        let clock = self.clock.clone();
        self.maybe_apply_update(|fm| {
            if options.checkpoint_id.is_some() {
                if let Some(existing) = fm.object().core.find_checkpoint(checkpoint_id) {
                    existing.validate_retained_identity()?;
                    return Ok(None);
                }
            }
            let checkpoint = Self::make_new_checkpoint(clock.clone(), fm, checkpoint_id, options)?;
            let mut dirty = fm.prepare_dirty()?;
            dirty.value.core.checkpoints.push(checkpoint);
            Ok(Some(dirty))
        })
        .await?;
        let checkpoint = self
            .inner
            .object()
            .core
            .find_checkpoint(checkpoint_id)
            .expect("update applied but checkpoint not found")
            .clone();
        Ok(checkpoint)
    }

    pub(crate) async fn maybe_apply_update<F>(&mut self, mutator: F) -> Result<(), SlateDBError>
    where
        F: Fn(
                &FenceableTransactionalObject<Manifest>,
            ) -> Result<Option<DirtyObject<Manifest>>, SlateDBError>
            + Send
            + Sync,
    {
        Ok(self.inner.maybe_apply_update(mutator).await?)
    }
}

// Represents the manifest stored in the object store. This type tracks the current
// contents and id of the stored manifest, and allows callers to read the db state
// stored therein. Callers can also use this type to update the db state stored in the
// manifest. The update is done with the next consecutive id, and is conditional on
// no other writer having made an update to the manifest using that id. Finally, callers
// can use the `refresh` method to refresh the locally stored manifest+id with the latest
// manifest stored in the object store.
pub(crate) struct StoredManifest {
    inner: SimpleTransactionalObject<Manifest>,
    clock: Arc<dyn SystemClock>,
    writes: watch::Receiver<u64>,
}

impl StoredManifest {
    fn new(
        inner: SimpleTransactionalObject<Manifest>,
        clock: Arc<dyn SystemClock>,
        writes: watch::Receiver<u64>,
    ) -> Self {
        Self {
            inner: inner.with_invariants(manifest_invariants()),
            clock,
            writes,
        }
    }

    async fn init(
        store: Arc<ManifestStore>,
        manifest: Manifest,
        clock: Arc<dyn SystemClock>,
    ) -> Result<Self, SlateDBError> {
        // Preserve original behavior: write via ManifestStore (object-store path and semantics)
        let inner = SimpleTransactionalObject::<Manifest>::init(
            store.transactional(),
            manifest.clone(),
        )
        .await?;
        Ok(Self::new(inner, clock, store.subscribe_writes()))
    }

    /// Create the initial manifest for a new database.
    pub(crate) async fn create_new_db(
        store: Arc<ManifestStore>,
        core: ManifestCore,
        clock: Arc<dyn SystemClock>,
    ) -> Result<Self, SlateDBError> {
        let manifest = Manifest::initial(core);
        Self::init(store, manifest, clock).await
    }

    /// Store a new manifest (for a new cloned database).
    pub(crate) async fn store_uninitialized_clone(
        clone_manifest_store: Arc<ManifestStore>,
        manifest: Manifest,
        clock: Arc<dyn SystemClock>,
    ) -> Result<Self, SlateDBError> {
        Self::init(clone_manifest_store, manifest, clock).await
    }

    /// Load the current manifest from the supplied manifest store. If there is no db at the
    /// manifest store's path then this fn returns None. Otherwise, on success it returns a
    /// Result with an instance of StoredManifest.
    pub(crate) async fn try_load(
        store: Arc<ManifestStore>,
        clock: Arc<dyn SystemClock>,
    ) -> Result<Option<Self>, SlateDBError> {
        let Some(inner) = SimpleTransactionalObject::<Manifest>::try_load(store.transactional())
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(Self::new(inner, clock, store.subscribe_writes())))
    }

    /// Load the current manifest from the supplied manifest store. If successful,
    /// this method returns a [`Result`] with an instance of [`StoredManifest`].
    /// If no manifests could be found, the error [`LatestTransactionalObjectVersionMissing`] is returned.
    pub(crate) async fn load(
        store: Arc<ManifestStore>,
        clock: Arc<dyn SystemClock>,
    ) -> Result<Self, SlateDBError> {
        Self::try_load(store, clock)
            .await?
            .ok_or(LatestTransactionalObjectVersionMissing)
    }

    #[allow(dead_code)]
    pub(crate) fn id(&self) -> u64 {
        self.inner.id().into()
    }

    pub(crate) fn manifest(&self) -> &Manifest {
        self.inner.object()
    }

    pub(crate) fn prepare_dirty(&self) -> Result<DirtyObject<Manifest>, SlateDBError> {
        Ok(self.inner.prepare_dirty()?)
    }

    pub(crate) fn db_state(&self) -> &ManifestCore {
        &self.manifest().core
    }

    #[allow(unused)]
    pub(crate) async fn refresh(&mut self) -> Result<&Manifest, SlateDBError> {
        Ok(self.inner.refresh().await?)
    }

    fn new_checkpoint(
        manifest: &Manifest,
        current_id: u64,
        clock: &dyn SystemClock,
        checkpoint_id: Uuid,
        options: &CheckpointOptions,
    ) -> Result<Checkpoint, SlateDBError> {
        let manifest_id = match options.source {
            Some(source_checkpoint_id) => {
                let Some(source_checkpoint) = manifest.core.find_checkpoint(source_checkpoint_id)
                else {
                    return Err(CheckpointMissing(source_checkpoint_id));
                };
                source_checkpoint.manifest_id
            }
            None => {
                if !manifest.core.initialized {
                    return Err(InvalidDBState);
                }
                current_id + 1
            }
        };
        Ok(Checkpoint {
            id: checkpoint_id,
            manifest_id,
            expire_time: options.lifetime.map(|l| clock.now() + l),
            create_time: clock.now(),
            name: options.name.clone(),
        })
    }

    pub(crate) async fn write_checkpoint(
        &mut self,
        checkpoint_id: Uuid,
        options: &CheckpointOptions,
    ) -> Result<Checkpoint, SlateDBError> {
        let clock = Arc::clone(&self.clock);
        self.inner
            .maybe_apply_update(|sr| {
                if options.checkpoint_id.is_some() {
                    if let Some(existing) = sr.object().core.find_checkpoint(checkpoint_id) {
                        existing.validate_retained_identity()?;
                        return Ok(None);
                    }
                }
                let mut new_val = sr.object().clone();
                let checkpoint = Self::new_checkpoint(
                    &new_val,
                    sr.id().into(),
                    clock.as_ref(),
                    checkpoint_id,
                    options,
                )?;
                new_val.core.checkpoints.push(checkpoint);
                let mut dirty = sr.prepare_dirty()?;
                dirty.value = new_val;
                let result: Result<Option<DirtyObject<Manifest>>, SlateDBError> = Ok(Some(dirty));
                result
            })
            .await?;
        Ok(self
            .db_state()
            .find_checkpoint(checkpoint_id)
            .expect("update applied but checkpoint not found")
            .clone())
    }

    pub(crate) async fn delete_checkpoint(
        &mut self,
        checkpoint_id: Uuid,
    ) -> Result<(), SlateDBError> {
        Ok(self
            .inner
            .maybe_apply_update(|sr| {
                let mut new_val = sr.object().clone();
                let before = new_val.core.checkpoints.len();
                new_val.core.checkpoints.retain(|cp| cp.id != checkpoint_id);

                let result: Result<Option<DirtyObject<Manifest>>, SlateDBError> =
                    if new_val.core.checkpoints.len() == before {
                        Ok(None)
                    } else {
                        let mut dirty = sr.prepare_dirty()?;
                        dirty.value = new_val;
                        Ok(Some(dirty))
                    };
                result
            })
            .await?)
    }

    /// Replace an existing checkpoint with a new checkpoint. If the old checkpoint
    /// is missing, the new checkpoint will still be added. This helps avoid
    /// issuing two manifest updates when creating a new checkpoint.
    pub(crate) async fn replace_checkpoint(
        &mut self,
        old_checkpoint_id: Uuid,
        new_checkpoint_id: Uuid,
        new_checkpoint_options: &CheckpointOptions,
    ) -> Result<Checkpoint, SlateDBError> {
        let clock = Arc::clone(&self.clock);
        self.inner
            .maybe_apply_update(|sr| {
                let mut new_val = sr.object().clone();
                // compute new checkpoint
                let checkpoint = Self::new_checkpoint(
                    &new_val,
                    sr.id().into(),
                    clock.as_ref(),
                    new_checkpoint_id,
                    new_checkpoint_options,
                )?;
                new_val
                    .core
                    .checkpoints
                    .retain(|cp| cp.id != old_checkpoint_id);
                new_val.core.checkpoints.push(checkpoint);
                let mut dirty = sr.prepare_dirty()?;
                dirty.value = new_val;
                let result: Result<Option<DirtyObject<Manifest>>, SlateDBError> = Ok(Some(dirty));
                result
            })
            .await?;
        let new_checkpoint = self
            .db_state()
            .find_checkpoint(new_checkpoint_id)
            .expect("update applied but checkpoint not found")
            .clone();
        Ok(new_checkpoint)
    }

    pub(crate) async fn refresh_checkpoint(
        &mut self,
        checkpoint_id: Uuid,
        new_lifetime: Duration,
    ) -> Result<Checkpoint, SlateDBError> {
        let clock = Arc::clone(&self.clock);
        self.inner
            .maybe_apply_update(|sr| {
                let mut new_val = sr.object().clone();
                let Some(cp) = new_val
                    .core
                    .checkpoints
                    .iter_mut()
                    .find(|c| c.id == checkpoint_id)
                else {
                    return Err(CheckpointMissing(checkpoint_id));
                };
                cp.expire_time = Some(clock.now() + new_lifetime);
                let mut dirty = sr.prepare_dirty()?;
                dirty.value = new_val;
                Ok(Some(dirty))
            })
            .await?;
        let checkpoint = self
            .db_state()
            .find_checkpoint(checkpoint_id)
            .expect("update applied but checkpoint not found")
            .clone();
        Ok(checkpoint)
    }

    pub(crate) async fn update(
        &mut self,
        dirty: DirtyObject<Manifest>,
    ) -> Result<(), SlateDBError> {
        Ok(self.inner.update(dirty).await?)
    }

    /// Apply an update to a stored manifest repeatedly retrying the update
    /// if the write fails due to a manifest version conflict caused by another client
    /// updating the manifest at the same time. The update to be applied is specified by
    /// the mutator parameter, which is a function that takes a &StoredManifest and returns
    /// an optional [`CoreDbState`]. If the mutator returns `None`, then no update will
    /// be applied.
    pub(crate) async fn maybe_apply_update<F>(&mut self, mutator: F) -> Result<(), SlateDBError>
    where
        F: Fn(
                &SimpleTransactionalObject<Manifest>,
            ) -> Result<Option<DirtyObject<Manifest>>, SlateDBError>
            + Send
            + Sync,
    {
        Ok(self.inner.maybe_apply_update(mutator).await?)
    }
}

pub(crate) struct ManifestStore {
    inner: Arc<dyn SequencedStorageProtocol<Manifest>>,
    /// Checked reads and writes, which every transactional manifest in this process uses.
    checked: Arc<SequencedStore<Manifest>>,
}

impl ManifestStore {
    pub(crate) fn new(root_path: &Path, object_store: Arc<dyn ObjectStore>) -> Self {
        Self::with_boundary_reuse(root_path, object_store, None)
    }

    /// A store whose latest-manifest reads reuse a recent boundary check; see
    /// [`crate::sequenced_store`].
    pub(crate) fn with_boundary_reuse(
        root_path: &Path,
        object_store: Arc<dyn ObjectStore>,
        reuse: Option<BoundaryReuse>,
    ) -> Self {
        let inner: Arc<dyn SequencedStorageProtocol<Manifest>> =
            Arc::new(ObjectStoreSequencedStorageProtocol::<Manifest>::new(
                root_path,
                object_store,
                "manifest",
                "manifest",
                Box::new(FlatBufferManifestCodec {}),
            ));
        let checked = Arc::new(SequencedStore::new(inner.clone(), reuse));
        Self { inner, checked }
    }

    fn transactional(&self) -> Arc<dyn TransactionalStorageProtocol<Manifest, MonotonicId>> {
        self.checked.clone()
    }

    /// Follows manifest versions written through this store.
    pub(crate) fn subscribe_writes(&self) -> watch::Receiver<u64> {
        self.checked.subscribe_writes()
    }

    pub(crate) async fn advance_boundary(&self, id: u64) -> Result<(), SlateDBError> {
        Ok(self.checked.advance(MonotonicId::new(id)).await?)
    }

    /// Delete a manifest from the object store.
    #[allow(unused)]
    pub(crate) async fn delete_manifest(&self, id: u64) -> Result<(), SlateDBError> {
        let latest_manifest = self.read_latest_manifest().await?;
        if latest_manifest.id == id {
            return Err(SlateDBError::InvalidDeletion);
        }

        if latest_manifest
            .manifest
            .core
            .checkpoints
            .iter()
            .any(|ck| ck.manifest_id == id)
        {
            return Err(SlateDBError::InvalidDeletion);
        }

        debug!("deleting manifest [id={}]", id);
        Ok(self.inner.delete(MonotonicId::new(id)).await?)
    }

    /// Delete a manifest without validating it against the latest manifest.
    ///
    /// Callers must ensure the manifest is not the latest manifest, is not referenced
    /// by an active checkpoint, and is safe to delete.
    pub(crate) async fn delete_manifest_unchecked(&self, id: u64) -> Result<(), SlateDBError> {
        debug!("deleting manifest [id={}]", id);
        Ok(self.inner.delete_unchecked(MonotonicId::new(id)).await?)
    }

    /// Read a manifest from the object store. The last element in an unbounded
    /// range is the current manifest.
    /// # Arguments
    /// * `id_range` - The range of IDs to list
    pub(crate) async fn list_manifests<R: RangeBounds<u64>>(
        &self,
        id_range: R,
    ) -> Result<Vec<IdentifiedObjectMetadata<u64>>, SlateDBError> {
        let manifests = self
            .inner
            .list(
                id_range.start_bound().map(|b| (*b).into()),
                id_range.end_bound().map(|b| (*b).into()),
            )
            .await?
            .into_iter()
            .map(|metadata| metadata.map_id(u64::from))
            .collect::<Vec<_>>();
        Ok(manifests)
    }

    /// Read the manifests referenced by the supplied manifest (including itself).
    pub(crate) async fn read_referenced_manifests(
        &self,
        manifest_id: u64,
        manifest: &Manifest,
    ) -> Result<BTreeMap<u64, Manifest>, SlateDBError> {
        let mut referenced_manifests = BTreeMap::new();
        referenced_manifests.insert(manifest_id, manifest.clone());

        let checkpoint_manifest_ids = manifest
            .core
            .checkpoints
            .iter()
            .map(|checkpoint| checkpoint.manifest_id)
            .collect::<Vec<_>>();
        for checkpoint_manifest_id in checkpoint_manifest_ids {
            if let std::collections::btree_map::Entry::Vacant(entry) =
                referenced_manifests.entry(checkpoint_manifest_id)
            {
                let checkpoint_manifest = self.read_manifest(checkpoint_manifest_id).await?;
                entry.insert(checkpoint_manifest);
            }
        }

        Ok(referenced_manifests)
    }

    pub(crate) async fn try_read_latest_manifest(
        &self,
    ) -> Result<Option<VersionedManifest>, SlateDBError> {
        Ok(self.checked.try_read_latest().await.map(|opt| {
            opt.map(|(id, manifest)| VersionedManifest::from_manifest(id.into(), manifest))
        })?)
    }

    pub(crate) async fn read_latest_manifest(&self) -> Result<VersionedManifest, SlateDBError> {
        self.try_read_latest_manifest()
            .await?
            .ok_or(LatestTransactionalObjectVersionMissing)
    }

    pub(crate) async fn try_read_manifest(
        &self,
        id: u64,
    ) -> Result<Option<Manifest>, SlateDBError> {
        Ok(self.inner.try_read_unchecked(MonotonicId::new(id)).await?)
    }

    pub(crate) async fn read_manifest(&self, id: u64) -> Result<Manifest, SlateDBError> {
        self.try_read_manifest(id).await?.ok_or(ManifestMissing(id))
    }
}

#[cfg(test)]
pub(crate) mod test_utils {
    use crate::manifest::Manifest;
    use crate::manifest::ManifestCore;
    use slatedb_txn_obj::test_utils::new_dirty_object;
    use slatedb_txn_obj::DirtyObject;

    pub(crate) fn new_dirty_manifest() -> DirtyObject<Manifest> {
        new_dirty_object(1u64, Manifest::initial(ManifestCore::new()))
    }
}

#[cfg(test)]
mod tests {
    use crate::checkpoint::Checkpoint;
    use crate::config::CheckpointOptions;
    use crate::error::SlateDBError;
    use crate::manifest::store::{FenceableManifest, ManifestStore, StoredManifest};
    use crate::manifest::ManifestCore;
    use crate::retrying_object_store::RetryingObjectStore;
    use crate::test_utils::{bounded_sst_view, FlakyObjectStore};
    use chrono::Timelike;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use slatedb_common::clock::{DefaultSystemClock, SystemClock};
    use slatedb_common::DbRand;
    use slatedb_txn_obj::object_store::ObjectStoreBoundaryObject;
    use slatedb_txn_obj::{BoundaryObject, MonotonicId, TransactionalObject};
    use std::sync::Arc;
    use std::time::Duration;

    const ROOT: &str = "/root/path";

    #[tokio::test]
    async fn test_should_fail_write_on_version_conflict() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        let mut sm2 = StoredManifest::load(ms.clone(), Arc::new(DefaultSystemClock::new()))
            .await
            .unwrap();
        sm.update(sm.prepare_dirty().unwrap()).await.unwrap();

        let result = sm2.update(sm2.prepare_dirty().unwrap()).await;

        assert!(matches!(
            result.unwrap_err(),
            SlateDBError::TransactionalObjectVersionExists
        ));
    }

    #[tokio::test]
    async fn test_should_write_with_new_version() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        sm.update(sm.prepare_dirty().unwrap()).await.unwrap();

        let version = ms.read_latest_manifest().await.unwrap().id;

        assert_eq!(version, 2);
    }

    #[tokio::test]
    async fn test_should_enforce_manifest_invariants_after_create() {
        let ms = new_memory_manifest_store();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            core_with_l0(100),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        assert_l0_invariant_rejects_update(&mut sm).await;
        assert_eq!(ms.read_latest_manifest().await.unwrap().id, 1);
    }

    #[tokio::test]
    async fn test_should_enforce_manifest_invariants_after_load() {
        let ms = new_memory_manifest_store();
        StoredManifest::create_new_db(
            ms.clone(),
            core_with_l0(100),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        let mut sm = StoredManifest::load(ms.clone(), Arc::new(DefaultSystemClock::new()))
            .await
            .unwrap();

        assert_l0_invariant_rejects_update(&mut sm).await;
        assert_eq!(ms.read_latest_manifest().await.unwrap().id, 1);
    }

    #[tokio::test]
    async fn test_should_fail_write_at_or_behind_boundary() {
        let object_store = Arc::new(InMemory::new());
        let ms = Arc::new(ManifestStore::new(&Path::from(ROOT), object_store.clone()));
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        let boundary = ObjectStoreBoundaryObject::new(&Path::from(ROOT), object_store, "manifest");
        boundary.advance(MonotonicId::new(2)).await.unwrap();

        let result = sm.update(sm.prepare_dirty().unwrap()).await;

        assert!(matches!(
            result,
            Err(SlateDBError::TransactionalObjectVersionExists)
        ));
    }

    #[tokio::test]
    async fn test_should_update_local_state_on_write() {
        let ms = new_memory_manifest_store();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        let mut dirty = sm.prepare_dirty().unwrap();
        dirty.value.core.next_wal_sst_id = 123;
        sm.update(dirty).await.unwrap();

        assert_eq!(sm.db_state().next_wal_sst_id, 123);
    }

    #[tokio::test]
    async fn test_should_refresh() {
        let ms = new_memory_manifest_store();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        let mut sm2 = StoredManifest::load(ms.clone(), Arc::new(DefaultSystemClock::new()))
            .await
            .unwrap();
        let mut dirty = sm.prepare_dirty().unwrap();
        dirty.value.core.next_wal_sst_id = 123;
        sm.update(dirty).await.unwrap();

        let refreshed = sm2.refresh().await.unwrap();

        assert_eq!(refreshed.core.next_wal_sst_id, 123);
        assert_eq!(sm2.db_state().next_wal_sst_id, 123);
    }

    #[tokio::test]
    async fn test_should_bump_writer_epoch() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        let timeout = Duration::from_secs(300);
        for i in 1..5 {
            let sm = StoredManifest::load(ms.clone(), Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
            FenceableManifest::init_writer(sm, timeout, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
            let manifest = ms.read_latest_manifest().await.unwrap();
            assert_eq!(manifest.manifest.writer_epoch, i);
        }
    }

    #[tokio::test]
    async fn test_should_fail_refresh_on_writer_fenced() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        let timeout = Duration::from_secs(300);
        let mut writer1 =
            FenceableManifest::init_writer(sm, timeout, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let sm2 = StoredManifest::load(ms.clone(), Arc::new(DefaultSystemClock::new()))
            .await
            .unwrap();

        FenceableManifest::init_writer(sm2, timeout, Arc::new(DefaultSystemClock::new()))
            .await
            .unwrap();

        let result = writer1.refresh().await;
        assert!(matches!(result, Err(SlateDBError::Fenced)));
    }

    #[tokio::test]
    async fn test_should_bump_compactor_epoch() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        let timeout = Duration::from_secs(300);
        for i in 1..5 {
            let sm = StoredManifest::load(ms.clone(), Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
            FenceableManifest::init_compactor(sm, timeout, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
            let manifest = ms.read_latest_manifest().await.unwrap();
            assert_eq!(manifest.manifest.compactor_epoch, i);
        }
    }

    #[tokio::test]
    async fn test_should_fail_refresh_on_compactor_fenced() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        let timeout = Duration::from_secs(300);
        let mut compactor1 =
            FenceableManifest::init_compactor(sm, timeout, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let sm2 = StoredManifest::load(ms.clone(), Arc::new(DefaultSystemClock::new()))
            .await
            .unwrap();

        FenceableManifest::init_compactor(sm2, timeout, Arc::new(DefaultSystemClock::new()))
            .await
            .unwrap();

        let result = compactor1.refresh().await;
        assert!(matches!(result, Err(SlateDBError::Fenced)));
    }

    #[tokio::test]
    async fn test_should_fail_manifest_write_of_stale_dirty_manifest() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        let stale = sm.prepare_dirty().unwrap();
        sm.update(sm.prepare_dirty().unwrap()).await.unwrap();

        let result = sm.update(stale).await;

        assert!(matches!(
            result,
            Err(SlateDBError::TransactionalObjectVersionExists)
        ));
    }

    #[tokio::test]
    async fn test_should_fail_write_checkpoint_when_fenced() {
        let ms = new_memory_manifest_store();
        let sm = StoredManifest::create_new_db(
            ms.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        let timeout = Duration::from_secs(300);
        let mut compactor1 =
            FenceableManifest::init_compactor(sm, timeout, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let sm2 = StoredManifest::load(ms.clone(), Arc::new(DefaultSystemClock::new()))
            .await
            .unwrap();
        let mut compactor2 =
            FenceableManifest::init_compactor(sm2, timeout, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();

        let result = compactor1
            .write_checkpoint(uuid::Uuid::new_v4(), &CheckpointOptions::default())
            .await;

        assert!(matches!(result, Err(SlateDBError::Fenced)));
        assert_state_not_updated(&mut compactor2).await;
    }

    #[tokio::test]
    async fn test_should_fail_state_update_when_fenced() {
        let ms = new_memory_manifest_store();
        let sm = StoredManifest::create_new_db(
            ms.clone(),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        let timeout = Duration::from_secs(300);
        let mut fm1 =
            FenceableManifest::init_writer(sm, timeout, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let sm2 = StoredManifest::load(ms.clone(), Arc::new(DefaultSystemClock::new()))
            .await
            .unwrap();
        let mut fm2 =
            FenceableManifest::init_writer(sm2, timeout, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();

        let result = fm1
            .maybe_apply_update(|fm| {
                let mut dirty = fm.prepare_dirty()?;
                dirty.value.core.last_l0_seq += 1;
                Ok(Some(dirty))
            })
            .await;

        assert!(matches!(result, Err(SlateDBError::Fenced)));
        assert_state_not_updated(&mut fm2).await;
    }

    async fn assert_state_not_updated(fm: &mut FenceableManifest) {
        let original_db_state = fm.inner.object().core.clone();
        fm.refresh().await.unwrap();
        let refreshed_db_state = fm.inner.object().core.clone();
        assert_eq!(refreshed_db_state, original_db_state);
    }

    #[tokio::test]
    async fn test_should_read_specific_manifest() {
        // Given
        let os = Arc::new(InMemory::new());
        let ms = Arc::new(ManifestStore::new(&Path::from(ROOT), os.clone()));
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let mut dirty = sm.prepare_dirty().unwrap();
        dirty
            .value
            .core
            .checkpoints
            .push(new_checkpoint(sm.inner.id().into()));
        sm.update(dirty).await.unwrap();

        // When
        let manifest = ms.try_read_manifest(2).await.unwrap().unwrap();

        // Then:
        assert_eq!(1, manifest.core.checkpoints.len());
    }

    #[tokio::test]
    async fn test_retry_write_manifest_on_timeout() {
        // Given a flaky store that times out on the first write
        let base = Arc::new(InMemory::new());
        let flaky = Arc::new(FlakyObjectStore::new(base.clone(), 1));
        let retrying = Arc::new(RetryingObjectStore::new(
            flaky.clone(),
            Arc::new(DbRand::default()),
            Arc::new(DefaultSystemClock::new()),
            None,
        ));
        let ms = Arc::new(ManifestStore::new(&Path::from(ROOT), retrying.clone()));

        // When creating a new DB (initial manifest write under retry)
        let core = ManifestCore::new();
        let _sm = StoredManifest::create_new_db(
            ms.clone(),
            core.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        // Then: a retry happened and the manifest matches input
        assert!(flaky.put_attempts() >= 2);
        let written = ms.try_read_manifest(1).await.unwrap().unwrap();
        assert_eq!(written, super::super::Manifest::initial(core));
    }

    #[tokio::test]
    async fn test_list_manifests_unbounded() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        sm.update(sm.prepare_dirty().unwrap()).await.unwrap();

        // Check unbounded
        let manifests = ms.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].id, 1);
        assert_eq!(manifests[1].id, 2);
        assert_eq!(
            Path::from(ROOT)
                .join("manifest")
                .join("00000000000000000001.manifest"),
            manifests[0].metadata.location
        );
        assert_eq!(
            Path::from(ROOT)
                .join("manifest")
                .join("00000000000000000002.manifest"),
            manifests[1].metadata.location
        );

        // Check bounded
        let manifests = ms.list_manifests(1..2).await.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].id, 1);

        // Check left bounded
        let manifests = ms.list_manifests(2..).await.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].id, 2);

        // Check right bounded
        let manifests = ms.list_manifests(..2).await.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].id, 1);
    }

    #[tokio::test]
    async fn test_delete_manifest() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        sm.update(sm.prepare_dirty().unwrap()).await.unwrap();
        let manifests = ms.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].id, 1);
        assert_eq!(manifests[1].id, 2);

        ms.advance_boundary(1).await.unwrap();
        ms.delete_manifest(1).await.unwrap();
        let manifests = ms.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].id, 2);
    }

    #[tokio::test]
    async fn test_delete_active_manifest_should_fail() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        sm.update(sm.prepare_dirty().unwrap()).await.unwrap();
        let manifests = ms.list_manifests(..).await.unwrap();
        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].id, 1);
        assert_eq!(manifests[1].id, 2);

        let result = ms.delete_manifest(2).await;
        assert!(matches!(result, Err(SlateDBError::InvalidDeletion)));
    }

    fn new_memory_manifest_store() -> Arc<ManifestStore> {
        let os = Arc::new(InMemory::new());
        Arc::new(ManifestStore::new(&Path::from(ROOT), os))
    }

    fn core_with_l0(timestamp_ms: u64) -> ManifestCore {
        let mut core = ManifestCore::new();
        Arc::make_mut(&mut core.tree)
            .l0
            .push_front(bounded_sst_view(timestamp_ms, b"a", b"z"));
        core
    }

    async fn assert_l0_invariant_rejects_update(sm: &mut StoredManifest) {
        let mut dirty = sm.prepare_dirty().unwrap();
        Arc::make_mut(&mut dirty.value.core.tree)
            .l0
            .push_front(bounded_sst_view(50, b"a", b"z"));

        let result = sm.update(dirty).await;

        assert!(matches!(
            result,
            Err(SlateDBError::InvalidClockTick {
                last_tick: 100,
                next_tick: 50
            })
        ));
    }

    fn new_checkpoint(manifest_id: u64) -> Checkpoint {
        let create_time = DefaultSystemClock::default()
            .now()
            .with_nanosecond(0)
            .unwrap();
        Checkpoint {
            id: uuid::Uuid::new_v4(),
            manifest_id,
            expire_time: None,
            create_time,
            name: None,
        }
    }

    #[tokio::test]
    async fn test_read_referenced_manifests_includes_checkpointed_manifests() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let initial_manifest = sm.manifest().clone();
        let initial_manifest_id = sm.id();

        // Baseline: with no checkpoints, only the latest manifest is referenced.
        let latest_manifest = ms.read_latest_manifest().await.unwrap();
        let referenced = ms
            .read_referenced_manifests(latest_manifest.id, &latest_manifest.manifest)
            .await
            .unwrap();

        assert_eq!(1, referenced.len());
        assert_eq!(
            Some(&initial_manifest),
            referenced.get(&initial_manifest_id)
        );

        // Add a checkpoint pointing at the initial manifest so both should be returned.
        let mut dirty = sm.prepare_dirty().unwrap();
        dirty
            .value
            .core
            .checkpoints
            .push(new_checkpoint(initial_manifest_id));
        sm.update(dirty).await.unwrap();

        let latest_manifest = ms.read_latest_manifest().await.unwrap();
        let referenced = ms
            .read_referenced_manifests(latest_manifest.id, &latest_manifest.manifest)
            .await
            .unwrap();

        assert_eq!(2, referenced.len());
        assert_eq!(
            Some(&initial_manifest),
            referenced.get(&initial_manifest_id)
        );
        assert_eq!(
            Some(&latest_manifest.manifest),
            referenced.get(&latest_manifest.id)
        );

        // Remove checkpoints to ensure only the latest manifest remains referenced.
        let mut dirty = sm.prepare_dirty().unwrap();
        dirty.value.core.checkpoints.clear();
        sm.update(dirty).await.unwrap();

        let latest_manifest = ms.read_latest_manifest().await.unwrap();
        let referenced = ms
            .read_referenced_manifests(latest_manifest.id, &latest_manifest.manifest)
            .await
            .unwrap();

        assert_eq!(1, referenced.len());
        assert_eq!(
            Some(&latest_manifest.manifest),
            referenced.get(&latest_manifest.id)
        );
    }

    #[tokio::test]
    async fn test_read_referenced_manifests_dedupes_checkpoint_ids() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let checkpoint_manifest_id = sm.id();
        let mut dirty = sm.prepare_dirty().unwrap();
        dirty
            .value
            .core
            .checkpoints
            .push(new_checkpoint(checkpoint_manifest_id));
        dirty
            .value
            .core
            .checkpoints
            .push(new_checkpoint(checkpoint_manifest_id));
        sm.update(dirty).await.unwrap();

        let latest_manifest = ms.read_latest_manifest().await.unwrap();
        let referenced = ms
            .read_referenced_manifests(latest_manifest.id, &latest_manifest.manifest)
            .await
            .unwrap();

        // 1 for the active manifest and 1 for the checkpointed manifest (deduped)
        assert_eq!(2, referenced.len());
    }

    #[tokio::test]
    async fn test_read_referenced_manifests_missing_manifest_returns_error() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let missing_manifest_id = sm.id() + 42;
        let mut dirty = sm.prepare_dirty().unwrap();
        dirty
            .value
            .core
            .checkpoints
            .push(new_checkpoint(missing_manifest_id));
        sm.update(dirty).await.unwrap();

        let latest_manifest = ms.read_latest_manifest().await.unwrap();
        let result = ms
            .read_referenced_manifests(latest_manifest.id, &latest_manifest.manifest)
            .await;

        assert!(matches!(
            result,
            Err(SlateDBError::ManifestMissing(id)) if id == missing_manifest_id
        ));
    }

    #[tokio::test]
    async fn test_maybe_apply_state_update() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let initial_id = sm.inner.id();
        sm.maybe_apply_update(|_| Ok(None)).await.unwrap();
        assert_eq!(initial_id, sm.inner.id());

        sm.maybe_apply_update(|sm| Ok(Some(sm.prepare_dirty().unwrap())))
            .await
            .unwrap();
        assert_eq!(initial_id + 1, sm.inner.id().id());
    }

    #[tokio::test]
    async fn test_deletion_of_manifest_with_checkpoint_reference_not_allowed() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let checkpoint1 = sm
            .write_checkpoint(uuid::Uuid::new_v4(), &CheckpointOptions::default())
            .await
            .unwrap();

        let _ = sm
            .write_checkpoint(uuid::Uuid::new_v4(), &CheckpointOptions::default())
            .await
            .unwrap();

        assert!(matches!(
            ms.delete_manifest(checkpoint1.manifest_id).await,
            Err(SlateDBError::InvalidDeletion)
        ));
    }

    #[tokio::test]
    async fn should_refresh_checkpoint() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let options = CheckpointOptions {
            lifetime: Some(Duration::from_secs(100)),
            ..CheckpointOptions::default()
        };

        let checkpoint = sm
            .write_checkpoint(uuid::Uuid::new_v4(), &options)
            .await
            .unwrap();
        let expire_time = checkpoint.expire_time.unwrap();

        let refreshed_checkpoint = sm
            .refresh_checkpoint(checkpoint.id, Duration::from_secs(500))
            .await
            .unwrap();
        let refreshed_expire_time = refreshed_checkpoint.expire_time.unwrap();
        assert!(refreshed_expire_time > expire_time);

        assert_eq!(
            Some(&refreshed_checkpoint),
            sm.manifest().core.find_checkpoint(checkpoint.id)
        );
    }

    #[tokio::test]
    async fn should_fail_refresh_if_checkpoint_missing() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let checkpoint_id = uuid::Uuid::new_v4();
        let result = sm
            .refresh_checkpoint(checkpoint_id, Duration::from_secs(100))
            .await;

        if let Err(SlateDBError::CheckpointMissing(missing_id)) = result {
            assert_eq!(checkpoint_id, missing_id);
        } else {
            panic!("Unexpected result {result:?}")
        }
    }

    #[tokio::test]
    async fn should_replace_checkpoint() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let checkpoint = sm
            .write_checkpoint(uuid::Uuid::new_v4(), &CheckpointOptions::default())
            .await
            .unwrap();

        let replaced_checkpoint = sm
            .replace_checkpoint(
                checkpoint.id,
                uuid::Uuid::new_v4(),
                &CheckpointOptions::default(),
            )
            .await
            .unwrap();
        assert_ne!(checkpoint.id, replaced_checkpoint.id);
        assert_eq!(None, sm.manifest().core.find_checkpoint(checkpoint.id));
        assert_eq!(
            Some(&replaced_checkpoint),
            sm.manifest().core.find_checkpoint(replaced_checkpoint.id),
        );
    }

    #[tokio::test]
    async fn should_ignore_missing_checkpoint_if_replacing() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let missing_checkpoint_id = uuid::Uuid::new_v4();
        let replaced_checkpoint = sm
            .replace_checkpoint(
                uuid::Uuid::new_v4(),
                missing_checkpoint_id,
                &CheckpointOptions::default(),
            )
            .await
            .unwrap();

        assert_eq!(
            Some(&replaced_checkpoint),
            sm.manifest().core.find_checkpoint(replaced_checkpoint.id),
        );
    }

    #[tokio::test]
    async fn should_delete_checkpoint() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let checkpoint = sm
            .write_checkpoint(uuid::Uuid::new_v4(), &CheckpointOptions::default())
            .await
            .unwrap();

        sm.delete_checkpoint(checkpoint.id).await.unwrap();
        assert_eq!(None, sm.manifest().core.find_checkpoint(checkpoint.id));
    }

    #[tokio::test]
    async fn should_ignore_missing_checkpoint_if_deleting() {
        let ms = new_memory_manifest_store();
        let state = ManifestCore::new();
        let mut sm = StoredManifest::create_new_db(
            ms.clone(),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let checkpoint_id = uuid::Uuid::new_v4();
        let manifest_id = sm.inner.id().id();
        sm.delete_checkpoint(checkpoint_id).await.unwrap();
        sm.refresh().await.unwrap();
        assert_eq!(manifest_id, sm.id());
    }

    #[tokio::test]
    async fn test_should_cretry_epoch_bump_if_manifest_version_exists() {
        let os = Arc::new(InMemory::new());
        let ms = Arc::new(ManifestStore::new(&Path::from(ROOT), os.clone()));
        let state = ManifestCore::new();

        // Mimic two writers A and B that try to bump the epoch at the same time
        let sm_a = StoredManifest::create_new_db(
            Arc::clone(&ms),
            state.clone(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let sm_b = StoredManifest::load(Arc::clone(&ms), Arc::new(DefaultSystemClock::new()))
            .await
            .unwrap();
        let timeout = Duration::from_secs(300);

        let mut fm_b =
            FenceableManifest::init_writer(sm_b, timeout, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        assert_eq!(1, fm_b.inner.local_epoch());

        // The last writer always wins
        let mut fm_a =
            FenceableManifest::init_writer(sm_a, timeout, Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        assert_eq!(2, fm_a.inner.local_epoch());

        assert!(matches!(
            fm_b.refresh().await.err(),
            Some(SlateDBError::Fenced)
        ));
        assert!(fm_a.refresh().await.is_ok());
    }
}
