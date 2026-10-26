pub use crate::db::builder::CloneBuilder;
pub use crate::db::builder::CloneSourceSpec;
use std::collections::BTreeSet;

use crate::checkpoint::{Checkpoint, CheckpointCreateResult};
use crate::compactions_store::CompactionsStore;
use crate::compactor::{Compaction, CompactionSpec, Compactor, CompactorStateView};
use crate::compactor_state::VersionedCompactions;
use crate::compactor_state_protocols::CompactorStateReader;
use crate::config::{CheckpointOptions, GarbageCollectorOptions};
use crate::db::builder::GarbageCollectorBuilder;
use crate::error::SlateDBError;
use crate::manifest::store::{ManifestStore, StoredManifest};
use crate::manifest::VersionedManifest;
use slatedb_common::clock::SystemClock;

use crate::retrying_object_store::RetryingObjectStore;
use crate::seq_tracker::FindOption;
use crate::utils::IdGenerator;
use crate::utils::ObjectStoreType;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use object_store::path::Path;
use object_store::{ObjectStore, ObjectStoreExt};
use rand::RngCore;
use slatedb_common::DbRand;
use std::env;
use std::env::VarError;
use std::ops::{Bound, RangeBounds};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use ulid::Ulid;
use uuid::Uuid;

pub use crate::db::builder::AdminBuilder;
use crate::merge_operator::MergeOperatorType;
use crate::wal::WalAdmin;
use slatedb_txn_obj::TransactionalObject;

/// An Admin struct for SlateDB administration operations.
///
/// This struct provides methods for administrative functions such as
/// reading manifests, creating checkpoints, cloning databases, and
/// running garbage collection.
pub struct Admin {
    /// The path to the database.
    pub(crate) path: Path,
    /// The main object store used for manifests and compacted SSTs.
    pub(crate) main_object_store: Arc<dyn ObjectStore>,
    /// The object store used for the WAL. This is the main object store when no
    /// dedicated WAL object store was configured.
    pub(crate) wal_object_store: Option<Arc<dyn ObjectStore>>,
    /// The system clock to use for operations.
    pub(crate) system_clock: Arc<dyn SystemClock>,
    /// The random number generator to use for randomness.
    pub(crate) rand: Arc<DbRand>,
    /// The retry policy applied to admin object-store operations.
    pub(crate) object_store_max_retries: Option<u32>,
    #[cfg(feature = "compaction_filters")]
    pub(crate) compaction_filter_supplier:
        Option<Arc<dyn crate::compaction_filter::CompactionFilterSupplier>>,
    pub(crate) merge_operator: Option<MergeOperatorType>,
    pub(crate) wal_admin: Arc<dyn WalAdmin>,
}

impl Admin {
    /// Read-only access to a specific or the latest manifest file.
    ///
    /// ## Arguments
    /// - `maybe_id`: Optional ID of the manifest file to read. If `None`, reads the latest.
    ///
    /// ## Returns
    /// - `Ok(Some(VersionedManifest))`: The manifest if found.
    /// - `Ok(None)`: If the manifest file does not exist.
    pub async fn read_manifest(
        &self,
        maybe_id: Option<u64>,
    ) -> Result<Option<VersionedManifest>, crate::Error> {
        let manifest_store = self.manifest_store();
        let manifest = if let Some(id) = maybe_id {
            manifest_store
                .try_read_manifest(id)
                .await
                .map_err(crate::Error::from)?
                .map(|manifest| VersionedManifest::from_manifest(id, manifest))
        } else {
            manifest_store
                .try_read_latest_manifest()
                .await
                .map_err(crate::Error::from)?
        };

        Ok(manifest)
    }

    /// List manifests within a range.
    ///
    /// ## Returns
    /// - `Ok(Vec<VersionedManifest>)`: The manifests in ascending ID order.
    pub async fn list_manifests<R: RangeBounds<u64>>(
        &self,
        range: R,
    ) -> Result<Vec<VersionedManifest>, crate::Error> {
        let manifest_store = self.manifest_store();
        let manifest_metadata = manifest_store
            .list_manifests(range)
            .await
            .map_err(crate::Error::from)?;
        let mut manifests = Vec::with_capacity(manifest_metadata.len());
        for metadata in manifest_metadata {
            match manifest_store
                .try_read_manifest(metadata.id)
                .await
                .map_err(crate::Error::from)?
            {
                Some(manifest) => {
                    manifests.push(VersionedManifest::from_manifest(metadata.id, manifest))
                }
                // Deleted after LIST by a concurrent GC
                // See https://github.com/slatedb/slatedb/issues/1215 for more details.
                None => log::warn!(
                    "listed manifest missing on read, skipping [id={}]",
                    metadata.id
                ),
            }
        }
        Ok(manifests)
    }

    /// Read-only access to a specific or the latest compactions file.
    ///
    /// ## Arguments
    /// - `maybe_id`: Optional ID of the compactions file to read. If None, reads from the latest.
    ///
    /// ## Returns
    /// - `Ok(Some(VersionedCompactions))`: The compactions if found.
    /// - `Ok(None)`: If the compactions file does not exist.
    pub async fn read_compactions(
        &self,
        maybe_id: Option<u64>,
    ) -> Result<Option<VersionedCompactions>, crate::Error> {
        let compactions_store = self.compactions_store();
        let compactions = if let Some(id) = maybe_id {
            compactions_store
                .try_read_compactions(id)
                .await
                .map_err(crate::Error::from)?
                .map(|compactions| VersionedCompactions::from_compactions(id, compactions))
        } else {
            compactions_store
                .try_read_latest_compactions()
                .await
                .map_err(crate::Error::from)?
        };

        Ok(compactions)
    }

    /// Read-only access to a compaction by id from a specific or latest compactions file.
    ///
    /// ## Arguments
    /// - `compaction_id`: The ULID of the compaction to read.
    /// - `maybe_id`: Optional ID of the compactions file to read from. If None, reads from the latest.
    ///
    /// ## Returns
    /// - `Ok(Some(Compaction))`: The compaction if found.
    /// - `Ok(None)`: If the compactions file or compaction ID does not exist.
    pub async fn read_compaction(
        &self,
        compaction_id: Ulid,
        maybe_id: Option<u64>,
    ) -> Result<Option<Compaction>, crate::Error> {
        let compactions_store = self.compactions_store();
        let compactions = if let Some(compactions_id) = maybe_id {
            compactions_store
                .try_read_compactions(compactions_id)
                .await
                .map_err(crate::Error::from)?
        } else {
            compactions_store
                .try_read_latest_compactions()
                .await
                .map_err(crate::Error::from)?
                .map(|compactions| compactions.compactions)
        };
        let Some(compactions) = compactions else {
            return Ok(None);
        };
        let Some(compaction) = compactions.get(&compaction_id) else {
            return Ok(None);
        };

        Ok(Some(compaction.clone()))
    }

    /// Returns a read-only view of the current compactor state.
    pub async fn read_compactor_state_view(&self) -> Result<CompactorStateView, crate::Error> {
        let manifest_store = Arc::new(self.manifest_store());
        let compactions_store = Arc::new(self.compactions_store());
        let reader = CompactorStateReader::new(&manifest_store, &compactions_store);
        reader.read_view().await.map_err(crate::Error::from)
    }

    /// Generate a compaction from a spec and submit it.
    ///
    /// ## Returns
    /// - `Ok(Compaction)`: The submitted compaction.
    /// - `Err`: If there was an error during submission or reading the submitted compaction.
    pub async fn submit_compaction(
        &self,
        spec: CompactionSpec,
    ) -> Result<Compaction, crate::Error> {
        let compactions_store = Arc::new(self.compactions_store());
        let rand = Arc::new(DbRand::new(self.rand.rng().next_u64()));
        let compaction_id =
            Compactor::submit(spec, compactions_store, rand, self.system_clock.clone()).await?;
        let Some(compaction) = self.read_compaction(compaction_id, None).await? else {
            return Err(crate::Error::from(SlateDBError::InvalidDBState));
        };

        Ok(compaction)
    }

    /// List compactions files within a range.
    ///
    /// ## Returns
    /// - `Ok(Vec<VersionedCompactions>)`: The compactions files in ascending ID order.
    pub async fn list_compactions<R: RangeBounds<u64>>(
        &self,
        range: R,
    ) -> Result<Vec<VersionedCompactions>, crate::Error> {
        let compactions_store = self.compactions_store();
        let compactions_metadata = compactions_store
            .list_compactions(range)
            .await
            .map_err(crate::Error::from)?;
        let mut compactions = Vec::with_capacity(compactions_metadata.len());
        for metadata in compactions_metadata {
            let stored_compactions = compactions_store
                .read_compactions(metadata.id)
                .await
                .map_err(crate::Error::from)?;
            compactions.push(VersionedCompactions::from_compactions(
                metadata.id,
                stored_compactions,
            ));
        }
        Ok(compactions)
    }

