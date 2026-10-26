// Local leadership/lifecycle modifications; see LEADERSHIP.md in this package.
//! Protocols for reading and writing compactor state safely.
//!
//! This module isolates the ordering rules for compactor state persistence:
//! - **Reads**: fetch compactions before manifests so GC sees a consistent view of
//!   in-flight/finished compactions alongside the active manifests.
//! - **Writes**: persist manifest updates before compactions so new SSTs are visible
//!   before trimming input references. Checkpoints are written first to keep inputs
//!   GC-safe during the update.
//!
//! Keeping these rules in one place makes it harder to regress GC safety or
//! compactor fencing logic elsewhere in the codebase.
use std::sync::Arc;
use std::time::Duration;

use log::{debug, info};

use crate::compactions_store::{CompactionsStore, FenceableCompactions, StoredCompactions};
use crate::compactor_state::{CompactionStatus, CompactorState, VersionedCompactions};
use crate::config::{CheckpointOptions, CompactorOptions};
use crate::error::SlateDBError;
use crate::manifest::store::{FenceableManifest, ManifestStore, StoredManifest};
use crate::manifest::VersionedManifest;
use crate::utils::IdGenerator;
use slatedb_common::clock::SystemClock;
use slatedb_common::DbRand;

/// A read-only view of compactor state suitable for consumers like GC.
///
/// This view intentionally avoids `DirtyObject` because reads should not create or mutate
/// remote state (e.g., a missing `.compactions` file on a fresh DB).
pub struct CompactorStateView {
    /// The latest compactions state if present, paired with its file version.
    pub(crate) compactions: Option<VersionedCompactions>,
    /// The latest manifest paired with its file version.
    pub(crate) manifest: VersionedManifest,
}

impl CompactorStateView {
    /// Returns a read-only view of the .compactions file if present.
    pub fn compactions(&self) -> Option<&VersionedCompactions> {
        self.compactions.as_ref()
    }

    /// Returns a read-only view of the .manifest file.
    pub fn manifest(&self) -> &VersionedManifest {
        &self.manifest
    }
}

/// Converts a full [`CompactorState`] into a read-only view.
impl From<&CompactorState> for CompactorStateView {
    fn from(state: &CompactorState) -> Self {
        CompactorStateView {
            compactions: Some(VersionedCompactions::from_compactions(
                state.compactions().id.into(),
                state.compactions().value.clone(),
            )),
            manifest: VersionedManifest::from_manifest(
                state.manifest().id.into(),
                state.manifest().value.clone(),
            ),
        }
    }
}

/// Reader that enforces compactions-first ordering when fetching state.
pub(crate) struct CompactorStateReader {
    /// Shared manifest store to read the latest manifest.
    manifest_store: Arc<ManifestStore>,
    /// Shared compactions store to fetch the latest compaction state first.
    compactions_store: Arc<CompactionsStore>,
}

impl CompactorStateReader {
    /// Creates a reader that returns a consistent view of compactions and active manifests.
    ///
    /// ## Arguments
    /// - `manifest_store`: Manifest store handle to read from.
    /// - `compactions_store`: Compactions store handle to read from.
    ///
    /// ## Returns
    /// - New reader instance that always fetches compactions before manifests.
    pub(crate) fn new(
        manifest_store: &Arc<ManifestStore>,
        compactions_store: &Arc<CompactionsStore>,
    ) -> Self {
        Self {
            manifest_store: manifest_store.clone(),
            compactions_store: compactions_store.clone(),
        }
    }

    /// Reads compactions then the latest manifest to keep consumers from observing an inconsistent view.
    pub(crate) async fn read_view(&self) -> Result<CompactorStateView, SlateDBError> {
        // Always read latest compactions before reading latest manifest.
        let compactions = self.compactions_store.try_read_latest_compactions().await?;
        let manifest = self.manifest_store.read_latest_manifest().await?;
        Ok(CompactorStateView {
            compactions,
            manifest,
        })
    }
}

/// Writer that fences and persists manifest-before-compactions with checkpointing.
pub(crate) struct CompactorStateWriter {
    /// Current in-memory compactor state (dirty manifest + compactions).
    pub(crate) state: CompactorState,
    /// Fenceable manifest handle used for refresh/update with fencing.
    manifest: FenceableManifest,
    /// Fenceable compactions handle used for refresh/update with fencing.
    compactions: FenceableCompactions,
    /// Lifetime of checkpoints that protect compaction inputs during manifest updates.
    checkpoint_lifetime: Duration,
    /// RNG for checkpoint ids.
    rand: Arc<DbRand>,
}

