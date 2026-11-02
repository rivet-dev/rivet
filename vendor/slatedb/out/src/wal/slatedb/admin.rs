use crate::format::sst::SsTableFormat;
use crate::garbage_collector::stats::GcStats;
use crate::paths::PathResolver;
use crate::tablestore::TableStoreKind;
use crate::wal::slatedb::gc::{SlateDbWalGc, WalGcMode};
use crate::wal::slatedb::store::{WalFileId, WalTableStore};
use crate::wal::{WalAdmin, WalError, WalGc};
use crate::VersionedManifest;
use async_trait::async_trait;
use fail_parallel::{fail_point, FailPointRegistry};
use futures::StreamExt;
use object_store::path::Path;
use object_store::{ObjectStore, ObjectStoreExt};
use slatedb_common::clock::DefaultSystemClock;
use slatedb_common::metrics::MetricsRecorderHelper;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SlateDbWalAdmin {
    object_store: Arc<dyn ObjectStore>,
    #[cfg_attr(not(test), allow(dead_code))]
    fp_registry: Arc<FailPointRegistry>,
}

impl SlateDbWalAdmin {
    pub(crate) fn new(
        object_store: Arc<dyn ObjectStore>,
        fp_registry: Arc<FailPointRegistry>,
    ) -> Self {
        Self {
            object_store,
            fp_registry,
        }
    }

    fn replay_range(manifest: &VersionedManifest) -> Result<(u64, u64), WalError> {
        let replay_after_wal_id = manifest.replay_after_wal_id();
        let wal_id_last_seen = manifest
            .next_wal_sst_id()
            .checked_sub(1)
            .ok_or_else(Self::invalid_manifest)?;
        Ok((replay_after_wal_id, wal_id_last_seen))
    }

    fn invalid_manifest() -> WalError {
        WalError::InternalError(Arc::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "source manifest must have a positive next WAL file ID",
        )))
    }

    fn has_wal_file_ids(replay_after_wal_id: u64, wal_id_last_seen: u64) -> bool {
        wal_id_last_seen > replay_after_wal_id
    }

    async fn paths_under(&self, path: &Path) -> Result<Vec<Path>, WalError> {
        let mut objects = self.object_store.list(Some(path));
        let mut paths = Vec::new();
        while let Some(object) = objects.next().await {
            let object = object.map_err(|err| WalError::Unavailable(Arc::new(err)))?;
            paths.push(object.location);
        }
        Ok(paths)
    }
}

#[async_trait]
impl WalAdmin for SlateDbWalAdmin {
    fn garbage_collector(&self, path: &Path) -> Arc<dyn WalGc> {
        let wal_store = Arc::new(WalTableStore::new(
            self.object_store.clone(),
            SsTableFormat::default(),
            path.clone(),
            TableStoreKind::GC,
        ));
        Arc::new(SlateDbWalGc::new(
            wal_store,
            Arc::new(GcStats::new(&MetricsRecorderHelper::noop())),
            WalGcMode::Regular,
            None,
            Arc::new(DefaultSystemClock::new()),
        ))
    }

    async fn delete_wal(&self, path: &Path, dry_run: bool) -> Result<Vec<String>, WalError> {
        // Collect the paths first so listing is complete before objects are removed.
        let wal_path = PathResolver::from_root(path.clone()).wal_path();
        let paths = self.paths_under(&wal_path).await?;
        if !dry_run {
            for object_path in &paths {
                self.object_store
                    .delete(object_path)
                    .await
                    .map_err(|err| WalError::Unavailable(Arc::new(err)))?;
            }
        }
        Ok(paths.iter().map(Path::to_string).collect())
    }

    async fn is_empty(
        &self,
        path: &Path,
        replay_after_wal_id: u64,
        wal_id_last_seen: u64,
    ) -> Result<bool, WalError> {
        // Avoid object-store requests when the manifest's WAL range contains no file IDs.
        if !Self::has_wal_file_ids(replay_after_wal_id, wal_id_last_seen) {
            return Ok(true);
        }

        let path_resolver = PathResolver::from_root(path.clone());
        for wal_id in (replay_after_wal_id + 1)..=wal_id_last_seen {
            let path = path_resolver.wal_sst_path(&WalFileId::from(wal_id));
            let metadata = self
                .object_store
                .head(&path)
                .await
                .map_err(|err| WalError::Unavailable(Arc::new(err)))?;

            // Native SlateDB WAL fences are zero-byte WAL objects and contain no records.
            if metadata.size > 0 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn clone_wal(
        &self,
        from_path: &Path,
        from_manifest: VersionedManifest,
        to_path: &Path,
    ) -> Result<(u64, u64), WalError> {
        let (replay_after_wal_id, wal_id_last_seen) = Self::replay_range(&from_manifest)?;
        let from_path_resolver = PathResolver::from_root(from_path.clone());
        let to_path_resolver = PathResolver::from_root(to_path.clone());

        if Self::has_wal_file_ids(replay_after_wal_id, wal_id_last_seen) {
            for wal_id in (replay_after_wal_id + 1)..=wal_id_last_seen {
                fail_point!(self.fp_registry.clone(), "copy-wal-ssts-io-error", |_| Err(
                    WalError::Unavailable(Arc::new(std::io::Error::other("oops")))
                ));

                let id = WalFileId::from(wal_id);
                let source = from_path_resolver.wal_sst_path(&id);
                let destination = to_path_resolver.wal_sst_path(&id);
                self.object_store
                    .as_ref()
                    .copy(&source, &destination)
                    .await
                    .map_err(|err| WalError::Unavailable(Arc::new(err)))?;
            }
        }

        Ok((replay_after_wal_id, wal_id_last_seen))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use object_store::memory::InMemory;

    #[tokio::test]
    async fn delete_wal_deletes_only_objects_under_the_wal_prefix() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let wal_admin =
            SlateDbWalAdmin::new(object_store.clone(), Arc::new(FailPointRegistry::new()));
        let db_path = Path::from("db");
        let wal_object = PathResolver::from_root(db_path.clone()).wal_sst_path(&WalFileId::from(1));
        let non_wal_object = db_path
            .clone()
            .join("manifest")
            .join("00000000000000000001");
        let sibling_object = Path::from("other/wal/00000000000000000002.sst");
        object_store
            .put(&wal_object, Bytes::from_static(b"wal").into())
            .await
            .unwrap();
        object_store
            .put(&non_wal_object, Bytes::from_static(b"keep").into())
            .await
            .unwrap();
        object_store
            .put(&sibling_object, Bytes::from_static(b"keep").into())
            .await
            .unwrap();

        wal_admin.delete_wal(&db_path, false).await.unwrap();

        assert!(matches!(
            object_store.head(&wal_object).await,
            Err(object_store::Error::NotFound { .. })
        ));
        assert!(object_store.head(&non_wal_object).await.is_ok());
        assert!(object_store.head(&sibling_object).await.is_ok());
    }

    #[test]
    fn creates_a_path_scoped_garbage_collector() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let wal_admin = SlateDbWalAdmin::new(object_store, Arc::new(FailPointRegistry::new()));

        let _collector = wal_admin.garbage_collector(&Path::from("db"));
    }
}