    /// List checkpoints, optionally filtering by name. When name is provided, only checkpoints
    /// with this exact name will be returned.
    ///
    /// # Arguments
    ///
    /// * `name_filter`: Name that will be used to filter checkpoints.
    pub async fn list_checkpoints(
        &self,
        name_filter: Option<&str>,
    ) -> Result<Vec<Checkpoint>, crate::Error> {
        let manifest_store = self.manifest_store();
        let manifest = manifest_store
            .read_latest_manifest()
            .await
            .map_err(crate::Error::from)?
            .manifest;

        let checkpoints = match name_filter {
            Some("") => manifest
                .core
                .checkpoints
                .into_iter()
                .filter(|cp| cp.name.as_deref() == Some("") || cp.name.is_none())
                .collect(),
            Some(name) => manifest
                .core
                .checkpoints
                .into_iter()
                .filter(|cp| cp.name.as_deref() == Some(name))
                .collect(),
            None => manifest.core.checkpoints,
        };

        Ok(checkpoints)
    }

    /// Run the garbage collector once in the foreground.
    ///
    /// This function runs the garbage collector letting Tokio decide when to run the task.
    ///
    /// # Arguments
    ///
    /// * `gc_opts`: The garbage collector options.
    ///
    pub async fn run_gc_once(&self, gc_opts: GarbageCollectorOptions) -> Result<(), crate::Error> {
        let gc = GarbageCollectorBuilder::new(
            self.path.clone(),
            self.object_store(ObjectStoreType::Main).clone(),
        )
        .with_system_clock(self.system_clock.clone())
        .with_wal_gc(self.wal_admin.garbage_collector(&self.path))
        .with_wal_object_store(self.object_store(ObjectStoreType::Wal).clone())
        .with_options(gc_opts)
        .with_seed(self.rand.rng().next_u64())
        .build();
        gc.run_gc_once().await;
        Ok(())
    }

    /// Run the garbage collector in the foreground until the provided cancellation token is cancelled.
    ///
    /// This method blocks until `cancellation_token` is cancelled, at which point it requests a
    /// graceful shutdown and waits for the garbage collector to stop.
    ///
    /// # Arguments
    ///
    /// * `cancellation_token`: Token used to request garbage collector shutdown.
    ///
    pub async fn run_gc(&self, cancellation_token: CancellationToken) -> Result<(), crate::Error> {
        self.run_gc_with_options(cancellation_token, GarbageCollectorOptions::default())
            .await
    }

    /// Like [`Admin::run_gc`] but accepts explicit [`GarbageCollectorOptions`].
    pub async fn run_gc_with_options(
        &self,
        cancellation_token: CancellationToken,
        gc_opts: GarbageCollectorOptions,
    ) -> Result<(), crate::Error> {
        let gc = GarbageCollectorBuilder::new(
            self.path.clone(),
            self.object_store(ObjectStoreType::Main).clone(),
        )
        .with_system_clock(self.system_clock.clone())
        .with_wal_gc(self.wal_admin.garbage_collector(&self.path))
        .with_wal_object_store(self.object_store(ObjectStoreType::Wal).clone())
        .with_options(gc_opts)
        .with_seed(self.rand.rng().next_u64())
        .build();

        gc.start()?;

        tokio::select! {
            result = gc.join() => result,
            _ = cancellation_token.cancelled() => {
                gc.stop().await
            }
        }
    }