impl CompactorStateWriter {
    /// Initializes a fenced compactor state writer with manifest-first ordering.
    ///
    /// ## Arguments
    /// - `manifest_store`: Manifest store backing the writer.
    /// - `compactions_store`: Compactions store backing the writer.
    /// - `system_clock`: Clock for fencing/timeouts.
    /// - `options`: Compactor options containing timeouts.
    /// - `rand`: RNG for checkpoint ids.
    ///
    /// ## Returns
    /// - A new writer seeded with dirty manifest/compactions and finished compactions trimmed.
    #[cfg(test)]
    pub(crate) async fn new(
        manifest_store: Arc<ManifestStore>,
        compactions_store: Arc<CompactionsStore>,
        system_clock: Arc<dyn SystemClock>,
        options: &CompactorOptions,
        rand: Arc<DbRand>,
    ) -> Result<Self, SlateDBError> {
        Self::new_with_epoch(
            manifest_store,
            compactions_store,
            system_clock,
            options,
            rand,
            None,
        )
        .await
    }

    pub(crate) async fn new_with_epoch(
        manifest_store: Arc<ManifestStore>,
        compactions_store: Arc<CompactionsStore>,
        system_clock: Arc<dyn SystemClock>,
        options: &CompactorOptions,
        rand: Arc<DbRand>,
        leadership_epoch: Option<std::num::NonZeroU64>,
    ) -> Result<Self, SlateDBError> {
        let stored_manifest =
            StoredManifest::load(manifest_store.clone(), system_clock.clone()).await?;
        let (manifest, mut compactions) = Self::fence(
            stored_manifest,
            compactions_store,
            system_clock.clone(),
            options,
            leadership_epoch,
        )
        .await?;
        let dirty_manifest = manifest.prepare_dirty()?;
        let dirty_compactions = loop {
            let mut dirty_compactions = compactions.prepare_dirty()?;
            // Reset unclaimed scheduled compactions back to submitted on restart.
            // Scheduled compactions have no worker yet, so they always reset.
            // Stale Running compactions are left alone here: reclaim_stale_workers
            // reclaims them on the first tick (and before scheduling), so handling
            // them at startup too would just duplicate that logic.
            dirty_compactions.value.iter_mut().for_each(|c| {
                if matches!(c.status(), CompactionStatus::Scheduled) {
                    c.set_status(CompactionStatus::Submitted);
                    c.set_worker(None);
                }
            });
            dirty_compactions.value.retain_active_and_last_finished();
            match compactions.update(dirty_compactions.clone()).await {
                Ok(()) => break dirty_compactions,
                Err(err) if err.is_sequenced_write_conflict() => {
                    compactions.refresh().await?;
                }
                Err(err) => return Err(err),
            }
        };
        let state = CompactorState::new(dirty_manifest, dirty_compactions);
        Ok(Self {
            state,
            manifest,
            compactions,
            checkpoint_lifetime: options.checkpoint_lifetime,
            rand,
        })
    }

    async fn fence(
        stored_manifest: StoredManifest,
        compactions_store: Arc<CompactionsStore>,
        system_clock: Arc<dyn SystemClock>,
        options: &CompactorOptions,
        leadership_epoch: Option<std::num::NonZeroU64>,
    ) -> Result<(FenceableManifest, FenceableCompactions), SlateDBError> {
        let fenceable_manifest = match leadership_epoch {
            Some(epoch) => {
                FenceableManifest::init_compactor_with_epoch(
                    stored_manifest,
                    options.manifest_update_timeout,
                    system_clock.clone(),
                    epoch,
                )
                .await?
            }
            None => {
                FenceableManifest::init_compactor(
                    stored_manifest,
                    options.manifest_update_timeout,
                    system_clock.clone(),
                )
                .await?
            }
        };
        let stored_compactions =
            match StoredCompactions::try_load(compactions_store.clone()).await? {
                Some(compactions) => compactions,
                None => {
                    info!("creating new compactions file [compactor_epoch=0]");
                    StoredCompactions::create(compactions_store.clone(), 0).await?
                }
            };
        let fenceable_compactions = FenceableCompactions::init_with_epoch(
            stored_compactions,
            options.manifest_update_timeout,
            system_clock.clone(),
            fenceable_manifest.local_epoch(),
        )
        .await?;
        Ok((fenceable_manifest, fenceable_compactions))
    }

    /// Refreshes the manifest and updates the local compactor state with any remote
    /// changes.
    ///
    /// ## Returns
    /// - `Ok(())` after state is refreshed, or `SlateDBError` on failure.
    pub(crate) async fn load_manifest(&mut self) -> Result<(), SlateDBError> {
        self.manifest.refresh().await?;
        self.state
            .merge_remote_manifest(self.manifest.prepare_dirty()?);
        Ok(())
    }