    /// Run the compactor in the foreground until the provided cancellation token is cancelled.
    ///
    /// This method blocks until `cancellation_token` is cancelled, at which point it requests a
    /// graceful shutdown and waits for the compactor to stop.
    ///
    /// To use compaction filters with the standalone compactor, configure the `AdminBuilder`
    /// with [`AdminBuilder::with_compaction_filter_supplier`] before building.
    pub async fn run_compactor(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<(), crate::Error> {
        self.run_compactor_with_options(
            cancellation_token,
            crate::config::CompactorOptions::default(),
        )
        .await
    }

    /// Like [`Admin::run_compactor`] but accepts explicit [`crate::config::CompactorOptions`].
    ///
    /// Useful for disabling the embedded worker (set worker: None) when running
    /// standalone [`crate::compaction_worker::CompactionWorker`] processes separately.
    pub async fn run_compactor_with_options(
        &self,
        cancellation_token: CancellationToken,
        options: crate::config::CompactorOptions,
    ) -> Result<(), crate::Error> {
        #[allow(unused_mut)]
        let mut builder = crate::CompactorBuilder::new(
            self.path.clone(),
            self.object_store(ObjectStoreType::Main).clone(),
        )
        .with_options(options)
        .with_system_clock(self.system_clock.clone())
        .with_seed(self.rand.rng().next_u64());

        #[cfg(feature = "compaction_filters")]
        if let Some(supplier) = &self.compaction_filter_supplier {
            builder = builder.with_compaction_filter_supplier(supplier.clone());
        }

        if let Some(merge_operator) = &self.merge_operator {
            builder = builder.with_merge_operator(merge_operator.clone());
        }

        let compactor = builder.build();

        compactor.start().await?;

        tokio::select! {
            result = compactor.join() => result,
            _ = cancellation_token.cancelled() => {
                compactor.stop().await
            }
        }
    }

    /// Run a standalone compaction worker in the foreground until the provided cancellation token
    /// is cancelled.
    ///
    /// This method blocks until `cancellation_token` is cancelled, at which point it requests a
    /// graceful shutdown and waits for the worker to stop, resetting any compactions it claimed
    /// back to `Scheduled` so other workers can pick them up.
    ///
    /// To use compaction filters with the standalone worker, configure the `AdminBuilder` with
    /// [`AdminBuilder::with_compaction_filter_supplier`] before building.
    ///
    /// # Arguments
    ///
    /// * `cancellation_token`: Token used to request worker shutdown.
    pub async fn run_compaction_worker(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<(), crate::Error> {
        self.run_compaction_worker_with_options(
            cancellation_token,
            crate::config::CompactionWorkerOptions::default(),
        )
        .await
    }

    /// Like [`Admin::run_compaction_worker`] but accepts explicit
    /// [`crate::config::CompactionWorkerOptions`].
    pub async fn run_compaction_worker_with_options(
        &self,
        cancellation_token: CancellationToken,
        options: crate::config::CompactionWorkerOptions,
    ) -> Result<(), crate::Error> {
        #[allow(unused_mut)]
        let mut builder = crate::CompactionWorkerBuilder::new(
            self.path.clone(),
            self.object_store(ObjectStoreType::Main).clone(),
        )
        .with_options(options)
        .with_system_clock(self.system_clock.clone())
        .with_seed(self.rand.rng().next_u64());

        #[cfg(feature = "compaction_filters")]
        if let Some(supplier) = &self.compaction_filter_supplier {
            builder = builder.with_compaction_filter_supplier(supplier.clone());
        }

        let worker = builder.build().await?;

        worker.start()?;

        tokio::select! {
            result = worker.join() => result,
            _ = cancellation_token.cancelled() => {
                worker.stop().await
            }
        }
    }

    /// Creates a checkpoint of the db stored in the object store at the specified path using the
    /// provided options. The checkpoint will reference the current active manifest of the db. This
    /// method does not flush writer memtables or WALs before creating the checkpoint. You will be
    /// responsible for refreshing checkpoints periodically.
    ///
    /// If you have a [`crate::Db`] instance open, you can use the [`crate::Db::create_checkpoint`]
    /// method instead. That method will flush the memtables and WALs before creating the checkpoint.
    ///
    /// If you're using a [`crate::DbReader`], you might wish to use
    /// [`crate::DbReaderMode::ManagedCheckpoint`]. The reader will create a checkpoint for you and
    /// periodically refresh it.
    ///
    /// # Examples
    ///
    /// ```
    /// use slatedb::admin::{Admin, AdminBuilder};
    /// use slatedb::config::CheckpointOptions;
    /// use slatedb::Db;
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::error::Error;
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///    let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///    let db = Db::open("parent_path", Arc::clone(&object_store)).await?;
    ///    db.put(b"key", b"value").await?;
    ///    db.close().await?;
    ///
    ///    let admin = AdminBuilder::new("parent_path", object_store).build();
    ///    let _ = admin.create_detached_checkpoint(
    ///      &CheckpointOptions::default(),
    ///    ).await?;
    ///
    ///    Ok(())
    /// }
    /// ```
    pub async fn create_detached_checkpoint(
        &self,
        options: &CheckpointOptions,
    ) -> Result<CheckpointCreateResult, crate::Error> {
        options.validate()?;
        let manifest_store = Arc::new(self.manifest_store());
        let mut stored_manifest =
            StoredManifest::load(manifest_store, self.system_clock.clone()).await?;

        let configured_wal_uri = self.wal_object_store.is_some().then(String::new);
        stored_manifest
            .db_state()
            .validate_wal_object_store_uri(configured_wal_uri.as_deref())?;

        let checkpoint_id = options
            .checkpoint_id
            .unwrap_or_else(|| self.rand.rng().gen_uuid());
        let checkpoint = stored_manifest
            .write_checkpoint(checkpoint_id, options)
            .await?;
        Ok(CheckpointCreateResult {
            id: checkpoint.id,
            manifest_id: checkpoint.manifest_id,
        })
    }

    /// Refresh the lifetime of an existing checkpoint. Takes the id of an existing checkpoint
    /// and a lifetime, and sets the lifetime of the checkpoint to the specified lifetime. If
    /// there is no checkpoint with the specified id, then this fn fails with
    /// SlateDBError::InvalidDbState
    pub async fn refresh_checkpoint(
        &self,
        id: Uuid,
        lifetime: Option<Duration>,
    ) -> Result<(), crate::Error> {
        let manifest_store = Arc::new(self.manifest_store());
        let mut stored_manifest =
            StoredManifest::load(manifest_store, self.system_clock.clone()).await?;
        stored_manifest
            .maybe_apply_update(|stored_manifest| {
                let mut dirty = stored_manifest.prepare_dirty()?;
                let expire_time = lifetime.map(|l| self.system_clock.now() + l);
                let Some(_) = dirty.value.core.checkpoints.iter_mut().find_map(|c| {
                    if c.id == id {
                        c.expire_time = expire_time;
                        return Some(());
                    }
                    None
                }) else {
                    return Err(SlateDBError::InvalidDBState);
                };
                Ok(Some(dirty))
            })
            .await
            .map_err(Into::into)
    }

    /// Deletes the checkpoint with the specified id.
    pub async fn delete_checkpoint(&self, id: Uuid) -> Result<(), crate::Error> {
        let manifest_store = Arc::new(self.manifest_store());
        let mut stored_manifest =
            StoredManifest::load(manifest_store, self.system_clock.clone()).await?;
        stored_manifest
            .maybe_apply_update(|stored_manifest| {
                let mut dirty = stored_manifest.prepare_dirty()?;
                let checkpoints: Vec<Checkpoint> = dirty
                    .value
                    .core
                    .checkpoints
                    .iter()
                    .filter(|c| c.id != id)
                    .cloned()
                    .collect();
                dirty.value.core.checkpoints = checkpoints;
                Ok(Some(dirty))
            })
            .await
            .map_err(Into::into)
    }

    /// Deletes a database, stripping any checkpoints it pinned in parent
    /// databases (a clone) before removing its own objects. Works for plain and
    /// cloned dbs alike: a plain db just has no parent checkpoints to strip.
    ///
    /// Without `confirm` this is a dry run: it returns every object it *would*
    /// delete and touches nothing. Pass `confirm` to actually delete.
    ///
    /// The delete writes a `.deleting` marker while the manifest still proves
    /// this is a slatedb dir, then removes everything else, then the marker. If a
    /// prior run crashed mid-delete, the leftover marker lets a rerun finish the
    /// job. A `confirm` delete of a dir with neither a manifest nor a marker is
    /// refused, so a fat-fingered `--path` can't wipe an unrelated directory.
    /// Idempotent.
    pub async fn delete_db(&self, confirm: bool) -> Result<Vec<String>, crate::Error> {
        let main = self.retrying_store(ObjectStoreType::Main);

        if !confirm {
            return self.list_prefix(&main).await;
        }

        let marker = self.path.clone().join(".deleting");
        let marker_exists = main.get(&marker).await.map(|_| true).or_else(|e| match e {
            object_store::Error::NotFound { .. } => Ok(false),
            other => Err(SlateDBError::from(other)),
        })?;

        let manifest = self.manifest_store().try_read_latest_manifest().await?;

        // No manifest and no marker means we never proved this is a slatedb dir.
        // If there are objects under the prefix, it may be a fat-fingered path we
        // must not wipe, so refuse. If empty, there's nothing to delete anyway
        // (also the already-deleted no-op), so fall through to a clean return.
        if manifest.is_none()
            && !marker_exists
            && !collect_prefix(&main, &self.path).await?.is_empty()
        {
            return Err(SlateDBError::InvalidDBState.into());
        }

        // Strip the checkpoints this db pinned in each parent. Needs the
        // manifest, which a resumed (marker-only) run may no longer have.
        if let Some(manifest) = manifest.as_ref() {
            for external_db in manifest.external_dbs() {
                let Some(final_checkpoint_id) = external_db.final_checkpoint_id else {
                    continue;
                };
                let parent_store = Arc::new(ManifestStore::new(
                    &Path::from(external_db.path.as_str()),
                    self.retrying_store(ObjectStoreType::Main),
                ));
                let mut parent =
                    match StoredManifest::load(parent_store, self.system_clock.clone()).await {
                        Ok(parent) => parent,
                        // parent already deleted: no checkpoint left to strip, skip it
                        Err(SlateDBError::LatestTransactionalObjectVersionMissing) => continue,
                        Err(e) => return Err(e.into()),
                    };
                parent.delete_checkpoint(final_checkpoint_id).await?;
            }
        }

        // Commit the intent to delete while the manifest still proves this dir.
        if !marker_exists {
            main.put(&marker, Bytes::new().into())
                .await
                .map_err(SlateDBError::from)?;
        }

        // Delete everything but the marker, then the marker last, so a crash in
        // between leaves the marker to prove a rerun should finish.
        let mut deleted = self.delete_prefix(&main, Some(&marker)).await?;
        deleted.extend(
            self.wal_admin
                .delete_wal(&self.path, false)
                .await
                .map_err(SlateDBError::from)?,
        );
        main.delete(&marker).await.map_err(SlateDBError::from)?;
        deleted.push(marker.to_string());
        Ok(deleted)
    }

    /// Lists every object under this db's path prefix across the main and WAL stores.
    async fn list_prefix(&self, main: &Arc<dyn ObjectStore>) -> Result<Vec<String>, crate::Error> {
        let paths = collect_prefix(main, &self.path).await?;
        // track the dry run paths in a set since the WAL and db may overlap
        let mut paths = paths.iter().map(Path::to_string).collect::<BTreeSet<_>>();
        let wal_paths = self
            .wal_admin
            .delete_wal(&self.path, true)
            .await
            .map_err(SlateDBError::from)?;
        for wp in wal_paths {
            paths.insert(wp);
        }
        Ok(paths.into_iter().collect())
    }

    /// Deletes every object under this db's path prefix in the given store,
    /// skipping `keep` if set. Returns the deleted paths.
    async fn delete_prefix(
        &self,
        store: &Arc<dyn ObjectStore>,
        keep: Option<&Path>,
    ) -> Result<Vec<String>, crate::Error> {
        let mut deleted = Vec::new();
        for path in collect_prefix(store, &self.path).await? {
            if Some(&path) == keep {
                continue;
            }
            store.delete(&path).await.map_err(SlateDBError::from)?;
            deleted.push(path);
        }
        Ok(deleted.into_iter().map(|p| p.to_string()).collect())
    }

    /// Returns the timestamp or sequence from the latest manifest's sequence tracker.
    /// When `round_up` is true, uses the next higher value; otherwise the previous one.
    pub async fn get_timestamp_for_sequence(
        &self,
        seq: u64,
        round_up: bool,
    ) -> Result<Option<DateTime<Utc>>, crate::Error> {
        let manifest_store = self.manifest_store();

        let id_manifest = manifest_store.try_read_latest_manifest().await?;
        let Some(manifest) = id_manifest else {
            return Ok(None);
        };

        let opt = if round_up {
            FindOption::RoundUp
        } else {
            FindOption::RoundDown
        };
        Ok(manifest.core().sequence_tracker.find_ts(seq, opt))
    }

    /// Returns the sequence for a given timestamp from the latest manifest's sequence tracker.
    /// When `round_up` is true, uses the next higher value; otherwise the previous one.
    pub async fn get_sequence_for_timestamp(
        &self,
        ts: DateTime<Utc>,
        round_up: bool,
    ) -> Result<Option<u64>, crate::Error> {
        let manifest_store = self.manifest_store();

        let id_manifest = manifest_store.try_read_latest_manifest().await?;
        let Some(manifest) = id_manifest else {
            return Ok(None);
        };

        let opt = if round_up {
            FindOption::RoundUp
        } else {
            FindOption::RoundDown
        };
        Ok(manifest.core().sequence_tracker.find_seq(ts, opt))
    }

    /// Wraps the configured object store of the given type in a
    /// [`RetryingObjectStore`] so that admin operations retry transient object
    /// store failures with exponential backoff. Retrying is safe here because
    /// `RetryingObjectStore` verifies conditional puts via a ULID written to
    /// object metadata, so an ambiguous failure after a successful write is
    /// detected rather than surfaced as a spurious error.
    fn retrying_store(&self, store_type: ObjectStoreType) -> Arc<dyn ObjectStore> {
        Arc::new(RetryingObjectStore::new(
            self.object_store(store_type).clone(),
            self.rand.clone(),
            self.system_clock.clone(),
            self.object_store_max_retries,
        ))
    }

    fn object_store(&self, store_type: ObjectStoreType) -> &Arc<dyn ObjectStore> {
        match store_type {
            ObjectStoreType::Main => &self.main_object_store,
            ObjectStoreType::Wal => self
                .wal_object_store
                .as_ref()
                .unwrap_or(&self.main_object_store),
        }
    }

    fn manifest_store(&self) -> ManifestStore {
        ManifestStore::new(&self.path, self.retrying_store(ObjectStoreType::Main))
    }

    fn compactions_store(&self) -> CompactionsStore {
        CompactionsStore::new(&self.path, self.retrying_store(ObjectStoreType::Main))
    }

    /// Clone a database using a builder pattern. If no db already exists at the specified path,
    /// then this will create a new db under the path that is a clone of the db at parent_path.
    ///
    /// A clone is a shallow copy of the parent database - it starts with a manifest that
    /// references the same SSTs, but doesn't actually copy those SSTs, except for the WAL.
    /// New writes will be written to the newly created db and will not be reflected in the
    /// parent database.
    ///
    /// The first source's [`CloneSourceSpec`] is passed directly, so any `projection_range`
    /// already set on it is preserved.  This matters when multiple sources are combined via
    /// [`CloneBuilder::with_source`]: each source must carry its own per-source range so that
    /// [`crate::manifest::Manifest::cloned_from_union`] sees non-overlapping effective ranges.
    ///
    /// Segmented sources only have to be non-overlapping within each segment that they share.
    ///
    /// # Examples
    ///
    /// ```
    /// use slatedb::admin::{Admin, AdminBuilder, CloneSourceSpec};
    /// use slatedb::Db;
    /// use slatedb::object_store::{ObjectStore, memory::InMemory};
    /// use std::error::Error;
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///    let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
    ///    let db = Db::open("parent_path", Arc::clone(&object_store)).await?;
    ///    db.put(b"key", b"value").await?;
    ///    db.close().await?;
    ///
    ///    let admin = AdminBuilder::new("clone_path", object_store).build();
    ///    admin.create_clone_builder_from_source(CloneSourceSpec::new("parent_path")).build().await?;
    ///
    ///    Ok(())
    /// }
    /// ```
    pub fn create_clone_builder_from_source(
        &self,
        source: CloneSourceSpec<(Bound<Bytes>, Bound<Bytes>)>,
    ) -> CloneBuilder<(Bound<Bytes>, Bound<Bytes>)> {
        CloneBuilder::new(
            self.path.clone(),
            source,
            self.retrying_store(ObjectStoreType::Main),
        )
        .with_wal_admin(self.wal_admin.clone())
    }

    /// Creates a new builder for an admin client at the given path.
    ///
    /// ## Arguments
    /// - `path`: the path to the database
    /// - `object_store`: the object store to use for the database
    ///
    /// ## Returns
    /// - `AdminBuilder`: the builder to initialize the admin client
    ///
    /// ## Examples
    ///
    /// ```
    /// use slatedb::admin::Admin;
    /// use slatedb::object_store::memory::InMemory;
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let object_store = Arc::new(InMemory::new());
    ///     let admin = Admin::builder("/tmp/test_db", object_store).build();
    /// }
    /// ```
    pub fn builder<P: Into<Path>>(path: P, object_store: Arc<dyn ObjectStore>) -> AdminBuilder<P> {
        AdminBuilder::new(path, object_store)
    }
}

fn get_env_variable(name: &str) -> Result<String, SlateDBError> {
    env::var(name).map_err(|e| match e {
        VarError::NotPresent => SlateDBError::InvalidEnvironmentVariable {
            key: name.to_string(),
            value: None,
        },
        VarError::NotUnicode(not_unicode_value) => SlateDBError::InvalidEnvironmentVariable {
            key: name.to_string(),
            value: Some(format!("{:?}", not_unicode_value)),
        },
    })
}

/// Loads an object store from configured environment variables.
/// The provider is specified using the CLOUD_PROVIDER variable.
/// For specific provider configurations, see the corresponding
/// method documentation:
///
/// | Provider | Value | Documentation |
/// |----------|-------|---------------|
/// | Local | `local` | [load_local] |
/// | Memory | `memory` | [load_memory] |
/// | AWS | `aws` | [load_aws] |
/// | Azure | `azure` | [load_azure] |
/// | GCP | `gcp` | [load_gcp] |
pub fn load_object_store_from_env(
    env_file: Option<String>,
) -> Result<Arc<dyn ObjectStore>, crate::Error> {
    dotenvy::from_filename(env_file.unwrap_or(String::from(".env"))).ok();
    let cloud_provider = get_env_variable("CLOUD_PROVIDER")?;
    match cloud_provider.to_lowercase().as_str() {
        "local" => load_local(),
        "memory" => load_memory(),
        #[cfg(feature = "aws")]
        "aws" => load_aws(),
        #[cfg(feature = "azure")]
        "azure" => load_azure(),
        #[cfg(feature = "gcp")]
        "gcp" => load_gcp(),
        invalid_value => Err(SlateDBError::InvalidEnvironmentVariable {
            key: "CLOUD_PROVIDER".to_string(),
            value: Some(invalid_value.to_string()),
        }
        .into()),
    }
}

/// Loads a local object store instance.
///
/// | Env Variable | Doc | Required |
/// |--------------|-----|----------|
/// | LOCAL_PATH | The path to the local directory where all data will be stored | Yes |
pub fn load_local() -> Result<Arc<dyn ObjectStore>, crate::Error> {
    let local_path = get_env_variable("LOCAL_PATH")?;
    let lfs =
        object_store::local::LocalFileSystem::new_with_prefix(local_path).map_err(|error| {
            SlateDBError::ObjectStoreError(Arc::new(object_store::Error::Generic {
                store: "local",
                source: Box::new(error),
            }))
        })?;
    Ok(Arc::new(lfs) as Arc<dyn ObjectStore>)
}

/// Loads an in-memory object store instance.
pub fn load_memory() -> Result<Arc<dyn ObjectStore>, crate::Error> {
    Ok(Arc::new(object_store::memory::InMemory::new()) as Arc<dyn ObjectStore>)
}

/// Loads an AWS S3 Object store instance. The environment variables consumed are
/// the same as those supported by [`AmazonS3Builder::from_env`]. Refer to the
/// builder documentation for the full list and meaning of supported variables:
/// <https://docs.rs/object_store/latest/object_store/aws/struct.AmazonS3Builder.html#method.with_config>
#[cfg(feature = "aws")]
pub fn load_aws() -> Result<Arc<dyn ObjectStore>, crate::Error> {
    let builder = object_store::aws::AmazonS3Builder::from_env();

    Ok(Arc::new(builder.build().map_err(|error| {
        SlateDBError::ObjectStoreError(Arc::new(object_store::Error::Generic {
            store: "AmazonS3",
            source: Box::new(error),
        }))
    })?) as Arc<dyn ObjectStore>)
}

/// Loads an Azure Object store instance. The environment variables consumed are
/// the same as those supported by [`MicrosoftAzureBuilder::from_env`]. Refer to
/// the builder documentation for the full list and meaning of supported variables:
/// <https://docs.rs/object_store/latest/object_store/azure/struct.MicrosoftAzureBuilder.html#method.with_config>
#[cfg(feature = "azure")]
pub fn load_azure() -> Result<Arc<dyn ObjectStore>, crate::Error> {
    let builder = object_store::azure::MicrosoftAzureBuilder::from_env();
    Ok(Arc::new(builder.build().map_err(|error| {
        SlateDBError::ObjectStoreError(Arc::new(object_store::Error::Generic {
            store: "MicrosoftAzure",
            source: Box::new(error),
        }))
    })?) as Arc<dyn ObjectStore>)
}

/// Loads a Google Cloud Storage object store instance. The environment variables
/// consumed are the same as those supported by [`GoogleCloudStorageBuilder::from_env`].
/// Refer to the builder documentation for the full list and meaning of supported variables:
/// <https://docs.rs/object_store/latest/object_store/gcp/struct.GoogleCloudStorageBuilder.html#method.with_config>
#[cfg(feature = "gcp")]
pub fn load_gcp() -> Result<Arc<dyn ObjectStore>, crate::Error> {
    let builder = object_store::gcp::GoogleCloudStorageBuilder::from_env();
    Ok(Arc::new(builder.build().map_err(|error| {
        SlateDBError::ObjectStoreError(Arc::new(object_store::Error::Generic {
            store: "GoogleCloudStorage",
            source: Box::new(error),
        }))
    })?) as Arc<dyn ObjectStore>)
}

/// Collects every object path under `prefix` in the given store.
async fn collect_prefix(
    store: &Arc<dyn ObjectStore>,
    prefix: &Path,
) -> Result<Vec<Path>, crate::Error> {
    let mut listing = store.list(Some(prefix));
    let mut paths = Vec::new();
    while let Some(meta) = listing
        .next()
        .await
        .transpose()
        .map_err(SlateDBError::from)?
    {
        paths.push(meta.location);
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use crate::admin::{load_object_store_from_env, AdminBuilder};
    use crate::compactions_store::{CompactionsStore, StoredCompactions};
    use crate::compactor_state::{Compaction, CompactionSpec, CompactionStatus, SourceId};
    use crate::config::{
        CheckpointOptions, CompactionWorkerOptions, CompactorOptions, GarbageCollectorOptions,
    };
    use crate::manifest::store::{ManifestStore, StoredManifest};
    use crate::manifest::ManifestCore;
    use crate::test_utils::{FlakyObjectStore, StringConcatMergeOperator};
    use crate::ErrorKind;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use slatedb_common::clock::DefaultSystemClock;
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;
    use ulid::Ulid;

    #[test]
    fn test_load_object_store_from_env() {
        figment::Jail::expect_with(|jail| {
            // creating an object store without CLOUD_PROVIDER env variable
            let err = load_object_store_from_env(None).expect_err("expected invalid env error");
            assert_eq!(err.kind(), ErrorKind::Invalid);
            assert_eq!(
                err.to_string(),
                "Invalid error: invalid environment variable CLOUD_PROVIDER value `null`"
            );

            jail.create_file("invalid.env", "CLOUD_PROVIDER=invalid")
                .expect("failed to create temp env file");
            let err = load_object_store_from_env(Some("invalid.env".to_string()))
                .expect_err("expected invalid provider error");
            assert_eq!(err.kind(), ErrorKind::Invalid);
            assert_eq!(
                err.to_string(),
                "Invalid error: invalid environment variable CLOUD_PROVIDER value `invalid`"
            );
            // unset since the environment variable loaded in from invalid.env
            // takes precedence over the memory.env file.
            std::env::remove_var("CLOUD_PROVIDER");

            jail.create_file("memory.env", "CLOUD_PROVIDER=memory")
                .expect("failed to create temp env file");
            let r = load_object_store_from_env(Some("memory.env".to_string()));
            let store = r.expect("expected memory object store");
            assert_eq!(store.to_string(), "InMemory");

            Ok(())
        });
    }

    #[test]
    fn test_load_local_invalid_path_maps_to_unavailable() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("LOCAL_PATH", "missing-local-path");

            let err = super::load_local().expect_err("expected invalid local-path error");

            assert_eq!(err.kind(), ErrorKind::Unavailable);
            Ok(())
        });
    }

    #[tokio::test]
    async fn test_admin_read_manifest() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_admin_read_manifest");
        let manifest_store = Arc::new(ManifestStore::new(&path, object_store.clone()));
        let mut stored = StoredManifest::create_new_db(
            manifest_store,
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let mut dirty = stored.prepare_dirty().unwrap();
        dirty.value.core.next_wal_sst_id = 17;
        dirty.value.core.last_l0_seq = 9;
        dirty.value.writer_epoch = 3;
        dirty.value.compactor_epoch = 5;
        stored.update(dirty).await.unwrap();

        let admin = AdminBuilder::new(path.clone(), object_store).build();

        let latest = admin
            .read_manifest(None)
            .await
            .unwrap()
            .expect("expected manifest");
        assert_eq!(latest.id, 2);
        assert_eq!(latest.manifest.writer_epoch, 3);
        assert_eq!(latest.manifest.compactor_epoch, 5);
        assert_eq!(latest.manifest.core.next_wal_sst_id, 17);
        assert_eq!(latest.manifest.core.last_l0_seq, 9);

        let first = admin
            .read_manifest(Some(1))
            .await
            .unwrap()
            .expect("expected manifest");
        assert_eq!(first.id, 1);
        assert_eq!(first.manifest.writer_epoch, 0);
        assert_eq!(first.manifest.compactor_epoch, 0);
        assert_eq!(first.manifest.core.next_wal_sst_id, 1);
        assert_eq!(first.manifest.core.last_l0_seq, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn test_admin_run_gc_with_options_stops_on_cancellation() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_admin_run_gc_with_options_stops_on_cancellation");
        let admin = AdminBuilder::new(path, object_store).build();
        let cancellation_token = CancellationToken::new();
        cancellation_token.cancel();

        let result = admin
            .run_gc_with_options(cancellation_token, GarbageCollectorOptions::default())
            .await;
        assert!(matches!(result, Ok(())));
    }

    #[tokio::test(start_paused = true)]
    async fn test_admin_run_compactor_with_options_stops_on_cancellation() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_admin_run_compactor_with_options_stops_on_cancellation");
        StoredManifest::create_new_db(
            Arc::new(ManifestStore::new(&path, object_store.clone())),
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let admin = AdminBuilder::new(path, object_store).build();
        let cancellation_token = CancellationToken::new();
        cancellation_token.cancel();

        let result = admin
            .run_compactor_with_options(
                cancellation_token,
                CompactorOptions {
                    worker: None,
                    ..CompactorOptions::default()
                },
            )
            .await;
        assert!(matches!(result, Ok(())));
    }

    #[tokio::test(start_paused = true)]
    async fn test_admin_run_compaction_worker_with_options_stops_on_cancellation() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path =
            Path::from("/tmp/test_admin_run_compaction_worker_with_options_stops_on_cancellation");
        let admin = AdminBuilder::new(path, object_store).build();
        let cancellation_token = CancellationToken::new();
        cancellation_token.cancel();

        let result = admin
            .run_compaction_worker_with_options(
                cancellation_token,
                CompactionWorkerOptions::default(),
            )
            .await;
        assert!(matches!(result, Ok(())));
    }