    /// Refreshes the compactions view and updates the local compactor state with any remote
    /// changes.
    ///
    /// ## Returns
    /// - `Ok(())` after compactions are refreshed, or `SlateDBError` on failure.
    pub(crate) async fn load_compactions(&mut self) -> Result<(), SlateDBError> {
        self.compactions.refresh().await?;
        self.state
            .merge_remote_compactions(self.compactions.prepare_dirty()?);
        Ok(())
    }

    /// The manifest version this writer last read or wrote.
    pub(crate) fn manifest_id(&self) -> u64 {
        self.manifest.manifest().0
    }

    /// Follows manifest versions written in this process by any component sharing the store.
    pub(crate) fn subscribe_manifest_writes(&self) -> tokio::sync::watch::Receiver<u64> {
        self.manifest.subscribe_writes()
    }

    /// Refreshes compactions first, then manifests, to preserve a consistent ordering.
    ///
    /// ## Returns
    /// - `Ok(())` after state is refreshed, or `SlateDBError` on failure.
    pub(crate) async fn refresh(&mut self) -> Result<(), SlateDBError> {
        self.load_compactions().await?;
        self.load_manifest().await?;
        Ok(())
    }

    /// Persists the updated manifest after a compaction finishes.
    ///
    /// A checkpoint is written first to prevent GC from deleting SSTs that are about
    /// to be removed. Its configured lifetime keeps them available to in-flight
    /// operations such as iterator scans.
    async fn write_manifest(&mut self) -> Result<(), SlateDBError> {
        // write the checkpoint first so that it points to the manifest with the ssts
        // being removed
        let checkpoint_id = self.rand.rng().gen_uuid();
        self.manifest
            .write_checkpoint(
                checkpoint_id,
                &CheckpointOptions {
                    lifetime: Some(self.checkpoint_lifetime),
                    ..CheckpointOptions::default()
                },
            )
            .await?;
        self.state
            .merge_remote_manifest(self.manifest.prepare_dirty()?);
        let dirty = self.state.manifest().clone();
        self.manifest.update(dirty).await
    }

    /// Writes the manifest, retrying on sequenced write conflicts by reloading and retrying.
    ///
    /// ## Returns
    /// - `Ok(())` when the manifest is successfully persisted.
    /// - `SlateDBError` if non-retryable errors occur.
    pub(crate) async fn write_manifest_safely(&mut self) -> Result<(), SlateDBError> {
        loop {
            self.load_manifest().await?;
            match self.write_manifest().await {
                Ok(_) => return Ok(()),
                Err(err) if err.is_sequenced_write_conflict() => {
                    debug!("conflicting manifest version. updating and retrying write again.");
                }
                Err(err) => return Err(err),
            }
        }
    }

    /// Persists the current compactions state to the compactions store and refreshes the
    /// local dirty object with the latest version.
    ///
    /// Process-local state retains every terminal transition until this write succeeds.
    /// Only the outgoing value is trimmed to the latest terminal entry. This preserves
    /// terminal entries as tombstones across conflict retries while keeping the persisted
    /// `.compactions` object bounded.
    ///
    /// ## Returns
    /// - `Ok(())` when compactions are successfully written.
    /// - `SlateDBError` if an unrecoverable error occurs.
    pub(crate) async fn write_compactions_safely(&mut self) -> Result<(), SlateDBError> {
        let mut desired_value = self.state.compactions().value.clone();
        desired_value.retain_active_and_last_finished();
        loop {
            let mut dirty_compactions = self.compactions.prepare_dirty()?;
            dirty_compactions.value = desired_value.clone();
            match self.compactions.update(dirty_compactions).await {
                Ok(()) => {
                    let refreshed = self.compactions.prepare_dirty()?;
                    self.state.set_compactions(refreshed);
                    return Ok(());
                }
                Err(err) if err.is_sequenced_write_conflict() => {
                    // Merge the latest remote state (e.g. a worker's Compacted write) into
                    // the coordinator's untrimmed local view before retrying. Without this,
                    // retrying with a stale desired_value could silently overwrite worker
                    // progress. Local terminal entries also prevent stale remote active states
                    // for the same ids from being resurrected.
                    self.load_compactions().await?;
                    desired_value = self.state.compactions().value.clone();
                    desired_value.retain_active_and_last_finished();
                }
                Err(err) => return Err(err),
            }
        }
    }