    #[tokio::test]
    async fn test_admin_list_manifests() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_admin_list_manifests");
        let manifest_store = Arc::new(ManifestStore::new(&path, object_store.clone()));
        let mut stored = StoredManifest::create_new_db(
            manifest_store,
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let mut dirty = stored.prepare_dirty().unwrap();
        dirty.value.core.next_wal_sst_id = 5;
        dirty.value.core.last_l0_seq = 10;
        dirty.value.writer_epoch = 2;
        dirty.value.compactor_epoch = 4;
        stored.update(dirty).await.unwrap();

        let mut dirty = stored.prepare_dirty().unwrap();
        dirty.value.core.next_wal_sst_id = 8;
        dirty.value.core.last_l0_seq = 20;
        dirty.value.writer_epoch = 3;
        dirty.value.compactor_epoch = 6;
        stored.update(dirty).await.unwrap();

        let admin = AdminBuilder::new(path.clone(), object_store).build();

        let all = admin.list_manifests(..).await.unwrap();
        assert_eq!(
            all.iter().map(|manifest| manifest.id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            all.iter()
                .map(|manifest| manifest.manifest.core.last_l0_seq)
                .collect::<Vec<_>>(),
            vec![0, 10, 20]
        );
        assert_eq!(
            all.iter()
                .map(|manifest| manifest.manifest.writer_epoch)
                .collect::<Vec<_>>(),
            vec![0, 2, 3]
        );
        assert_eq!(
            all.iter()
                .map(|manifest| manifest.manifest.compactor_epoch)
                .collect::<Vec<_>>(),
            vec![0, 4, 6]
        );

        let bounded = admin.list_manifests(2..3).await.unwrap();
        assert_eq!(
            bounded
                .iter()
                .map(|manifest| manifest.id)
                .collect::<Vec<_>>(),
            vec![2]
        );

        let left_bounded = admin.list_manifests(2..).await.unwrap();
        assert_eq!(
            left_bounded
                .iter()
                .map(|manifest| manifest.id)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );

        let right_bounded = admin.list_manifests(..3).await.unwrap();
        assert_eq!(
            right_bounded
                .iter()
                .map(|manifest| manifest.id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[tokio::test]
    async fn test_admin_list_manifests_retries_transient_failure() {
        // Admin operations wrap the object store in a RetryingObjectStore, so a
        // transient list failure should be retried rather than surfaced.
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let flaky = Arc::new(FlakyObjectStore::new(inner, 0).with_list_failures(1, 0));
        let path = Path::from("/tmp/test_admin_list_manifests_retries_transient_failure");
        let admin = AdminBuilder::new(path, flaky.clone()).build();

        let manifests = admin
            .list_manifests(..)
            .await
            .expect("list should succeed after retrying the transient failure");

        assert!(manifests.is_empty());
        // 1 transient failure + 1 successful retry.
        assert_eq!(flaky.list_attempts(), 2);
    }

    #[tokio::test]
    async fn test_admin_create_detached_checkpoint_retries_transient_put() {
        // A transient put failure during checkpoint creation should be retried
        // by the RetryingObjectStore rather than failing the operation.
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_admin_create_detached_checkpoint_retries_transient_put");
        let db = crate::Db::open(path.clone(), inner.clone()).await.unwrap();
        db.put(b"key", b"value").await.unwrap();
        db.close().await.unwrap();

        // Fail the first put_opts, which the retrying store should transparently retry.
        let flaky = Arc::new(FlakyObjectStore::new(inner, 1));
        let admin = AdminBuilder::new(path, flaky.clone()).build();

        admin
            .create_detached_checkpoint(&CheckpointOptions::default())
            .await
            .expect("checkpoint should succeed after retrying the transient put");

        assert!(flaky.put_attempts() >= 2);
    }

    #[tokio::test]
    async fn test_admin_terminal_object_store_error_maps_to_unavailable() {
        // The retry layer retries transient errors forever, so it never exhausts
        // and surfaces a transient failure. A terminal (non-retryable) error,
        // however, must still pass through the retry wrapper and map to
        // ErrorKind::Unavailable rather than being swallowed. A conditional put
        // that always fails with Precondition is such a terminal error.
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_admin_terminal_object_store_error_maps_to_unavailable");
        let db = crate::Db::open(path.clone(), inner.clone()).await.unwrap();
        db.put(b"key", b"value").await.unwrap();
        db.close().await.unwrap();

        let failing = Arc::new(FlakyObjectStore::new(inner, 0).with_put_precondition_always());
        let admin = AdminBuilder::new(path, failing.clone()).build();

        let err = admin
            .create_detached_checkpoint(&CheckpointOptions::default())
            .await
            .expect_err("expected terminal precondition failure to surface");

        assert_eq!(err.kind(), ErrorKind::Unavailable);
        // Terminal error: attempted exactly once, no retries.
        assert_eq!(failing.put_attempts(), 1);
    }

    #[tokio::test]
    async fn test_admin_read_compactor_state_view_missing_manifest_maps_to_data() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_admin_read_compactor_state_view_missing_manifest");
        let admin = AdminBuilder::new(path, object_store).build();

        let err = admin
            .read_compactor_state_view()
            .await
            .err()
            .expect("expected missing manifest error");

        assert_eq!(err.kind(), ErrorKind::Data);
    }

    #[tokio::test]
    async fn test_admin_read_compactions() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_admin_read_compactions");
        let compactions_store = Arc::new(CompactionsStore::new(&path, object_store.clone()));
        let mut stored = StoredCompactions::create(compactions_store.clone(), 7)
            .await
            .unwrap();

        let compaction_id = Ulid::new();
        let compaction = Compaction::new(
            compaction_id,
            CompactionSpec::new(vec![SourceId::SortedRun(3)], 7),
        );
        let mut dirty = stored.prepare_dirty().unwrap();
        dirty.value.insert(compaction);
        dirty.value.compactor_epoch = 9;
        stored.update(dirty).await.unwrap();

        let admin = AdminBuilder::new(path.clone(), object_store).build();

        let latest = admin
            .read_compactions(None)
            .await
            .unwrap()
            .expect("expected compactions");
        let expected_latest = compactions_store.read_compactions(2).await.unwrap();
        assert_eq!(latest.id, 2);
        assert_eq!(latest.compactions.compactor_epoch, 9);
        assert_eq!(latest.compactions, expected_latest);

        let first = admin
            .read_compactions(Some(1))
            .await
            .unwrap()
            .expect("expected compactions");
        let expected_first = compactions_store.read_compactions(1).await.unwrap();
        assert_eq!(first.id, 1);
        assert_eq!(first.compactions.compactor_epoch, 7);
        assert_eq!(first.compactions, expected_first);
    }

    #[tokio::test]
    async fn test_admin_list_compactions() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_admin_list_compactions");
        let compactions_store = Arc::new(CompactionsStore::new(&path, object_store.clone()));
        let mut stored = StoredCompactions::create(compactions_store.clone(), 2)
            .await
            .unwrap();

        let mut dirty = stored.prepare_dirty().unwrap();
        dirty.value.insert(Compaction::new(
            Ulid::new(),
            CompactionSpec::new(vec![SourceId::SortedRun(3)], 7),
        ));
        dirty.value.compactor_epoch = 4;
        stored.update(dirty).await.unwrap();

        let mut dirty = stored.prepare_dirty().unwrap();
        dirty.value.insert(Compaction::new(
            Ulid::new(),
            CompactionSpec::new(vec![SourceId::SortedRun(5)], 9),
        ));
        dirty.value.compactor_epoch = 6;
        stored.update(dirty).await.unwrap();

        let admin = AdminBuilder::new(path.clone(), object_store).build();
        let listed = admin.list_compactions(..).await.unwrap();
        let ids: Vec<u64> = listed.iter().map(|compactions| compactions.id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
        assert_eq!(
            listed
                .iter()
                .map(|compactions| compactions.compactions.core.recent_compactions().count())
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(
            listed
                .iter()
                .map(|compactions| compactions.compactions.compactor_epoch)
                .collect::<Vec<_>>(),
            vec![2, 4, 6]
        );

        let bounded = admin.list_compactions(2..3).await.unwrap();
        assert_eq!(
            bounded
                .iter()
                .map(|compactions| compactions.id)
                .collect::<Vec<_>>(),
            vec![2]
        );

        let left_bounded = admin.list_compactions(2..).await.unwrap();
        assert_eq!(
            left_bounded
                .iter()
                .map(|compactions| compactions.id)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );

        let right_bounded = admin.list_compactions(..3).await.unwrap();
        assert_eq!(
            right_bounded
                .iter()
                .map(|compactions| compactions.id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[tokio::test]
    async fn test_admin_read_compaction() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_admin_read_compaction");
        let compactions_store = Arc::new(CompactionsStore::new(&path, object_store.clone()));
        let mut stored = StoredCompactions::create(compactions_store.clone(), 0)
            .await
            .unwrap();

        let compaction_id = Ulid::new();
        let compaction = Compaction::new(
            compaction_id,
            CompactionSpec::new(vec![SourceId::SortedRun(3)], 7),
        );
        let mut dirty = stored.prepare_dirty().unwrap();
        dirty.value.insert(compaction);
        stored.update(dirty).await.unwrap();

        let admin = AdminBuilder::new(path.clone(), object_store).build();
        let compaction = admin
            .read_compaction(compaction_id, None)
            .await
            .unwrap()
            .expect("expected compaction");
        assert_eq!(compaction.id(), compaction_id);
    }

    #[tokio::test]
    async fn test_admin_submit_compaction() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_admin_submit_compaction");
        let compactions_store = Arc::new(CompactionsStore::new(&path, object_store.clone()));
        StoredCompactions::create(compactions_store.clone(), 0)
            .await
            .unwrap();

        let admin = AdminBuilder::new(path.clone(), object_store).build();
        let spec = CompactionSpec::new(vec![SourceId::SortedRun(3)], 3);
        let compaction = admin.submit_compaction(spec).await.unwrap();

        assert_eq!(compaction.spec().destination(), Some(3));
        assert_eq!(compaction.spec().sources(), &[SourceId::SortedRun(3)]);
        assert_eq!(compaction.status(), CompactionStatus::Submitted);
    }

    #[cfg(feature = "compaction_filters")]
    #[test]
    fn test_admin_builder_with_compaction_filter_supplier() {
        use crate::compaction_filter::{
            CompactionFilter, CompactionFilterDecision, CompactionFilterError,
            CompactionFilterSupplier, CompactionJobContext,
        };
        use crate::types::RowEntry;

        struct NoopFilter;

        #[async_trait::async_trait]
        impl CompactionFilter for NoopFilter {
            async fn filter(
                &mut self,
                _entry: &RowEntry,
            ) -> Result<CompactionFilterDecision, CompactionFilterError> {
                Ok(CompactionFilterDecision::Keep)
            }
            async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
                Ok(())
            }
        }

        struct NoopFilterSupplier;

        #[async_trait::async_trait]
        impl CompactionFilterSupplier for NoopFilterSupplier {
            async fn create_compaction_filter(
                &self,
                _context: &CompactionJobContext,
            ) -> Result<Box<dyn CompactionFilter>, CompactionFilterError> {
                Ok(Box::new(NoopFilter))
            }
        }

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let admin = AdminBuilder::new("/tmp/test_filter_supplier", object_store)
            .with_compaction_filter_supplier(Arc::new(NoopFilterSupplier))
            .build();

        assert!(admin.compaction_filter_supplier.is_some());
    }

    #[test]
    fn test_admin_builder_with_merge_operator() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let admin = AdminBuilder::new("/tmp/test_merge_operator", object_store)
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build();

        assert!(admin.merge_operator.is_some());
    }

    #[tokio::test]
    async fn test_create_clone_builder() {
        use crate::admin::CloneSourceSpec;
        use crate::manifest::store::ManifestStore;
        use crate::Db;

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let parent_path = Path::from("/tmp/test_parent");
        let clone_path = Path::from("/tmp/test_clone");

        let parent_db = Db::open(parent_path.clone(), object_store.clone())
            .await
            .unwrap();
        parent_db.close().await.unwrap();

        let admin = AdminBuilder::new(clone_path.clone(), object_store.clone()).build();

        // Test basic builder without checkpoint
        let r = admin.create_clone_builder_from_source(CloneSourceSpec::new(parent_path.clone()));
        r.build().await.expect("clone should succeed");

        // Verify clone was created
        let clone_manifest_store = ManifestStore::new(&clone_path, object_store.clone());
        let manifest = clone_manifest_store.read_latest_manifest().await;
        assert!(manifest.is_ok(), "cloned manifest should exist");
    }

    #[tokio::test]
    async fn test_delete_db_removes_checkpoint_from_parent() {
        use crate::admin::CloneSourceSpec;
        use crate::config::CheckpointOptions;
        use crate::manifest::store::{ManifestStore, StoredManifest};
        use crate::Db;

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let system_clock = Arc::new(DefaultSystemClock::new());
        let parent_path = Path::from("/tmp/test_cleanup_parent");
        let clone_path = Path::from("/tmp/test_cleanup_clone");

        let parent_db = Db::open(parent_path.clone(), object_store.clone())
            .await
            .unwrap();
        parent_db.close().await.unwrap();

        // An unrelated checkpoint in the parent that cleanup must not touch.
        let parent_admin = AdminBuilder::new(parent_path.clone(), object_store.clone()).build();
        let unrelated = parent_admin
            .create_detached_checkpoint(&CheckpointOptions::default())
            .await
            .unwrap()
            .id;

        let clone_admin = AdminBuilder::new(clone_path.clone(), object_store.clone()).build();
        clone_admin
            .create_clone_builder_from_source(CloneSourceSpec::new(parent_path.clone()))
            .build()
            .await
            .expect("clone should succeed");

        // The checkpoint the clone pinned in the parent.
        let clone_ms = Arc::new(ManifestStore::new(&clone_path, object_store.clone()));
        let clone_stored = StoredManifest::load(clone_ms, system_clock.clone())
            .await
            .unwrap();
        let pinned = clone_stored.manifest().external_dbs[0]
            .final_checkpoint_id
            .expect("clone pins a final_checkpoint_id in the parent");

        let read_parent_checkpoints = || {
            let object_store = object_store.clone();
            let system_clock = system_clock.clone();
            let parent_path = parent_path.clone();
            async move {
                let ms = Arc::new(ManifestStore::new(&parent_path, object_store));
                let stored = StoredManifest::load(ms, system_clock).await.unwrap();
                stored
                    .manifest()
                    .core
                    .checkpoints
                    .iter()
                    .map(|c| c.id)
                    .collect::<Vec<_>>()
            }
        };

        let before = read_parent_checkpoints().await;
        assert!(
            before.contains(&pinned),
            "parent should have pinned checkpoint before cleanup"
        );
        assert!(
            before.contains(&unrelated),
            "parent should have unrelated checkpoint"
        );

        clone_admin
            .delete_db(true)
            .await
            .expect("delete should succeed");

        let after = read_parent_checkpoints().await;
        assert!(
            !after.contains(&pinned),
            "pinned checkpoint should be gone from parent"
        );
        assert!(
            after.contains(&unrelated),
            "unrelated checkpoint should remain"
        );
    }

    #[tokio::test]
    async fn test_delete_db_deletes_clone_after_parent_already_gone() {
        use crate::admin::CloneSourceSpec;
        use crate::Db;
        use futures::StreamExt;
        use object_store::ObjectStoreExt;

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let parent_path = Path::from("/tmp/test_delete_orphan_parent");
        let clone_path = Path::from("/tmp/test_delete_orphan_clone");

        Db::open(parent_path.clone(), object_store.clone())
            .await
            .unwrap()
            .close()
            .await
            .unwrap();

        let clone_admin = AdminBuilder::new(clone_path.clone(), object_store.clone()).build();
        clone_admin
            .create_clone_builder_from_source(CloneSourceSpec::new(parent_path.clone()))
            .build()
            .await
            .expect("clone should succeed");

        // Wipe the parent out from under the clone. The clone still names it in
        // external_dbs with a pinned checkpoint, but the parent manifest is gone.
        let mut parent_listing = object_store.list(Some(&parent_path));
        while let Some(meta) = parent_listing.next().await {
            object_store.delete(&meta.unwrap().location).await.unwrap();
        }

        // A missing parent means there is no checkpoint left to strip, so the
        // clone must still be deletable rather than wedged on a Data error.
        clone_admin
            .delete_db(true)
            .await
            .expect("clone should delete even with parent already gone");
        assert_eq!(
            object_store.list(Some(&clone_path)).count().await,
            0,
            "clone objects should be gone"
        );
    }

    #[tokio::test]
    async fn test_delete_db_deletes_own_objects_only_with_confirm() {
        use crate::admin::CloneSourceSpec;
        use crate::Db;
        use futures::StreamExt;

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let parent_path = Path::from("/tmp/test_delete_confirm_parent");
        let clone_path = Path::from("/tmp/test_delete_confirm_clone");

        Db::open(parent_path.clone(), object_store.clone())
            .await
            .unwrap()
            .close()
            .await
            .unwrap();

        let clone_admin = AdminBuilder::new(clone_path.clone(), object_store.clone()).build();
        clone_admin
            .create_clone_builder_from_source(CloneSourceSpec::new(parent_path.clone()))
            .build()
            .await
            .expect("clone should succeed");

        let count_under = |prefix: Path| {
            let object_store = object_store.clone();
            async move { object_store.list(Some(&prefix)).count().await }
        };

        let initial = count_under(clone_path.clone()).await;
        assert!(initial > 0, "clone should have objects");

        // confirm = false is a dry run: it reports what it would delete and
        // touches nothing.
        let would_delete = clone_admin
            .delete_db(false)
            .await
            .expect("dry run should succeed");
        assert_eq!(
            would_delete.len(),
            initial,
            "dry run should report every object under the prefix"
        );
        assert_eq!(
            count_under(clone_path.clone()).await,
            initial,
            "dry run must not delete anything"
        );

        // confirm = true deletes the clone's own objects.
        clone_admin
            .delete_db(true)
            .await
            .expect("delete should succeed");
        assert_eq!(
            count_under(clone_path.clone()).await,
            0,
            "clone objects should be gone after confirm"
        );

        // Idempotent: a second run over an already-deleted db is a clean no-op.
        clone_admin
            .delete_db(true)
            .await
            .expect("second delete should be a no-op");
    }

    #[tokio::test]
    async fn test_delete_db_finishes_partial_deletion_via_marker() {
        use crate::admin::CloneSourceSpec;
        use crate::Db;
        use futures::StreamExt;
        use object_store::ObjectStoreExt;

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let parent_path = Path::from("/tmp/test_delete_partial_parent");
        let clone_path = Path::from("/tmp/test_delete_partial_clone");

        Db::open(parent_path.clone(), object_store.clone())
            .await
            .unwrap()
            .close()
            .await
            .unwrap();

        let clone_admin = AdminBuilder::new(clone_path.clone(), object_store.clone()).build();
        clone_admin
            .create_clone_builder_from_source(CloneSourceSpec::new(parent_path.clone()))
            .build()
            .await
            .expect("clone should succeed");

        // Simulate a crash mid-delete: the marker was written, then the manifests
        // (and some objects) were removed, but the marker and other objects remain.
        object_store
            .put(
                &clone_path.clone().join(".deleting"),
                bytes::Bytes::new().into(),
            )
            .await
            .unwrap();
        let manifest_prefix = Path::from("/tmp/test_delete_partial_clone/manifest");
        let mut listing = object_store.list(Some(&manifest_prefix));
        while let Some(meta) = listing.next().await {
            object_store.delete(&meta.unwrap().location).await.unwrap();
        }
        let count_under = |prefix: Path| {
            let object_store = object_store.clone();
            async move { object_store.list(Some(&prefix)).count().await }
        };
        assert!(
            count_under(clone_path.clone()).await > 0,
            "leftover clone objects should remain after partial deletion"
        );

        // The marker proves this is a real slatedb dir, so delete resumes and
        // finishes the job even though the manifest is already gone.
        clone_admin
            .delete_db(true)
            .await
            .expect("delete should finish partial deletion");
        assert_eq!(
            count_under(clone_path.clone()).await,
            0,
            "leftover objects, including the marker, should be gone"
        );
    }

    #[tokio::test]
    async fn test_delete_db_refuses_without_manifest_or_marker() {
        use futures::StreamExt;
        use object_store::ObjectStoreExt;

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let dir = Path::from("/tmp/test_delete_fat_finger");

        // A directory that is NOT a slatedb dir: no manifest, no .deleting marker.
        // This stands in for a fat-fingered --path.
        object_store
            .put(
                &dir.clone().join("important.txt"),
                bytes::Bytes::from_static(b"keepme").into(),
            )
            .await
            .unwrap();

        let admin = AdminBuilder::new(dir.clone(), object_store.clone()).build();
        admin
            .delete_db(true)
            .await
            .expect_err("delete must refuse a dir with no manifest and no marker");

        let count = object_store.list(Some(&dir)).count().await;
        assert_eq!(count, 1, "the unrelated object must be left untouched");
    }

    #[cfg(feature = "wal_disable")]
    #[tokio::test]
    async fn test_create_clone_with_multiple_sources() {
        use crate::config::{PutOptions, Settings, WriteOptions};
        use crate::manifest::store::ManifestStore;
        use crate::{admin::CloneSourceSpec, Db};

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let grandparent_path1 = Path::from("/tmp/test_grandparent1");
        let grandparent_path2 = Path::from("/tmp/test_grandparent2");
        let parent_path1 = Path::from("/tmp/test_parent1");
        let parent_path2 = Path::from("/tmp/test_parent2");
        let clone_path = Path::from("/tmp/test_clone_multi");

        let settings = Settings {
            wal_enabled: false,
            ..Settings::default()
        };
        let write_opts = WriteOptions {
            ..Default::default()
        };

        // Two grandparents, each with a single-key SST. Disjoint ranges are required
        // because the union path rejects overlapping source manifests.
        let grandparent_db1 = Db::builder(grandparent_path1.clone(), object_store.clone())
            .with_settings(settings.clone())
            .build()
            .await
            .unwrap();
        grandparent_db1
            .put_with_options(b"a", b"1", &PutOptions::default(), &write_opts)
            .await
            .unwrap();
        grandparent_db1.close().await.unwrap();

        let grandparent_db2 = Db::builder(grandparent_path2.clone(), object_store.clone())
            .with_settings(settings)
            .build()
            .await
            .unwrap();
        grandparent_db2
            .put_with_options(b"z", b"2", &PutOptions::default(), &write_opts)
            .await
            .unwrap();
        grandparent_db2.close().await.unwrap();

        // Make each source a clone, so its manifest carries an external_db entry that
        // propagates through `cloned_from_union`.
        AdminBuilder::new(parent_path1.clone(), object_store.clone())
            .build()
            .create_clone_builder_from_source(CloneSourceSpec::new(grandparent_path1.clone()))
            .build()
            .await
            .expect("parent clone 1 should succeed");

        AdminBuilder::new(parent_path2.clone(), object_store.clone())
            .build()
            .create_clone_builder_from_source(CloneSourceSpec::new(grandparent_path2.clone()))
            .build()
            .await
            .expect("parent clone 2 should succeed");

        let admin = AdminBuilder::new(clone_path.clone(), object_store.clone()).build();

        admin
            .create_clone_builder_from_source(CloneSourceSpec::new(parent_path1.clone()))
            .with_source(CloneSourceSpec::new(parent_path2.clone()))
            .build()
            .await
            .expect("clone with multiple sources should succeed");

        let clone_manifest_store = ManifestStore::new(&clone_path, object_store.clone());
        let manifest = clone_manifest_store.read_latest_manifest().await;
        assert!(manifest.is_ok(), "cloned manifest should exist");

        let manifest_data = manifest.unwrap();
        assert_eq!(
            manifest_data.manifest.external_dbs.len(),
            2,
            "clone should have an external database for each parent"
        );
    }

    #[cfg(feature = "wal_disable")]
    #[tokio::test]
    async fn test_delete_db_removes_checkpoints_from_all_parents() {
        use crate::config::{PutOptions, Settings, WriteOptions};
        use crate::manifest::store::{ManifestStore, StoredManifest};
        use crate::{admin::CloneSourceSpec, Db};
        use uuid::Uuid;

        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let system_clock = Arc::new(DefaultSystemClock::new());
        let parent_path1 = Path::from("/tmp/test_cleanup_multi_parent1");
        let parent_path2 = Path::from("/tmp/test_cleanup_multi_parent2");
        let clone_path = Path::from("/tmp/test_cleanup_multi_clone");

        let settings = Settings {
            wal_enabled: false,
            ..Settings::default()
        };
        let write_opts = WriteOptions::default();

        // Two parents with disjoint single-key SSTs (the union path rejects overlaps).
        for (path, key) in [(&parent_path1, b"a"), (&parent_path2, b"z")] {
            let db = Db::builder(path.clone(), object_store.clone())
                .with_settings(settings.clone())
                .build()
                .await
                .unwrap();
            db.put_with_options(key, b"1", &PutOptions::default(), &write_opts)
                .await
                .unwrap();
            db.close().await.unwrap();
        }

        let clone_admin = AdminBuilder::new(clone_path.clone(), object_store.clone()).build();
        clone_admin
            .create_clone_builder_from_source(CloneSourceSpec::new(parent_path1.clone()))
            .with_source(CloneSourceSpec::new(parent_path2.clone()))
            .build()
            .await
            .expect("clone with multiple sources should succeed");

        // Collect the checkpoint each parent got pinned with.
        let clone_ms = Arc::new(ManifestStore::new(&clone_path, object_store.clone()));
        let clone_stored = StoredManifest::load(clone_ms, system_clock.clone())
            .await
            .unwrap();
        let pinned: Vec<(String, Uuid)> = clone_stored
            .manifest()
            .external_dbs
            .iter()
            .map(|e| (e.path.clone(), e.final_checkpoint_id.unwrap()))
            .collect();
        assert_eq!(pinned.len(), 2);

        clone_admin
            .delete_db(true)
            .await
            .expect("delete should succeed");

        for (parent_path, checkpoint_id) in pinned {
            let ms = Arc::new(ManifestStore::new(
                &parent_path.into(),
                object_store.clone(),
            ));
            let stored = StoredManifest::load(ms, system_clock.clone())
                .await
                .unwrap();
            assert!(
                !stored
                    .manifest()
                    .core
                    .checkpoints
                    .iter()
                    .any(|c| c.id == checkpoint_id),
                "pinned checkpoint should be removed from every parent"
            );
        }
    }
}

#[cfg(test)]
mod gc_tolerant_list_manifests_tests {
    use crate::admin::AdminBuilder;
    use crate::manifest::store::{ManifestStore, StoredManifest};
    use crate::manifest::ManifestCore;
    use crate::test_utils::FlakyObjectStore;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use slatedb_common::clock::DefaultSystemClock;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_list_manifests_skips_manifest_gced_between_list_and_read() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let flaky = Arc::new(FlakyObjectStore::new(inner, 0));
        let store: Arc<dyn ObjectStore> = flaky.clone();
        let path = Path::from("/tmp/test_gc_tolerant_list_manifests");

        let manifest_store = Arc::new(ManifestStore::new(&path, store.clone()));
        let mut sm = StoredManifest::create_new_db(
            manifest_store,
            ManifestCore::new(),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();
        sm.update(sm.prepare_dirty().unwrap()).await.unwrap();
        sm.update(sm.prepare_dirty().unwrap()).await.unwrap();

        let admin = AdminBuilder::new(path.clone(), store.clone()).build();
        let ids: Vec<u64> = admin
            .list_manifests(..)
            .await
            .unwrap()
            .iter()
            .map(|vm| vm.id())
            .collect();
        assert_eq!(ids, vec![1, 2, 3]);

        // manifest 1 is listed, but missing on read
        flaky.with_get_not_found_failures(1);

        let ids: Vec<u64> = admin
            .list_manifests(..)
            .await
            .unwrap()
            .iter()
            .map(|vm| vm.id())
            .collect();
        assert_eq!(
            ids,
            vec![2, 3],
            "a manifest GC'd mid-listing should be skipped, not fail the operation"
        );
    }
}