    /// Writes manifest first, then compactions, retrying manifest conflicts.
    ///
    /// ## Returns
    /// - `Ok(())` when both manifest and compactions are persisted in order.
    /// - `SlateDBError` on unrecoverable failures.
    pub(crate) async fn write_state_safely(&mut self) -> Result<(), SlateDBError> {
        // Writes are always manifest-first
        self.write_manifest_safely().await?;
        self.write_compactions_safely().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::AdminBuilder;
    use crate::bytes_range::BytesRange;
    use crate::compactions_store::{CompactionsStore, StoredCompactions};
    use crate::compactor_state::{
        Compaction, CompactionContext, CompactionSpec, CompactionStatus, Compactions,
        CompactorState, VersionedCompactions, WorkerSpec,
    };
    use crate::db_state::{SsTableHandle, SsTableId, SsTableInfo};
    use crate::error::SlateDBError;
    use crate::format::sst::SST_FORMAT_VERSION_LATEST;
    use crate::manifest::store::{ManifestStore, StoredManifest};
    use crate::manifest::{ExternalDb, Manifest, ManifestCore, VersionedManifest};
    use crate::subcompaction::Subcompaction;
    use crate::test_utils::GatedObjectStore;
    use bytes::Bytes;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use slatedb_common::clock::{DefaultSystemClock, SystemClock};
    use slatedb_common::MockSystemClock;
    use slatedb_txn_obj::test_utils::new_dirty_object;
    use std::sync::Arc;
    use ulid::Ulid;

    const ROOT: &str = "/compactor-state";

    #[tokio::test]
    async fn test_new_fences_manifest_and_compactions() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let compactions_store = Arc::new(CompactionsStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            system_clock.clone(),
        )
        .await
        .unwrap();

        let options = CompactorOptions::default();
        let rand = Arc::new(DbRand::new(7));

        let mut first_writer = CompactorStateWriter::new(
            manifest_store.clone(),
            compactions_store.clone(),
            system_clock.clone(),
            &options,
            Arc::clone(&rand),
        )
        .await
        .unwrap();

        let _second_writer = CompactorStateWriter::new(
            manifest_store,
            compactions_store,
            system_clock,
            &options,
            rand,
        )
        .await
        .unwrap();

        let manifest_result = first_writer.manifest.refresh().await;
        assert!(matches!(manifest_result, Err(SlateDBError::Fenced)));

        let compactions_result = first_writer.compactions.refresh().await;
        assert!(matches!(compactions_result, Err(SlateDBError::Fenced)));
    }

    #[test]
    fn test_compactor_state_to_view() {
        let mut manifest = Manifest::initial(ManifestCore::new());
        manifest.compactor_epoch = 11;
        Arc::make_mut(&mut manifest.core.tree).last_compacted_l0_sst_view_id =
            Some(Ulid::from_parts(5, 0));

        let mut compactions = Compactions::new(manifest.compactor_epoch);
        let compaction_id = Ulid::from_parts(10, 0);
        let spec = CompactionSpec::new(vec![], 7);
        let compaction =
            Compaction::new(compaction_id, spec.clone()).with_status(CompactionStatus::Running);
        compactions.insert(compaction);

        let manifest_id = 3u64;
        let compactions_id = 8u64;
        let state = CompactorState::new(
            new_dirty_object(manifest_id, manifest.clone()),
            new_dirty_object(compactions_id, compactions.clone()),
        );

        let view = CompactorStateView::from(&state);

        assert_eq!(
            view.manifest,
            VersionedManifest::from_manifest(manifest_id, manifest)
        );

        let view_compactions = view.compactions.expect("missing compactions");
        assert_eq!(
            view_compactions,
            VersionedCompactions::from_compactions(compactions_id, compactions.clone())
        );
        assert_eq!(
            view_compactions.compactions.compactor_epoch,
            compactions.compactor_epoch
        );

        let view_compaction = view_compactions
            .compactions
            .get(&compaction_id)
            .expect("missing compaction");
        assert_eq!(view_compaction.status(), CompactionStatus::Running);
        assert_eq!(view_compaction.spec(), &spec);
    }

    #[tokio::test]
    async fn test_fence_syncs_compactor_epochs() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let compactions_store = Arc::new(CompactionsStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            system_clock.clone(),
        )
        .await
        .unwrap();

        let mut stored_manifest =
            StoredManifest::load(manifest_store.clone(), system_clock.clone())
                .await
                .unwrap();
        let mut dirty_manifest = stored_manifest.prepare_dirty().unwrap();
        dirty_manifest.value.compactor_epoch = 8;
        stored_manifest.update(dirty_manifest).await.unwrap();

        StoredCompactions::create(compactions_store.clone(), 7)
            .await
            .unwrap();

        let options = CompactorOptions::default();
        let rand = Arc::new(DbRand::new(7));

        CompactorStateWriter::new(
            manifest_store.clone(),
            compactions_store.clone(),
            system_clock,
            &options,
            rand,
        )
        .await
        .unwrap();

        let manifest = manifest_store.read_latest_manifest().await.unwrap();
        let compactions = compactions_store.read_latest_compactions().await.unwrap();
        assert_eq!(manifest.manifest.compactor_epoch, 9);
        assert_eq!(
            manifest.manifest.compactor_epoch,
            compactions.compactions.compactor_epoch
        );
    }

    #[tokio::test]
    async fn test_new_resets_scheduled_to_submitted_and_preserves_submitted() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let compactions_store = Arc::new(CompactionsStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            system_clock.clone(),
        )
        .await
        .unwrap();

        // Seed the compactions store with Submitted/Running/Finished entries.
        let mut stored_compactions = StoredCompactions::create(compactions_store.clone(), 0)
            .await
            .unwrap();
        let submitted_id = Ulid::from_parts(1, 0);
        let failed_old_id = Ulid::from_parts(2, 0);
        let completed_old_id = Ulid::from_parts(3, 0);
        let scheduled_id = Ulid::from_parts(4, 0);
        let mut dirty = stored_compactions.prepare_dirty().unwrap();
        dirty.value.insert(Compaction::new(
            submitted_id,
            CompactionSpec::new(vec![], 0),
        ));
        dirty.value.insert(
            Compaction::new(failed_old_id, CompactionSpec::new(vec![], 0))
                .with_status(CompactionStatus::Failed),
        );
        dirty.value.insert(
            Compaction::new(completed_old_id, CompactionSpec::new(vec![], 0))
                .with_status(CompactionStatus::Completed),
        );
        dirty.value.insert(
            Compaction::new(scheduled_id, CompactionSpec::new(vec![], 0))
                .with_status(CompactionStatus::Scheduled),
        );
        stored_compactions.update(dirty).await.unwrap();

        // Initialize a new writer (restart) which should flip Scheduled -> Submitted and trim.
        let options = CompactorOptions::default();
        let rand = Arc::new(DbRand::new(7));

        let writer = CompactorStateWriter::new(
            manifest_store,
            compactions_store,
            system_clock,
            &options,
            rand,
        )
        .await
        .unwrap();

        // Submitted should remain; Scheduled should become Submitted; older finished should be trimmed.
        let compactions = &writer.state.compactions().value;
        assert_eq!(
            compactions
                .get(&submitted_id)
                .expect("missing submitted compaction")
                .status(),
            CompactionStatus::Submitted
        );
        assert_eq!(
            compactions
                .get(&scheduled_id)
                .expect("missing scheduled compaction")
                .status(),
            CompactionStatus::Submitted
        );
        assert!(!compactions.contains(&failed_old_id));
        assert!(compactions.contains(&completed_old_id));
    }

    #[tokio::test]
    async fn test_new_preserves_running_output_ssts() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let compactions_store = Arc::new(CompactionsStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            system_clock.clone(),
        )
        .await
        .unwrap();

        let output_ssts = vec![
            SsTableHandle::new(
                SsTableId::from(Ulid::from_parts(10, 0)),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo {
                    first_entry: Some(Bytes::copy_from_slice(b"a")),
                    ..Default::default()
                },
            ),
            SsTableHandle::new(
                SsTableId::from(Ulid::from_parts(11, 0)),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo {
                    first_entry: Some(Bytes::copy_from_slice(b"m")),
                    ..Default::default()
                },
            ),
        ];
        let job_ctx = Some(CompactionContext::new(
            vec![Subcompaction::new(BytesRange::unbounded()).with_output_ssts(output_ssts.clone())],
            Some(0),
        ));

        let mut stored_compactions = StoredCompactions::create(compactions_store.clone(), 0)
            .await
            .unwrap();
        let running_id = Ulid::from_parts(4, 0);
        let mut dirty = stored_compactions.prepare_dirty().unwrap();
        dirty.value.insert(
            Compaction::new(running_id, CompactionSpec::new(vec![], 0))
                .with_status(CompactionStatus::Running)
                .with_ctx(job_ctx.clone()),
        );
        stored_compactions.update(dirty).await.unwrap();

        let options = CompactorOptions::default();
        let rand = Arc::new(DbRand::new(7));

        let writer = CompactorStateWriter::new(
            manifest_store,
            compactions_store,
            system_clock,
            &options,
            rand,
        )
        .await
        .unwrap();

        let compaction = writer
            .state
            .compactions()
            .value
            .get(&running_id)
            .expect("missing running compaction");
        // new() leaves Running entries untouched and preserves their output
        // through the load; reclaiming stale ones is the ticker's job.
        assert_eq!(compaction.status(), CompactionStatus::Running);
        assert_eq!(compaction.ctx(), job_ctx.as_ref());
    }

    /// `CompactorStateWriter::new` no longer reclaims stale `Running`
    /// compactions on restart — that moved to `reclaim_stale_workers`, which
    /// runs on the first tick (covered by `test_reclaim_stale_running_compaction`
    /// and `test_does_not_reclaim_fresh_running_compaction` in compactor.rs). So
    /// `new` must leave every `Running` compaction untouched, stale or fresh, and
    /// only reset `Scheduled` entries.
    #[tokio::test]
    async fn test_new_preserves_running_compactions() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let compactions_store = Arc::new(CompactionsStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));

        // Fix the clock so heartbeat ages are deterministic.
        let mock_clock = Arc::new(MockSystemClock::new());
        let now_ms: u64 = 100_000;
        mock_clock.set(now_ms as i64);
        let system_clock: Arc<dyn SystemClock> = mock_clock.clone();

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            system_clock.clone(),
        )
        .await
        .unwrap();

        let options = CompactorOptions::default();
        let timeout_ms = options.worker_heartbeat_timeout.as_millis() as u64;

        // fresh: heartbeat at "now" (age 0). stale: heartbeat well past timeout.
        let fresh_id = Ulid::from_parts(1, 0);
        let stale_id = Ulid::from_parts(2, 0);
        let stale_hb_ms = now_ms - (timeout_ms * 2);

        let mut stored_compactions = StoredCompactions::create(compactions_store.clone(), 0)
            .await
            .unwrap();
        let mut dirty = stored_compactions.prepare_dirty().unwrap();
        dirty.value.insert(
            Compaction::new(fresh_id, CompactionSpec::new(vec![], 0))
                .with_status(CompactionStatus::Running)
                .with_worker(Some(WorkerSpec::new("worker-fresh".to_string(), now_ms))),
        );
        dirty.value.insert(
            Compaction::new(stale_id, CompactionSpec::new(vec![], 0))
                .with_status(CompactionStatus::Running)
                .with_worker(Some(WorkerSpec::new(
                    "worker-stale".to_string(),
                    stale_hb_ms,
                ))),
        );
        stored_compactions.update(dirty).await.unwrap();

        let rand = Arc::new(DbRand::new(7));
        let writer = CompactorStateWriter::new(
            manifest_store,
            compactions_store,
            system_clock,
            &options,
            rand,
        )
        .await
        .unwrap();

        let compactions = &writer.state.compactions().value;

        // new() leaves the live worker undisturbed.
        let fresh = compactions
            .get(&fresh_id)
            .expect("missing fresh compaction");
        assert_eq!(fresh.status(), CompactionStatus::Running);
        assert_eq!(
            fresh.worker().map(|w| w.worker_id.as_str()),
            Some("worker-fresh")
        );

        // new() also leaves the stale worker in Running; reclaiming it is the
        // ticker's job (reclaim_stale_workers), not the writer's at startup.
        let stale = compactions
            .get(&stale_id)
            .expect("missing stale compaction");
        assert_eq!(stale.status(), CompactionStatus::Running);
        assert_eq!(
            stale.worker().map(|w| w.worker_id.as_str()),
            Some("worker-stale")
        );
    }

    #[tokio::test]
    async fn test_load_compactions_merges_external_submitted() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let compactions_store = Arc::new(CompactionsStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            system_clock.clone(),
        )
        .await
        .unwrap();

        let options = CompactorOptions::default();
        let rand = Arc::new(DbRand::new(7));
        let mut writer = CompactorStateWriter::new(
            manifest_store,
            compactions_store.clone(),
            system_clock,
            &options,
            rand,
        )
        .await
        .unwrap();

        let submitted_id = Ulid::from_parts(10, 0);
        let mut external = StoredCompactions::load(compactions_store).await.unwrap();
        let mut dirty = external.prepare_dirty().unwrap();
        dirty.value.insert(Compaction::new(
            submitted_id,
            CompactionSpec::new(vec![], 0),
        ));
        external.update(dirty).await.unwrap();

        writer.load_compactions().await.unwrap();

        let compactions = &writer.state.compactions().value;
        assert_eq!(
            compactions
                .get(&submitted_id)
                .expect("missing submitted compaction")
                .status(),
            CompactionStatus::Submitted
        );
    }

    #[tokio::test]
    async fn test_write_compactions_safely_retries_on_version_conflict() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let compactions_store = Arc::new(CompactionsStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            system_clock.clone(),
        )
        .await
        .unwrap();

        let options = CompactorOptions::default();
        let rand = Arc::new(DbRand::new(7));

        let mut writer = CompactorStateWriter::new(
            manifest_store,
            compactions_store.clone(),
            system_clock,
            &options,
            rand,
        )
        .await
        .unwrap();

        // Record the version after fencing.
        let start_id = compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .id;

        // Simulate an external writer racing and creating the next version.
        let mut external = StoredCompactions::load(compactions_store.clone())
            .await
            .unwrap();
        external
            .update(external.prepare_dirty().unwrap())
            .await
            .unwrap();

        let conflicting_id = compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .id;
        assert_eq!(conflicting_id, start_id + 1);

        // This should retry on conflict and succeed with a new version.
        writer.write_compactions_safely().await.unwrap();

        let final_id = compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .id;
        assert_eq!(final_id, start_id + 2);
    }

    #[tokio::test]
    async fn test_write_compactions_safely_does_not_resurrect_terminal_on_conflict() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let compactions_store = Arc::new(CompactionsStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            system_clock.clone(),
        )
        .await
        .unwrap();

        let mut writer = CompactorStateWriter::new(
            manifest_store,
            compactions_store.clone(),
            system_clock,
            &CompactorOptions::default(),
            Arc::new(DbRand::new(7)),
        )
        .await
        .unwrap();

        let completed_id = Ulid::from_parts(10, 0);
        let failed_id = Ulid::from_parts(20, 0);
        let spec = CompactionSpec::new(vec![], 0);
        writer
            .state
            .insert_compaction_for_test(Compaction::new(completed_id, spec.clone()));
        writer
            .state
            .insert_compaction_for_test(Compaction::new(failed_id, spec.clone()));
        writer.state.update_compaction(&completed_id, |compaction| {
            compaction.set_status(CompactionStatus::Completed)
        });
        writer.state.update_compaction(&failed_id, |compaction| {
            compaction.set_status(CompactionStatus::Failed)
        });

        // Advance the remote version with the pre-transition state of the completed
        // compaction. This forces the coordinator's first write to conflict and reload.
        let mut external = StoredCompactions::load(compactions_store.clone())
            .await
            .unwrap();
        let mut dirty = external.prepare_dirty().unwrap();
        dirty.value.insert(Compaction::new(completed_id, spec));
        external.update(dirty).await.unwrap();

        writer.write_compactions_safely().await.unwrap();

        let persisted = compactions_store.read_latest_compactions().await.unwrap();
        assert!(
            persisted
                .recent_compactions()
                .all(|compaction| compaction.id() != completed_id),
            "stale Submitted compaction was resurrected"
        );
        assert_eq!(
            persisted
                .recent_compactions()
                .find(|compaction| compaction.id() == failed_id)
                .expect("latest terminal compaction was not retained")
                .status(),
            CompactionStatus::Failed
        );
        assert_eq!(writer.state.active_compactions().count(), 0);
    }

    #[tokio::test]
    async fn test_write_compactions_safely_retries_on_boundary_conflict() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let compactions_store = Arc::new(CompactionsStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            system_clock.clone(),
        )
        .await
        .unwrap();

        let options = CompactorOptions::default();
        let rand = Arc::new(DbRand::new(7));

        let mut writer = CompactorStateWriter::new(
            manifest_store,
            compactions_store.clone(),
            system_clock,
            &options,
            rand,
        )
        .await
        .unwrap();

        // The writer starts with a stale compactions view. An external writer then creates
        // a live newer version before GC deletes and fences the stale writer's next id.
        let start_id = compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .id;

        // The external writer wins the stale writer's intended id.
        let mut external = StoredCompactions::load(compactions_store.clone())
            .await
            .unwrap();
        external
            .update(external.prepare_dirty().unwrap())
            .await
            .unwrap();
        assert_eq!(external.id(), start_id + 1);

        // A second external write leaves a live latest version above the future boundary,
        // so the stale writer can make progress after refreshing.
        external
            .update(external.prepare_dirty().unwrap())
            .await
            .unwrap();
        assert_eq!(external.id(), start_id + 2);

        // GC fences and removes the stale writer's next id, but preserves the live latest
        // version at start_id + 2.
        compactions_store
            .advance_boundary(start_id + 1)
            .await
            .unwrap();
        compactions_store
            .delete_compactions(start_id + 1)
            .await
            .unwrap();
        let latest = compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .id;
        assert_eq!(latest, start_id + 2);

        writer.write_compactions_safely().await.unwrap();

        let final_id = compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .id;
        assert_eq!(final_id, start_id + 3);
    }

    #[tokio::test]
    async fn test_write_manifest_uses_configured_checkpoint_lifetime() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let compactions_store = Arc::new(CompactionsStore::new(&Path::from(ROOT), object_store));
        let clock = Arc::new(MockSystemClock::with_time(1_000));
        let system_clock: Arc<dyn SystemClock> = clock.clone();

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            system_clock.clone(),
        )
        .await
        .unwrap();

        let checkpoint_lifetime = Duration::from_secs(42);
        let options = CompactorOptions {
            checkpoint_lifetime,
            ..CompactorOptions::default()
        };
        let mut writer = CompactorStateWriter::new(
            manifest_store.clone(),
            compactions_store,
            system_clock.clone(),
            &options,
            Arc::new(DbRand::new(7)),
        )
        .await
        .unwrap();

        writer.write_manifest_safely().await.unwrap();

        let latest = manifest_store.read_latest_manifest().await.unwrap();
        let checkpoint = latest
            .manifest
            .core
            .checkpoints
            .last()
            .expect("missing compactor checkpoint");
        assert_eq!(
            Some(system_clock.now() + checkpoint_lifetime),
            checkpoint.expire_time
        );
    }

    #[tokio::test]
    async fn write_manifest_safely_retries_on_version_conflict() {
        let inner_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let gated_store = Arc::new(GatedObjectStore::new(Arc::clone(&inner_store)));
        let object_store: Arc<dyn ObjectStore> = gated_store.clone();
        let manifest_store = Arc::new(ManifestStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let compactions_store = Arc::new(CompactionsStore::new(
            &Path::from(ROOT),
            Arc::clone(&object_store),
        ));
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());

        let mut stored_manifest = StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            system_clock.clone(),
        )
        .await
        .unwrap();
        let stale_sst_id = SsTableId::from(Ulid::new());
        let source_checkpoint_id = uuid::Uuid::new_v4();
        let final_checkpoint_id = uuid::Uuid::new_v4();
        let mut dirty = stored_manifest.prepare_dirty().unwrap();
        dirty.value.external_dbs = vec![ExternalDb {
            path: "/parent/db".to_string(),
            source_checkpoint_id,
            final_checkpoint_id: Some(final_checkpoint_id),
            sst_ids: vec![stale_sst_id],
        }];
        stored_manifest.update(dirty).await.unwrap();

        let options = CompactorOptions::default();
        let rand = Arc::new(DbRand::new(7));

        let mut writer = CompactorStateWriter::new(
            manifest_store.clone(),
            compactions_store.clone(),
            system_clock,
            &options,
            rand,
        )
        .await
        .unwrap();

        // Record the version after fencing.
        let start_id = manifest_store.read_latest_manifest().await.unwrap().id;

        // Allow write_manifest's checkpoint through, then block its manifest update.
        // The safe path has already loaded and pruned local state at that boundary.
        let baseline_puts = gated_store.put_opts_gate.arrivals();
        gated_store.put_opts_gate.close();
        gated_store.put_opts_gate.admit(1);
        let write_task = tokio::spawn(async move { writer.write_manifest_safely().await });
        gated_store
            .put_opts_gate
            .wait_for_arrivals(baseline_puts + 2)
            .await;

        // Race a checkpoint into the exact version intended by the blocked update.
        let admin = AdminBuilder::new(ROOT, inner_store).build();
        let remote_checkpoint = admin
            .create_detached_checkpoint(&CheckpointOptions::default())
            .await
            .expect("create checkpoint failed");

        let conflicting_id = manifest_store.read_latest_manifest().await.unwrap().id;
        assert_eq!(conflicting_id, start_id + 2);

        // Reloading and retrying must neither resurrect the pruned SST nor lose
        // checkpoint metadata from either the external DB or the racing writer.
        gated_store.put_opts_gate.release();
        write_task.await.unwrap().unwrap();

        let final_manifest = manifest_store.read_latest_manifest().await.unwrap();
        let external = &final_manifest.manifest.external_dbs[0];
        assert!(external.sst_ids.is_empty());
        assert_eq!(external.source_checkpoint_id, source_checkpoint_id);
        assert_eq!(external.final_checkpoint_id, Some(final_checkpoint_id));
        assert!(final_manifest
            .manifest
            .core
            .checkpoints
            .iter()
            .any(|checkpoint| checkpoint.id == remote_checkpoint.id));

        let final_id = final_manifest.id;
        // write_manifest_safely now bumps the manifest twice per successful call because write_manifest
        // writes a checkpoint first:
        // - write_manifest() calls self.manifest.write_checkpoint(...) to create the checkpoint, then
        // - write_manifest() calls self.manifest.update(...) to update the manifest
        // So we do +1 for the first checkpoint, +1 for the external update, and +2 for
        // the successful retry.
        assert_eq!(final_id, start_id + 4);
    }
}
